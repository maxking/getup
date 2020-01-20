use config::{Config, ConfigError, Environment, File};
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub stdout: String,
    pub stderr: String,
    pub pidfile: String,
    pub workdir: String,
    pub services_path: String,
    pub port: u32,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(File::with_name("config/default.toml"))
            .unwrap()
            .merge(Environment::with_prefix("GETUP"))
            .unwrap();
        println!("Loading default config...");
        s.try_into()
    }
}

lazy_static! {
    pub static ref SETTINGS: Settings = Settings::new().expect("Failed to load config");
}

pub fn initialize_config() {}
