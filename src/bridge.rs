use crate::components::TransactionItem;
use bitcoin::{Address, Txid};
use fedimint_core::api::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum UICoreMsg {
    SendLightning(Bolt11Invoice),
    ReceiveLightning(Amount),
    SendOnChain { address: Address, amount_sats: u64 },
    ReceiveOnChain,
    AddFederation(InviteCode),
    Unlock(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SendSuccessMsg {
    Lightning { preimage: [u8; 32] },
    Onchain { txid: Txid },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReceiveSuccessMsg {
    Lightning,
    Onchain { txid: Txid },
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
    BalanceUpdated(Amount),
    // todo probably want a way to incrementally add items to the history
    TransactionHistoryUpdated(Vec<TransactionItem>),
    AddFederationFailed(String),
    AddFederationSuccess,
    Unlocking,
    UnlockSuccess,
    UnlockFailed(String),
}

#[derive(Debug)]
pub struct UIHandle {
    ui_to_core_tx: mpsc::Sender<UICoreMsg>,
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

    pub async fn send_lightning(&self, invoice: Bolt11Invoice) {
        self.msg_send(UICoreMsg::SendLightning(invoice)).await;
    }

    pub async fn send_onchain(&self, address: Address, amount_sats: u64) {
        self.msg_send(UICoreMsg::SendOnChain {
            address,
            amount_sats,
        })
        .await;
    }

    pub async fn receive(&self, amount: u64) {
        self.msg_send(UICoreMsg::ReceiveLightning(Amount::from_sats(amount)))
            .await;
    }

    pub async fn receive_onchain(&self) {
        self.msg_send(UICoreMsg::ReceiveOnChain).await;
    }

    pub async fn unlock(&self, password: String) {
        self.msg_send(UICoreMsg::Unlock(password)).await;
    }

    pub async fn add_federation(&self, invite: InviteCode) {
        self.msg_send(UICoreMsg::AddFederation(invite)).await;
    }
}

impl CoreHandle {
    pub async fn recv(&mut self) -> Option<UICoreMsg> {
        self.core_from_ui_rx.recv().await
    }
}

#[derive(Debug)]
pub struct CoreHandle {
    core_from_ui_rx: mpsc::Receiver<UICoreMsg>,
}

pub fn create_handles() -> (UIHandle, CoreHandle) {
    let (ui_to_core_tx, core_from_ui_rx) = mpsc::channel::<UICoreMsg>(1);

    let ui_handle = UIHandle { ui_to_core_tx };

    let core_handle = CoreHandle { core_from_ui_rx };

    (ui_handle, core_handle)
}
