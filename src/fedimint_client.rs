use crate::bridge::{CoreUIMsg, ReceiveSuccessMsg, SendSuccessMsg};
use crate::Message;
use bip39::Mnemonic;
use bitcoin::hashes::hex::FromHex;
use bitcoin::Network;
use fedimint_bip39::Bip39RootSecretStrategy;
use fedimint_client::oplog::UpdateStreamOrOutcome;
use fedimint_client::secret::{get_default_client_secret, RootSecretStrategy};
use fedimint_client::ClientHandleArc;
use fedimint_core::api::InviteCode;
use fedimint_core::config::ClientConfig;
use fedimint_core::db::mem_impl::MemDatabase;
use fedimint_core::db::IRawDatabaseExt;
use fedimint_ln_client::{
    InternalPayState, LightningClientInit, LightningClientModule, LnPayState, LnReceiveState,
};
use fedimint_ln_common::LightningGateway;
use fedimint_mint_client::MintClientInit;
use fedimint_wallet_client::{WalletClientInit, WalletClientModule};
use iced::futures::channel::mpsc::Sender;
use iced::futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::spawn;

#[derive(Debug, Clone)]
#[allow(unused)] // TODO: remove
pub(crate) struct FedimintClient {
    pub(crate) fedimint_client: ClientHandleArc,
    invite_code: InviteCode,
    stop: Arc<AtomicBool>,
}

impl FedimintClient {
    pub(crate) async fn new(
        invite_code: InviteCode,
        mnemonic: &Mnemonic,
        network: Network,
        stop: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        let federation_id = invite_code.federation_id();
        info!("initializing a new federation client: {federation_id}");

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
            let config = ClientConfig::download_from_invite_code(&invite_code)
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
        let stop_clone = stop.clone();
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
                "Updating gateway cache took: {}ms",
                start.elapsed().as_millis()
            );

            // continually update gateway cache
            loop {
                lightning_module
                    .update_gateway_cache_continuously(|g| async { g })
                    .await;
                if stop_clone.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        debug!("Built fedimint client");

        Ok(FedimintClient {
            fedimint_client,
            invite_code,
            stop,
        })
    }
}

pub(crate) async fn select_gateway(client: &ClientHandleArc) -> Option<LightningGateway> {
    let ln = client.get_first_module::<LightningClientModule>();
    let gateways = ln.list_gateways().await;
    let mut selected_gateway: Option<LightningGateway> = None;
    for gateway in gateways.iter() {
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
                // only select gateways that support private payments, unless we don't have a gateway
                if g.supports_private_payments || selected_gateway.is_none() {
                    selected_gateway = Some(g);
                }
            }
        }
    }

    // if no gateway found, just select the first one we can find
    if selected_gateway.is_none() {
        for gateway in gateways {
            if let Some(g) = ln.select_gateway(&gateway.info.gateway_id).await {
                selected_gateway = Some(g);
                break;
            }
        }
    }

    selected_gateway
}

pub(crate) async fn spawn_invoice_receive_subscription(
    mut sender: Sender<Message>,
    client: ClientHandleArc,
    subscription: UpdateStreamOrOutcome<LnReceiveState>,
) {
    spawn(async move {
        let mut stream = subscription.into_stream();
        while let Some(op_state) = stream.next().await {
            match op_state {
                LnReceiveState::Canceled { reason } => {
                    error!("Payment canceled, reason: {:?}", reason);
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::ReceiveFailed(
                            reason.to_string(),
                        )))
                        .await
                        .unwrap();
                }
                LnReceiveState::Claimed => {
                    info!("Payment claimed");
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::ReceiveSuccess(
                            ReceiveSuccessMsg::Lightning,
                        )))
                        .await
                        .unwrap();

                    let new_balance = client.get_balance().await;
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::BalanceUpdated(new_balance)))
                        .await
                        .unwrap();

                    break;
                }
                _ => {}
            }
        }
    });
}

pub(crate) async fn spawn_invoice_payment_subscription(
    mut sender: Sender<Message>,
    client: ClientHandleArc,
    subscription: UpdateStreamOrOutcome<LnPayState>,
) {
    spawn(async move {
        let mut stream = subscription.into_stream();
        while let Some(op_state) = stream.next().await {
            match op_state {
                LnPayState::Canceled => {
                    error!("Payment canceled");
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::SendFailure(
                            "Canceled".to_string(),
                        )))
                        .await
                        .unwrap();
                }
                LnPayState::UnexpectedError { error_message } => {
                    error!("Unexpected payment error: {:?}", error_message);
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::SendFailure(error_message)))
                        .await
                        .unwrap();
                }
                LnPayState::Success { preimage } => {
                    info!("Payment success");
                    let preimage: [u8; 32] =
                        FromHex::from_hex(&preimage).expect("Invalid preimage");
                    let params = SendSuccessMsg::Lightning { preimage };
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::SendSuccess(params)))
                        .await
                        .unwrap();

                    let new_balance = client.get_balance().await;
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::BalanceUpdated(new_balance)))
                        .await
                        .unwrap();

                    break;
                }
                _ => {}
            }
        }
    });
}

pub(crate) async fn spawn_internal_payment_subscription(
    mut sender: Sender<Message>,
    client: ClientHandleArc,
    subscription: UpdateStreamOrOutcome<InternalPayState>,
) {
    spawn(async move {
        let mut stream = subscription.into_stream();
        while let Some(op_state) = stream.next().await {
            match op_state {
                InternalPayState::FundingFailed { error } => {
                    error!("Funding failed: {error:?}");
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::ReceiveFailed(
                            error.to_string(),
                        )))
                        .await
                        .unwrap();
                }
                InternalPayState::UnexpectedError(error_message) => {
                    error!("Unexpected payment error: {error_message:?}");
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::SendFailure(error_message)))
                        .await
                        .unwrap();
                }
                InternalPayState::Preimage(preimage) => {
                    info!("Payment success");
                    let params = SendSuccessMsg::Lightning {
                        preimage: preimage.0,
                    };
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::SendSuccess(params)))
                        .await
                        .unwrap();

                    let new_balance = client.get_balance().await;
                    sender
                        .send(Message::CoreMessage(CoreUIMsg::BalanceUpdated(new_balance)))
                        .await
                        .unwrap();

                    break;
                }
                _ => {}
            }
        }
    });
}
