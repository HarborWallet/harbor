use crate::db::DBConnection;
use crate::db_models::FederationItem;
use crate::db_models::transaction_item::TransactionItem;
use crate::fedimint_client::{
    FederationInviteOrId, FedimintClient, select_gateway, spawn_internal_payment_subscription,
    spawn_invoice_payment_subscription, spawn_invoice_receive_subscription,
    spawn_onchain_payment_subscription, spawn_onchain_receive_subscription,
};
use crate::metadata::{CACHE, FederationData, FederationMeta, get_federation_metadata};
use ::fedimint_client::ClientHandleArc;
use anyhow::anyhow;
use bip39::Mnemonic;
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Network, Txid};
use fedimint_client::{spawn_lnv2_payment_subscription, spawn_lnv2_receive_subscription};
use fedimint_core::Amount;
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::core::{ModuleKind, OperationId};
use fedimint_core::invite_code::InviteCode;
use fedimint_ln_client::{LightningClientModule, PayType};
use fedimint_ln_common::config::FeeToAmount;
use fedimint_ln_common::lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Description};
use fedimint_wallet_client::WalletClientModule;
use futures::{SinkExt, channel::mpsc::Sender};
use lightning_address::make_lnurl_request;
use lnurl::lnurl::LnUrl;
use log::{error, trace};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// The directory where all application data is stored
/// Defaults to ~/.harbor as the root directory
/// Network-specific data goes in ~/.harbor/<network>
pub fn data_dir(network: Option<Network>) -> PathBuf {
    let home = home::home_dir().expect("Could not find home directory");
    let root = home.join(".harbor");
    if let Some(network) = network {
        match network {
            Network::Bitcoin => root.join("bitcoin"),
            Network::Testnet => root.join("testnet3"),
            Network::Testnet4 => root.join("testnet4"),
            Network::Regtest => root.join("regtest"),
            Network::Signet => root.join("signet"),
            _ => panic!("Invalid network"),
        }
    } else {
        root
    }
}

pub mod db;
pub mod db_models;
pub mod fedimint_client;
mod http;
pub mod lightning_address;
pub mod metadata;

#[derive(Debug, Clone)]
pub struct UICoreMsgPacket {
    pub id: Uuid,
    pub msg: UICoreMsg,
}

