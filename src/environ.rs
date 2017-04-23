use std::env;
use std::fs::create_dir_all;
use std::path::PathBuf;

use get_lib_path;
use config::{Config, Source, Pager};


pub struct Environ {
    home: PathBuf,
    pub man_dir: PathBuf,
    config_dir: PathBuf,
    config_file: PathBuf,
    pub config: Config,

    index_db_re: PathBuf,
    index_db: PathBuf,

    pager: Pager,
    pager_config: PathBuf,
    pager_script: PathBuf,

    source: Source,
}

impl Environ {
    fn new() -> Environ {
        let home = env::home_dir().unwrap();

        let man_dir = home.join(".local/share/man/");
        let config_dir = home.join(".config/cppman/");
        let config_file = config_dir.join("cppman.cfg");

        let config = Config::new_from_file(&config_file);

        let _ = create_dir_all(&config_dir);

        let index_db_re = config_dir.join("index.db");
        let index_db = if index_db_re.exists() { 
            index_db_re.clone()
        } else {
            get_lib_path("index.db")
        };

        let pager = config.pager();
        let pager_config = get_lib_path("cppman.vim");
        let pager_script = get_lib_path("pager.sh");

        let source = config.source();

        Environ {
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
        }
    }
}
