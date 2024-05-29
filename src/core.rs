use anyhow::anyhow;
use bip39::Mnemonic;
use bitcoin::{Address, Network};
use fedimint_core::api::InviteCode;
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::Amount;
use fedimint_ln_client::{LightningClientModule, PayType};
use fedimint_ln_common::config::FeeToAmount;
use fedimint_ln_common::lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Description};
use fedimint_wallet_client::WalletClientModule;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use iced::{
    futures::{channel::mpsc::Sender, SinkExt},
    subscription::{self, Subscription},
};
use log::{error, info, trace, warn};
use tokio::sync::RwLock;
use tokio::task::spawn_blocking;
use uuid::Uuid;

use crate::components::FederationItem;
use crate::db::check_password;
use crate::fedimint_client::{
    spawn_onchain_payment_subscription, spawn_onchain_receive_subscription, FederationInviteOrId,
};
use crate::{
    bridge::{self, CoreUIMsg, UICoreMsg},
    conf::{self, get_mnemonic},
    db::DBConnection,
    Message,
};
use crate::{
    db::setup_db,
    fedimint_client::{
        select_gateway, spawn_internal_payment_subscription, spawn_invoice_payment_subscription,
        spawn_invoice_receive_subscription, FedimintClient,
    },
};

const PEG_IN_TIMEOUT_YEAR: Duration = Duration::from_secs(86400 * 365);

#[derive(Clone)]
struct HarborCore {
    network: Network,
    mnemonic: Mnemonic,
    tx: Sender<Message>,
    clients: Arc<RwLock<HashMap<FederationId, FedimintClient>>>,
    storage: Arc<dyn DBConnection + Send + Sync>,
    stop: Arc<AtomicBool>,
}

impl HarborCore {
    async fn msg(&self, id: Option<Uuid>, msg: CoreUIMsg) {
        self.tx
            .clone()
            .send(Message::core_msg(id, msg))
            .await
            .unwrap();
    }

    // Sends updates to the UI to refelect the initial state
    async fn init_ui_state(&self) {
        let mut balance = Amount::ZERO;
        for client in self.clients.read().await.values() {
            balance += client.fedimint_client.get_balance().await;
        }

        self.msg(None, CoreUIMsg::BalanceUpdated(balance)).await;

        let history = self.storage.get_transaction_history().unwrap();
        self.msg(None, CoreUIMsg::TransactionHistoryUpdated(history))
            .await;

        let federation_items = self.get_federation_items().await;
        self.msg(None, CoreUIMsg::FederationListUpdated(federation_items))
            .await;
    }

    // todo for now just use the first client, but eventually we'll want to have a way to select a client
    async fn get_client(&self) -> FedimintClient {
        self.clients.read().await.values().next().unwrap().clone()
    }

