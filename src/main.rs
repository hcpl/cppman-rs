extern crate either;
extern crate ini;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate select;
extern crate rusqlite;
extern crate isatty;

mod config;
mod crawler;
mod environ;
mod util;

use std::fs;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use isatty::stdout_isatty;
use regex::Regex;
use rusqlite::Connection;
//use url::Url;

use crawler::Crawler;
use environ::Environ;


pub fn get_lib_path(s: &str) -> PathBuf {
    PathBuf::from(s)
}


struct Cppman {
    crawler: Crawler,
    results: HashSet<(String, Url)>,
    forced: bool,
    success_count: Option<u32>,
    failure_count: Option<u32>,
    force_columns: Option<usize>,

    blacklist: Vec<Url>,
    name_exceptions: Vec<String>,
    env: Environ,
}

impl Cppman {
    fn new_default(env: &Environ) -> Cppman {
        Cppman::new(false, None, env)
    }

    fn new(forced: bool, force_columns: Option<usize>, env: &Environ) -> Cppman {
        Cppman {
            crawler: Crawler::new(),
            results: HashSet::new(),
            forced: forced,
            success_count: None,
            failure_count: None,
            force_columns: force_columns,

            blacklist: Vec::new(),
            name_exceptions: vec!["http://www.cplusplus.com/reference/string/swap/".to_owned()],
            env: env.clone(),
        }
    }

    /// Extract man page name from web page.
    fn extract_name(&self, data: &[u8]) -> String {
        unimplemented!();
    }

    /// Rebuild index database from cplusplus.com and cppreference.com.
    fn rebuild_index(&self) {
        let _ = fs::remove_file(env.index_db_re);

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
    fn insert_index(&self, table: &str, name: &str, url: &str) {
        unimplemented!();
    }

    /// Cache all available man pages
    fn cache_all(&self) {
        unimplemented!();
    }

    /// callback to cache new man page
    fn cache_man_page(&self) {
        unimplemented!();
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
        if !Path::new(self.env.indexdb).exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "can't find index.db"));
        }

        let conn = try!(Connection::open(self.env.index_db));
        let stmt = try!(conn.prepare(&format!(
            "SELECT * FROM \"{}\" WHERE name \
             LIKE \"%{}%\" ORDER BY LENGTH(name)",
            self.env.source, pattern)));
        let selected = try!(stmt.query_map(&[], |&row| {
            (try!(row.get_checked(0)), try!(row.get_checked(1)))
        })).collect::<Vec<_>>();

        let pat = try!(Regex::new(&format!("(?i)\\({}\\)", pattern)));

        if selected.len() > 0 {
            for (name, url) in selected {
                if stdout_isatty() {
                    println!("{}", pat.replace(name, "\\033[1;31m$1\\033[0m"));
                } else {
                    println!("{}", name);
                }
            }

            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "{}: nothing appropriate".format(pattern)))
        }
    }

    /// Update mandb.
    fn update_mandb(&self, quiet: Option<bool>) -> io::Result<ExitStatus> {
        let quiet = quiet.unwrap_or(true);

        if !self.env.config.update_man_path() {
            return;
        }

        println!("\nrunning mandb...");
        let cmd = format!("mandb {}", if quiet { "-q" } else { "" });
        Command::new("mandb")
                .args(if quiet { &["-q"] } else { &[] })
                .status()
    }

    fn get_page_path(&self, source: &str, name: &str) {
        let name = get_normalized_page_name(name);
        PathBuf::from_iter(vec![self.env.man_dir, source, name + ".3.gz"])
    }
}

fn get_normalized_page_name(name: &str) -> String {
    name.replace("/", "_")
}


fn main() {
    println!("Hello, world!");
}
