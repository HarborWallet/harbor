use keyring::{set_global_service_name, KeyringEntry};
use log::warn;

const KEYRING_SERVICE: &str = "Harbor Wallet";
const KEYRING_USERNAME: &str = "harbor_user";

pub async fn try_get_keyring_password() -> Option<String> {
    set_global_service_name(KEYRING_SERVICE);
    match KeyringEntry::try_new(KEYRING_USERNAME) {
        Ok(entry) => entry.get_secret().await.ok(),
        Err(e) => {
            warn!("Failed to create keyring entry: {}", e);
            None
        }
    }
}

pub async fn save_to_keyring(password: &str) {
    set_global_service_name(KEYRING_SERVICE);
    match KeyringEntry::try_new(KEYRING_USERNAME) {
        Ok(entry) => {
            if let Err(e) = entry.set_secret(password).await {
                warn!("Failed to save password to keyring: {}", e);
            }
        }
        Err(e) => {
            warn!("Failed to create keyring entry: {}", e);
        }
    }
}
