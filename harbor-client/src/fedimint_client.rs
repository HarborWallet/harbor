use crate::{CoreUIMsg, CoreUIMsgPacket, ReceiveSuccessMsg, SendSuccessMsg};
use crate::{db::DBConnection, db_models::NewFedimint};
use anyhow::anyhow;
use async_trait::async_trait;
use bip39::Mnemonic;
use bitcoin::Network;
use bitcoin::hashes::hex::FromHex;
use fedimint_bip39::Bip39RootSecretStrategy;
use fedimint_client::ClientHandleArc;
use fedimint_client::oplog::UpdateStreamOrOutcome;
use fedimint_client::secret::{RootSecretStrategy, get_default_client_secret};
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_core::db::IDatabaseTransactionOps;
use fedimint_core::db::IRawDatabase;
use fedimint_core::db::IRawDatabaseTransaction;
use fedimint_core::db::PrefixStream;
use fedimint_core::db::mem_impl::MemDatabase;
use fedimint_core::db::mem_impl::MemTransaction;
use fedimint_core::{db::IDatabaseTransactionOpsCore, invite_code::InviteCode};
use fedimint_ln_client::{
    InternalPayState, LightningClientInit, LightningClientModule, LnPayState, LnReceiveState,
};
use fedimint_ln_common::LightningGateway;
use fedimint_mint_client::MintClientInit;
use fedimint_wallet_client::{DepositStateV2, WalletClientInit, WalletClientModule, WithdrawState};
use futures::channel::mpsc::Sender;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace};
use std::fmt::Debug;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use std::{fmt, sync::atomic::AtomicBool};
use tokio::spawn;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FedimintClient {
    pub(crate) fedimint_client: ClientHandleArc,
    stop: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub enum FederationInviteOrId {
    Invite(InviteCode),
    Id(FederationId),
}

impl FederationInviteOrId {
    pub fn federation_id(&self) -> FederationId {
        match self {
            FederationInviteOrId::Invite(i) => i.federation_id(),
            FederationInviteOrId::Id(i) => *i,
        }
    }

    pub fn invite_code(&self) -> Option<InviteCode> {
        match self {
            FederationInviteOrId::Invite(i) => Some(i.clone()),
            FederationInviteOrId::Id(_) => None,
        }
    }
}

impl FedimintClient {
    pub async fn new(
        storage: Arc<dyn DBConnection + Send + Sync>,
        invite_or_id: FederationInviteOrId,
        mnemonic: &Mnemonic,
        network: Network,
        stop: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        let federation_id = invite_or_id.federation_id();

        info!("initializing a new federation client: {federation_id}");

        trace!("Building fedimint client db");

        let db = FedimintStorage::new(storage.clone(), federation_id, invite_or_id.invite_code())
            .await?;

        let is_initialized = fedimint_client::Client::is_initialized(&db.clone().into()).await;

        let mut client_builder = fedimint_client::Client::builder(db.into()).await?;

        // Check if tor is enabled in profile
        let profile = storage.get_profile()?;
        let tor_enabled = profile.expect("must have profile").tor_enabled();
        if tor_enabled {
            client_builder.with_tor_connector();
        }

        client_builder.with_module(WalletClientInit(None));
        client_builder.with_module(MintClientInit);
        client_builder.with_module(LightningClientInit::default());

        client_builder.with_primary_module_kind(fedimint_mint_client::KIND);

        trace!("Building fedimint client db");
        let secret = Bip39RootSecretStrategy::<12>::to_root_secret(mnemonic);

        let fedimint_client = if is_initialized {
            Some(
                client_builder
                    .open(get_default_client_secret(&secret, &federation_id))
                    .await
                    .map_err(|e| {
                        error!("Could not open federation client: {e}");
                        e
                    })?,
            )
        } else if let FederationInviteOrId::Invite(invite_code) = invite_or_id {
            let download = Instant::now();
            let config = {
                let config = if tor_enabled {
                    fedimint_api_client::api::net::Connector::Tor
                        .download_from_invite_code(&invite_code)
                        .await
                } else {
                    fedimint_api_client::api::net::Connector::Tcp
                        .download_from_invite_code(&invite_code)
                        .await
                };
                config.map_err(|e| {
                    error!("Could not download federation info: {e}");
                    e
                })?
            };
            trace!(
                "Downloaded federation info in: {}ms",
                download.elapsed().as_millis()
            );

            Some(
                client_builder
                    .join(
                        get_default_client_secret(&secret, &federation_id),
                        config,
                        None,
                    )
                    .await
                    .map_err(|e| {
                        error!("Could not join federation: {e}");
                        e
                    })?,
            )
        } else {
            None
        };

        let fedimint_client = match fedimint_client {
            None => {
                error!("did not have enough information to join federation");
                return Err(anyhow!(
                    "did not have enough information to join federation"
                ));
            }
            Some(client) => Arc::new(client),
        };

        trace!("Retrieving fedimint wallet client module");

        // check federation is on expected network
        let wallet_client = fedimint_client
            .get_first_module::<WalletClientModule>()
            .expect("must have wallet module");
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
            let lightning_module = client_clone
                .get_first_module::<LightningClientModule>()
                .expect("must have ln module");

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
            lightning_module
                .update_gateway_cache_continuously(|g| async { g })
                .await;
        });

        debug!("Built fedimint client");

        Ok(FedimintClient {
            fedimint_client,
            stop,
        })
    }

    pub fn federation_id(&self) -> FederationId {
        self.fedimint_client.federation_id()
    }
}

