use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufRead, Write};
use std::os::{unix, windows};
use std::os::raw::c_ushort;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use environ::Environ;


/// Add ~/.local/share/man to $HOME/.manpath
fn update_mandb_path(env: &Environ) {
    let home = env::home_dir().unwrap();
    let manpath_file = home.join(".manpath");
    let manpath = PathBuf::from(".local/share/man");

    let mut lines: Vec<String> = Vec::new();
    match File::open(manpath_file) {
        Ok(f)  => {
            lines.extend(BufReader::new(f).lines());
        },
        Err(_) => {
            if !env.config.update_man_path() {
                return;
            }
        },
    }

    let has_path = lines.iter().any(|l| l.contains(manpath));

    let Ok(f) = File::create(manpath_file);
    if env.config.update_man_path() {
        if !has_path {
            lines.push(format!("MANDATORY_MANPATH\t{}\n", home.join(manpath).display()));
        }
    } else {
        lines = lines.into_iter().filter(|l| l.contains(manpath)).collect();
    }

    lines.into_iter().map(|l| write!(f, "{}\n", l));
}

fn update_man3_link(env: &Environ) {
    let man3_path = env.man_dir.join("man3");

    if let Ok(metadata) = fs::symlink_metadata(man3_path) {
        if metadata.file_type().is_symlink() {
            let Ok(link_to) = fs::read_link(man3_path);
            if link_to == env.config.source() {
                return;
            } else {
                fs::remove_file(man3_path);
            }
        } else {
            panic!("Can't create link since `{}' already exists", man3_path.display());
        }
    }

    let _ = fs::create_dir_all(env.man_dir.join(env.config.source()));

    let _ = create_file_symlink(env.config.source(), man3_path);
}

struct WinSize {
    lines: c_ushort,
    columns: c_ushort,
    x: c_ushort,
    y: c_ushort,
}

/// TODO: implement
/// Get terminal width
fn get_width() -> usize {
    // Get terminal size
    let ws = ..;
    unimplemented!();
}

/// Read groff-formatted text and output man pages.
fn groff2man(data: &[u8]) -> String {
    let width = get_width();

    let cmd = format!("-t -Tascii -m man -rLL={}n -rLT={}n", width, width);
    let handle = Command::new("groff")
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

        stdin.write_all(data);
    }

    let Ok(output) = handle.wait_with_output();
    let Ok(man_text) = String::from_utf8(output.stdout);

    man_text
}

/// Convert HTML text from cplusplus.com to man pages.
fn html2man(data: &[u8], formatter: T) -> String {
    let groff_text = formatter(data);
    let man_text = groff2man(groff_text);
    man_text
}

/// TODO: Use something to fixup HTML
fn fixupHTML(data: &[u8]) -> String {
    unimplemented!();
}

#[cfg(not(target_os = "windows"))]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<()> {
    unix::fs::symlink(src, dst)
}

#[cfg(target_os = "windows")]
fn create_file_symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<()> {
    windows::fs::symlink_file(src, dst)
}
