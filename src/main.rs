use bitcoin::Address;
use components::TransactionItem;
use core::run_core;
use fedimint_core::api::InviteCode;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use iced::widget::qr_code::Data;
use routes::Route;
use std::str::FromStr;
use std::sync::Arc;

use bridge::{CoreUIMsg, ReceiveSuccessMsg, SendSuccessMsg};
use iced::subscription::Subscription;
use iced::widget::row;
use iced::Element;
use iced::{clipboard, program, Color};
use iced::{Command, Font};
use log::{error, info};

use crate::components::focus_input_id;

pub mod bridge;
pub mod components;
pub mod conf;
pub mod core;
pub mod db;
pub mod db_models;
mod fedimint_client;
pub mod routes;

// This starts the program. Importantly, it registers the update and view methods, along with a subscription.
// We can also run logic during load if we need to.
pub fn main() -> iced::Result {
    pretty_env_logger::init();
    program("Harbor", HarborWallet::update, HarborWallet::view)
        // .load(HarborWallet::load)
        .font(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-Bold.ttf").as_slice())
        .theme(HarborWallet::theme)
        .default_font(Font {
            family: iced::font::Family::Name("Inter-Regular.ttf"),
            weight: iced::font::Weight::Normal,
            stretch: iced::font::Stretch::Normal,
            style: iced::font::Style::Normal,
        })
        .subscription(HarborWallet::subscription)
        .run()
}

#[derive(Default, Debug, Clone, PartialEq)]
enum SendStatus {
    #[default]
    Idle,
    Sending,
}

#[derive(Default, Debug, Clone, PartialEq)]
enum ReceiveStatus {
    #[default]
    Idle,
    Generating,
    WaitingToReceive,
}

#[derive(Default, Debug, Clone, PartialEq)]
enum UnlockStatus {
    #[default]
    Locked,
    Unlocked,
    Unlocking,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveMethod {
    #[default]
    Lightning,
    OnChain,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Setup
    UIHandlerLoaded(Arc<bridge::UIHandle>),
    // Local state changes
    Navigate(Route),
    ReceiveAmountChanged(String),
    ReceiveStateReset,
    SendDestInputChanged(String),
    SendAmountInputChanged(String),
    SendStateReset,
    PasswordInputChanged(String),
    MintInviteCodeInputChanged(String),
    DonateAmountChanged(String),
    CopyToClipboard(String),
    ReceiveMethodChanged(ReceiveMethod),
    // Async commands we fire from the UI to core
    Noop,
    Send(String),
    GenerateInvoice,
    GenerateAddress,
    Unlock(String),
    AddFederation(String),
    Donate,
    // Core messages we get from core
    CoreMessage(CoreUIMsg),
    // Fake stuff for testing
    FakeAddTransaction,
}

// This is the UI state. It should only contain data that is directly rendered by the UI
// More complicated state should be in Core, and bridged to the UI in a UI-friendly format.
#[derive(Default, Debug)]
pub struct HarborWallet {
    ui_handle: Option<Arc<bridge::UIHandle>>,
    balance_sats: u64,
    active_route: Route,
    send_status: SendStatus,
    send_failure_reason: Option<String>,
    send_success_msg: Option<SendSuccessMsg>,
    send_dest_input_str: String,
    send_amount_input_str: String,
    password_input_str: String,
    unlock_status: UnlockStatus,
    unlock_failure_reason: Option<String>,
    receive_failure_reason: Option<String>,
    receive_success_msg: Option<ReceiveSuccessMsg>,
    receive_status: ReceiveStatus,
    receive_amount_str: String,
    receive_invoice: Option<Bolt11Invoice>,
    receive_address: Option<Address>,
    receive_qr_data: Option<Data>,
    receive_method: ReceiveMethod,
    mint_invite_code_str: String,
    add_federation_failure_reason: Option<String>,
    donate_amount_str: String,
    transaction_history: Vec<TransactionItem>,
}

impl HarborWallet {
    fn subscription(&self) -> Subscription<Message> {
        run_core()
    }

    async fn async_send_lightning(
        ui_handle: Option<Arc<bridge::UIHandle>>,
        invoice: Bolt11Invoice,
    ) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().send_lightning(invoice).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_send_onchain(
        ui_handle: Option<Arc<bridge::UIHandle>>,
        address: Address,
        amount_sats: u64,
    ) {
        println!("Got to async_send");
        if let Some(ui_handle) = ui_handle {
            println!("Have a ui_handle, sending the invoice over");
            ui_handle.clone().send_onchain(address, amount_sats).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_receive(ui_handle: Option<Arc<bridge::UIHandle>>, amount: u64) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().receive(amount).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_receive_onchain(ui_handle: Option<Arc<bridge::UIHandle>>) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().receive_onchain().await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_unlock(ui_handle: Option<Arc<bridge::UIHandle>>, password: String) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().unlock(password).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_add_federation(ui_handle: Option<Arc<bridge::UIHandle>>, invite: InviteCode) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().add_federation(invite).await;
        } else {
            panic!("UI handle is None");
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // Setup
            Message::UIHandlerLoaded(ui_handle) => {
                self.ui_handle = Some(ui_handle);
                println!("Core loaded");

                focus_input_id("password_unlock_input")

                // Command::none()
                // Mess
            }
            // Internal app state stuff like navigation and text inputs
            Message::Navigate(route) => {
                self.active_route = route;
                Command::none()
            }
            Message::ReceiveAmountChanged(amount) => {
                self.receive_amount_str = amount;
                Command::none()
            }
            Message::SendDestInputChanged(input) => {
                self.send_dest_input_str = input;
                Command::none()
            }
            Message::SendAmountInputChanged(input) => {
                self.send_amount_input_str = input;
                Command::none()
            }
            Message::PasswordInputChanged(input) => {
                self.password_input_str = input;
                Command::none()
            }
            Message::MintInviteCodeInputChanged(input) => {
                self.mint_invite_code_str = input;
                Command::none()
            }
            Message::DonateAmountChanged(input) => {
                self.donate_amount_str = input;
                Command::none()
            }
            Message::SendStateReset => {
                self.send_failure_reason = None;
                self.send_success_msg = None;
                self.send_dest_input_str = String::new();
                self.send_amount_input_str = String::new();
                self.send_status = SendStatus::Idle;
                Command::none()
            }
            Message::ReceiveStateReset => {
                self.receive_failure_reason = None;
                self.receive_amount_str = String::new();
                self.receive_invoice = None;
                self.receive_success_msg = None;
                self.receive_address = None;
                self.receive_qr_data = None;
                self.receive_status = ReceiveStatus::Idle;
                Command::none()
            }
            Message::ReceiveMethodChanged(method) => {
                self.receive_method = method;
                Command::none()
            }
            // Async commands we fire from the UI to core
            Message::Noop => Command::none(),
            Message::Send(invoice_str) => match self.send_status {
                SendStatus::Sending => Command::none(),
                _ => {
                    self.send_failure_reason = None;
                    if let Ok(invoice) = Bolt11Invoice::from_str(&invoice_str) {
                        Command::perform(
                            Self::async_send_lightning(self.ui_handle.clone(), invoice),
                            |_| Message::Noop,
                        )
                    } else if let Ok(address) = Address::from_str(&invoice_str) {
                        let amount = self.send_amount_input_str.parse::<u64>().unwrap(); // TODO: error handling
                        Command::perform(
                            Self::async_send_onchain(self.ui_handle.clone(), address, amount),
                            |_| Message::Noop,
                        )
                    } else {
                        error!("Invalid invoice or address");
                        Command::none()
                    }
                }
            },
            Message::GenerateInvoice => match self.receive_status {
                ReceiveStatus::Generating => Command::none(),
                _ => {
                    self.receive_failure_reason = None;
                    match self.receive_amount_str.parse::<u64>() {
                        Ok(amount) => Command::perform(
                            Self::async_receive(self.ui_handle.clone(), amount),
                            |_| Message::Noop,
                        ),
                        Err(e) => {
                            self.receive_amount_str = String::new();
                            eprintln!("Error parsing amount: {e}");
                            Command::none()
                        }
                    }
                }
            },
            Message::GenerateAddress => match self.receive_status {
                ReceiveStatus::Generating => Command::none(),
                _ => {
                    self.receive_failure_reason = None;
                    Command::perform(Self::async_receive_onchain(self.ui_handle.clone()), |_| {
                        Message::Noop
                    })
                }
            },
            Message::Donate => match self.donate_amount_str.parse::<u64>() {
                Ok(amount) => {
                    // TODO: don't hardcode this!
                    let hardcoded_donation_address = "tb1qd28npep0s8frcm3y7dxqajkcy2m40eysplyr9v";
                    let address = Address::from_str(hardcoded_donation_address).unwrap();

                    Command::perform(
                        Self::async_send_onchain(self.ui_handle.clone(), address, amount),
                        |_| Message::Noop,
                    )
                }
                Err(e) => {
                    self.receive_amount_str = String::new();
                    eprintln!("Error parsing amount: {e}");
                    Command::none()
                }
            },
            Message::Unlock(password) => match self.unlock_status {
                UnlockStatus::Unlocking => Command::none(),
                _ => {
                    self.unlock_failure_reason = None;
                    Command::perform(Self::async_unlock(self.ui_handle.clone(), password), |_| {
                        Message::Noop
                    })
                }
            },
            Message::AddFederation(invite_code) => {
                let invite = InviteCode::from_str(&invite_code);
                if let Ok(invite) = invite {
                    Command::perform(
                        Self::async_add_federation(self.ui_handle.clone(), invite),
                        |_| Message::Noop,
                    )
                } else {
                    self.add_federation_failure_reason = Some("Invalid invite code".to_string());
                    Command::none()
                }
            }
            Message::CopyToClipboard(s) => {
                println!("Copying to clipboard: {s}");
                clipboard::write(s)
            }
            Message::FakeAddTransaction => {
                if self.transaction_history.len() % 2 == 0 {
                    self.transaction_history
                        .push(TransactionItem::make_dummy_onchain());
                } else {
                    self.transaction_history.push(TransactionItem::make_dummy());
                }
                Command::none()
            }
            // Handle any messages we get from core
            Message::CoreMessage(msg) => match msg {
                CoreUIMsg::Sending => {
                    self.send_status = SendStatus::Sending;
                    Command::none()
                }
                CoreUIMsg::SendSuccess(params) => {
                    info!("Send success: {params:?}");
                    self.send_success_msg = Some(params);
                    Command::none()
                }
                CoreUIMsg::SendFailure(reason) => {
                    self.send_status = SendStatus::Idle;
                    self.send_failure_reason = Some(reason);
                    Command::none()
                }
                CoreUIMsg::ReceiveSuccess(params) => {
                    info!("Receive success: {params:?}");
                    self.receive_success_msg = Some(params);
                    Command::none()
                }
                CoreUIMsg::ReceiveFailed(reason) => {
                    self.receive_status = ReceiveStatus::Idle;
                    self.receive_failure_reason = Some(reason);
                    Command::none()
                }
                CoreUIMsg::BalanceUpdated(balance) => {
                    self.balance_sats = balance.sats_round_down();
                    Command::none()
                }
                CoreUIMsg::ReceiveGenerating => {
                    self.receive_status = ReceiveStatus::Generating;
                    Command::none()
                }
                CoreUIMsg::ReceiveInvoiceGenerated(invoice) => {
                    self.receive_status = ReceiveStatus::WaitingToReceive;
                    println!("Received invoice: {invoice}");
                    self.receive_qr_data = Some(
                        Data::with_error_correction(
                            format!("lightning:{invoice}"),
                            iced::widget::qr_code::ErrorCorrection::Low,
                        )
                        .unwrap(),
                    );
                    self.receive_invoice = Some(invoice);
                    Command::none()
                }
                CoreUIMsg::AddFederationFailed(reason) => {
                    self.add_federation_failure_reason = Some(reason);
                    Command::none()
                }
                CoreUIMsg::AddFederationSuccess => {
                    self.mint_invite_code_str = String::new();
                    Command::none()
                }
                CoreUIMsg::ReceiveAddressGenerated(address) => {
                    self.receive_status = ReceiveStatus::WaitingToReceive;
                    println!("Received address: {address}");
                    self.receive_qr_data = Some(
                        Data::with_error_correction(
                            format!("bitcoin:{address}"),
                            iced::widget::qr_code::ErrorCorrection::Low,
                        )
                        .unwrap(),
                    );
                    self.receive_address = Some(address);
                    Command::none()
                }
                CoreUIMsg::Unlocking => {
                    info!("Got unlocking message");
                    self.unlock_status = UnlockStatus::Unlocking;
                    Command::none()
                }
                CoreUIMsg::UnlockSuccess => {
                    self.unlock_status = UnlockStatus::Unlocked;
                    self.active_route = Route::Home;
                    Command::none()
                }
                CoreUIMsg::UnlockFailed(reason) => {
                    self.unlock_status = UnlockStatus::Locked;
                    self.unlock_failure_reason = Some(reason);
                    Command::none()
                }
            },
        }
    }

    fn view(&self) -> Element<Message> {
        let sidebar = crate::components::sidebar(self);

        let active_route = match self.active_route {
            Route::Unlock => crate::routes::unlock(self),
            Route::Home => row![sidebar, crate::routes::home(self)].into(),
            Route::Receive => row![sidebar, crate::routes::receive(self)].into(),
            Route::Send => row![sidebar, crate::routes::send(self)].into(),
            Route::Mints => row![sidebar, crate::routes::mints(self)].into(),
            Route::Donate => row![sidebar, crate::routes::donate(self)].into(),
            Route::History => row![sidebar, crate::routes::history(self)].into(),
            Route::Transfer => row![sidebar, crate::routes::transfer(self)].into(),
            // TODO: just add settings route and we can remove this
            _ => row![sidebar, crate::routes::home(self)].into(),
        };

        active_route
    }

    fn theme(&self) -> iced::Theme {
        let mutiny_red = Color::from_rgb8(250, 0, 80);
        iced::Theme::custom(
            String::from("Custom"),
            iced::theme::Palette {
                background: Color::from_rgb8(23, 23, 25),
                primary: mutiny_red,
                text: Color::WHITE,
                success: Color::WHITE,
                danger: mutiny_red,
            },
        )
    }
}
