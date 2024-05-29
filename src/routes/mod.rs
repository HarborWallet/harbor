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

pub mod history;
pub use history::*;

pub mod settings;
pub use settings::*;

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum MintSubroute {
    #[default]
    List,
    Add,
}

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum Route {
    #[default]
    Unlock,
    Home,
    Mints(MintSubroute),
    Transfer,
    History,
    Settings,
    Receive,
    Send,
    Donate,
}
