extern crate either;
extern crate ini;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate select;

mod config;
mod crawler;
mod environ;
mod util;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use url::Url;

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

    fn process_document(&self, doc: Document) {
        if !self.blacklist.contains(doc.url) {
            println!("Indexing '{}' ...", doc.url);
            let name = self.extract_name(doc.text);
            self.results.insert((name, doc.url));
        } else {
            println!("Skipping blacklisted page '{}' ...", doc.url);
        }
    }

    fn insert_index(&self, table: &str, name: &str, url: &str) {
        unimplemented!();
    }

    fn cache_all(&self) {
        unimplemented!();
    }

    fn cache_man_page(&self) {
        unimplemented!();
    }

    fn clear_cache(&self) {
        unimplemented!();
    }

    fn man(&self, pattern: &str) {
        unimplemented!();
    }

    fn find(&self, pattern: &str) {
        unimplemented!();
    }

    fn update_mandb(&self, quiet: Option<bool>) {
        unimplemented!();
    }

    fn get_normalized_page_name(&self, name: &str) {
        unimplemented!();
    }

    fn get_page_path(&self, source: &str, name: &str) {
        unimplemented!();
    }
}


fn main() {
    println!("Hello, world!");
}
