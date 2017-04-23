extern crate either;
extern crate ini;

mod config;
mod environ;

use std::path::PathBuf;


pub fn get_lib_path(s: &str) -> PathBuf {
    PathBuf::from(s)
}

fn main() {
    println!("Hello, world!");
}
