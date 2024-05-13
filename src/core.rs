use bip39::Mnemonic;
use bitcoin::Network;
use fedimint_client::module::IClientModule;
use fedimint_core::api::InviteCode;
use fedimint_core::Amount;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::{sync::Arc, time::Duration};

use iced::{
    futures::{channel::mpsc::Sender, SinkExt},
    subscription::{self, Subscription},
};
use tokio::time::sleep;

use crate::fedimint_client::FedimintClient;
use crate::{
    bridge::{self, CoreUIMsg, UICoreMsg},
    Message,
};

struct HarborCore {
    balance: Amount,
    tx: Sender<Message>,
    client: FedimintClient, // todo multiple clients
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

            let handles = bridge::create_handles();
            let (ui_handle, mut core_handle) = handles;
            let arc_ui_handle = Arc::new(ui_handle);

            // fixme, properly initialize this
            let mnemonic = Mnemonic::from_str("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about").unwrap();
            let client = FedimintClient::new(
                "test".to_string(),
                InviteCode::from_str("fed11qgqzc2nhwden5te0vejkg6tdd9h8gepwvejkg6tdd9h8garhduhx6at5d9h8jmn9wshxxmmd9uqqzgxg6s3evnr6m9zdxr6hxkdkukexpcs3mn7mj3g5pc5dfh63l4tj6g9zk4er").unwrap(),
                &mnemonic,
                Network::Signet,
                Arc::new(AtomicBool::new(false)),
            )
            .await
            .expect("Could not create fedimint client");

            let balance = client.fedimint_client.get_balance().await;

            let mut core = HarborCore {
                balance,
                tx,
                // TODO: add a database handle that works across async stuff
                client,
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
                                UICoreMsg::Send(amount) => {
                                    core.fake_send(amount).await;
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}
