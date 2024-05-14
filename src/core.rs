use anyhow::anyhow;
use bip39::Mnemonic;
use bitcoin::{Address, Network};
use fedimint_core::api::InviteCode;
use fedimint_core::config::FederationId;
use fedimint_core::Amount;
use fedimint_ln_client::{LightningClientModule, PayType};
use fedimint_ln_common::lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Description};
use fedimint_wallet_client::WalletClientModule;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use iced::{
    futures::{channel::mpsc::Sender, SinkExt},
    subscription::{self, Subscription},
};
use log::{error, warn};
use tokio::sync::RwLock;

use crate::fedimint_client::{
    spawn_onchain_payment_subscription, spawn_onchain_receive_subscription,
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

struct HarborCore {
    balance: Amount,
    network: Network,
    mnemonic: Mnemonic,
    tx: Sender<Message>,
    clients: Arc<RwLock<HashMap<FederationId, FedimintClient>>>,
    storage: Arc<dyn DBConnection + Send + Sync>,
    stop: Arc<AtomicBool>,
}

const INVITE: &str = "fed11qgqzc2nhwden5te0vejkg6tdd9h8gepwvejkg6tdd9h8garhduhx6at5d9h8jmn9wshxxmmd9uqqzgxg6s3evnr6m9zdxr6hxkdkukexpcs3mn7mj3g5pc5dfh63l4tj6g9zk4er";

impl HarborCore {
    async fn msg(&self, msg: CoreUIMsg) {
        self.tx
            .clone()
            .send(Message::CoreMessage(msg))
            .await
            .unwrap();
    }

    // TODO: probably just do this in core_loaded?
    async fn set_balance(&self) {
        self.msg(CoreUIMsg::BalanceUpdated(self.balance)).await;
    }

    // todo for now just use the first client, but eventually we'll want to have a way to select a client
    async fn get_client(&self) -> FedimintClient {
        self.clients.read().await.values().next().unwrap().clone()
    }

    async fn send_lightning(&self, invoice: Bolt11Invoice) -> anyhow::Result<()> {
        // todo go through all clients and select the first one that has enough balance
        let client = self.get_client().await.fedimint_client;
        let lightning_module = client.get_first_module::<LightningClientModule>();

        let gateway = select_gateway(&client)
            .await
            .ok_or(anyhow!("Internal error: No gateway found for federation"))?;

        let outgoing = lightning_module
            .pay_bolt11_invoice(Some(gateway), invoice, ())
            .await?;

        match outgoing.payment_type {
            PayType::Internal(op_id) => {
                let sub = lightning_module.subscribe_internal_pay(op_id).await?;
                spawn_internal_payment_subscription(self.tx.clone(), client.clone(), sub).await;
            }
            PayType::Lightning(op_id) => {
                let sub = lightning_module.subscribe_ln_pay(op_id).await?;
                spawn_invoice_payment_subscription(self.tx.clone(), client.clone(), sub).await;
            }
        }

        log::info!("Invoice sent");

        Ok(())
    }

    async fn receive_lightning(&self, amount: Amount) -> anyhow::Result<Bolt11Invoice> {
        let client = self.get_client().await.fedimint_client;
        let lightning_module = client.get_first_module::<LightningClientModule>();

        let gateway = select_gateway(&client)
            .await
            .ok_or(anyhow!("Internal error: No gateway found for federation"))?;

        let desc = Description::new(String::new()).expect("empty string is valid");
        let (op_id, invoice, _) = lightning_module
            .create_bolt11_invoice(
                amount,
                Bolt11InvoiceDescription::Direct(&desc),
                None,
                (),
                Some(gateway),
            )
            .await?;

        println!("{}", invoice);

        // Create subscription to operation if it exists
        if let Ok(subscription) = lightning_module.subscribe_ln_receive(op_id).await {
            spawn_invoice_receive_subscription(self.tx.clone(), client.clone(), subscription).await;
        } else {
            error!("Could not create subscription to lightning receive");
        }

        Ok(invoice)
    }

    async fn send_onchain(&self, address: Address, sats: u64) -> anyhow::Result<()> {
        // todo go through all clients and select the first one that has enough balance
        let client = self.get_client().await.fedimint_client;
        let onchain = client.get_first_module::<WalletClientModule>();

        let amount = bitcoin::Amount::from_sat(sats);

        // todo add manual fee selection
        let fees = onchain.get_withdraw_fees(address.clone(), amount).await?;

        let op_id = onchain
            .withdraw(address, bitcoin::Amount::from_sat(sats), fees, ())
            .await?;

        let sub = onchain.subscribe_withdraw_updates(op_id).await?;

        spawn_onchain_payment_subscription(self.tx.clone(), client.clone(), sub).await;

        Ok(())
    }

    async fn receive_onchain(&self) -> anyhow::Result<Address> {
        // todo add federation id selection
        let client = self.get_client().await.fedimint_client;
        let onchain = client.get_first_module::<WalletClientModule>();

        // expire the address in 1 year
        let valid_until = SystemTime::now() + PEG_IN_TIMEOUT_YEAR;

        let (op_id, address) = onchain.get_deposit_address(valid_until, ()).await?;

        let sub = onchain.subscribe_deposit_updates(op_id).await?;

        spawn_onchain_receive_subscription(self.tx.clone(), client.clone(), sub).await;

        Ok(address)
    }

    async fn add_federation(&self, invite_code: InviteCode) -> anyhow::Result<()> {
        let id = invite_code.federation_id();

        let mut clients = self.clients.write().await;
        if clients.get(&id).is_some() {
            return Err(anyhow!("Federation already added"));
        }

        let client = FedimintClient::new(
            self.storage.clone(),
            invite_code,
            &self.mnemonic,
            self.network,
            self.stop.clone(),
        )
        .await?;

        clients.insert(client.fedimint_client.federation_id(), client);

        // todo add to database

        Ok(())
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
            tx.send(Message::UIHandlerLoaded(arc_ui_handle.clone()))
                .await
                .expect("should send");

            let network = Network::Signet;

            // Create the datadir if it doesn't exist
            let path = PathBuf::from(&conf::data_dir(network));
            std::fs::create_dir_all(path.clone()).expect("Could not create datadir");
            log::info!("Using datadir: {path:?}");

            loop {
                let msg = core_handle.recv().await;

                match msg {
                    Some(UICoreMsg::Unlock(password)) => {
                        tx.send(Message::CoreMessage(CoreUIMsg::Unlocking))
                            .await
                            .expect("should send");

                        // attempting to unlock
                        let db = setup_db(
                            path.join("harbor.sqlite")
                                .to_str()
                                .expect("path must be correct"),
                            password,
                        );

                        if let Err(e) = db {
                            // probably invalid password
                            error!("error using password: {e}");

                            tx.send(Message::CoreMessage(CoreUIMsg::UnlockFailed(
                                "Invalid Password".to_string(),
                            )))
                            .await
                            .expect("should send");
                            continue;
                        }
                        let db = db.expect("no error");

                        let mnemonic = get_mnemonic(db.clone()).expect("should get seed");

                        let stop = Arc::new(AtomicBool::new(false));

                        // fixme, properly initialize this
                        let client = FedimintClient::new(
                            db.clone(),
                            InviteCode::from_str(INVITE).unwrap(),
                            &mnemonic,
                            network,
                            stop.clone(),
                        )
                        .await
                        .expect("Could not create fedimint client");

                        let mut clients = HashMap::new();
                        clients.insert(client.fedimint_client.federation_id(), client);

                        let mut balance = Amount::ZERO;
                        for client in clients.values() {
                            balance += client.fedimint_client.get_balance().await;
                        }

                        let core = HarborCore {
                            storage: db.clone(),
                            balance,
                            tx: tx.clone(),
                            mnemonic,
                            network,
                            clients: Arc::new(RwLock::new(clients)),
                            stop,
                        };

                        tx.send(Message::CoreMessage(CoreUIMsg::UnlockSuccess))
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
    core.set_balance().await;

    loop {
        let msg = core_handle.recv().await;

        if let Some(msg) = msg {
            match msg {
                UICoreMsg::SendLightning(invoice) => {
                    log::info!("Got UICoreMsg::Send");
                    core.msg(CoreUIMsg::Sending).await;
                    if let Err(e) = core.send_lightning(invoice).await {
                        error!("Error sending: {e}");
                        core.msg(CoreUIMsg::SendFailure(e.to_string())).await;
                    }
                }
                UICoreMsg::ReceiveLightning(amount) => {
                    core.msg(CoreUIMsg::ReceiveGenerating).await;
                    match core.receive_lightning(amount).await {
                        Err(e) => {
                            core.msg(CoreUIMsg::ReceiveFailed(e.to_string())).await;
                        }
                        Ok(invoice) => {
                            core.msg(CoreUIMsg::ReceiveInvoiceGenerated(invoice)).await;
                        }
                    }
                }
                UICoreMsg::SendOnChain {
                    address,
                    amount_sats,
                } => {
                    log::info!("Got UICoreMsg::SendOnChain");
                    core.msg(CoreUIMsg::Sending).await;
                    if let Err(e) = core.send_onchain(address, amount_sats).await {
                        error!("Error sending: {e}");
                        core.msg(CoreUIMsg::SendFailure(e.to_string())).await;
                    }
                }
                UICoreMsg::ReceiveOnChain => {
                    core.msg(CoreUIMsg::ReceiveGenerating).await;
                    match core.receive_onchain().await {
                        Err(e) => {
                            core.msg(CoreUIMsg::ReceiveFailed(e.to_string())).await;
                        }
                        Ok(address) => {
                            core.msg(CoreUIMsg::ReceiveAddressGenerated(address)).await;
                        }
                    }
                }
                UICoreMsg::AddFederation(invite_code) => {
                    if let Err(e) = core.add_federation(invite_code).await {
                        error!("Error adding federation: {e}");
                        core.msg(CoreUIMsg::AddFederationFailed(e.to_string()))
                            .await;
                    } else {
                        core.msg(CoreUIMsg::AddFederationSuccess).await;
                    }
                }
                UICoreMsg::Unlock(_password) => {
                    unreachable!("should already be unlocked")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn example_test() {
        assert_eq!(true, true);
    }
}
