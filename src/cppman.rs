use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::ops::AddAssign;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use flate2::Compression;
use flate2::write::GzEncoder;
use isatty::stdout_isatty;
use regex::Regex;
use reqwest;
use rusqlite::{self, Connection};
use url::Url;

use crawler::{Crawler, Document};
use environ::Environ;
use util::{new_io_error, get_width};


lazy_static! {
    static ref H1_INNER_HTML: Regex = Regex::new("<h1[^>]*>(.+?)</h1>").unwrap();
    static ref TAG: Regex = Regex::new("<([^>]+)>").unwrap();
    static ref GREATER_THAN: Regex = Regex::new("&gt;").unwrap();
    static ref LESSER_THAN: Regex = Regex::new("&lt;").unwrap();

    static ref OPERATOR: Regex = Regex::new("^\\s*(.*?::(?:operator)?)([^:]*)\\s*$").unwrap();
}


pub struct Cppman {
    crawler: Crawler,
    results: HashSet<(String, Url)>,
    forced: bool,
    success_count: Cell<Option<u32>>,
    failure_count: Cell<Option<u32>>,
    force_columns: Option<usize>,

    blacklist: Vec<Url>,
    name_exceptions: Vec<String>,
    env: Environ,

    db_conn: Option<RefCell<Connection>>,
}

impl Cppman {
    pub fn new_default(env: &Environ) -> Cppman {
        Cppman::new(Some(false), None, env)
    }

    pub fn new(forced: Option<bool>, force_columns: Option<usize>, env: &Environ) -> Cppman {
        Cppman {
            crawler: Crawler::new(),
            results: HashSet::new(),
            forced: forced.unwrap_or(false),
            success_count: Cell::new(None),
            failure_count: Cell::new(None),
            force_columns: force_columns,

            blacklist: Vec::new(),
            name_exceptions: vec!["http://www.cplusplus.com/reference/string/swap/".to_owned()],
            env: env.clone(),

            db_conn: None,
        }
    }

    /// Extract man page name from web page.
    fn extract_name(&self, data: &str) -> io::Result<String> {
        H1_INNER_HTML.captures(data)
            .ok_or(new_io_error("No captures found at all"))
            .and_then(|cap| {
                cap.get(1).map(|m| {
                    let mut name = m.as_str().to_owned();
                    name = TAG.replace(&name, "").into_owned();
                    name = GREATER_THAN.replace(&name, ">").into_owned();
                    name = LESSER_THAN.replace(&name, ">").into_owned();
                    name
                }).ok_or(new_io_error("No capture #1 found"))
            })
    }

    /// Rebuild index database from cplusplus.com and cppreference.com.
    pub fn rebuild_index(&self) {
        // Continue even if it fails
        let _ = fs::remove_file(&self.env.index_db_re);

        /*let db_conn = try!(Connection::open(self.env.index_db_re).map_err(new_io_error));
        self.db_conn.set(Some(db_conn));

        db_conn.execute("CREATE TABLE \"cplusplus.com\" \
                         (name VARCHAR(255), url VARCHAR(255))")
        db_conn.execute("CREATE TABLE \"cppreference.com\" \
                         (name VARCHAR(255), url VARCHAR(255))")*/

        unimplemented!();
    }

    /// callback to insert index
    fn process_document(&mut self, doc: Document) -> io::Result<()> {
        if !self.blacklist.contains(&doc.url) {
            println!("Indexing '{}' ...", doc.url);
            let name = try!(self.extract_name(&doc.text));
            self.results.insert((name, doc.url));
        } else {
            println!("Skipping blacklisted page '{}' ...", doc.url);
        }

        Ok(())
    }

    /// callback to insert index
    fn insert_index(&self, table: &str, name: &str, url: &str) -> io::Result<()> {
        let mut names = name.split(',').map(str::to_owned).collect::<Vec<_>>();

        if names.len() > 1 {
            if let Some(caps) = OPERATOR.captures(&names[0].clone()) {
                let prefix = try!(caps.get(1).ok_or(new_io_error("No capture $1"))).as_str().to_owned();
                names[0] = try!(caps.get(2).ok_or(new_io_error("No capture $2"))).as_str().to_owned();
                names = names.into_iter().map(|n| prefix.to_owned() + &n).collect::<Vec<_>>();
            }
        }

        for n in names {
            match self.db_conn {
                Some(ref db_conn) => {
                    try!(db_conn.borrow().execute(&format!(
                        "INSERT INTO \"{}\" (name, url) VALUES (?, ?)", table), &[&n.trim(), &url])
                        .map_err(new_io_error));
                },
                None => return Err(new_io_error("No Cppman::dn_conn available!")),
            }
        }

        Ok(())
    }