pub(crate) async fn select_gateway(client: &ClientHandleArc) -> Option<LightningGateway> {
    let ln = client
        .get_first_module::<LightningClientModule>()
        .expect("must have ln module");

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

async fn update_history(
    storage: Arc<dyn DBConnection + Send + Sync>,
    msg_id: Uuid,
    sender: &mut Sender<CoreUIMsgPacket>,
) {
    if let Ok(history) = storage.get_transaction_history() {
        sender
            .send(CoreUIMsgPacket {
                id: Some(msg_id),
                msg: CoreUIMsg::TransactionHistoryUpdated(history),
            })
            .await
            .unwrap();
    }
}

pub(crate) async fn spawn_invoice_receive_subscription(
    mut sender: Sender<CoreUIMsgPacket>,
    client: ClientHandleArc,
    storage: Arc<dyn DBConnection + Send + Sync>,
    operation_id: OperationId,
    msg_id: Uuid,
    is_transfer: bool,
    subscription: UpdateStreamOrOutcome<LnReceiveState>,
) {
    info!(
        "Spawning lightning receive subscription for operation id: {}",
        operation_id.fmt_full()
    );
    spawn(async move {
        let mut stream = subscription.into_stream();
        while let Some(op_state) = stream.next().await {
            match op_state {
                LnReceiveState::Canceled { reason } => {
                    error!("Payment canceled, reason: {:?}", reason);
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::ReceiveFailed(reason.to_string()),
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.mark_ln_receive_as_failed(operation_id) {
                        error!("Could not mark lightning receive as failed: {e}");
                    }
                    break;
                }
                LnReceiveState::Claimed => {
                    info!("Payment claimed");
                    let params = if is_transfer {
                        ReceiveSuccessMsg::Transfer
                    } else {
                        ReceiveSuccessMsg::Lightning
                    };
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::ReceiveSuccess(params),
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.mark_ln_receive_as_success(operation_id) {
                        error!("Could not mark lightning receive as success: {e}");
                    }

                    let new_balance = client.get_balance().await;
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::FederationBalanceUpdated {
                                id: client.federation_id(),
                                balance: new_balance,
                            },
                        })
                        .await
                        .unwrap();

                    update_history(storage.clone(), msg_id, &mut sender).await;

                    break;
                }
                _ => {}
            }
        }
    });
}

