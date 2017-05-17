use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use ini::Ini;

use ::errors;
use util::new_io_error;


#[derive(Copy, Clone)] pub enum Pager { Vim, Less, System }
#[derive(Copy, Clone)] struct UpdateManPath(bool);
#[derive(Copy, Clone)] pub enum Source { CPlusPlus, CppReference }


impl Display for Pager {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", Into::<&'static str>::into(*self))
    }
}

impl Display for UpdateManPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", Into::<&'static str>::into(*self))
    }
}


impl Pager {
    pub fn try_from(s: &str) -> errors::Result<Pager> {
        match s {
            "vim"    => Ok(Pager::Vim),
            "less"   => Ok(Pager::Less),
            "system" => Ok(Pager::System),
            _        => Err(errors::ErrorKind::ParsePager(s.to_owned()).into()),
        }
    }
}

impl<'a> From<&'a str> for Pager {
    fn from(s: &str) -> Pager {
        Pager::try_from(s).unwrap_or_default()
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


impl Source {
    pub fn try_from(s: &str) -> errors::Result<Source> {
        match s {
            "cplusplus.com"    => Ok(Source::CPlusPlus),
            "cppreference.com" => Ok(Source::CppReference),
            _                  => Err(errors::ErrorKind::ParseSource(s.to_owned()).into()),
        }
    }
}

impl<'a> From<&'a str> for Source {
    fn from(s: &str) -> Source {
        Source::try_from(s).unwrap_or_default()
    }
}


impl Into<&'static str> for Pager {
    fn into(self) -> &'static str {
        match self {
            Pager::Vim    => "vim",
            Pager::Less   => "less",
            Pager::System => "system",
        }
    }
}

impl Into<&'static str> for Source {
    fn into(self) -> &'static str {
        match self {
            Source::CPlusPlus    => "cplusplus.com",
            Source::CppReference => "cppreference.com",
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


#[derive(Clone)]
pub struct Config {
    config_file: PathBuf,
    config: RefCell<Ini>,
}

impl Config {
    pub fn new_from_file<P: AsRef<Path>>(config_file: P) -> Config {
        Config::new_try_from_file(config_file).expect("Cannot create a Config struct")
    }

    pub fn new_try_from_file<P: AsRef<Path>>(config_file: P) -> errors::Result<Config> {
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
    fn default_config<P: AsRef<Path>>(config_file: P) -> errors::Result<Config> {
        let mut config = Ini::new();
        config.with_section(Some("Settings".to_owned()))
              .set("Source", Source::default().to_string())
              .set("UpdateManPath", UpdateManPath::default().to_string())
              .set("Pager", Pager::default().to_string());

        let dir = try!(config_file.as_ref().parent()
            .ok_or(new_io_error("Not a filename since it does not have a parent path")));
        try!(fs::create_dir_all(dir));
        let mut file = try!(File::create(&config_file));

        match config.write_to(&mut file) {
            Ok(_)  => Ok(Config {
                config_file: config_file.as_ref().to_owned(),
                config: RefCell::new(config),
            }),
            Err(e) => Err(e.into()),
        }
    }


    /// Store config back to file.
    fn save(&self) -> errors::Result<()> {
        Ok(try!(self.config.borrow().write_to_file(&self.config_file)))
    }

    /// Reload config from file.
    fn reload(&self) -> errors::Result<()> {
        match Ini::load_from_file(&self.config_file) {
            Ok(ini) => { *self.config.borrow_mut() = ini; Ok(()) },
            Err(e)  => Err(e.into()),
        }
    }


    pub fn pager(&self) -> Pager {
        self.try_pager().expect("Couldn't get pager")
    }

    pub fn set_pager(&self, pager: Pager) {
        self.try_set_pager(pager).expect("Couldn't set pager")
    }

    pub fn try_pager(&self) -> errors::Result<Pager> {
        if let Some(s) = self.config.borrow().get_from(Some("Settings"), "Pager") {
            return Ok(Pager::from(s));
        }

        let pager = Pager::default();
        try!(self.try_set_pager(pager));
        try!(self.reload());
        Ok(pager)
    }

    pub fn try_set_pager(&self, pager: Pager) -> errors::Result<()> {
        self.config.borrow_mut().set_to(Some("Settings"), "Pager".to_owned(), pager.to_string());
        self.save()
    }


    pub fn update_man_path(&self) -> bool {
        self.try_update_man_path().expect("Couldn't get update_man_path")
    }

    pub fn set_update_man_path(&self, update_man_path: bool) {
        self.try_set_update_man_path(update_man_path).expect("Couldn't set update_man_path")
    }

    pub fn try_update_man_path(&self) -> errors::Result<bool> {
        if let Some(s) = self.config.borrow().get_from(Some("Settings"), "UpdateManPath") {
            return Ok(UpdateManPath::from(s).0);
        }

        let update_man_path = UpdateManPath::default();
        try!(self.try_set_update_man_path(update_man_path.0));
        try!(self.reload());
        Ok(update_man_path.0)
    }

    pub fn try_set_update_man_path(&self, update_man_path: bool) -> errors::Result<()> {
        self.config.borrow_mut().set_to(Some("Settings"), "UpdateManPath".to_owned(), update_man_path.to_string());
        self.save()
    }


    pub fn source(&self) -> Source {
        self.try_source().expect("Couldn't get source")
    }

    pub fn set_source(&self, source: Source) {
        self.try_set_source(source).expect("Couldn't set source")
    }

    pub fn try_source(&self) -> errors::Result<Source> {
        if let Some(s) = self.config.borrow().get_from(Some("Settings"), "Source") {
            return Ok(Source::from(s));
        }

        let source = Source::default();
        try!(self.try_set_source(source));
        try!(self.reload());
        Ok(source)
    }

    pub fn try_set_source(&self, source: Source) -> errors::Result<()> {
        self.config.borrow_mut().set_to(Some("Settings"), "Source".to_owned(), source.to_string());
        self.save()
    }
}
