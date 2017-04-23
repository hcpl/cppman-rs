use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::io;

use either::Either::{self, Left, Right};
use ini::Ini;


enum Setting { Source, UpdateManPath, Pager }
enum Pager { Vim, Less, System }
enum Source { CPlusPlus, CppReference }

impl Display for Setting {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            Setting::Source        => write!(f, "{}", "Source"),
            Setting::UpdateManPath => write!(f, "{}", "UpdateManPath"),
            Setting::Pager         => write!(f, "{}", "Pager"),
        }
    }
}

impl Display for Pager {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match *self {
            Pager::Vim    => write!(f, "{}", "vim"),
            Pager::Less   => write!(f, "{}", "less"),
            Pager::System => write!(f, "{}", "system"),
        }
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

impl Into<String> for Setting {
    fn into(self) -> String {
        self.to_string()
    }
}


static DEFAULT_SOURCE: Source = Source::CPlusPlus;
static DEFAULT_UPDATE_MAN_PATH: bool = false;
static DEFAULT_PAGER: Pager = Pager::Vim;


fn parse_bool(s: &str) -> Either<bool, String> {
    match s.parse::<bool>() {
        Ok(b)  => Left(b),
        Err(_) => Right(s.to_owned()),
    }
}


struct Config {
    config_file: String,
    config: RefCell<Ini>,
}

impl Config {
    fn new_from_file(config_file: &str) -> io::Result<Config> {
        if let Ok(ini) = Ini::load_from_file(config_file) {
            Ok(Config { config_file: config_file.to_owned(), config: RefCell::new(ini) })
        } else {
            Config::default_config(config_file)
        }
    }

    /// Get default config.
    fn default_config(config_file: &str) -> io::Result<Config> {
        let mut config = Ini::new();
        config.with_section(Some("Settings".to_owned()))
              .set("Source", DEFAULT_SOURCE.to_string())
              .set("UpdateManPath", DEFAULT_UPDATE_MAN_PATH.to_string())
              .set("Pager", DEFAULT_PAGER.to_string());

        match config.write_to_file(config_file) {
            Ok(_)  => Ok(Config { config_file: config_file.to_owned(), config: RefCell::new(config) }),
            Err(e) => Err(e),
        }
    }

    fn get(&self, setting: Setting) -> io::Result<Either<bool, String>> {
        let value = match self.config.borrow().get_from(Some("Settings"), &setting.to_string()) {
            Some(v) => Ok(v.to_owned()),
            None    => {
                let v = match setting {
                    Setting::Source        => DEFAULT_SOURCE.to_string(),
                    Setting::UpdateManPath => DEFAULT_UPDATE_MAN_PATH.to_string(),
                    Setting::Pager         => DEFAULT_PAGER.to_string(),
                };

                self.set(setting, &v).and(
                    Ini::load_from_file(&self.config_file)
                        .map(|ini| { *self.config.borrow_mut() = ini; v })
                        .map_err(|err| io::Error::new(io::ErrorKind::Other, err)))
            },
        };

        value.map(|v| parse_bool(&v))
    }

    fn set(&self, setting: Setting, value: &str) -> io::Result<()> {
        self.config.borrow_mut().set_to(Some("Settings"), setting.to_string(), value.to_owned());
        self.save()
    }

    /// Store config back to file.
    fn save(&self) -> io::Result<()> {
        self.config.borrow().write_to_file(&self.config_file)
    }
}
