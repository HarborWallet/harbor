pub mod profile;
pub use profile::*;

pub mod fedimint;
pub use fedimint::*;

pub mod lightning_payment;
pub use lightning_payment::*;

pub mod lightning_receive;
pub use lightning_receive::*;

pub mod onchain_payment;
pub use onchain_payment::*;

pub mod onchain_receive;
pub use onchain_receive::*;

pub mod schema;

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
