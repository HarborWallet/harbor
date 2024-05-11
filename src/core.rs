use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use crate::bridge::{BridgeError, CoreHandle, UICoreMsg, UIHandle};

pub async fn run(mut core_handle: CoreHandle) {
    loop {
        let msg = core_handle.recv().await;
        if let Some(msg) = msg {
            match msg {
                UICoreMsg::Test(counter) => {
                    println!("{counter}");
                }
                UICoreMsg::Send(amount) => {
                    println!("Sending {amount}");
                }
            }
        }
        // println!("Hello, world!");
    }
}
