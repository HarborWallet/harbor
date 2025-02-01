use bitcoin::Network;
use fedimint_core::anyhow;
use harbor_client::data_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub network: Network,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: Network::Signet, // todo change to mainnet when launching
        }
    }
}

pub fn read_config() -> anyhow::Result<Config> {
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
        anyhow::bail!("Config file is invalid");
    }

    // read the config file
    let data = std::fs::read_to_string(config_path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn write_config(config: &Config) -> anyhow::Result<()> {
    let root = PathBuf::from(&data_dir(None));
    let config_path = root.join("harbor.config.json");

    std::fs::create_dir_all(&root).expect("Could not create datadir");
    let json_string = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, json_string)?;

    Ok(())
}