#[derive(Debug, Clone)]
pub enum UICoreMsg {
    SendLightning {
        federation_id: FederationId,
        invoice: Bolt11Invoice,
    },
    SendLnurlPay {
        federation_id: FederationId,
        lnurl: LnUrl,
        amount_sats: u64,
    },
    ReceiveLightning {
        federation_id: FederationId,
        amount: Amount,
    },
    SendOnChain {
        address: Address<NetworkUnchecked>,
        federation_id: FederationId,
        amount_sats: Option<u64>,
    },
    ReceiveOnChain {
        federation_id: FederationId,
    },
    Transfer {
        to: FederationId,
        from: FederationId,
        amount: Amount,
    },
    GetFederationInfo(InviteCode),
    AddFederation(InviteCode),
    RemoveFederation(FederationId),
    RejoinFederation(FederationId),
    FederationListNeedsUpdate,
    Unlock(String),
    Init {
        password: String,
        seed: Option<String>,
    },
    GetSeedWords,
    SetOnchainReceiveEnabled(bool),
    SetTorEnabled(bool),
    TestStatusUpdates,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SendSuccessMsg {
    Lightning { preimage: [u8; 32] },
    Onchain { txid: Txid },
    Transfer,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReceiveSuccessMsg {
    Lightning,
    Onchain { txid: Txid },
    Transfer,
}

#[derive(Debug, Clone)]
pub struct CoreUIMsgPacket {
    pub id: Option<Uuid>,
    pub msg: CoreUIMsg,
}

#[derive(Debug, Clone)]
pub enum CoreUIMsg {
    Sending,
    SendSuccess(SendSuccessMsg),
    SendFailure(String),
    ReceiveGenerating,
    ReceiveInvoiceGenerated(Bolt11Invoice),
    ReceiveAddressGenerated(Address),
    ReceiveSuccess(ReceiveSuccessMsg),
    ReceiveFailed(String),
    TransferFailure(String),
    TransactionHistoryUpdated(Vec<TransactionItem>),
    FederationBalanceUpdated {
        id: FederationId,
        balance: Amount,
    },
    AddFederationFailed(String),
    RemoveFederationFailed(String),
    FederationInfo {
        config: ClientConfig,
        metadata: FederationMeta,
    },
    AddFederationSuccess,
    RemoveFederationSuccess,
    FederationListNeedsUpdate,
    FederationListUpdated(Vec<FederationItem>),
    NeedsInit,
    Initing,
    InitSuccess,
    InitFailed(String),
    Locked,
    Unlocking,
    UnlockSuccess,
    UnlockFailed(String),
    SeedWords(String),
    OnchainReceiveEnabled(bool),
    TorEnabled(bool),
    InitialProfile {
        seed_words: String,
        onchain_receive_enabled: bool,
        tor_enabled: bool,
    },
    StatusUpdate {
        message: String,
        operation_id: Option<Uuid>,
    },
}

#[derive(Clone)]
#[non_exhaustive]
pub struct HarborCore {
    pub network: Network,
    pub mnemonic: Mnemonic,
    pub tx: Sender<CoreUIMsgPacket>,
    pub clients: Arc<RwLock<HashMap<FederationId, FedimintClient>>>,
    pub storage: Arc<dyn DBConnection + Send + Sync>,
    pub stop: Arc<AtomicBool>,
    pub tor_enabled: Arc<AtomicBool>,
    pub metadata_fetch_cancel: Arc<AtomicBool>,
}

impl HarborCore {
    pub async fn new(
        network: Network,
        mnemonic: Mnemonic,
        tx: Sender<CoreUIMsgPacket>,
        clients: Arc<RwLock<HashMap<FederationId, FedimintClient>>>,
        storage: Arc<dyn DBConnection + Send + Sync>,
        stop: Arc<AtomicBool>,
        tor_enabled: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        // start subscription to pending events
        let pending_onchain_recv = storage.get_pending_onchain_receives()?;
        let pending_onchain_payments = storage.get_pending_onchain_payments()?;
        let pending_lightning_recv = storage.get_pending_lightning_receives()?;
        let pending_lightning_payments = storage.get_pending_lightning_payments()?;

        let c = clients.clone();
        let fed_clients = c.read().await;
        for item in pending_onchain_recv {
            if let Some(client) = fed_clients.get(&item.fedimint_id()) {
                let onchain = client
                    .fedimint_client
                    .get_first_module::<WalletClientModule>()
                    .expect("must have wallet module");

                let op_id = item.operation_id();
                if let Ok(sub) = onchain.subscribe_deposit(op_id).await {
                    spawn_onchain_receive_subscription(
                        tx.clone(),
                        client.fedimint_client.clone(),
                        storage.clone(),
                        op_id,
                        Uuid::nil(),
                        sub,
                    )
                    .await;
                }
            }
        }

        for item in pending_onchain_payments {
            if let Some(client) = fed_clients.get(&item.fedimint_id()) {
                let onchain = client
                    .fedimint_client
                    .get_first_module::<WalletClientModule>()
                    .expect("must have wallet module");

                let op_id = item.operation_id();
                if let Ok(sub) = onchain.subscribe_withdraw_updates(op_id).await {
                    spawn_onchain_payment_subscription(
                        tx.clone(),
                        client.fedimint_client.clone(),
                        storage.clone(),
                        op_id,
                        Uuid::nil(),
                        sub,
                    )
                    .await;
                }
            }
        }

        for item in pending_lightning_recv {
            if let Some(client) = fed_clients.get(&item.fedimint_id()) {
                let lightning_module = client
                    .fedimint_client
                    .get_first_module::<LightningClientModule>()
                    .expect("must have ln module");

                let op_id = item.operation_id();

                if let Ok(sub) = lightning_module.subscribe_ln_receive(op_id).await {
                    spawn_invoice_receive_subscription(
                        tx.clone(),
                        client.fedimint_client.clone(),
                        storage.clone(),
                        op_id,
                        Uuid::nil(),
                        false,
                        sub,
                    )
                    .await;
                }
            }
        }

        for item in pending_lightning_payments {
            if let Some(client) = fed_clients.get(&item.fedimint_id()) {
                let lightning_module = client
                    .fedimint_client
                    .get_first_module::<LightningClientModule>()
                    .expect("must have ln module");

                let op_id = item.operation_id();

                // need to attempt for internal and external subscriptions for lightning payments
                if let Ok(sub) = lightning_module.subscribe_ln_pay(op_id).await {
                    spawn_invoice_payment_subscription(
                        tx.clone(),
                        client.fedimint_client.clone(),
                        storage.clone(),
                        op_id,
                        Uuid::nil(),
                        false,
                        sub,
                    )
                    .await;
                } else if let Ok(sub) = lightning_module.subscribe_internal_pay(op_id).await {
                    spawn_internal_payment_subscription(
                        tx.clone(),
                        client.fedimint_client.clone(),
                        storage.clone(),
                        op_id,
                        Uuid::nil(),
                        sub,
                    )
                    .await;
                }
            }
        }

        Ok(Self {
            network,
            mnemonic,
            tx,
            clients,
            storage,
            stop,
            tor_enabled,
            metadata_fetch_cancel: Arc::new(AtomicBool::new(false)),
        })
    }

    // Initial setup messages that don't have an id
    // Panics if fails to send
    async fn send_system_msg(&self, msg: CoreUIMsg) {
        self.tx
            .clone()
            .send(CoreUIMsgPacket { id: None, msg })
            .await
            .expect("Could not communicate with the UI");
    }

    // Standard core->ui communication with an id
    // Panics if fails to send
    pub async fn msg(&self, id: Uuid, msg: CoreUIMsg) {
        self.tx
            .clone()
            .send(CoreUIMsgPacket { id: Some(id), msg })
            .await
            .expect("Could not communicate with the UI");
    }

    // Convenience method for sending status updates
    pub async fn status_update(&self, id: Uuid, message: &str) {
        self.msg(
            id,
            CoreUIMsg::StatusUpdate {
                message: message.to_string(),
                operation_id: Some(id),
            },
        )
        .await;
    }

    // Sends updates to the UI to reflect the initial state
    pub async fn init_ui_state(&self) -> anyhow::Result<()> {
        let federation_items = self.get_federation_items().await;
        self.send_system_msg(CoreUIMsg::FederationListUpdated(federation_items))
            .await;

        for client in self.clients.read().await.values() {
            let fed_balance = client.fedimint_client.get_balance().await;
            self.send_system_msg(CoreUIMsg::FederationBalanceUpdated {
                id: client.fedimint_client.federation_id(),
                balance: fed_balance,
            })
            .await;
        }

        let history = self.storage.get_transaction_history()?;
        self.send_system_msg(CoreUIMsg::TransactionHistoryUpdated(history))
            .await;

        let profile = self.storage.get_profile()?;
        if let Some(profile) = profile {
            // Send all profile settings in one message
            self.send_system_msg(CoreUIMsg::InitialProfile {
                seed_words: profile.seed_words.clone(),
                onchain_receive_enabled: profile.onchain_receive_enabled(),
                tor_enabled: profile.tor_enabled(),
            })
            .await;
        }

        Ok(())
    }

    async fn get_client(&self, federation_id: FederationId) -> FedimintClient {
        let clients = self.clients.read().await;
        clients
            .get(&federation_id)
            .expect("No client found for federation")
            .clone()
    }

    async fn send_lnv2(
        &self,
        client: &ClientHandleArc,
        msg_id: Uuid,
        invoice: Bolt11Invoice,
    ) -> anyhow::Result<OperationId> {
        log::info!("Trying to pay {invoice} with LNv2...");
        let lnv2_module =
            client.get_first_module::<fedimint_lnv2_client::LightningClientModule>()?;
        let operation_id = lnv2_module.send(invoice.clone(), None, ().into()).await?;
        self.status_update(msg_id, "Creating payment transaction")
            .await;
        Ok(operation_id)
    }

    pub async fn send_lightning(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        invoice: Bolt11Invoice,
        is_transfer: bool,
    ) -> anyhow::Result<()> {
        self.status_update(msg_id, "Preparing to send lightning payment")
            .await;

        log::info!("Paying lightning invoice: {invoice} from federation: {federation_id}");
        if invoice.amount_milli_satoshis().is_none() {
            return Err(anyhow!("Invoice must have an amount"));
        }
        let amount = Amount::from_msats(invoice.amount_milli_satoshis().expect("must have amount"));

        let client = self.get_client(federation_id).await.fedimint_client;

        // Try sending using LNv2 first, if that doesn't work fall back to using LNv1
        match self.send_lnv2(&client, msg_id, invoice.clone()).await {
            Ok(operation_id) => {
                let lnv2_module = client
                    .get_first_module::<fedimint_lnv2_client::LightningClientModule>()
                    .expect("LNv2 should be available");
                let operation = client
                    .operation_log()
                    .get_operation(operation_id)
                    .await
                    .expect("Should have started operation");
                let fedimint_lnv2_client::LightningOperationMeta::Send(meta) =
                    operation.meta::<fedimint_lnv2_client::LightningOperationMeta>()
                else {
                    anyhow::bail!("Operation is not a Lightning payment");
                };

                let fees = meta
                    .contract
                    .amount
                    .checked_sub(amount)
                    .expect("Fees should never be negative");
                self.storage.create_lightning_payment(
                    operation_id,
                    client.federation_id(),
                    invoice,
                    amount,
                    fees,
                )?;

                let sub = lnv2_module
                    .subscribe_send_operation_state_updates(operation_id)
                    .await?;
                spawn_lnv2_payment_subscription(
                    self.tx.clone(),
                    client,
                    self.storage.clone(),
                    operation_id,
                    msg_id,
                    is_transfer,
                    sub,
                )
                .await;
            }
            Err(err) => {
                log::warn!("LNv2 payment failed, trying LNv1. {err}");
                let lightning_module = client
                    .get_first_module::<LightningClientModule>()
                    .expect("must have ln module");

                self.status_update(msg_id, "Selecting gateway and calculating fees")
                    .await;

                let gateway = select_gateway(&client)
                    .await
                    .ok_or(anyhow!("Internal error: No gateway found for federation"))?;

                let fees = gateway.fees.to_amount(&amount);
                let total = fees + amount;
                let balance = client.get_balance().await;
                if total > balance {
                    return Err(anyhow!(
                        "Insufficient balance: Cannot pay {} sats, current balance is only {} sats",
                        total.sats_round_down(),
                        balance.sats_round_down()
                    ));
                }

                log::info!("Sending lightning invoice: {invoice}, paying fees: {fees}");

                // Send another update
                self.status_update(msg_id, "Creating payment transaction")
                    .await;

                let outgoing = lightning_module
                    .pay_bolt11_invoice(Some(gateway), invoice.clone(), ())
                    .await?;

                self.status_update(msg_id, "Waiting for payment confirmation")
                    .await;

                self.storage.create_lightning_payment(
                    outgoing.payment_type.operation_id(),
                    client.federation_id(),
                    invoice,
                    amount,
                    fees,
                )?;

                match outgoing.payment_type {
                    PayType::Internal(op_id) => {
                        let sub = lightning_module.subscribe_internal_pay(op_id).await?;
                        spawn_internal_payment_subscription(
                            self.tx.clone(),
                            client,
                            self.storage.clone(),
                            op_id,
                            msg_id,
                            sub,
                        )
                        .await;
                    }
                    PayType::Lightning(op_id) => {
                        let sub = lightning_module.subscribe_ln_pay(op_id).await?;
                        spawn_invoice_payment_subscription(
                            self.tx.clone(),
                            client,
                            self.storage.clone(),
                            op_id,
                            msg_id,
                            is_transfer,
                            sub,
                        )
                        .await;
                    }
                }
            }
        }

        log::info!("Payment sent");

        Ok(())
    }

    pub async fn send_lnurl_pay(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        lnurl: LnUrl,
        amount_sats: u64,
    ) -> anyhow::Result<()> {
        self.status_update(msg_id, "Starting LNURL-pay flow").await;

        log::info!("Sending lnurl pay: {lnurl} from federation: {federation_id}");

        let tor_enabled = self.tor_enabled.load(Ordering::Relaxed);
        self.status_update(msg_id, "Fetching payment details from recipient")
            .await;

        let pay_response =
            make_lnurl_request(&lnurl, tor_enabled, self.metadata_fetch_cancel.clone()).await?;
        log::info!("Pay response: {pay_response:?}");

        self.status_update(msg_id, "Requesting invoice from recipient")
            .await;

        let amount_msats = amount_sats * 1000;
        let invoice_response = lightning_address::get_invoice(
            &pay_response,
            amount_msats,
            tor_enabled,
            self.metadata_fetch_cancel.clone(),
        )
        .await?;
        log::info!("Invoice response: {invoice_response:?}");

        let invoice =
            fedimint_ln_common::lightning_invoice::Bolt11Invoice::from_str(&invoice_response.pr)?;

        // Now we'll let send_lightning handle the rest of the status updates
        self.send_lightning(msg_id, federation_id, invoice, false)
            .await?;

        Ok(())
    }

    async fn receive_lnv2(
        &self,
        client: &ClientHandleArc,
        msg_id: Uuid,
        amount: Amount,
    ) -> anyhow::Result<(Bolt11Invoice, OperationId)> {
        log::info!("Trying to pay receive {amount} with LNv2...");
        let lnv2_module =
            client.get_first_module::<fedimint_lnv2_client::LightningClientModule>()?;
        const DEFAULT_EXPIRY_TIME_SECS: u32 = 86400;
        self.status_update(msg_id, "Generating invoice").await;
        let receive = lnv2_module
            .receive(
                amount,
                DEFAULT_EXPIRY_TIME_SECS,
                fedimint_lnv2_common::Bolt11InvoiceDescription::Direct(String::new()),
                None,
                ().into(),
            )
            .await?;
        Ok(receive)
    }

    pub async fn receive_lightning(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        amount: Amount,
        is_transfer: bool,
    ) -> anyhow::Result<Bolt11Invoice> {
        let tor_enabled = self.tor_enabled.load(Ordering::Relaxed);
        log::info!(
            "Creating lightning invoice, amount: {amount} for federation: {federation_id}. Tor enabled: {tor_enabled}"
        );

        let client = self.get_client(federation_id).await.fedimint_client;
        match self.receive_lnv2(&client, msg_id, amount).await {
            Ok((invoice, operation_id)) => {
                let operation = client
                    .operation_log()
                    .get_operation(operation_id)
                    .await
                    .expect("Should have started operation");
                let fedimint_lnv2_client::LightningOperationMeta::Receive(meta) =
                    operation.meta::<fedimint_lnv2_client::LightningOperationMeta>()
                else {
                    anyhow::bail!("Operation is not a Lightning payment");
                };

                let fees = amount
                    .checked_sub(meta.contract.commitment.amount)
                    .expect("Fees should not be negative");

                log::info!("LNv2 Invoice created: {invoice}");

                self.storage.create_ln_receive(
                    operation_id,
                    client.federation_id(),
                    invoice.clone(),
                    amount,
                    fees,
                    [0; 32], // We don't know the preimage because the gateway generated the invoice
                )?;

                let lnv2_module = client
                    .get_first_module::<fedimint_lnv2_client::LightningClientModule>()
                    .expect("Should have LNv2 module");
                let sub = lnv2_module
                    .subscribe_receive_operation_state_updates(operation_id)
                    .await?;
                spawn_lnv2_receive_subscription(
                    self.tx.clone(),
                    client.clone(),
                    self.storage.clone(),
                    operation_id,
                    msg_id,
                    is_transfer,
                    sub,
                )
                .await;
                Ok(invoice)
            }
            Err(err) => {
                log::warn!("LNv2 invoice generation failed, trying LNv1. {err}");
                self.status_update(msg_id, "Connecting to mint").await;

                let lightning_module = client
                    .get_first_module::<LightningClientModule>()
                    .expect("must have ln module");
                log::info!("Lightning module: {:?}", lightning_module.id);

                self.status_update(msg_id, "Selecting gateway").await;

                let gateway = select_gateway(&client)
                    .await
                    .ok_or(anyhow!("Internal error: No gateway found for federation"))?;
                log::info!("Gateway: {gateway:?}");

                self.status_update(msg_id, "Generating invoice").await;

                let desc = Description::new(String::new()).expect("empty string is valid");
                let (op_id, invoice, preimage) = lightning_module
                    .create_bolt11_invoice(
                        amount,
                        Bolt11InvoiceDescription::Direct(&desc),
                        None,
                        (),
                        Some(gateway),
                    )
                    .await?;

                log::info!("Invoice created: {invoice}");

                self.storage.create_ln_receive(
                    op_id,
                    client.federation_id(),
                    invoice.clone(),
                    amount,
                    Amount::ZERO, // todo one day there will be receive fees
                    preimage,
                )?;

                // Create subscription to operation if it exists
                match lightning_module.subscribe_ln_receive(op_id).await {
                    Ok(subscription) => {
                        spawn_invoice_receive_subscription(
                            self.tx.clone(),
                            client.clone(),
                            self.storage.clone(),
                            op_id,
                            msg_id,
                            is_transfer,
                            subscription,
                        )
                        .await;
                    }
                    _ => {
                        error!("Could not create subscription to lightning receive");
                    }
                }

                Ok(invoice)
            }
        }
    }

    pub async fn transfer(
        &self,
        msg_id: Uuid,
        to: FederationId,
        from: FederationId,
        amount: Amount,
    ) -> anyhow::Result<()> {
        log::info!("Transferring {amount} from {from} to {to}");

        self.status_update(msg_id, "Generating invoice on destination mint")
            .await;

        let invoice = self.receive_lightning(msg_id, to, amount, true).await?;

        self.status_update(msg_id, "Paying invoice from source mint")
            .await;

        self.send_lightning(msg_id, from, invoice, true).await?;
        Ok(())
    }

    /// Sends a given amount of sats to a given address, if the amount is None, send all funds
    pub async fn send_onchain(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        address: Address<NetworkUnchecked>,
        sats: Option<u64>,
    ) -> anyhow::Result<()> {
        let address = address
            .require_network(self.network)
            .map_err(|_| anyhow!("Address is for wrong network"))?;

        log::info!(
            "Sending onchain payment to address: {address} from federation: {federation_id}",
        );
        let client = self.get_client(federation_id).await.fedimint_client;
        let onchain = client
            .get_first_module::<WalletClientModule>()
            .expect("must have wallet module");

        let (fees, amount) = match sats {
            Some(sats) => {
                let amount = bitcoin::Amount::from_sat(sats);
                let fees = onchain.get_withdraw_fees(&address, amount).await?;
                (fees, amount)
            }
            None => {
                let balance = client.get_balance().await;

                if balance.sats_round_down() == 0 {
                    return Err(anyhow!("No funds in wallet"));
                }

                // get fees for the entire balance
                let fees = onchain
                    .get_withdraw_fees(
                        &address,
                        bitcoin::Amount::from_sat(balance.sats_round_down()),
                    )
                    .await?;

                let fees_paid = Amount::from_sats(fees.amount().to_sat());
                let amount = balance.saturating_sub(fees_paid);

                if amount.sats_round_down() < 546 {
                    return Err(anyhow!("Not enough funds to send"));
                }

                (fees, bitcoin::Amount::from_sat(amount.sats_round_down()))
            }
        };

        let total = fees.amount() + amount;
        let balance = client.get_balance().await;
        if total > bitcoin::Amount::from_sat(balance.sats_round_down()) {
            return Err(anyhow!(
                "Insufficient balance: Cannot pay {} sats, current balance is only {} sats",
                total.to_sat(),
                balance.sats_round_down()
            ));
        }

        let op_id = onchain.withdraw(&address, amount, fees, ()).await?;

        self.storage.create_onchain_payment(
            op_id,
            client.federation_id(),
            address,
            amount.to_sat(),
            fees.amount().to_sat(),
        )?;

        let sub = onchain.subscribe_withdraw_updates(op_id).await?;

        spawn_onchain_payment_subscription(
            self.tx.clone(),
            client.clone(),
            self.storage.clone(),
            op_id,
            msg_id,
            sub,
        )
        .await;

        Ok(())
    }

    pub async fn receive_onchain(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
    ) -> anyhow::Result<Address> {
        // check if on-chain receive is enabled
        let profile = self.storage.get_profile()?;
        if profile.is_none() || !profile.unwrap().onchain_receive_enabled() {
            return Err(anyhow!("on-chain receive is not enabled"));
        }

        log::info!("Generating address for federation: {federation_id}");

        self.status_update(msg_id, "Connecting to mint").await;

        let client = self.get_client(federation_id).await.fedimint_client;
        let onchain = client
            .get_first_module::<WalletClientModule>()
            .expect("must have wallet module");

        self.status_update(msg_id, "Generating address").await;

        let (op_id, address, _) = onchain.allocate_deposit_address_expert_only(()).await?;

        self.storage
            .create_onchain_receive(op_id, client.federation_id(), address.clone())?;

        let sub = onchain.subscribe_deposit(op_id).await?;

        spawn_onchain_receive_subscription(
            self.tx.clone(),
            client.clone(),
            self.storage.clone(),
            op_id,
            msg_id,
            sub,
        )
        .await;

        Ok(address)
    }

    pub async fn get_federation_info(
        &self,
        msg_id: Uuid,
        invite_code: InviteCode,
    ) -> anyhow::Result<(ClientConfig, FederationMeta)> {
        log::info!("Getting federation info for invite code: {invite_code}");

        self.status_update(msg_id, "Connecting to mint").await;

        let tor_enabled = self.tor_enabled.load(Ordering::Relaxed);
        let download = Instant::now();
        let config = {
            let connector = if tor_enabled {
                fedimint_api_client::api::net::Connector::Tor
            } else {
                fedimint_api_client::api::net::Connector::Tcp
            };
            connector
                .download_from_invite_code(&invite_code)
                .await
                .map_err(|e| {
                    error!("Could not download federation info: {e}");
                    e
                })?
        };
        trace!(
            "Downloaded federation info in: {}ms",
            download.elapsed().as_millis()
        );

        self.status_update(msg_id, "Retrieving mint metadata").await;

        let mut cache = CACHE.write().await;
        let metadata = match cache.get(&invite_code.federation_id()).cloned() {
            None => {
                let m = get_federation_metadata(
                    FederationData::Config(&config),
                    tor_enabled,
                    self.metadata_fetch_cancel.clone(),
                )
                .await;
                cache.insert(invite_code.federation_id(), m.clone());
                m
            }
            Some(metadata) => metadata,
        };

        Ok((config, metadata))
    }

    pub async fn add_federation(
        &self,
        msg_id: Uuid,
        invite_code: InviteCode,
    ) -> anyhow::Result<()> {
        log::info!("Adding federation with invite code: {invite_code}");
        let id = invite_code.federation_id();

        self.status_update(msg_id, "Starting mint setup").await;

        let mut clients = self.clients.write().await;
        if clients.get(&id).is_some() {
            return Err(anyhow!("Federation already added"));
        }

        self.status_update(msg_id, "Initializing mint connection")
            .await;

        let client = FedimintClient::new(
            self.storage.clone(),
            FederationInviteOrId::Invite(invite_code.clone()),
            &self.mnemonic,
            self.network,
            self.stop.clone(),
        )
        .await?;

        self.status_update(msg_id, "Registering with mint").await;

        clients.insert(id, client.clone());

        let tx = self.tx.clone();
        let tor_enabled = self.tor_enabled.load(Ordering::Relaxed);
        let metadata_fetch_cancel = self.metadata_fetch_cancel.clone();
        let storage = self.storage.clone();
        tokio::task::spawn(async move {
            Self::update_mint_metadata(
                vec![client.fedimint_client],
                metadata_fetch_cancel,
                tor_enabled,
                storage,
                tx,
            )
            .await;
        });

        self.status_update(msg_id, "Mint setup complete!").await;

        Ok(())
    }

    pub async fn remove_federation(&self, _msg_id: Uuid, id: FederationId) -> anyhow::Result<()> {
        log::info!("Removing federation with id: {id}");

        // Cancel any ongoing metadata fetch
        self.metadata_fetch_cancel.store(true, Ordering::Relaxed);

        // Small delay to allow any in-progress operations to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut clients = self.clients.write().await;

        // Check if federation exists before attempting removal
        if !clients.contains_key(&id) {
            return Err(anyhow!("Federation doesn't exist"));
        }

        // Remove from clients first
        clients.remove(&id);
        drop(clients);

        // Then remove from storage
        self.storage.remove_federation(id)?;

        // Reset cancellation flag
        self.metadata_fetch_cancel.store(false, Ordering::Relaxed);

        log::info!("Successfully removed federation: {id}");
        Ok(())
    }

    pub async fn get_federation_items(&self) -> Vec<FederationItem> {
        let clients = self.clients.read().await;

        let metadata_cache = CACHE.read().await;

        let mut needs_metadata = vec![];

        // Tell the UI about any clients we have
        let mut res = Vec::with_capacity(clients.len());
        for c in clients.values() {
            let balance = c.fedimint_client.get_balance().await;
            let config = c.fedimint_client.config().await;

            let guardians: Vec<String> = config
                .global
                .api_endpoints
                .values()
                .map(|url| url.name.clone())
                .collect();

            let module_kinds = config
                .modules
                .into_values()
                .map(|module_config| module_config.kind().to_owned())
                .collect::<Vec<ModuleKind>>();

            // get metadata from in memory cache
            let metadata = metadata_cache
                .get(&c.fedimint_client.federation_id())
                .cloned()
                // if not in cache, get from db
                .or_else(|| {
                    needs_metadata.push(c.fedimint_client.clone());
                    self.storage
                        .get_federation_metadata(c.fedimint_client.federation_id())
                        .ok()
                        .flatten()
                });

            let on_chain_supported =
                match c.fedimint_client.get_first_module::<WalletClientModule>() {
                    Ok(w) => w.supports_safe_deposit().await,
                    Err(_) => false,
                };

            res.push(FederationItem {
                id: c.fedimint_client.federation_id(),
                name: c
                    .fedimint_client
                    .get_config_meta("federation_name")
                    .unwrap_or("Unknown".to_string()),
                balance: balance.sats_round_down(),
                guardians: Some(guardians),
                module_kinds: Some(module_kinds),
                metadata: metadata.unwrap_or_default(),
                on_chain_supported,
                active: true,
            });
        }

        drop(metadata_cache);

        // if we're missing metadata for federations, start background task to populate it
        if !needs_metadata.is_empty() {
            let tx = self.tx.clone();
            let tor_enabled = self.tor_enabled.load(Ordering::Relaxed);
            let metadata_fetch_cancel = self.metadata_fetch_cancel.clone();
            let storage = self.storage.clone();
            tokio::task::spawn(async move {
                Self::update_mint_metadata(
                    needs_metadata,
                    metadata_fetch_cancel,
                    tor_enabled,
                    storage,
                    tx,
                )
                .await;
            });
        }

        // get archived mints
        let archived = self.storage.get_archived_mints().expect("archived mints");
        for m in archived {
            let item = FederationItem {
                id: FederationId::from_str(&m.id).unwrap(),
                name: m.name.clone().unwrap_or("Unknown".to_string()),
                balance: 0,
                guardians: None,
                module_kinds: None,
                metadata: m.into(),
                on_chain_supported: false,
                active: false,
            };
            res.push(item);
        }

        res
    }

    async fn update_mint_metadata(
        needs_metadata: Vec<ClientHandleArc>,
        metadata_fetch_cancel: Arc<AtomicBool>,
        tor_enabled: bool,
        storage: Arc<dyn DBConnection + Send + Sync>,
        mut tx: Sender<CoreUIMsgPacket>,
    ) {
        let mut w = CACHE.write().await;
        for client in needs_metadata {
            // Check if we should cancel
            if metadata_fetch_cancel.load(Ordering::Relaxed) {
                break;
            }
            let id = client.federation_id();
            let metadata = get_federation_metadata(
                FederationData::Client(&client),
                tor_enabled,
                metadata_fetch_cancel.clone(),
            )
            .await;
            w.insert(id, metadata.clone());
            storage.upsert_federation_metadata(id, metadata).ok();
            log::info!("Saved federation metadata: {id}");
        }
        drop(w);

        // Only update the UI if we weren't cancelled
        if !metadata_fetch_cancel.load(Ordering::Relaxed) {
            // update list in front end
            tx.send(CoreUIMsgPacket {
                id: None,
                msg: CoreUIMsg::FederationListNeedsUpdate,
            })
            .await
            .expect("federation list needs update");
        }
    }

    pub async fn get_seed_words(&self) -> String {
        self.mnemonic.to_string()
    }

    pub async fn set_onchain_receive_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        log::info!("Setting on-chain receive enabled to: {}", enabled);
        self.storage.set_onchain_receive_enabled(enabled)?;
        log::info!(
            "Successfully {} on-chain receive",
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    }

    pub async fn set_tor_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        log::info!("Setting Tor enabled to: {}", enabled);
        self.tor_enabled.swap(enabled, Ordering::Relaxed);
        self.storage.set_tor_enabled(enabled)?;
        log::info!(
            "Successfully {} Tor",
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    }

    pub async fn test_status_updates(&self, msg_id: Uuid) {
        self.status_update(msg_id, "Starting test sequence").await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        self.status_update(msg_id, "Phase 1: Initializing test")
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        self.status_update(msg_id, "Phase 2: Running calculations")
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        self.status_update(msg_id, "Phase 3: Almost there").await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        self.status_update(msg_id, "Test sequence complete!").await;
    }
}
