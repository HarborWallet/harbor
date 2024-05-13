use bip39::Mnemonic;
use bitcoin::Network;
use fedimint_bip39::Bip39RootSecretStrategy;
use fedimint_client::secret::{get_default_client_secret, RootSecretStrategy};
use fedimint_client::ClientHandleArc;
use fedimint_core::api::InviteCode;
use fedimint_core::config::ClientConfig;
use fedimint_core::db::mem_impl::MemDatabase;
use fedimint_core::db::IRawDatabaseExt;
use fedimint_ln_client::{LightningClientInit, LightningClientModule};
use fedimint_ln_common::LightningGateway;
use fedimint_mint_client::MintClientInit;
use fedimint_wallet_client::{WalletClientInit, WalletClientModule};
use log::{debug, error, info, trace};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tokio::spawn;

#[derive(Debug, Clone)]
pub(crate) struct FedimintClient {
    pub(crate) uuid: String,
    pub(crate) fedimint_client: ClientHandleArc,
    invite_code: InviteCode,
    stop: Arc<AtomicBool>,
}

impl FedimintClient {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        uuid: String,
        federation_code: InviteCode,
        mnemonic: &Mnemonic,
        network: Network,
        stop: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        info!("initializing a new federation client: {uuid}");

        let federation_id = federation_code.federation_id();

        trace!("Building fedimint client db");
        // todo use a real db
        let db = MemDatabase::new().into_database();

        let is_initialized = fedimint_client::Client::is_initialized(&db).await;

        let mut client_builder = fedimint_client::Client::builder(db);
        client_builder.with_module(WalletClientInit(None));
        client_builder.with_module(MintClientInit);
        client_builder.with_module(LightningClientInit);

        client_builder.with_primary_module(1);

        trace!("Building fedimint client db");
        let secret = Bip39RootSecretStrategy::<12>::to_root_secret(mnemonic);

        let fedimint_client = if is_initialized {
            client_builder
                .open(get_default_client_secret(&secret, &federation_id))
                .await
                .map_err(|e| {
                    error!("Could not open federation client: {e}");
                    e
                })?
        } else {
            let download = Instant::now();
            let config = ClientConfig::download_from_invite_code(&federation_code)
                .await
                .map_err(|e| {
                    error!("Could not download federation info: {e}");
                    e
                })?;
            trace!(
                "Downloaded federation info in: {}ms",
                download.elapsed().as_millis()
            );

            client_builder
                .join(get_default_client_secret(&secret, &federation_id), config)
                .await
                .map_err(|e| {
                    error!("Could not join federation: {e}");
                    e
                })?
        };
        let fedimint_client = Arc::new(fedimint_client);

        trace!("Retrieving fedimint wallet client module");

        // check federation is on expected network
        let wallet_client = fedimint_client.get_first_module::<WalletClientModule>();
        // compare magic bytes because different versions of rust-bitcoin
        if network != wallet_client.get_network() {
            error!(
                "Fedimint on different network {}, expected: {network}",
                wallet_client.get_network()
            );

            return Err(anyhow::anyhow!("Network mismatch, expected: {network}"));
        }

        // Update gateway cache in background
        let client_clone = fedimint_client.clone();
        spawn(async move {
            let start = Instant::now();
            let lightning_module = client_clone.get_first_module::<LightningClientModule>();

            match lightning_module.update_gateway_cache().await {
                Ok(_) => {
                    trace!("Updated lightning gateway cache");
                }
                Err(e) => {
                    error!("Could not update lightning gateway cache: {e}");
                }
            }

            trace!(
                "Setting active gateway took: {}ms",
                start.elapsed().as_millis()
            );
        });

        debug!("Built fedimint client");

        Ok(FedimintClient {
            uuid,
            fedimint_client,
            invite_code: federation_code,
            stop,
        })
    }
}

pub(crate) async fn select_gateway(client: &ClientHandleArc) -> Option<LightningGateway> {
    let ln = client.get_first_module::<LightningClientModule>();
    let mut selected_gateway = None;
    for gateway in ln.list_gateways().await {
        // first try to find a vetted gateway
        if gateway.vetted {
            // if we can select the gateway, return it
            if let Some(gateway) = ln.select_gateway(&gateway.info.gateway_id).await {
                return Some(gateway);
            }
        }

        // if no vetted gateway found, try to find a gateway with reasonable fees
        let fees = gateway.info.fees;
        if fees.base_msat >= 1_000 && fees.proportional_millionths >= 100 {
            if let Some(g) = ln.select_gateway(&gateway.info.gateway_id).await {
                selected_gateway = Some(g);
            }
        }
    }

    // if no gateway found, just select the first one we can find
    if selected_gateway.is_none() {
        for gateway in ln.list_gateways().await {
            if let Some(g) = ln.select_gateway(&gateway.info.gateway_id).await {
                selected_gateway = Some(g);
                break;
            }
        }
    }

    selected_gateway
}
