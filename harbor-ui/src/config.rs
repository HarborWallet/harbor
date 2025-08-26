use harbor_client::bitcoin::Network;
use harbor_client::data_dir;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ConfigError {
    InvalidConfig,
    IoError(io::Error),
    SerdeError(serde_json::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig => write!(f, "Config file is invalid"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::SerdeError(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(error: serde_json::Error) -> Self {
        Self::SerdeError(error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub network: Network,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: Network::Bitcoin,
        }
    }
}

pub fn read_config() -> Result<Config, ConfigError> {
    // Create the datadir if it doesn't exist
    let root = PathBuf::from(&data_dir(None));
    std::fs::create_dir_all(&root).expect("Could not create datadir");

    let config_path = root.join("harbor.config.json");

    // if no config file, return default config
    if !config_path.exists() {
        let config = Config::default();
        // create default config file if we don't have one
        let json_string = serde_json::to_string_pretty(&config)?;
        std::fs::write(config_path, json_string)?;
        return Ok(config);
    } else if !config_path.is_file() {
        // if config is not a file, throw an error
        return Err(ConfigError::InvalidConfig);
    }

    // read the config file
    let data = std::fs::read_to_string(config_path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn write_config(config: &Config) -> Result<(), ConfigError> {
    let root = PathBuf::from(&data_dir(None));
    let config_path = root.join("harbor.config.json");

    std::fs::create_dir_all(&root).expect("Could not create datadir");
    let json_string = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, json_string)?;

    Ok(())
}
