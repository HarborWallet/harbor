use crate::components::{FederationItem, TransactionItem};
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Txid};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::invite_code::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use tokio::sync::mpsc;
use uuid::Uuid;

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
    GetFederationInfo(InviteCode),
    AddFederation(InviteCode),
    Unlock(String),
    Init {
        password: String,
        seed: Option<String>,
    },
    GetSeedWords,
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
    // todo probably want a way to incrementally add items to the history
    TransactionHistoryUpdated(Vec<TransactionItem>),
    FederationBalanceUpdated { id: FederationId, balance: Amount },
    AddFederationFailed(String),
    FederationInfo(ClientConfig),
    AddFederationSuccess,
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
}

#[derive(Debug)]
pub struct UIHandle {
    ui_to_core_tx: mpsc::Sender<UICoreMsgPacket>,
}

#[derive(Debug, Clone)]
pub enum BridgeError {
    SendFailed,
    Unknown,
}

impl UIHandle {
    pub async fn msg_send(&self, msg: UICoreMsgPacket) {
        self.ui_to_core_tx.send(msg).await.unwrap();
    }

    pub async fn send_lightning(
        &self,
        id: Uuid,
        federation_id: FederationId,
        invoice: Bolt11Invoice,
    ) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::SendLightning {
                federation_id,
                invoice,
            },
            id,
        })
        .await;
    }

    pub async fn send_onchain(
        &self,
        id: Uuid,
        federation_id: FederationId,
        address: Address<NetworkUnchecked>,
        amount_sats: Option<u64>,
    ) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::SendOnChain {
                federation_id,
                address,
                amount_sats,
            },
            id,
        })
        .await;
    }

    pub async fn receive(&self, id: Uuid, federation_id: FederationId, amount: u64) {
        let amount = Amount::from_sats(amount);
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::ReceiveLightning {
                federation_id,
                amount,
            },
            id,
        })
        .await;
    }

    pub async fn receive_onchain(&self, id: Uuid, federation_id: FederationId) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::ReceiveOnChain { federation_id },
            id,
        })
        .await;
    }

    pub async fn unlock(&self, id: Uuid, password: String) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::Unlock(password),
            id,
        })
        .await;
    }

    pub async fn init(&self, id: Uuid, password: String) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::Init {
                password,
                seed: None, // FIXME: Use this
            },
            id,
        })
        .await;
    }

    pub async fn add_federation(&self, id: Uuid, invite: InviteCode) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::AddFederation(invite),
            id,
        })
        .await;
    }

    pub async fn peek_federation(&self, id: Uuid, invite: InviteCode) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::GetFederationInfo(invite),
            id,
        })
        .await;
    }

    pub async fn get_seed_words(&self, id: Uuid) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::GetSeedWords,
            id,
        })
        .await;
    }
}

impl CoreHandle {
    pub async fn recv(&mut self) -> Option<UICoreMsgPacket> {
        self.core_from_ui_rx.recv().await
    }
}

#[derive(Debug)]
pub struct CoreHandle {
    core_from_ui_rx: mpsc::Receiver<UICoreMsgPacket>,
}

pub fn create_handles() -> (UIHandle, CoreHandle) {
    let (ui_to_core_tx, core_from_ui_rx) = mpsc::channel::<UICoreMsgPacket>(50);

    let ui_handle = UIHandle { ui_to_core_tx };

    let core_handle = CoreHandle { core_from_ui_rx };

    (ui_handle, core_handle)
}
