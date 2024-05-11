use std::{sync::Arc, time::Duration};

use iced::{
    futures::{channel::mpsc::Sender, SinkExt},
    subscription::{self, Subscription},
};
use tokio::time::sleep;

use crate::{
    bridge::{self, CoreUIMsg, UICoreMsg},
    Message,
};

struct HarborCore {
    balance: u64,
    tx: Sender<Message>,
    // db: TODO!! RwLock<Connection>,
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

    async fn send(&mut self, amount: u64) {
        self.msg(CoreUIMsg::Sending).await;
        sleep(Duration::from_secs(1)).await;
        println!("Sending {amount}");
        if let Some(b) = self.balance.checked_sub(amount) {
            // Save it in our struct
            self.balance = b;
            // Tell the UI we did a good job
            self.msg(CoreUIMsg::SendSuccess).await;
            // Tell the UI the new balance
            self.msg(CoreUIMsg::BalanceUpdated(self.balance)).await;
        } else {
            self.msg(CoreUIMsg::SendFailure("Insufficient funds".to_string()))
                .await;
        }
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

            let mut core = HarborCore {
                balance: 200,
                tx,
                // TODO: add a database handle that works across async stuff
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
                                    core.send(amount).await;
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}
