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
extern crate term_size;

mod config;
mod cppman;
mod crawler;
mod environ;
mod formatter;
mod util;

use std::io::{self, Write, BufRead};
use std::path::PathBuf;
use std::process;

use clap::{App, Arg};

use ::config::{Source, Pager};
use ::cppman::Cppman;
use ::environ::Environ;


pub fn get_lib_path(s: &str) -> PathBuf {
    PathBuf::from(s)
}


fn main() {
    let matches = App::new("cppman-rs")
        .version(crate_version!())
        .about("Rust port of cppman, originally written in Python")
        .arg(Arg::with_name("source")
                 .help(&format!("Select source, either 'cppreference.com' or \
                                 'cplusplus.com'. [default: {}]",
                                 Into::<&'static str>::into(Source::default())))
                 .short("s")
                 .long("source")
                 .takes_value(true))
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
                 .help(&format!("Select pager to use, accepts 'vim', 'less' or \
                                'system'. 'system' uses $PAGER environment as pager. \
                                [default: {}]", Into::<&'static str>::into(Pager::default())))
                 .short("p")
                 .long("pager")
                 .takes_value(true))
        .arg(Arg::with_name("rebuild-index")
                 .help("rebuild index database for the selected source, \
                        either 'cppreference.com' or 'cplusplus.com'.")
                 .short("r")
                 .long("rebuild-index"))
        .arg(Arg::with_name("force-columns")
                 .help("Force terminal columns.")
                 .long("force-columns"))
        .arg(Arg::with_name("manpage")
                 .multiple(true)
                 .help("Requested manpages"))
        .get_matches();

    let source = matches.value_of("source");
    let cache_all = matches.is_present("cache-all");
    let clear_cache = matches.is_present("clear-cache");
    let find_page = matches.value_of("find-page");
    let force_update = matches.is_present("force-update");
    let use_mandb = matches.occurrences_of("use-mandb");
    let pager = matches.value_of("pager");
    let rebuild_index = matches.is_present("rebuild-index");
    let force_columns = value_t!(matches, "force-columns", usize).ok();
    let manpage = matches.values_of("manpage");

    let env = Environ::new();

    if cache_all {
        let cm = Cppman::new(Some(force_update), None, &env);
        let _ = cm.cache_all().expect("Error while caching manpages");
    }

    if clear_cache {
        let cm = Cppman::new_default(&env);
        let _ = cm.clear_cache().expect("Error while clearing the cache");
    }

    if find_page.is_some() {
        let cm = Cppman::new_default(&env);
        let _ = cm.find(find_page.unwrap()).expect("Error while finding a page");
    }

    if let Some(source) = source {
        if let Ok(source) = Source::try_from(source) {
            env.config.set_source(source);
            println!("Source set to `{}'.", source);
        } else {
            writeln!(&mut io::stderr(), "Invalid value `{}' for option `--source'", source)
                .expect("Failed printing to stderr");
            process::exit(1);
        }
    }

    if let Some(pager) = pager {
        if let Ok(pager) = Pager::try_from(pager) {
            env.config.set_pager(pager);
            println!("Pager set to `{}'.", pager);
        } else {
            writeln!(&mut io::stderr(), "Invalid value `{}' for option `--pager'", pager)
                .expect("Failed printing to stderr");
            process::exit(1);
        }
    }

    if use_mandb > 0 {
        if !env.config.update_man_path() {
            env.config.set_update_man_path(true);
        }
        if env.config.update_man_path() {
            util::update_mandb_path(&env).expect("Cannot update mandb path");
            util::update_man3_link(&env).expect("Cannot update man3 link");
        }
    }

    if rebuild_index {
        let cm = Cppman::new_default(&env);
        cm.rebuild_index();
    }

    if manpage.is_none() {
        writeln!(&mut io::stderr(), "What manual page do you want?").expect("Failed printing to stderr");
        process::exit(1);
    }

    let cm = Cppman::new(Some(force_update), force_columns, &env);

    for (i, arg) in manpage.unwrap().enumerate() {
        if i > 0 {
            println!("--CppMan-- next: {}(3) [ view (return) | skip (Ctrl-D) \
                      | quit (Ctrl-C) ]", arg);
            let stdin = io::stdin();
            // Ignore the actual input, we only need a user's Enter
            let _ = stdin.lock().lines().next().expect("Cannot read a line from stdin");
        }

        cm.man(arg).expect("Error while printing the manpage");
    }
}
