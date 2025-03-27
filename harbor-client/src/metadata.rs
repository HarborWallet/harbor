use crate::http::{make_get_request_direct, make_get_request_tor};
use bitcoin::secp256k1::PublicKey;
use cdk::nuts::MintInfo;
use fedimint_client::ClientHandleArc;
use fedimint_core::config::ClientConfig;
use fedimint_core::config::FederationId;
use fedimint_core::module::serde_json;
use log::error;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

/// Global cache of federation metadata
pub(crate) static CACHE: Lazy<RwLock<HashMap<FederationId, FederationMeta>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub(crate) enum FederationData<'a> {
    Client(&'a ClientHandleArc),
    Config(&'a ClientConfig),
}

impl FederationData<'_> {
    pub(crate) fn get_meta(&self, str: &str) -> Option<String> {
        match self {
            FederationData::Client(c) => c.get_config_meta(str),
            FederationData::Config(c) => c.meta(str).ok().flatten(),
        }
    }

    pub(crate) fn federation_id(&self) -> FederationId {
        match self {
            FederationData::Client(c) => c.federation_id(),
            FederationData::Config(c) => c.global.calculate_federation_id(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct FederationMetaConfig {
    #[serde(flatten)]
    pub federations: HashMap<String, FederationMeta>,
}

/// Metadata we might get from the federation
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug, Default)]
pub struct FederationMeta {
    // https://github.com/fedimint/fedimint/tree/master/docs/meta_fields
    pub federation_name: Option<String>,
    pub federation_expiry_timestamp: Option<String>,
    pub welcome_message: Option<String>,
    pub vetted_gateways: Option<String>,
    // undocumented parameters that fedi uses: https://meta.dev.fedibtc.com/meta.json
    pub federation_icon_url: Option<String>,
    pub meta_external_url: Option<String>,
    pub preview_message: Option<String>,
    pub popup_end_timestamp: Option<String>,
    pub popup_countdown_message: Option<String>,
}

impl FederationMeta {
    pub fn federation_expiry_timestamp(&self) -> Option<u64> {
        self.federation_expiry_timestamp
            .as_ref()
            .and_then(|s| s.parse().ok())
    }

    pub fn popup_end_timestamp(&self) -> Option<u64> {
        self.popup_end_timestamp
            .as_ref()
            .and_then(|s| s.parse().ok())
    }

    pub fn vetted_gateways(&self) -> Vec<PublicKey> {
        match self.vetted_gateways.as_deref() {
            None => vec![],
            Some(str) => serde_json::from_str(str).unwrap_or_default(),
        }
    }
}

impl From<Option<MintInfo>> for FederationMeta {
    fn from(info: Option<MintInfo>) -> Self {
        FederationMeta {
            federation_name: info.as_ref().and_then(|i| i.name.clone()),
            federation_expiry_timestamp: None,
            welcome_message: None,
            vetted_gateways: None,
            federation_icon_url: info.as_ref().and_then(|i| i.icon_url.clone()),
            meta_external_url: None,
            preview_message: info.and_then(|i| i.description),
            popup_end_timestamp: None,
            popup_countdown_message: None,
        }
    }
}

impl From<MintInfo> for FederationMeta {
    fn from(info: MintInfo) -> Self {
        Some(info).into()
    }
}

pub(crate) async fn get_federation_metadata(
    data: FederationData<'_>,
    tor_enabled: bool,
    cancel_handle: Arc<AtomicBool>,
) -> FederationMeta {
    // Check if cancelled before starting
    if cancel_handle.load(Ordering::Relaxed) {
        return FederationMeta::default();
    }

    let meta_external_url = data.get_meta("meta_external_url");
    let config: Option<FederationMeta> = match meta_external_url.as_ref() {
        None => None,
        Some(url) => {
            // Check if cancelled before making request
            if cancel_handle.load(Ordering::Relaxed) {
                return FederationMeta::default();
            }

            let result = if tor_enabled {
                make_get_request_tor::<FederationMetaConfig>(url, cancel_handle.clone()).await
            } else {
                make_get_request_direct::<FederationMetaConfig>(url).await
            };
            match result {
                Ok(m) => m
                    .federations
                    .get(&data.federation_id().to_string())
                    .cloned(),
                Err(e) => {
                    error!("Error fetching external metadata: {}", e);
                    None
                }
            }
        }
    };

    // Check if cancelled before constructing response
    if cancel_handle.load(Ordering::Relaxed) {
        return FederationMeta::default();
    }

    FederationMeta {
        meta_external_url, // Already set...
        federation_name: merge_values(
            data.get_meta("federation_name").clone(),
            config.as_ref().and_then(|c| c.federation_name.clone()),
        ),
        federation_expiry_timestamp: merge_values(
            data.get_meta("federation_expiry_timestamp"),
            config
                .as_ref()
                .and_then(|c| c.federation_expiry_timestamp.clone()),
        ),
        welcome_message: merge_values(
            data.get_meta("welcome_message"),
            config.as_ref().and_then(|c| c.welcome_message.clone()),
        ),
        vetted_gateways: config.as_ref().and_then(|c| c.vetted_gateways.clone()),
        federation_icon_url: merge_values(
            data.get_meta("federation_icon_url"),
            config.as_ref().and_then(|c| c.federation_icon_url.clone()),
        ),
        preview_message: merge_values(
            data.get_meta("preview_message"),
            config.as_ref().and_then(|c| c.preview_message.clone()),
        ),
        popup_end_timestamp: merge_values(
            data.get_meta("popup_end_timestamp"),
            config.as_ref().and_then(|c| c.popup_end_timestamp.clone()),
        ),
        popup_countdown_message: merge_values(
            data.get_meta("popup_countdown_message")
                .map(|v| v.to_string()),
            config
                .as_ref()
                .and_then(|c| c.popup_countdown_message.clone()),
        ),
    }
}

fn merge_values<T>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        // If a has value return that; otherwise, use the one from b if available.
        (Some(val), _) => Some(val),
        (None, Some(val)) => Some(val),
        (None, None) => None,
    }
}
