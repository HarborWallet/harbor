use crate::db::DBConnection;
use crate::db_models::transaction_item::TransactionItem;
use crate::db_models::FederationItem;
use crate::fedimint_client::{
    select_gateway, spawn_internal_payment_subscription, spawn_invoice_payment_subscription,
    spawn_invoice_receive_subscription, spawn_onchain_payment_subscription,
    spawn_onchain_receive_subscription, FederationInviteOrId, FedimintClient,
};
use anyhow::anyhow;
use bip39::Mnemonic;
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Network, Txid};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::core::ModuleKind;
use fedimint_core::invite_code::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_client::{LightningClientModule, PayType};
use fedimint_ln_common::config::FeeToAmount;
use fedimint_ln_common::lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Description};
use fedimint_wallet_client::WalletClientModule;
use futures::future::join_all;
use futures::{channel::mpsc::Sender, SinkExt};
use log::{error, trace};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

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
        _ => panic!("Invalid network"),
    }
}

pub mod db;
pub mod db_models;
pub mod fedimint_client;

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
    Unlock(String),
    Init {
        password: String,
        seed: Option<String>,
    },
    GetSeedWords,
    SetOnchainReceiveEnabled(bool),
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
    // todo probably want a way to incrementally add items to the history
    TransactionHistoryUpdated(Vec<TransactionItem>),
    FederationBalanceUpdated { id: FederationId, balance: Amount },
    AddFederationFailed(String),
    RemoveFederationFailed(String),
    FederationInfo(ClientConfig),
    AddFederationSuccess,
    RemoveFederationSuccess,
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
}

#[derive(Clone)]
pub struct HarborCore {
    pub network: Network,
    pub mnemonic: Mnemonic,
    pub tx: Sender<CoreUIMsgPacket>,
    pub clients: Arc<RwLock<HashMap<FederationId, FedimintClient>>>,
    pub storage: Arc<dyn DBConnection + Send + Sync>,
    pub stop: Arc<AtomicBool>,
}

impl HarborCore {
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

    // Sends updates to the UI to refelect the initial state
    pub async fn init_ui_state(&self) -> anyhow::Result<()> {
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

        let federation_items = self.get_federation_items().await;
        self.send_system_msg(CoreUIMsg::FederationListUpdated(federation_items))
            .await;

        let profile = self.storage.get_profile()?;
        if let Some(profile) = profile {
            self.send_system_msg(CoreUIMsg::OnchainReceiveEnabled(
                profile.onchain_receive_enabled(),
            ))
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

    pub async fn send_lightning(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        invoice: Bolt11Invoice,
        is_transfer: bool,
    ) -> anyhow::Result<()> {
        if invoice.amount_milli_satoshis().is_none() {
            return Err(anyhow!("Invoice must have an amount"));
        }
        let amount = Amount::from_msats(invoice.amount_milli_satoshis().expect("must have amount"));

        let client = self.get_client(federation_id).await.fedimint_client;
        let lightning_module = client
            .get_first_module::<LightningClientModule>()
            .expect("must have ln module");

        let gateway = select_gateway(&client)
            .await
            .ok_or(anyhow!("Internal error: No gateway found for federation"))?;

        let fees = gateway.fees.to_amount(&amount);

        log::info!("Sending lightning invoice: {invoice}, paying fees: {fees}");

        let outgoing = lightning_module
            .pay_bolt11_invoice(Some(gateway), invoice.clone(), ())
            .await?;

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
                    client.clone(),
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
                    client.clone(),
                    self.storage.clone(),
                    op_id,
                    msg_id,
                    is_transfer,
                    sub,
                )
                .await;
            }
        }

        log::info!("Invoice sent");

