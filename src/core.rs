use anyhow::anyhow;
use bip39::Mnemonic;
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Network};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::invite_code::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_client::{LightningClientModule, PayType};
use fedimint_ln_common::config::FeeToAmount;
use fedimint_ln_common::lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Description};
use fedimint_wallet_client::WalletClientModule;
use iced::futures::Stream;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::futures::{channel::mpsc::Sender, SinkExt};
use log::{error, trace, warn};
use tokio::sync::RwLock;
use tokio::task::spawn_blocking;
use uuid::Uuid;

use crate::db::check_password;
use crate::fedimint_client::{
    spawn_onchain_payment_subscription, spawn_onchain_receive_subscription, FederationInviteOrId,
};
use crate::{
    bridge::{self, CoreUIMsg, UICoreMsg},
    conf::{self, retrieve_mnemonic},
    db::DBConnection,
    Message,
};
use crate::{components::FederationItem, conf::generate_mnemonic};
use crate::{
    db::setup_db,
    fedimint_client::{
        select_gateway, spawn_internal_payment_subscription, spawn_invoice_payment_subscription,
        spawn_invoice_receive_subscription, FedimintClient,
    },
};

const HARBOR_FILE_NAME: &str = "harbor.sqlite";

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
        for client in self.clients.read().await.values() {
            let fed_balance = client.fedimint_client.get_balance().await;
            self.msg(
                None,
                CoreUIMsg::FederationBalanceUpdated {
                    id: client.fedimint_client.federation_id(),
                    balance: fed_balance,
                },
            )
            .await;
        }

        let history = self.storage.get_transaction_history().unwrap();
        self.msg(None, CoreUIMsg::TransactionHistoryUpdated(history))
            .await;

        let federation_items = self.get_federation_items().await;
        self.msg(None, CoreUIMsg::FederationListUpdated(federation_items))
            .await;
    }

    async fn get_client(&self, federation_id: FederationId) -> FedimintClient {
        let clients = self.clients.read().await;
        clients
            .get(&federation_id)
            .expect("No client found for federation")
            .clone()
    }

    async fn send_lightning(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        invoice: Bolt11Invoice,
    ) -> anyhow::Result<()> {
        if invoice.amount_milli_satoshis().is_none() {
            return Err(anyhow!("Invoice must have an amount"));
        }
        let amount = Amount::from_msats(invoice.amount_milli_satoshis().expect("must have amount"));

        // todo go through all clients and select the first one that has enough balance
        let client = self.get_client(federation_id).await.fedimint_client;
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
        federation_id: FederationId,
        amount: Amount,
    ) -> anyhow::Result<Bolt11Invoice> {
        let client = self.get_client(federation_id).await.fedimint_client;
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
        federation_id: FederationId,
        address: Address<NetworkUnchecked>,
        sats: Option<u64>,
    ) -> anyhow::Result<()> {
        // todo go through all clients and select the first one that has enough balance
        let client = self.get_client(federation_id).await.fedimint_client;
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

    async fn receive_onchain(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
    ) -> anyhow::Result<Address> {
        let client = self.get_client(federation_id).await.fedimint_client;
        let onchain = client.get_first_module::<WalletClientModule>();

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

    async fn get_federation_info(&self, invite_code: InviteCode) -> anyhow::Result<ClientConfig> {
        let download = Instant::now();
        let config = fedimint_api_client::api::net::Connector::Tor
            .download_from_invite_code(&invite_code)
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
                guardians: None,
                module_kinds: None,
            })
            .collect::<Vec<FederationItem>>()
    }

    async fn get_seed_words(&self) -> String {
        self.mnemonic.to_string()
    }
}

