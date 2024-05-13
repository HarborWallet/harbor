pub mod home;
pub use home::*;

pub mod mints;
pub use mints::*;

pub mod transfer;
pub use transfer::*;

pub mod receive;
pub use receive::*;

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum Route {
    #[default]
    Home,
    Mints,
    Transfer,
    History,
    Settings,
    Receive,
}