        Ok(())
    }

    pub async fn receive_lightning(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        amount: Amount,
        is_transfer: bool,
    ) -> anyhow::Result<Bolt11Invoice> {
        let client = self.get_client(federation_id).await.fedimint_client;
        let lightning_module = client
            .get_first_module::<LightningClientModule>()
            .expect("must have ln module");

        let gateway = select_gateway(&client)
            .await
            .ok_or(anyhow!("Internal error: No gateway found for federation"))?;

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
        if let Ok(subscription) = lightning_module.subscribe_ln_receive(op_id).await {
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
        } else {
            error!("Could not create subscription to lightning receive");
        }

        Ok(invoice)
    }

    pub async fn transfer(
        &self,
        msg_id: Uuid,
        to: FederationId,
        from: FederationId,
        amount: Amount,
    ) -> anyhow::Result<()> {
        log::info!("Transferring {amount} from {from} to {to}");
        let invoice = self.receive_lightning(msg_id, to, amount, true).await?;
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
        let client = self.get_client(federation_id).await.fedimint_client;
        let onchain = client
            .get_first_module::<WalletClientModule>()
            .expect("must have wallet module");

        let (fees, amount) = match sats {
            Some(sats) => {
                let amount = bitcoin::Amount::from_sat(sats);
                let fees = onchain.get_withdraw_fees(address.clone(), amount).await?;
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
                        address.clone(),
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

        let op_id = onchain.withdraw(address.clone(), amount, fees, ()).await?;

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

        let client = self.get_client(federation_id).await.fedimint_client;
        let onchain = client
            .get_first_module::<WalletClientModule>()
            .expect("must have wallet module");

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
        invite_code: InviteCode,
    ) -> anyhow::Result<ClientConfig> {
        let download = Instant::now();
        let config = {
            #[cfg(feature = "disable-tor")]
            let config = fedimint_api_client::api::net::Connector::Tcp
                .download_from_invite_code(&invite_code)
                .await
                .map_err(|e| {
                    error!("Could not download federation info: {e}");
                    e
                })?;
            #[cfg(not(feature = "disable-tor"))]
            let config = fedimint_api_client::api::net::Connector::Tor
                .download_from_invite_code(&invite_code)
                .await
                .map_err(|e| {
                    error!("Could not download federation info: {e}");
                    e
                })?;
            config
        };
        trace!(
            "Downloaded federation info in: {}ms",
            download.elapsed().as_millis()
        );

        Ok(config)
    }

    pub async fn add_federation(&self, invite_code: InviteCode) -> anyhow::Result<()> {
        let id = invite_code.federation_id();

        let mut clients = self.clients.write().await;
        if clients.get(&id).is_some() {
            return Err(anyhow!("Federation already added"));
        }

        let client = FedimintClient::new(
            self.storage.clone(),
            FederationInviteOrId::Invite(invite_code),
            &self.mnemonic,
            self.network,
            self.stop.clone(),
        )
        .await?;

        clients.insert(client.fedimint_client.federation_id(), client);

        Ok(())
    }

    pub async fn remove_federation(&self, id: FederationId) -> anyhow::Result<()> {
        let mut clients = self.clients.write().await;
        if clients.remove(&id).is_none() {
            return Err(anyhow!("Federation doesn't exist"));
        }

        self.storage.remove_federation(id)?;

        Ok(())
    }

    pub async fn get_federation_items(&self) -> Vec<FederationItem> {
        let clients = self.clients.read().await;

        // Tell the UI about any clients we have
        join_all(clients.values().map(|c| async {
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

            FederationItem {
                id: c.fedimint_client.federation_id(),
                name: c
                    .fedimint_client
                    .get_meta("federation_name")
                    .unwrap_or("Unknown".to_string()),
                balance: balance.sats_round_down(),
                guardians: Some(guardians),
                module_kinds: Some(module_kinds),
            }
        }))
        .await
    }

    pub async fn get_seed_words(&self) -> String {
        self.mnemonic.to_string()
    }

    pub async fn set_onchain_receive_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        self.storage.set_onchain_receive_enabled(enabled)?;
        Ok(())
    }
}
