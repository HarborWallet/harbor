use crate::db_models::transaction_item::TransactionItem;
use crate::db_models::FederationItem;
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Txid};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::invite_code::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use uuid::Uuid;

pub mod core;
pub mod db;
pub mod db_models;
pub mod fedimint_client;

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
