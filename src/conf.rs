use bip39::{Language, Mnemonic};
use bitcoin::Network;
use log::info;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::{db::DBConnection, db_models::NewProfile};

/// The directory where all application data is stored
/// Defaults to ~/.harbor, if we're on a test network
/// Otherwise defaults to ~/.harbor/<network>
pub fn data_dir(network: Network) -> PathBuf {
    let home = home::home_dir().expect("Could not find home directory");
    let default = home.join(".harbor");
    match network {
        Network::Bitcoin => default,
        Network::Testnet => default.join("testnet3"),
        Network::Regtest => default.join("regtest"),
        Network::Signet => default.join("signet"),
    }
}

pub fn get_mnemonic(db: Arc<dyn DBConnection + Send + Sync>) -> anyhow::Result<Mnemonic> {
    match db.get_seed()? {
        Some(m) => {
            info!("retrieved existing seed");
            Ok(Mnemonic::from_str(&m)?)
        }
        None => {
            let new_profile = NewProfile {
                id: uuid::Uuid::new_v4().to_string(),
                seed_words: Mnemonic::generate_in(Language::English, 12)?.to_string(),
            };

            let p = db.insert_new_profile(new_profile)?;

            info!("creating new seed");
            Ok(Mnemonic::from_str(&p.seed_words)?)
        }
    }
}

pub fn generate_mnemonic(
    db: Arc<dyn DBConnection + Send + Sync>,
    words: Option<String>,
) -> anyhow::Result<Mnemonic> {
    let mnemonic_words = words.unwrap_or(Mnemonic::generate_in(Language::English, 12)?.to_string());

    let new_profile = NewProfile {
        id: uuid::Uuid::new_v4().to_string(),
        seed_words: mnemonic_words,
    };

    let p = db.insert_new_profile(new_profile)?;

    info!("creating new seed");
    Ok(Mnemonic::from_str(&p.seed_words)?)
}
