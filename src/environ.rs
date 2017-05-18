use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

use ::get_lib_path;
use ::config::{Config, Source, Pager};
use ::errors;


#[derive(Clone)]
pub struct Environ {
    home: PathBuf,
    pub man_dir: PathBuf,
    config_dir: PathBuf,
    config_file: PathBuf,
    pub config: Config,

    pub index_db_re: PathBuf,
    pub index_db: PathBuf,

    pub pager: Pager,
    pub pager_config: PathBuf,
    pub pager_script: PathBuf,

    pub source: Source,
}

impl Environ {
    pub fn new() -> Environ {
        Environ::try_new().expect("Coundn't create an Environ struct")
    }

    pub fn try_new() -> errors::Result<Environ> {
        let home = env::home_dir().unwrap();

        let man_dir = home.join(".local/share/man/");
        let config_dir = home.join(".config/cppman-rs/");
        let config_file = config_dir.join("cppman-rs.cfg");

        let config = Config::new_from_file(&config_file);

        try!(fs::create_dir_all(&config_dir));

        let index_db_re = config_dir.join("index.db");
        let index_db = if index_db_re.exists() {
            index_db_re.clone()
        } else {
            get_lib_path("index.db")
        };

        let pager = config.pager();
        let pager_config = get_lib_path("cppman-rs.vim");
        let pager_script = get_lib_path("pager.sh");

        let source = config.source();

        Ok(Environ {
            home: home,
            man_dir: man_dir,
            config_dir: config_dir,
            config_file: config_file,
            config: config,
            index_db_re: index_db_re,
            index_db: index_db,
            pager: pager,
            pager_config: pager_config,
            pager_script: pager_script,
            source: source,
        })
    }
}
