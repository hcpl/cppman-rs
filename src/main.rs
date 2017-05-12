extern crate either;
extern crate ini;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate select;
#[macro_use]
extern crate nom;
extern crate rusqlite;
extern crate isatty;
extern crate flate2;
extern crate reqwest;
extern crate url;
extern crate ordermap;
#[macro_use]
extern crate mime;
#[macro_use]
extern crate clap;

mod config;
mod cppman;
mod crawler;
mod environ;
mod formatter;
mod util;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use clap::{App, Arg};

use ::cppman::Cppman;
use ::environ::Environ;


pub fn get_lib_path(s: &str) -> PathBuf {
    PathBuf::from(s)
}


fn main() {
    let matches = App::new("cppman-rs")
        .version(crate_version!())
        .about("Rust port of cppman, written in Python")
        .arg(Arg::with_name("source")
                 .help("Select source, either 'cppreference.com' or \
                        'cplusplus.com'.")
                 .short("s")
                 .long("source")
                 .default_value("cplusplus.com"))
        .arg(Arg::with_name("cache-all")
                 .help("Cache all available man pages from cppreference.com \
                        and cplusplus.com to enable offline browsing.")
                 .short("c")
                 .long("cache-all"))
        .arg(Arg::with_name("clear-cache")
                 .help("Clear all cached files.")
                 .short("C")
                 .long("clear-cache"))
        .arg(Arg::with_name("find-page")
                 .help("Find man page.")
                 .short("f")
                 .long("find-page")
                 .takes_value(true))
        .arg(Arg::with_name("force-update")
                 .help("Force cppman-rs to update existing cache when \
                        '--cache-all' or browsing man pages that were already \
                        cached.")
                 .short("o")
                 .long("force-update"))
        .arg(Arg::with_name("use-mandb")
                 .help("If provided, cppman-rs adds \
                        manpage path to mandb so that you can view C++ manpages \
                        with `man' command.")
                 .short("m")
                 .long("use-mandb"))
        .arg(Arg::with_name("pager")
                 .help("Select pager to use, accepts 'vim', 'less' or \
                        'system'. 'system' uses $PAGER environment as pager.")
                 .short("p")
                 .long("pager")
                 .default_value("vim"))
        .arg(Arg::with_name("rebuild-index")
                 .help("rebuild index database for the selected source, \
                        either 'cppreference.com' or 'cplusplus.com'.")
                 .short("r")
                 .long("rebuild-index"))
        .arg(Arg::with_name("force-columns")
                 .help("Force terminal columns.")
                 .long("force-columns"))
        .arg(Arg::with_name("manpage")
                 .help("Requested manpage"))
        .get_matches();

    let source = matches.value_of("source").unwrap();
    let cache_all = matches.is_present("cache-all");
    let clear_cache = matches.is_present("clear-cache");
    let find_page = matches.value_of("find-page");
    let force_update = matches.is_present("force-update");
    let use_mandb = matches.is_present("use-mandb");
    let pager = matches.value_of("pager").unwrap();
    let rebuild_index = matches.is_present("rebuild-index");
    let force_columns = value_t!(matches, "force-columns", usize).ok();
    let manpage = matches.value_of("manpage");

    let env = Environ::new();

    if cache_all {
        let cm = Cppman::new(Some(force_update), None, &env);
        cm.cache_all();
        process::exit(0);
    }

    if clear_cache {
        let cm = Cppman::new_default(&env);
        cm.clear_cache();
        process::exit(0);
    }

    if find_page.is_some() {
        let cm = Cppman::new_default(&env);
        cm.find(find_page.unwrap());
        process::exit(0);
    }

    if rebuild_index {
        let cm = Cppman::new_default(&env);
        cm.rebuild_index();
        process::exit(0);
    }

    if manpage.is_none() {
        writeln!(&mut io::stderr(), "What manual page do you want?").expect("failed printing to stderr");
        process::exit(1);
    }

    let cm = Cppman::new(Some(force_update), force_columns, &env);

}