    async fn send_lightning(&self, msg_id: Uuid, invoice: Bolt11Invoice) -> anyhow::Result<()> {
        if invoice.amount_milli_satoshis().is_none() {
            return Err(anyhow!("Invoice must have an amount"));
        }
        let amount = Amount::from_msats(invoice.amount_milli_satoshis().expect("must have amount"));

        // todo go through all clients and select the first one that has enough balance
        let client = self.get_client().await.fedimint_client;
        let lightning_module = client.get_first_module::<LightningClientModule>();

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
                    sub,
                )
                .await;
            }
        }

        log::info!("Invoice sent");

        Ok(())
    }

    async fn receive_lightning(
        &self,
        msg_id: Uuid,
        amount: Amount,
    ) -> anyhow::Result<Bolt11Invoice> {
        let client = self.get_client().await.fedimint_client;
        let lightning_module = client.get_first_module::<LightningClientModule>();

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
                subscription,
            )
            .await;
        } else {
            error!("Could not create subscription to lightning receive");
        }

        Ok(invoice)
    }

    /// Sends a given amount of sats to a given address, if the amount is None, send all funds
    async fn send_onchain(
        &self,
        msg_id: Uuid,
        address: Address,
        sats: Option<u64>,
    ) -> anyhow::Result<()> {
        // todo go through all clients and select the first one that has enough balance
        let client = self.get_client().await.fedimint_client;
        let onchain = client.get_first_module::<WalletClientModule>();

        // todo add manual fee selection
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

    async fn receive_onchain(&self, msg_id: Uuid) -> anyhow::Result<Address> {
        // todo add federation id selection
        let client = self.get_client().await.fedimint_client;
        let onchain = client.get_first_module::<WalletClientModule>();

        // expire the address in 1 year
        let valid_until = SystemTime::now() + PEG_IN_TIMEOUT_YEAR;

        let (op_id, address) = onchain.get_deposit_address(valid_until, ()).await?;

        self.storage
            .create_onchain_receive(op_id, client.federation_id(), address.clone())?;

        let sub = onchain.subscribe_deposit_updates(op_id).await?;

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

    async fn get_federation_info(&self, invite_code: InviteCode) -> anyhow::Result<ClientConfig> {
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

        Ok(config)
    }

    async fn add_federation(&self, invite_code: InviteCode) -> anyhow::Result<()> {
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

    async fn get_federation_items(&self) -> Vec<FederationItem> {
        let clients = self.clients.read().await;

        // Tell the UI about any clients we have
        clients
            .values()
            .map(|c| FederationItem {
                id: c.fedimint_client.federation_id(),
                name: c
                    .fedimint_client
                    .get_meta("federation_name")
                    .unwrap_or("Unknown".to_string()),
                // TODO: get the balance per fedimint
                balance: 420,
            })
            .collect::<Vec<FederationItem>>()
    }

    async fn get_seed_words(&self) -> String {
        self.mnemonic.to_string()
    }
}

pub fn run_core() -> Subscription<Message> {
    struct Connect;
    subscription::channel(
        std::any::TypeId::of::<Connect>(),
        100,
        |mut tx: Sender<Message>| async move {
            // Setup UI Handle
            let (ui_handle, mut core_handle) = bridge::create_handles();
            let arc_ui_handle = Arc::new(ui_handle);
            tx.send(Message::UIHandlerLoaded(arc_ui_handle))
                .await
                .expect("should send");

            let network = Network::Signet;

            // Create the datadir if it doesn't exist
            let path = PathBuf::from(&conf::data_dir(network));
            std::fs::create_dir_all(path.clone()).expect("Could not create datadir");
            log::info!("Using datadir: {path:?}");

            loop {
                let msg = core_handle.recv().await;

                let id = msg.as_ref().map(|m| m.id);

                match msg.map(|m| m.msg) {
                    Some(UICoreMsg::Unlock(password)) => {
                        log::info!("Sending unlock message");
                        tx.send(Message::core_msg(id, CoreUIMsg::Unlocking))
                            .await
                            .expect("should send");

                        // attempting to unlock
                        let db_path = path.join("harbor.sqlite");

                        let db_path = db_path.to_str().unwrap().to_string();

                        // if the db file doesn't exist, dont call check_password
                        if !std::path::Path::new(&db_path).exists() {
                            info!("Database does not exist, it will be created");
                        } else {
                            if let Err(e) = check_password(&db_path, &password) {
                                // probably invalid password
                                error!("error using password: {e}");

                                tx.send(Message::core_msg(
                                    id,
                                    CoreUIMsg::UnlockFailed(e.to_string()),
                                ))
                                .await
                                .expect("should send");
                                continue;
                            }

                            log::info!("Correct password");
                        }

                        let db = spawn_blocking(move || setup_db(&db_path, password))
                            .await
                            .expect("Could not create join handle");

                        if let Err(e) = db {
                            error!("error opening database: {e}");

                            tx.send(Message::core_msg(
                                id,
                                CoreUIMsg::UnlockFailed(e.to_string()),
                            ))
                            .await
                            .expect("should send");
                            continue;
                        }
                        let db = db.expect("no error");

                        let mnemonic = get_mnemonic(db.clone()).expect("should get seed");

                        let stop = Arc::new(AtomicBool::new(false));

                        // check db for fedimints
                        let mut clients = HashMap::new();
                        let federation_ids = db
                            .list_federations()
                            .expect("should load initial fedimints");
                        for f in federation_ids {
                            let client = FedimintClient::new(
                                db.clone(),
                                FederationInviteOrId::Id(
                                    FederationId::from_str(&f).expect("should parse federation id"),
                                ),
                                &mnemonic,
                                network,
                                stop.clone(),
                            )
                            .await
                            .expect("Could not create fedimint client");

                            clients.insert(client.fedimint_client.federation_id(), client);
                        }

                        let core = HarborCore {
                            storage: db.clone(),
                            tx: tx.clone(),
                            mnemonic,
                            network,
                            clients: Arc::new(RwLock::new(clients)),
                            stop,
                        };

                        tx.send(Message::core_msg(id, CoreUIMsg::UnlockSuccess))
                            .await
                            .expect("should send");

                        process_core(&mut core_handle, &core).await;
                    }
                    _ => {
                        warn!("Ignoring unrelated message to locked core")
                    }
                }
            }
        },
    )
}

async fn process_core(core_handle: &mut bridge::CoreHandle, core: &HarborCore) {
    // Initialize the ui's state
    core.init_ui_state().await;

    loop {
        let msg = core_handle.recv().await;

        let core = core.clone();
        tokio::spawn(async move {
            if let Some(msg) = msg {
                match msg.msg {
                    UICoreMsg::SendLightning(invoice) => {
                        log::info!("Got UICoreMsg::Send");
                        core.msg(Some(msg.id), CoreUIMsg::Sending).await;
                        if let Err(e) = core.send_lightning(msg.id, invoice).await {
                            error!("Error sending: {e}");
                            core.msg(Some(msg.id), CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveLightning(amount) => {
                        core.msg(Some(msg.id), CoreUIMsg::ReceiveGenerating).await;
                        match core.receive_lightning(msg.id, amount).await {
                            Err(e) => {
                                core.msg(Some(msg.id), CoreUIMsg::ReceiveFailed(e.to_string()))
                                    .await;
                            }
                            Ok(invoice) => {
                                core.msg(Some(msg.id), CoreUIMsg::ReceiveInvoiceGenerated(invoice))
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::SendOnChain {
                        address,
                        amount_sats,
                    } => {
                        log::info!("Got UICoreMsg::SendOnChain");
                        core.msg(Some(msg.id), CoreUIMsg::Sending).await;
                        if let Err(e) = core.send_onchain(msg.id, address, amount_sats).await {
                            error!("Error sending: {e}");
                            core.msg(Some(msg.id), CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveOnChain => {
                        core.msg(Some(msg.id), CoreUIMsg::ReceiveGenerating).await;
                        match core.receive_onchain(msg.id).await {
                            Err(e) => {
                                core.msg(Some(msg.id), CoreUIMsg::ReceiveFailed(e.to_string()))
                                    .await;
                            }
                            Ok(address) => {
                                core.msg(Some(msg.id), CoreUIMsg::ReceiveAddressGenerated(address))
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::GetFederationInfo(invite_code) => {
                        match core.get_federation_info(invite_code).await {
                            Err(e) => {
                                error!("Error getting federation info: {e}");
                                core.msg(
                                    Some(msg.id),
                                    CoreUIMsg::AddFederationFailed(e.to_string()),
                                )
                                .await;
                            }
                            Ok(config) => {
                                core.msg(Some(msg.id), CoreUIMsg::FederationInfo(config))
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::AddFederation(invite_code) => {
                        if let Err(e) = core.add_federation(invite_code).await {
                            error!("Error adding federation: {e}");
                            core.msg(Some(msg.id), CoreUIMsg::AddFederationFailed(e.to_string()))
                                .await;
                        } else {
                            core.msg(Some(msg.id), CoreUIMsg::AddFederationSuccess)
                                .await;
                            let new_federation_list = core.get_federation_items().await;
                            core.msg(
                                Some(msg.id),
                                CoreUIMsg::FederationListUpdated(new_federation_list),
                            )
                            .await;
                        }
                    }
                    UICoreMsg::Unlock(_password) => {
                        unreachable!("should already be unlocked")
                    }
                    UICoreMsg::GetSeedWords => {
                        let seed_words = core.get_seed_words().await;
                        core.msg(Some(msg.id), CoreUIMsg::SeedWords(seed_words))
                            .await;
                    }
                }
            }
        });
    }
}
