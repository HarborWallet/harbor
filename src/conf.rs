use bip39::{Language, Mnemonic};
use bitcoin::Network;
use log::info;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::str::FromStr;

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

// todo store in encrypted database
pub fn get_mnemonic(network: Network) -> anyhow::Result<Mnemonic> {
    let seed_file = data_dir(network).join("seed.txt");
    let mnemonic = if seed_file.exists() {
        info!("Loading mnemonic from seed.txt");
        let mut file = std::fs::File::open(seed_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Mnemonic::from_str(&contents)?
    } else {
        info!("No seed.txt found, generating new mnemonic");
        let mnemonic = Mnemonic::generate_in(Language::English, 12)?;
        let mut file = std::fs::File::create(seed_file)?;
        file.write_all(mnemonic.to_string().as_bytes())?;
        mnemonic
    };

    Ok(mnemonic)
}
