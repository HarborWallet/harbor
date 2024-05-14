pub mod home;
pub use home::*;

pub mod mints;
pub use mints::*;

pub mod transfer;
pub use transfer::*;

pub mod receive;
pub use receive::*;

pub mod send;
pub use send::*;

pub mod unlock;
pub use unlock::*;

pub mod donate;
pub use donate::*;

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum Route {
    #[default]
    Unlock,
    Home,
    Mints,
    Transfer,
    History,
    Settings,
    Receive,
    Send,
    Donate,
}
