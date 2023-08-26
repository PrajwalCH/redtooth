use std::env;
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
const HOME_ENV_KEY: &str = "HOME";
#[cfg(windows)]
const HOME_ENV_KEY: &str = "USERPROFILE";
/// Directory where all the received files will live.
const DIR_NAME: &str = env!("CARGO_PKG_NAME");

pub struct Config {
    /// Path where the received file will be saved.
    pub save_location: PathBuf,
}

impl Default for Config {
    fn default() -> Config {
        let home = env::var(HOME_ENV_KEY)
            .unwrap_or_else(|_| panic!("your OS should set env variable {HOME_ENV_KEY}"));

        Config {
            save_location: Path::new(&home).join(DIR_NAME),
        }
    }
}