pub(crate) async fn spawn_invoice_payment_subscription(
    mut sender: Sender<CoreUIMsgPacket>,
    client: ClientHandleArc,
    storage: Arc<dyn DBConnection + Send + Sync>,
    operation_id: OperationId,
    msg_id: Uuid,
    is_transfer: bool,
    subscription: UpdateStreamOrOutcome<LnPayState>,
) {
    info!(
        "Spawning lightning payment subscription for operation id: {}",
        operation_id.fmt_full()
    );
    spawn(async move {
        let mut stream = subscription.into_stream();
        while let Some(op_state) = stream.next().await {
            match op_state {
                LnPayState::Canceled => {
                    error!("Payment canceled");
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: if is_transfer {
                                CoreUIMsg::TransferFailure("Canceled".to_string())
                            } else {
                                CoreUIMsg::SendFailure("Canceled".to_string())
                            },
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.mark_lightning_payment_as_failed(operation_id) {
                        error!("Could not mark lightning payment as failed: {e}");
                    }
                    break;
                }
                LnPayState::UnexpectedError { error_message } => {
                    error!("Unexpected payment error: {:?}", error_message);
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: if is_transfer {
                                CoreUIMsg::TransferFailure(error_message)
                            } else {
                                CoreUIMsg::SendFailure(error_message)
                            },
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.mark_lightning_payment_as_failed(operation_id) {
                        error!("Could not mark lightning payment as failed: {e}");
                    }
                    break;
                }
                LnPayState::Success { preimage } => {
                    info!("Payment success");
                    let preimage: [u8; 32] =
                        FromHex::from_hex(&preimage).expect("Invalid preimage");
                    let params = if is_transfer {
                        SendSuccessMsg::Transfer
                    } else {
                        SendSuccessMsg::Lightning { preimage }
                    };
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::SendSuccess(params),
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.set_lightning_payment_preimage(operation_id, preimage) {
                        error!("Could not mark lightning payment as success: {e}");
                    }

                    let new_balance = client.get_balance().await;
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::FederationBalanceUpdated {
                                id: client.federation_id(),
                                balance: new_balance,
                            },
                        })
                        .await
                        .unwrap();

                    update_history(storage.clone(), msg_id, &mut sender).await;

                    break;
                }
                _ => {}
            }
        }
    });
}

pub(crate) async fn spawn_internal_payment_subscription(
    mut sender: Sender<CoreUIMsgPacket>,
    client: ClientHandleArc,
    storage: Arc<dyn DBConnection + Send + Sync>,
    operation_id: OperationId,
    msg_id: Uuid,
    subscription: UpdateStreamOrOutcome<InternalPayState>,
) {
    info!(
        "Spawning internal payment subscription for operation id: {}",
        operation_id.fmt_full()
    );
    spawn(async move {
        let mut stream = subscription.into_stream();
        while let Some(op_state) = stream.next().await {
            match op_state {
                InternalPayState::FundingFailed { error } => {
                    error!("Funding failed: {error:?}");
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::ReceiveFailed(error.to_string()),
                        })
                        .await
                        .unwrap();
                    if let Err(e) = storage.mark_lightning_payment_as_failed(operation_id) {
                        error!("Could not mark lightning payment as failed: {e}");
                    }
                    break;
                }
                InternalPayState::UnexpectedError(error_message) => {
                    error!("Unexpected payment error: {error_message:?}");
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::SendFailure(error_message),
                        })
                        .await
                        .unwrap();
                    if let Err(e) = storage.mark_lightning_payment_as_failed(operation_id) {
                        error!("Could not mark lightning payment as failed: {e}");
                    }
                    break;
                }
                InternalPayState::Preimage(preimage) => {
                    info!("Payment success");
                    let params = SendSuccessMsg::Lightning {
                        preimage: preimage.0,
                    };
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::SendSuccess(params),
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.set_lightning_payment_preimage(operation_id, preimage.0)
                    {
                        error!("Could not mark lightning payment as success: {e}");
                    }

                    let new_balance = client.get_balance().await;
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::FederationBalanceUpdated {
                                id: client.federation_id(),
                                balance: new_balance,
                            },
                        })
                        .await
                        .unwrap();

                    update_history(storage, msg_id, &mut sender).await;

                    break;
                }
                _ => {}
            }
        }
    });
}

pub(crate) async fn spawn_onchain_payment_subscription(
    mut sender: Sender<CoreUIMsgPacket>,
    client: ClientHandleArc,
    storage: Arc<dyn DBConnection + Send + Sync>,
    operation_id: OperationId,
    msg_id: Uuid,
    subscription: UpdateStreamOrOutcome<WithdrawState>,
) {
    info!(
        "Spawning onchain payment subscription for operation id: {}",
        operation_id.fmt_full()
    );
    spawn(async move {
        let mut stream = subscription.into_stream();
        while let Some(op_state) = stream.next().await {
            match op_state {
                WithdrawState::Created => {}
                WithdrawState::Failed(error) => {
                    error!("Onchain payment failed: {error:?}");
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::SendFailure(error),
                        })
                        .await
                        .unwrap();
                    if let Err(e) = storage.mark_onchain_payment_as_failed(operation_id) {
                        error!("Could not mark onchain payment as failed: {e}");
                    }

                    break;
                }
                WithdrawState::Succeeded(txid) => {
                    info!("Onchain payment success: {txid}");
                    let params = SendSuccessMsg::Onchain { txid };
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::SendSuccess(params),
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.set_onchain_payment_txid(operation_id, txid) {
                        error!("Could not mark onchain payment txid: {e}");
                    }

                    let new_balance = client.get_balance().await;
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::FederationBalanceUpdated {
                                id: client.federation_id(),
                                balance: new_balance,
                            },
                        })
                        .await
                        .unwrap();

                    update_history(storage.clone(), msg_id, &mut sender).await;

                    break;
                }
            }
        }
    });
}

