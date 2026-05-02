use std::{env, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration directory not found")]
    ConfigDirNotFound,
}

pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    #[allow(unused)]
    pub config_dir: PathBuf,
    pub certs_dir: PathBuf,
}

impl Config {
    const DEFAULT_DIR_ENV: &str = "CHARLOTTE_CONFIG_DIR";
    const DEFAULT_DIR: &str = ".config/charlotte";
    const CERTS_DIR: &str = "certs";

    pub fn new(server_host: String, server_port: u16) -> Result<Self, Error> {
        let config_dir = Self::default_directory()?;
        let certs_dir = config_dir.join(Self::CERTS_DIR);

        Ok(Self {
            server_host,
            server_port,
            config_dir,
            certs_dir,
        })
    }

    fn default_directory() -> Result<PathBuf, Error> {
        env::var(Self::DEFAULT_DIR_ENV)
            .ok()
            .map(PathBuf::from)
            .or_else(|| env::home_dir().map(|home| home.join(Self::DEFAULT_DIR)))
            .ok_or(Error::ConfigDirNotFound)
    }
}
