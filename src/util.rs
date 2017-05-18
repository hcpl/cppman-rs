use std::env;
use std::error;
use std::fs::{self, File};
use std::io::{self, BufReader, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(not(target_os = "windows"))]
use std::os::unix;
#[cfg(target_os = "windows")]
use std::os::windows;

use select::document::Document;
use term_size;

use ::environ::Environ;
use ::errors;


/// Add ~/.local/share/man to $HOME/.manpath
pub fn update_mandb_path(env: &Environ) -> errors::Result<()> {
    let home = env::home_dir().unwrap();
    let manpath_file = home.join(".manpath");
    let manpath = PathBuf::from(".local/share/man");

    let mut lines: Vec<String> = Vec::new();
    match File::open(&manpath_file) {
        Ok(f)  => {
            lines.extend(BufReader::new(f).lines().filter_map(Result::ok));
        },
        Err(_) => {
            if !env.config.update_man_path() {
                return Ok(());
            }
        },
    }

    let has_path = lines.iter().any(|l| manpath.to_str().map(|p| l.contains(p)).unwrap_or(false));

    let mut f = try!(File::create(&manpath_file));
    if env.config.update_man_path() {
        if !has_path {
            lines.push(format!("MANDATORY_MANPATH\t{}\n", home.join(manpath).display()));
        }
    } else {
        lines = lines
            .into_iter()
            .filter(|l| manpath.to_str().map(|p| l.contains(p)).unwrap_or(false))
            .collect();
    }

    for line in lines {
        try!(write!(f, "{}\n", line));
    }

    Ok(())
}

pub fn update_man3_link(env: &Environ) -> errors::Result<()> {
    let man3_path = env.man_dir.join("man3");

    if let Ok(metadata) = fs::symlink_metadata(&man3_path) {
        if metadata.file_type().is_symlink() {
            let link_to = try!(fs::read_link(&man3_path));
            if link_to == Path::new(&env.config.source().to_string()) {
                return Ok(());
            } else {
                try!(fs::remove_file(&man3_path));
            }
        } else {
            panic!("Can't create link since `{}' already exists", man3_path.display());
        }
    }

    try!(fs::create_dir_all(env.man_dir.join(env.config.source().to_string())));

    create_file_symlink(env.config.source().to_string(), &man3_path)
}

/// Get terminal width
pub fn get_width() -> errors::Result<usize> {
    term_size::dimensions_stdout().map(|(w, _)| w).ok_or(errors::ErrorKind::StdoutNoTermWidth.into())
}

/// Read groff-formatted text and output man pages.
fn groff2man(data: &[u8]) -> errors::Result<String> {
    let width = try!(get_width());

    let mut handle = Command::new("groff")
                             .arg("-t")
                             .arg("-Tascii")
                             .args(&["-m", "man"])
                             .arg(format!("-rLL={}n", width))
                             .arg(format!("-rLT={}n", width))
                             .spawn()
                             .ok()
                             .expect("Failed to execute");

    {
        let stdin = handle.stdin.as_mut().expect("Couldn't get mutable Pipestream");

        try!(stdin.write_all(data));
    }

    let output = try!(handle.wait_with_output());
    let man_text = try!(String::from_utf8(output.stdout));

    Ok(man_text)
}

/// Convert HTML text from cplusplus.com to man pages.
fn html2man(data: &[u8], formatter: fn(&[u8]) -> String) -> errors::Result<String> {
    let groff_text = formatter(data);
    let man_text = try!(groff2man(groff_text.as_bytes()));
    Ok(man_text)
}

pub fn fixup_html(data: &str) -> errors::Result<String> {
    let doc = Document::from(data);
    let node = try!(doc.nth(0).ok_or(new_io_error("No 0th node in Document")));
    Ok(node.html())
}

#[cfg(not(target_os = "windows"))]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> errors::Result<()> {
    Ok(try!(unix::fs::symlink(src, dst)))
}

#[cfg(target_os = "windows")]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> errors::Result<()> {
    Ok(try!(windows::fs::symlink_file(src, dst)))
}

pub fn new_io_error<E>(error: E) -> io::Error
        where E: Into<Box<error::Error + Send + Sync>> {
    io::Error::new(io::ErrorKind::Other, error)
}
