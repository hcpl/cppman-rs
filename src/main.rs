extern crate either;
extern crate ini;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate select;
extern crate rusqlite;
extern crate isatty;
extern crate flate2;

mod config;
mod crawler;
mod environ;
mod util;

use std::borrow::Borrow;
use std::cell::Cell;
use std::error;
use std::fs::{self, File};
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use flate2::Compression;
use flate2::write::GzEncoder;
use isatty::stdout_isatty;
use regex::Regex;
use rusqlite::Connection;
//use url::Url;

use crawler::Crawler;
use environ::Environ;


pub fn get_lib_path(s: &str) -> PathBuf {
    PathBuf::from(s)
}


lazy_static! {
    static ref H1_INNER_HTML: Regex = Regex::new("<h1[^>]*>(.+?)</h1>").unwrap();
    static ref TAG: Regex = Regex::new("<([^>]+)>").unwrap();
    static ref GREATER_THAN: Regex = Regex::new("&gt;").unwrap();
    static ref LESSER_THAN: Regex = Regex::new("&lt;").unwrap();

    static ref OPERATOR: Regex = Regex::new("^\\s*(.*?::(?:operator)?)([^:]*)\\s*$").unwrap();
}


struct Cppman {
    crawler: Crawler,
    results: HashSet<(String, Url)>,
    forced: bool,
    success_count: Cell<Option<u32>>,
    failure_count: Cell<Option<u32>>,
    force_columns: Option<usize>,

    blacklist: Vec<Url>,
    name_exceptions: Vec<String>,
    env: Environ,

    db_conn: Connection,
}

impl Cppman {
    fn new_default(env: &Environ) -> Cppman {
        Cppman::new(false, None, env)
    }

    fn new(forced: Option<bool>, force_columns: Option<usize>, env: &Environ) -> Cppman {
        Cppman {
            crawler: Crawler::new(),
            results: HashSet::new(),
            forced: forced.unwrap_or(false),
            success_count: None,
            failure_count: None,
            force_columns: force_columns,

            blacklist: Vec::new(),
            name_exceptions: vec!["http://www.cplusplus.com/reference/string/swap/".to_owned()],
            env: env.clone(),
        }
    }

    /// Extract man page name from web page.
    fn extract_name(&self, data: &str) -> io::Result<String> {
        H1_INNER_HTML.captures(data)
            .ok_or(new_io_error("No captures found at all"))
            .and_then(|cap| {
                cap.get(1).map(|m| {
                    let mut name = m.as_str();
                    name = TAG.replace(name, "").borrow();
                    name = GREATER_THAN.replace(name, ">").borrow();
                    name = LESSER_THAN.replace(name, ">").borrow();
                    name
                }).ok_or(new_io_error("No capture #1 found"))
            })
    }

    /// Rebuild index database from cplusplus.com and cppreference.com.
    fn rebuild_index(&self) {
        let _ = fs::remove_file(self.env.index_db_re);

        unimplemented!();
    }

    /// callback to insert index
    fn process_document(&self, doc: Document) {
        if !self.blacklist.contains(doc.url) {
            println!("Indexing '{}' ...", doc.url);
            let name = self.extract_name(doc.text);
            self.results.insert((name, doc.url));
        } else {
            println!("Skipping blacklisted page '{}' ...", doc.url);
        }
    }

