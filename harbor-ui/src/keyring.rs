use keyring::{KeyringEntry, set_global_service_name};
use log::{info, warn};

const KEYRING_SERVICE: &str = "Harbor Wallet";
const KEYRING_USERNAME: &str = "harbor_user";

pub async fn try_get_keyring_password() -> Option<String> {
    set_global_service_name(KEYRING_SERVICE);
    match KeyringEntry::try_new(KEYRING_USERNAME) {
        Ok(entry) => match entry.get_secret().await {
            Ok(password) => {
                info!("Successfully retrieved password from keyring");
                Some(password)
            }
            Err(e) => {
                warn!("Failed to get password from keyring: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("Failed to create keyring entry: {}", e);
            None
        }
    }
}

pub async fn save_to_keyring(password: &str) {
    set_global_service_name(KEYRING_SERVICE);
    match KeyringEntry::try_new(KEYRING_USERNAME) {
        Ok(entry) => match entry.set_secret(password).await {
            Ok(_) => {
                info!("Successfully saved password to keyring");
            }
            Err(e) => {
                warn!("Failed to save password to keyring: {}", e);
            }
        },
        Err(e) => {
            warn!("Failed to create keyring entry: {}", e);
        }
    }
}