pub(crate) async fn spawn_onchain_receive_subscription(
    mut sender: Sender<CoreUIMsgPacket>,
    client: ClientHandleArc,
    storage: Arc<dyn DBConnection + Send + Sync>,
    operation_id: OperationId,
    msg_id: Uuid,
    subscription: UpdateStreamOrOutcome<DepositStateV2>,
) {
    info!(
        "Spawning onchain receive subscription for operation id: {}",
        operation_id.fmt_full()
    );
    spawn(async move {
        let mut stream = subscription.into_stream();
        while let Some(op_state) = stream.next().await {
            match op_state {
                DepositStateV2::WaitingForTransaction => {}
                DepositStateV2::Failed(error) => {
                    error!("Onchain receive failed: {error:?}");
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::ReceiveFailed(error),
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.mark_onchain_receive_as_failed(operation_id) {
                        error!("Could not mark onchain receive as failed: {e}");
                    }

                    break;
                }
                DepositStateV2::WaitingForConfirmation {
                    btc_deposited,
                    btc_out_point,
                } => {
                    info!(
                        "Onchain receive waiting for confirmation: {btc_deposited} from {btc_out_point:?}"
                    );

                    let recv = storage.get_onchain_receive(operation_id).ok().flatten();

                    // only update txid and send notification, if txid isn't already set
                    // we don't want to do this multiple times
                    if recv.is_none_or(|r| r.txid().is_none()) {
                        let txid = btc_out_point.txid;
                        let params = ReceiveSuccessMsg::Onchain { txid };
                        sender
                            .send(CoreUIMsgPacket {
                                id: Some(msg_id),
                                msg: CoreUIMsg::ReceiveSuccess(params),
                            })
                            .await
                            .unwrap();

                        let fee_sats = 0; // fees for receives may exist one day
                        if let Err(e) = storage.set_onchain_receive_txid(
                            operation_id,
                            txid,
                            btc_deposited.to_sat(),
                            fee_sats,
                        ) {
                            error!("Could not mark onchain payment txid: {e}");
                        }
                    }

                    update_history(storage.clone(), msg_id, &mut sender).await;
                }
                DepositStateV2::Confirmed {
                    btc_deposited,
                    btc_out_point,
                } => {
                    info!("Onchain receive confirmed: {btc_deposited} from {btc_out_point:?}");
                }
                DepositStateV2::Claimed {
                    btc_deposited,
                    btc_out_point,
                } => {
                    info!("Onchain receive claimed: {btc_deposited} from {btc_out_point:?}");
                    let new_balance = client.get_balance().await;
                    sender
                        .send(CoreUIMsgPacket {
                            id: Some(msg_id),
                            msg: CoreUIMsg::FederationBalanceUpdated {
                                id: client.federation_id(),
                                balance: new_balance,
                            },
                        })
                        .await
                        .unwrap();

                    if let Err(e) = storage.mark_onchain_receive_as_confirmed(operation_id) {
                        error!("Could not mark onchain payment txid: {e}");
                    }

                    update_history(storage.clone(), msg_id, &mut sender).await;

                    break;
                }
            }
        }
    });
}

#[derive(Clone)]
pub struct FedimintStorage {
    storage: Arc<dyn DBConnection + Send + Sync>,
    fedimint_memory: Arc<MemDatabase>,
    federation_id: FederationId,
}