pub fn run_core() -> impl Stream<Item = Message> {
    iced::stream::channel(100, |mut tx: Sender<Message>| async move {
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

        // FIXME: Artificial sleep because it loads too fast
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Check if the database file exists already, if so tell UI to unlock
        if std::fs::metadata(path.join(HARBOR_FILE_NAME)).is_ok() {
            tx.send(Message::core_msg(None, CoreUIMsg::Locked))
                .await
                .expect("should send");
        } else {
            tx.send(Message::core_msg(None, CoreUIMsg::NeedsInit))
                .await
                .expect("should send");
        }

        loop {
            let msg = core_handle.recv().await;

            let id = msg.as_ref().map(|m| m.id);

            // Watch for either Unlock or Init, ignore everything else until started
            match msg.map(|m| m.msg) {
                Some(UICoreMsg::Unlock(password)) => {
                    log::info!("Sending unlock message");
                    tx.send(Message::core_msg(id, CoreUIMsg::Unlocking))
                        .await
                        .expect("should send");

                    // attempting to unlock
                    let db_path = path.join(HARBOR_FILE_NAME);
                    let db_path = db_path.to_str().unwrap().to_string();

                    // if the db file doesn't exist, error out to go through init flow
                    if !std::path::Path::new(&db_path).exists() {
                        error!("Database does not exist, new wallet is required");

                        tx.send(Message::core_msg(
                            id,
                            CoreUIMsg::UnlockFailed(
                                "Database does not exist, new wallet is required".to_string(),
                            ),
                        ))
                        .await
                        .expect("should send");

                        continue;
                    }

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

                    let mnemonic = retrieve_mnemonic(db.clone()).expect("should get seed");

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
                Some(UICoreMsg::Init { password, seed }) => {
                    log::info!("Sending init message");
                    tx.send(Message::core_msg(id, CoreUIMsg::Initing))
                        .await
                        .expect("should send");

                    // set up the DB with the provided password
                    let db_path = path.join(HARBOR_FILE_NAME);
                    let db = spawn_blocking(move || setup_db(db_path.to_str().unwrap(), password))
                        .await
                        .expect("Could not create join handle");

                    if let Err(e) = db {
                        error!("error creating DB: {e}");

                        tx.send(Message::core_msg(id, CoreUIMsg::InitFailed(e.to_string())))
                            .await
                            .expect("should send");

                        continue;
                    }
                    let db = db.expect("no error");

                    let core = HarborCore {
                        storage: db.clone(),
                        tx: tx.clone(),
                        mnemonic: generate_mnemonic(db.clone(), seed)
                            .expect("should generate words"),
                        network,
                        clients: Arc::new(RwLock::new(HashMap::new())),
                        stop: Arc::new(AtomicBool::new(false)),
                    };

                    tx.send(Message::core_msg(id, CoreUIMsg::InitSuccess))
                        .await
                        .expect("should send");

                    process_core(&mut core_handle, &core).await;
                }

                _ => {
                    warn!("Ignoring unrelated message to locked core")
                }
            }
        }
    })
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
                    UICoreMsg::SendLightning {
                        federation_id,
                        invoice,
                    } => {
                        log::info!("Got UICoreMsg::Send");
                        core.msg(Some(msg.id), CoreUIMsg::Sending).await;
                        if let Err(e) = core.send_lightning(msg.id, federation_id, invoice).await {
                            error!("Error sending: {e}");
                            core.msg(Some(msg.id), CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveLightning {
                        federation_id,
                        amount,
                    } => {
                        core.msg(Some(msg.id), CoreUIMsg::ReceiveGenerating).await;
                        match core.receive_lightning(msg.id, federation_id, amount).await {
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
                        federation_id,
                        address,
                        amount_sats,
                    } => {
                        log::info!("Got UICoreMsg::SendOnChain");
                        core.msg(Some(msg.id), CoreUIMsg::Sending).await;
                        if let Err(e) = core
                            .send_onchain(msg.id, federation_id, address, amount_sats)
                            .await
                        {
                            error!("Error sending: {e}");
                            core.msg(Some(msg.id), CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveOnChain { federation_id } => {
                        core.msg(Some(msg.id), CoreUIMsg::ReceiveGenerating).await;
                        match core.receive_onchain(msg.id, federation_id).await {
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
                    UICoreMsg::Init { .. } => {
                        unreachable!("should already be inited")
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
