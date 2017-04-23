extern crate either;
extern crate ini;

mod config;
mod crawler;
mod environ;
mod util;

use std::path::PathBuf;


pub fn get_lib_path(s: &str) -> PathBuf {
    PathBuf::from(s)
}


fn main() {
    println!("Hello, world!");
}
