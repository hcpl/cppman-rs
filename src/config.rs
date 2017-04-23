use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};

use ini::Ini;


#[derive(Copy, Clone)] pub enum Pager { Vim, Less, System }
#[derive(Copy, Clone)] struct UpdateManPath(bool);
#[derive(Copy, Clone)] pub enum Source { CPlusPlus, CppReference }

impl Display for Pager {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            Pager::Vim    => write!(f, "{}", "vim"),
            Pager::Less   => write!(f, "{}", "less"),
            Pager::System => write!(f, "{}", "system"),
        }
    }
}

impl Display for UpdateManPath {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0.to_string())
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            Source::CPlusPlus    => write!(f, "{}", "cplusplus.com"),
            Source::CppReference => write!(f, "{}", "cppreference.com"),
        }
    }
}

impl<'a> From<&'a str> for Pager {
    fn from(s: &str) -> Pager {
        match s {
            "vim"    => Pager::Vim,
            "less"   => Pager::Less,
            "system" => Pager::System,
            _        => Pager::default(),
        }
    }
}

impl<'a> From<&'a str> for UpdateManPath {
    fn from(s: &str) -> UpdateManPath {
        if let Ok(b) = s.parse::<bool>() {
            UpdateManPath(b)
        } else {
            UpdateManPath::default()
        }
    }
}

impl<'a> From<&'a str> for Source {
    fn from(s: &str) -> Source {
        match s {
            "cplusplus.com"    => Source::CPlusPlus,
            "cppreference.com" => Source::CppReference,
            _                  => Source::default(),
        }
    }
}

impl Default for Pager {
    fn default() -> Pager {
        Pager::Vim
    }
}

impl Default for UpdateManPath {
    fn default() -> UpdateManPath {
        UpdateManPath(false)
    }
}

impl Default for Source {
    fn default() -> Source {
        Source::CPlusPlus
    }
}


pub struct Config {
    config_file: PathBuf,
    config: RefCell<Ini>,
}

impl Config {
    pub fn new_from_file<P: AsRef<Path>>(config_file: P) -> Config {
        Config::new_try_from_file(config_file).unwrap()
    }

    pub fn new_try_from_file<P: AsRef<Path>>(config_file: P) -> io::Result<Config> {
        if let Ok(ini) = Ini::load_from_file(&config_file) {
            Ok(Config {
                config_file: config_file.as_ref().to_owned(),
                config: RefCell::new(ini),
            })
        } else {
            Config::default_config(config_file)
        }
    }

    /// Get default config.
    fn default_config<P: AsRef<Path>>(config_file: P) -> io::Result<Config> {
        let mut config = Ini::new();
        config.with_section(Some("Settings".to_owned()))
              .set("Source", Source::default().to_string())
              .set("UpdateManPath", UpdateManPath::default().to_string())
              .set("Pager", Pager::default().to_string());

        match config.write_to_file(&config_file) {
            Ok(_)  => Ok(Config {
                config_file: config_file.as_ref().to_owned(),
                config: RefCell::new(config),
            }),
            Err(e) => Err(e),
        }
    }

    /// Store config back to file.
    fn save(&self) -> io::Result<()> {
        self.config.borrow().write_to_file(&self.config_file)
    }

    /// Reload config from file.
    fn reload(&self) -> io::Result<()> {
        match Ini::load_from_file(&self.config_file) {
            Ok(ini) => { *self.config.borrow_mut() = ini; Ok(()) },
            Err(e)  => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    pub fn pager(&self) -> Pager {
        match self.config.borrow().get_from(Some("Settings"), "Pager") {
            Some(s) => Pager::from(s),
            None    => {
                let pager = Pager::default();
                self.set_pager(pager);
                let _ = self.reload();
                pager
            },
        }
    }

    pub fn set_pager(&self, pager: Pager) {
        self.config.borrow_mut().set_to(Some("Settings"), "Pager".to_owned(), pager.to_string());
        let _ = self.save();
    }

    pub fn update_man_path(&self) -> bool {
        match self.config.borrow().get_from(Some("Settings"), "UpdateManPath") {
            Some(s) => UpdateManPath::from(s).0,
            None    => {
                let update_man_path = UpdateManPath::default();
                self.set_update_man_path(update_man_path.0);
                let _ = self.reload();
                update_man_path.0
            },
        }
    }

    pub fn set_update_man_path(&self, update_man_path: bool) {
        self.config.borrow_mut().set_to(Some("Settings"), "UpdateManPath".to_owned(), update_man_path.to_string());
        let _ = self.save();
    }

     pub fn source(&self) -> Source {
        match self.config.borrow().get_from(Some("Settings"), "Source") {
            Some(s) => Source::from(s),
            None    => {
                let source = Source::default();
                self.set_source(source);
                let _ = self.reload();
                source
            },
        }
    }

    pub fn set_source(&self, source: Source) {
        self.config.borrow_mut().set_to(Some("Settings"), "Source".to_owned(), source.to_string());
        let _ = self.save();
    }
}
