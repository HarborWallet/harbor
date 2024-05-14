use anyhow::anyhow;
use bip39::Mnemonic;
use bitcoin::Network;
use fedimint_core::api::InviteCode;
use fedimint_core::config::FederationId;
use fedimint_core::Amount;
use fedimint_ln_client::{LightningClientModule, PayType};
use fedimint_ln_common::lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Description};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::{sync::Arc, time::Duration};

use iced::{
    futures::{channel::mpsc::Sender, SinkExt},
    subscription::{self, Subscription},
};
use log::error;
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::{
    bridge::{self, CoreUIMsg, UICoreMsg},
    conf::{self, get_mnemonic},
    Message,
};
use crate::{
    db::setup_db,
    fedimint_client::{
        select_gateway, spawn_internal_payment_subscription, spawn_invoice_payment_subscription,
        spawn_invoice_receive_subscription, FedimintClient,
    },
};

struct HarborCore {
    balance: Amount,
    network: Network,
    mnemonic: Mnemonic,
    tx: Sender<Message>,
    clients: Arc<RwLock<HashMap<FederationId, FedimintClient>>>,
    stop: Arc<AtomicBool>,
}

impl HarborCore {
    async fn core_loaded(&self, ui_handle: Arc<bridge::UIHandle>) {
        self.tx
            .clone()
            .send(Message::CoreLoaded(ui_handle))
            .await
            .unwrap();
    }

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

    async fn fake_send(&mut self, amount: u64) {
        self.msg(CoreUIMsg::Sending).await;
        sleep(Duration::from_secs(1)).await;
        println!("Sending {amount}");

        let amount = Amount::from_sats(amount);
        if amount > self.balance {
            self.msg(CoreUIMsg::SendFailure("Insufficient funds".to_string()))
                .await;
            return;
        }

        // Save it in our struct
        self.balance = self.balance.saturating_sub(amount);
        // Tell the UI we did a good job
        self.msg(CoreUIMsg::SendSuccess).await;
        // Tell the UI the new balance
        self.msg(CoreUIMsg::BalanceUpdated(self.balance)).await;
    }

    // todo for now just use the first client, but eventually we'll want to have a way to select a client
    async fn get_client(&self) -> FedimintClient {
        self.clients.read().await.values().next().unwrap().clone()
    }

    async fn send(&self, invoice: Bolt11Invoice) -> anyhow::Result<()> {
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

    async fn receive(&self, amount: u64) -> anyhow::Result<Bolt11Invoice> {
        let client = self.get_client().await.fedimint_client;
        let lightning_module = client.get_first_module::<LightningClientModule>();

        let gateway = select_gateway(&client)
            .await
            .ok_or(anyhow!("Internal error: No gateway found for federation"))?;

        let desc = Description::new(String::new()).expect("empty string is valid");
        let (op_id, invoice, _) = lightning_module
            .create_bolt11_invoice(
                Amount::from_sats(amount),
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

    async fn add_federation(&self, invite_code: InviteCode) -> anyhow::Result<()> {
        let id = invite_code.federation_id();

        let mut clients = self.clients.write().await;
        if clients.get(&id).is_some() {
            return Err(anyhow!("Federation already added"));
        }

        let client =
            FedimintClient::new(invite_code, &self.mnemonic, self.network, self.stop.clone())
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
        |tx: Sender<Message>| async move {
            enum State {
                NeedsInit,
                Running,
            }
            let mut state = State::NeedsInit;

            let (ui_handle, mut core_handle) = bridge::create_handles();
            let arc_ui_handle = Arc::new(ui_handle);

            let network = Network::Signet;

            // Create the datadir if it doesn't exist
            let path = PathBuf::from(&conf::data_dir(network));
            std::fs::create_dir_all(path.clone()).expect("Could not create datadir");

            // Create or get the database
            // FIXME: pass in password
            let db = setup_db(
                path.join("harbor.sqlite")
                    .to_str()
                    .expect("path must be correct"),
                "password123".to_string(),
            );

            let mnemonic = get_mnemonic(db).expect("should get seed");

            let stop = Arc::new(AtomicBool::new(false));

            // fixme, properly initialize this
            let client = FedimintClient::new(
                InviteCode::from_str("fed11qgqzc2nhwden5te0vejkg6tdd9h8gepwvejkg6tdd9h8garhduhx6at5d9h8jmn9wshxxmmd9uqqzgxg6s3evnr6m9zdxr6hxkdkukexpcs3mn7mj3g5pc5dfh63l4tj6g9zk4er").unwrap(),
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

            let mut core = HarborCore {
                balance,
                tx,
                mnemonic,
                network,
                clients: Arc::new(RwLock::new(clients)),
                stop,
            };

            loop {
                match &mut state {
                    State::NeedsInit => {
                        // Hand the frontend its handle for talking to us
                        core.core_loaded(arc_ui_handle.clone()).await;

                        // Initialize the ui's state
                        core.set_balance().await;

                        state = State::Running;
                    }
                    State::Running => {
                        let msg = core_handle.recv().await;

                        if let Some(msg) = msg {
                            match msg {
                                UICoreMsg::Test(counter) => {
                                    println!("{counter}");
                                }
                                UICoreMsg::FakeSend(amount) => {
                                    core.fake_send(amount).await;
                                }
                                UICoreMsg::Send(invoice) => {
                                    log::info!("Got UICoreMsg::Send");
                                    core.msg(CoreUIMsg::Sending).await;
                                    if let Err(e) = core.send(invoice).await {
                                        error!("Error sending: {e}");
                                        core.msg(CoreUIMsg::SendFailure(e.to_string())).await;
                                    }
                                    core.msg(CoreUIMsg::SendSuccess).await;
                                }
                                UICoreMsg::Receive(amount) => {
                                    core.msg(CoreUIMsg::ReceiveInvoiceGenerating).await;
                                    match core.receive(amount).await {
                                        Err(e) => {
                                            core.msg(CoreUIMsg::ReceiveFailed(e.to_string())).await;
                                        }
                                        Ok(invoice) => {
                                            core.msg(CoreUIMsg::ReceiveInvoiceGenerated(
                                                invoice.clone(),
                                            ))
                                            .await;
                                        }
                                    }
                                }
                                UICoreMsg::AddFederation(invite_code) => {
                                    if let Err(e) = core.add_federation(invite_code).await {
                                        error!("Error adding federation: {e}");
                                        core.msg(CoreUIMsg::AddFederationFailed(e.to_string()))
                                            .await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn example_test() {
        assert_eq!(true, true);
    }
}
