use crate::http::make_get_request;
use bitcoin::secp256k1::PublicKey;
use fedimint_client::ClientHandleArc;
use fedimint_core::config::ClientConfig;
use fedimint_core::config::FederationId;
use fedimint_core::module::serde_json;
use log::error;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
            FederationData::Client(c) => c.get_meta(str),
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
    federation_expiry_timestamp: Option<String>,
    pub welcome_message: Option<String>,
    vetted_gateways: Option<String>,
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

    pub fn vetted_gateways(&self) -> Vec<PublicKey> {
        match self.vetted_gateways.as_deref() {
            None => vec![],
            Some(str) => serde_json::from_str(str).unwrap_or_default(),
        }
    }
}

pub(crate) async fn get_federation_metadata(data: FederationData<'_>) -> FederationMeta {
    let meta_external_url = data.get_meta("meta_external_url");
    let config: Option<FederationMeta> = match meta_external_url.as_ref() {
        None => None,
        Some(url) => match make_get_request::<FederationMetaConfig>(url).await {
            Ok(m) => m
                .federations
                .get(&data.federation_id().to_string())
                .cloned(),
            Err(e) => {
                error!("Error fetching external metadata: {}", e);
                None
            }
        },
    };

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