    /// Cache all available man pages
    pub fn cache_all(&self) -> io::Result<()> {
        println!("By default, cppman-rs fetches pages on-the-fly if corresponding \
                  page is not found in the cache. The \"cache-all\" option is only \
                  useful if you want to view man pages offline. \
                  Caching all contents will take several minutes, \
                  do you want to continue [y/N]?");

        let mut respond = String::new();
        try!(io::stdin().read_line(&mut respond));
        if !["y", "ye", "yes"].contains(&respond.trim().to_lowercase().as_str()) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Not a positive answer"));
        }

        try!(fs::create_dir_all(&self.env.man_dir));

        self.success_count.set(Some(0));
        self.failure_count.set(Some(0));

        if !self.env.index_db.exists() {
            return Err(new_io_error("can't find index.db"));
        }

        {
            let conn = try!(Connection::open(&self.env.index_db).map_err(new_io_error));

            let source = self.env.config.source();
            println!("Caching manpages from {} ...", source);
            let mut stmt = try!(conn.prepare(&format!("SELECT * FROM \"{}\"", source)).map_err(new_io_error));
            let data = try!(stmt.query_and_then(&[], |&ref row| {
                let a = try!(row.get_checked(0));
                let b = try!(row.get_checked(1));
                Ok((a, b))
            }).map_err(new_io_error)).collect::<Result<Vec<(String, String)>, rusqlite::Error>>();

            if let Ok(d) = data {
                for (name, url) in d {
                    let mut retries = 3;
                    println!("Caching {} ...", name);
                    while retries > 0 {
                        match self.cache_man_page(&source.to_string(), &url, &name) {
                            Ok(_)  => break,
                            Err(_) => {
                                println!("Retrying ...");
                                retries -= 1;
                            },
                        }
                    }

                    if retries == 0 {
                        println!("Error caching {} ...", name);
                        update_add_cell_op(&self.failure_count, 1);
                    } else {
                        update_add_cell_op(&self.success_count, 1);
                    }
                }
            }
        }

