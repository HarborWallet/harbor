pub mod profile;
pub use profile::*;

pub mod fedimint;
pub use fedimint::*;

pub mod cashu_mint;
pub use cashu_mint::*;

pub mod lightning_payment;
pub use lightning_payment::*;

pub mod lightning_receive;
pub use lightning_receive::*;

pub mod onchain_payment;
pub use onchain_payment::*;

pub mod onchain_receive;
pub use onchain_receive::*;

pub(crate) mod schema;

pub mod mint_metadata;
pub mod transaction_item;

use crate::MintIdentifier;
use crate::metadata::FederationMeta;
use fedimint_core::config::FederationId;
use fedimint_core::core::ModuleKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintItem {
    pub id: MintIdentifier,
    pub name: String,
    pub balance: u64,
    pub guardians: Option<Vec<String>>,
    pub module_kinds: Option<Vec<ModuleKind>>,
    pub metadata: FederationMeta,
    pub on_chain_supported: bool,
    pub active: bool,
}

impl MintItem {
    pub fn unknown(id: FederationId) -> Self {
        Self {
            id: MintIdentifier::Fedimint(id),
            name: "Unknown".to_string(),
            balance: 0,
            guardians: None,
            module_kinds: None,
            metadata: FederationMeta::default(),
            on_chain_supported: false,
            active: true,
        }
    }
}

impl PartialOrd for MintItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MintItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .balance
            .cmp(&self.balance)
            .then_with(|| self.id.cmp(&other.id))
            .then_with(|| self.name.cmp(&other.name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PaymentStatus {
    /// Payment is in flight or has not been received yet
    Pending = 0,
    /// Payment has been received and is waiting for confirmations
    WaitingConfirmation = 1,
    /// Payment has been confirmed and successfully received
    Success = 2,
    /// Payment failed
    Failed = 3,
}

impl PaymentStatus {
    pub fn from_i32(status: i32) -> Self {
        match status {
            0 => PaymentStatus::Pending,
            1 => PaymentStatus::WaitingConfirmation,
            2 => PaymentStatus::Success,
            3 => PaymentStatus::Failed,
            _ => panic!("invalid status"),
        }
    }
}