impl FedimintStorage {
    pub async fn new(
        storage: Arc<dyn DBConnection + Send + Sync>,
        federation_id: FederationId,
        invite_code: Option<InviteCode>,
    ) -> anyhow::Result<Self> {
        let fedimint_memory = MemDatabase::new();

        // get the fedimint data or create a new fedimint entry if it doesn't exist
        let fedimint_data: Vec<(Vec<u8>, Vec<u8>)> =
            match storage.get_federation_value(federation_id.to_string())? {
                Some(v) => {
                    storage.set_federation_active(federation_id)?;
                    bincode::deserialize(&v)?
                }
                None => {
                    let invite_code = invite_code.ok_or(anyhow::anyhow!("invite_code missing"))?;
                    storage.insert_new_federation(NewFedimint {
                        id: federation_id.to_string(),
                        value: vec![],
                        invite_code: invite_code.to_string(),
                    })?;
                    vec![]
                }
            };

        // get the value and load it into fedimint memory
        if !fedimint_data.is_empty() {
            let mut mem_db_tx = fedimint_memory.begin_transaction().await;
            for (key, value) in fedimint_data {
                mem_db_tx.raw_insert_bytes(&key, &value).await?;
            }
            mem_db_tx.commit_tx().await?;
        }

        Ok(Self {
            storage,
            federation_id,
            fedimint_memory: Arc::new(fedimint_memory),
        })
    }
}

impl fmt::Debug for FedimintStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FedimintDB").finish()
    }
}

#[async_trait]
impl IRawDatabase for FedimintStorage {
    type Transaction<'a> = SQLPseudoTransaction<'a>;

    async fn begin_transaction<'a>(&'a self) -> SQLPseudoTransaction {
        SQLPseudoTransaction {
            storage: self.storage.clone(),
            federation_id: self.federation_id.to_string(),
            mem: self.fedimint_memory.begin_transaction().await,
        }
    }

    fn checkpoint(&self, _backup_path: &Path) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct SQLPseudoTransaction<'a> {
    pub(crate) storage: Arc<dyn DBConnection + Send + Sync>,
    federation_id: String,
    mem: MemTransaction<'a>,
}

impl Debug for SQLPseudoTransaction<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SQLPseudoTransaction")
            .field("federation_id", &self.federation_id)
            .field("mem", &self.mem)
            .finish()
    }
}

#[async_trait]
impl IRawDatabaseTransaction for SQLPseudoTransaction<'_> {
    async fn commit_tx(mut self) -> anyhow::Result<()> {
        let key_value_pairs = self
            .mem
            .raw_find_by_prefix(&[])
            .await?
            .collect::<Vec<(Vec<u8>, Vec<u8>)>>()
            .await;
        self.mem.commit_tx().await?;

        let serialized_data = bincode::serialize(&key_value_pairs).map_err(anyhow::Error::new)?;

        self.storage
            .update_fedimint_data(self.federation_id, serialized_data)
    }
}

#[async_trait]
impl IDatabaseTransactionOpsCore for SQLPseudoTransaction<'_> {
    async fn raw_insert_bytes(
        &mut self,
        key: &[u8],
        value: &[u8],
    ) -> anyhow::Result<Option<Vec<u8>>> {
        self.mem.raw_insert_bytes(key, value).await
    }

    async fn raw_get_bytes(&mut self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        self.mem.raw_get_bytes(key).await
    }

    async fn raw_remove_entry(&mut self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        self.mem.raw_remove_entry(key).await
    }

    async fn raw_find_by_prefix(&mut self, key_prefix: &[u8]) -> anyhow::Result<PrefixStream<'_>> {
        self.mem.raw_find_by_prefix(key_prefix).await
    }

    async fn raw_remove_by_prefix(&mut self, key_prefix: &[u8]) -> anyhow::Result<()> {
        self.mem.raw_remove_by_prefix(key_prefix).await
    }

    async fn raw_find_by_prefix_sorted_descending(
        &mut self,
        key_prefix: &[u8],
    ) -> anyhow::Result<PrefixStream<'_>> {
        self.mem
            .raw_find_by_prefix_sorted_descending(key_prefix)
            .await
    }

    async fn raw_find_by_range(&mut self, range: Range<&[u8]>) -> anyhow::Result<PrefixStream<'_>> {
        self.mem.raw_find_by_range(range).await
    }
}

#[async_trait]
impl IDatabaseTransactionOps for SQLPseudoTransaction<'_> {
    async fn rollback_tx_to_savepoint(&mut self) -> anyhow::Result<()> {
        self.mem.rollback_tx_to_savepoint().await
    }

    async fn set_tx_savepoint(&mut self) -> anyhow::Result<()> {
        self.mem.set_tx_savepoint().await
    }
}
