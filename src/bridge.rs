use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum UICoreMsg {
    Test(u64),
    Send(u64),
}

#[derive(Debug, Clone)]
pub enum CoreUIMsg {
    Sending,
    SendSuccess,
    SendFailure,
    ReceiveSuccess,
    BalanceUpdated(u64),
}

#[derive(Debug)]
pub struct UIHandle {
    ui_to_core_tx: mpsc::Sender<UICoreMsg>,
    ui_from_core_rx: mpsc::Receiver<CoreUIMsg>,
}

#[derive(Debug, Clone)]
pub enum BridgeError {
    SendFailed,
    Unknown,
}

impl UIHandle {
    pub async fn msg_send(&self, msg: UICoreMsg) {
        self.ui_to_core_tx.send(msg).await.unwrap();
    }

    pub async fn recv(&mut self) -> Option<CoreUIMsg> {
        self.ui_from_core_rx.recv().await
    }

    pub async fn send(&self, amount: u64) {
        self.msg_send(UICoreMsg::Send(amount)).await;
    }
}

impl CoreHandle {
    pub async fn msg_send(&self, msg: CoreUIMsg) {
        self.core_to_ui_tx.send(msg).await.unwrap();
    }

    pub async fn recv(&mut self) -> Option<UICoreMsg> {
        self.core_from_ui_rx.recv().await
    }
}

#[derive(Debug)]
pub struct CoreHandle {
    core_to_ui_tx: mpsc::Sender<CoreUIMsg>,
    core_from_ui_rx: mpsc::Receiver<UICoreMsg>,
}

pub fn create_handles() -> (UIHandle, CoreHandle) {
    let (ui_to_core_tx, core_from_ui_rx) = mpsc::channel::<UICoreMsg>(1);
    let (core_to_ui_tx, ui_from_core_rx) = mpsc::channel::<CoreUIMsg>(1);

    let ui_handle = UIHandle {
        ui_to_core_tx,
        ui_from_core_rx,
    };

    let core_handle = CoreHandle {
        core_to_ui_tx,
        core_from_ui_rx,
    };

    (ui_handle, core_handle)
}
