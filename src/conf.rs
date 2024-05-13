use bitcoin::Network;
use std::path::PathBuf;

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