        println!("\n{} manual pages cached successfully.", self.success_count.get().map_or(-1, |x| x as i32));
        println!("{} manual pages failed to cache.", self.failure_count.get().map_or(-1, |x| x as i32));
        self.update_mandb(Some(false))
    }

    /// callback to cache new man page
    fn cache_man_page(&self, source: &str, url: &str, name: &str) -> io::Result<()> {
        // Skip if already exists, override if forced flag is true
        let outname = self.get_page_path(source, name);
        if outname.exists() && !self.forced {
            return Ok(());
        }

        try!(fs::create_dir_all(self.env.man_dir.join(source)));

        // There are often some errors in the HTML, for example: missing closing
        // tag. We use fixupHTML to fix this.
        let mut data = String::new();
        let mut resp = try!(reqwest::get(url).map_err(new_io_error));
        try!(resp.read_to_string(&mut data));

        let html2groff: fn(&str, &str) -> String;

        match &source[..source.len()-4] {
            "cplusplus"    => html2groff = ::formatter::cplusplus::html2groff,
            "cppreference" => html2groff = ::formatter::cppreference::html2groff,
            _ => return Err(new_io_error("wrong source")),
        }

        let groff_text = html2groff(&data, name);

        let mut file = try!(File::create(outname));
        let mut enc = GzEncoder::new(file, Compression::Default);
        try!(enc.write_all(groff_text.as_bytes()));
        try!(enc.finish());

        Ok(())
    }

    /// Clear all cache in man3
    pub fn clear_cache(&self) -> io::Result<()> {
        fs::remove_dir_all(&self.env.man_dir)
    }

    /// Call viewer.sh to view man page
    pub fn man(&self, pattern: &str) -> io::Result<()> {
        let avail = try!(fs::read_dir(self.env.man_dir.join(self.env.source.to_string())));
        let avail = avail.collect::<Result<Vec<_>, _>>().unwrap_or(Vec::new()).iter().map(|d| d.path()).collect::<Vec<_>>();

        if !self.env.index_db.exists() {
            return Err(new_io_error("can't find index.db"));
        }

        let page_name;
        let url;

        {
            let conn = try!(Connection::open(&self.env.index_db).map_err(new_io_error));

            macro_rules! get_pair {
                ($sql:expr) => {
                    conn.query_row_and_then(&format!($sql, self.env.source, pattern), &[],
                        |row| {
                            let a: String = try!(row.get_checked(0));
                            let b: String = try!(row.get_checked(1));
                            Ok((a, b))
                        }).map_err(|e: rusqlite::Error| new_io_error(e))
                }
            }

            let pair = try!({
                // Try direct match
                get_pair!("SELECT name,url FROM \"{}\" \
                          WHERE name='{}' ORDER BY LENGTH(name)").or(
                    // Try standard library
                    get_pair!("SELECT name,url FROM \"{}\" \
                              WHERE name=\"std::{}\" ORDER BY LENGTH(name)").or(
                        get_pair!("SELECT name,url FROM \"{}\" \
                                  WHERE name LIKE \"%{}%\" ORDER BY LENGTH(name)"))).map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::NotFound,
                                format!("No manual entry for {}", pattern))
                        })
            });

            page_name = pair.0;
            url = pair.1;
        }

        let page_filename = get_normalized_page_name(&page_name);
        if self.forced || !avail.contains(&(page_filename + ".3.gz").into()) {
            self.cache_man_page(&self.env.source.to_string(), &url, &page_name);
        }

        let pager_type = if stdout_isatty() { self.env.pager.to_string() } else { "pipe".to_owned() };

        // Call viewer
        let columns = try!(self.force_columns.or(get_width())
            .ok_or(new_io_error("Cannot determine width: either use --force-columns or switch to tty")));

        Command::new("/bin/sh")
                .arg(&self.env.pager_script)
                .arg(pager_type)
                .arg(self.get_page_path(&self.env.source.to_string(), &page_name))
                .arg(columns.to_string())
                .arg(&self.env.pager_config)
                .arg(page_name)
                .status()
                .map(|_| ())
    }

    /// Find pages in database.
    pub fn find(&self, pattern: &str) -> io::Result<()> {
        if !self.env.index_db.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "can't find index.db"));
        }

        let conn = try!(Connection::open(&self.env.index_db).map_err(new_io_error));
        let mut stmt = try!(conn.prepare(&format!(
            "SELECT * FROM \"{}\" WHERE name \
             LIKE \"%{}%\" ORDER BY LENGTH(name)",
            self.env.source, pattern)).map_err(new_io_error));
        let selected = try!(stmt.query_and_then(&[], |&ref row| {
            let a = try!(row.get_checked(0));
            let b = try!(row.get_checked(1));
            Ok((a, b))
        }).map_err(new_io_error)).collect::<Result<Vec<(String, String)>, rusqlite::Error>>();

        let pat = try!(Regex::new(&format!("(?i)\\({}\\)", pattern)).map_err(new_io_error));

        if let Ok(sel) = selected {
            if sel.len() > 0 {
                for (name, url) in sel {
                    if stdout_isatty() {
                        println!("{}", pat.replace(&name, "\\033[1;31m$1\\033[0m"));
                    } else {
                        println!("{}", name);
                    }
                }

                return Ok(());
            }
        }

        Err(io::Error::new(io::ErrorKind::NotFound, format!("{}: nothing appropriate", pattern)))
    }

    /// Update mandb.
    fn update_mandb(&self, quiet: Option<bool>) -> io::Result<()> {
        let quiet = quiet.unwrap_or(true);

        if !self.env.config.update_man_path() {
            return Ok(());
        }

        println!("\nrunning mandb...");
        let mut cmd = Command::new("mandb");

        if quiet {
            cmd.arg("-q");
        }

        cmd.status().map(|_| ())
    }

    fn get_page_path(&self, source: &str, name: &str) -> PathBuf {
        let name = get_normalized_page_name(name);
        let mut path = PathBuf::from(&self.env.man_dir);
        path.push(source);
        path.push(name + ".3.gz");
        path
    }
}

fn get_normalized_page_name(name: &str) -> String {
    name.replace("/", "_")
}


fn update_add_cell_op<T>(cell: &Cell<Option<T>>, value: T)
        where T: Copy + Default + AddAssign {
    cell.set(cell.get()
                 .and(Some(Default::default()))
                 .map(|v: T| { let mut v = v; v.add_assign(value); v }));
}