    /// callback to insert index
    fn insert_index(&self, table: &str, name: &str, url: &str) -> io::Result<()> {
        let mut names = name.split(',').collect::<Vec<_>>();

        if names.len() > 1 {
            if let Some(caps) = OPERATOR.captures(names[0]) {
                let prefix = try!(caps.get(1).ok_or(new_io_error("No such capture $1"))).as_str();
                names[0] = try!(caps.get(2).ok_or(new_io_error("No such capture $2"))).as_str();
                names = names.into_iter().map(|n| prefix + n).collect::<Vec<_>>();
            }
        }

        for n in names {
            try!(self.db_conn.execute(
                format!("INSERT INTO \"{}\" (name, url) VALUES (?, ?)", table), &[n.trim(), url])
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e)));
        }
    }

    /// Cache all available man pages
    fn cache_all(&self) -> io::Result<()> {
        println!("By default, cppman fetches pages on-the-fly if corresponding \
                  page is not found in the cache. The \"cache-all\" option is only \
                  useful if you want to view man pages offline. \
                  Caching all contents will take several minutes, \
                  do you want to continue [y/N]?");

        let mut respond = String::new();
        try!(io::stdin().read_line(&mut respond));
        if !["y", "ye", "yes"].contains(&&respond.to_lowercase()) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, ""));
        }

        try!(fs::create_dir_all());

        self.success_count = Some(0);
        self.failure_count = Some(0);

        if !self.env.index_db.exists() {
            return new_io_error("can't find index.db");
        }

        {
            let conn = Connection::open(self.env.index_db);

            let source = self.env.config.source;
            println!("Caching manpages from {} ...", source);
            let stmt = try!(conn.prepare(format!("SELECT * FROM \"{}\"", source)).map_err(new_io_error));
            let data = try!(stmt.query_map(&[], |&row| {
                Ok((try!(row.get_checked(0).map_err(new_io_error)),
                    try!(row.get_checked(1).map_err(new_io_error))))
            }));

            for Ok((name, url)) in data {
                let retries = 3;
                println!("Caching {} ...", name);
                while retries > 0 {
                    match self.cache_man_page(source, url, name) {
                        Ok(_)  => break,
                        Err(_) => {
                            println!("Retrying ...");
                            retries -= 1;
                        },
                    }
                }

                if retries == 0 {
                    println!("Error caching {} ...", name);
                    self.failure_count += 1;
                } else {
                    self.success_count += 1;
                }
            }
        }

        println!("\n{} manual pages cached successfully.", self.success_count);
        println!("{} manual pages failed to cache.", self.failure_count);
        self.update_mandb(Some(false))
    }

    /// callback to cache new man page
    fn cache_man_page(&self, source: &str, url: &str, name: &str) -> io::Result<()> {
        unimplemented!();
        // Skip if already exists, override if forced flag is true
        let outname = self.get_page_path(source, name);
        if outname.exists() && !self.forced {
            return Ok(());
        }

        try!(fs::create_dir_all(environ.man_dir.join(source)));

        // There are often some errors in the HTML, for example: missing closing
        // tag. We use fixupHTML to fix this.
        let mut data = String::new();
        let resp = try!(reqwest::get(url).map_err(new_io_error));
        try!(resp.read_to_string(&mut data));

        let html2groff;

        match source[..-4] {
            "cplusplus"    => html2groff = ::formatter::cplusplus::html2groff,
            "cppreference" => html2groff = ::formatter::cppreference::html2groff,
            _ => Err(new_io_error("wrong source")),
        }

        let groff_text = html2groff(data, name);

        let mut file = try!(File::create(outname));
        let mut enc = GzEncoder::new(file, Compression::Default);
        try!(enc.write_all(groff_text.as_bytes()));
        try!(enc.finish());
    }

    /// Clear all cache in man3
    fn clear_cache(&self) -> io::Result<()> {
        fs::remove_dir_all(self.env.man_dir)
    }

    /// Call viewer.sh to view man page
    fn man(&self, pattern: &str) {
        unimplemented!();
    }

    /// Find pages in database.
    fn find(&self, pattern: &str) -> io::Result<()> {
        if !self.env.index_db.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "can't find index.db"));
        }

        let conn = try!(Connection::open(self.env.index_db)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e)));
        let stmt = try!(conn.prepare(&format!(
            "SELECT * FROM \"{}\" WHERE name \
             LIKE \"%{}%\" ORDER BY LENGTH(name)",
            self.env.source, pattern)));
        let selected = try!(stmt.query_map(&[], |&row| {
            Ok((try!(row.get_checked(0).map_err(new_io_error)),
                try!(row.get_checked(1).map_err(new_io_error))))
        })).collect::<Vec<_>>();

        let pat = try!(Regex::new(&format!("(?i)\\({}\\)", pattern))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e)));

        if selected.len() > 0 {
            for Ok((name, url)) in selected {
                if stdout_isatty() {
                    println!("{}", pat.replace(name, "\\033[1;31m$1\\033[0m"));
                } else {
                    println!("{}", name);
                }
            }

            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, format!("{}: nothing appropriate", pattern)))
        }
    }

    /// Update mandb.
    fn update_mandb(&self, quiet: Option<bool>) -> io::Result<()> {
        let quiet = quiet.unwrap_or(true);

        if !self.env.config.update_man_path() {
            return Ok(());
        }

        println!("\nrunning mandb...");
        let cmd = format!("mandb {}", if quiet { "-q" } else { "" });
        Command::new("mandb")
                .args(if quiet { &["-q"] } else { &[] })
                .status()
                .map(|_| ())
    }

    fn get_page_path(&self, source: &str, name: &str) -> PathBuf {
        let name = get_normalized_page_name(name);
        let mut path = PathBuf::from(self.env.man_dir);
        path.push(source);
        path.push(name + ".3.gz");
        path
    }
}

fn get_normalized_page_name(name: &str) -> String {
    name.replace("/", "_")
}


fn new_io_error<E>(error: E) -> io::Error
        where E: Into<Box<error::Error + Send + Sync>> {
    io::Error::new(io::ErrorKind::Other, error)
}


fn main() {
    println!("Hello, world!");
}
