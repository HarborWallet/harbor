use core::run_core;
use fedimint_core::api::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use iced::widget::qr_code::Data;
use routes::Route;
use std::str::FromStr;
use std::sync::Arc;

use bridge::CoreUIMsg;
use iced::subscription::Subscription;
use iced::widget::row;
use iced::Element;
use iced::{clipboard, program, Color};
use iced::{Command, Font};
use log::info;

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

// This is the UI state. It should only contain data that is directly rendered by the UI
// More complicated state should be in Core, and bridged to the UI in a UI-friendly format.
pub struct HarborWallet {
    ui_handle: Option<Arc<bridge::UIHandle>>,
    balance: Amount,
    active_route: Route,
    transfer_amount_str: String,
    send_status: SendStatus,
    send_failure_reason: Option<String>,
    send_dest_input_str: String,
    send_amount_input_str: String,
    password_input_str: String,
    unlock_status: UnlockStatus,
    unlock_failure_reason: Option<String>,
    receive_failure_reason: Option<String>,
    receive_status: ReceiveStatus,
    receive_amount_str: String,
    receive_invoice: Option<Bolt11Invoice>,
    receive_qr_data: Option<Data>,
    mint_invite_code_str: String,
    add_federation_failure_reason: Option<String>,
}

impl Default for HarborWallet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug, Clone)]
enum SendStatus {
    #[default]
    Idle,
    Sending,
}

#[derive(Default, Debug, Clone)]
enum ReceiveStatus {
    #[default]
    Idle,
    Generating,
    WaitingToReceive,
}

#[derive(Default, Debug, Clone)]
enum UnlockStatus {
    #[default]
    Locked,
    Unlocked,
    Unlocking,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Setup
    CoreLoaded(Arc<bridge::UIHandle>),
    // Local state changes
    Navigate(Route),
    TransferAmountChanged(String),
    ReceiveAmountChanged(String),
    SendDestInputChanged(String),
    SendAmountInputChanged(String),
    PasswordInputChanged(String),
    MintInviteCodeInputChanged(String),
    CopyToClipboard(String),
    // Async commands we fire from the UI to core
    Noop,
    Send(String),
    Receive(u64),
    GenerateInvoice,
    Unlock(String),
    AddFederation(String),
    // Core messages we get from core
    CoreMessage(CoreUIMsg),
}

impl HarborWallet {
    fn new() -> Self {
        Self {
            ui_handle: None,
            balance: Amount::ZERO,
            active_route: Route::Unlock,
            transfer_amount_str: String::new(),
            receive_amount_str: String::new(),
            send_dest_input_str: String::new(),
            send_amount_input_str: String::new(),
            send_status: SendStatus::Idle,
            send_failure_reason: None,
            unlock_status: UnlockStatus::Locked,
            unlock_failure_reason: None,
            password_input_str: String::new(),
            receive_failure_reason: None,
            receive_status: ReceiveStatus::Idle,
            receive_invoice: None,
            receive_qr_data: None,
            mint_invite_code_str: String::new(),
            add_federation_failure_reason: None,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        run_core()
    }

    async fn async_send(ui_handle: Option<Arc<bridge::UIHandle>>, invoice: Bolt11Invoice) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().send(invoice).await;
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
            Message::CoreLoaded(ui_handle) => {
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
            Message::TransferAmountChanged(amount) => {
                self.transfer_amount_str = amount;
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
            // Async commands we fire from the UI to core
            Message::Noop => Command::none(),
            Message::Send(invoice_str) => match self.send_status {
                SendStatus::Sending => Command::none(),
                _ => {
                    self.send_failure_reason = None;
                    // todo get invoice from user
                    let invoice = Bolt11Invoice::from_str(&invoice_str).unwrap();
                    println!("Sending to invoice: {invoice}");
                    // let invoice = Bolt11Invoice::from_str(&invoice_str).unwrap();
                    Command::perform(Self::async_send(self.ui_handle.clone(), invoice), |_| {
                        // I don't know if this is the best way to do this but we don't really know anyting after we've fired the message
                        Message::Noop
                    })
                }
            },
            Message::Receive(amount) => match self.send_status {
                SendStatus::Sending => Command::none(),
                _ => {
                    self.send_failure_reason = None;
                    Command::perform(Self::async_receive(self.ui_handle.clone(), amount), |_| {
                        // I don't know if this is the best way to do this but we don't really know anyting after we've fired the message
                        Message::Noop
                    })
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
            // Handle any messages we get from core
            Message::CoreMessage(msg) => match msg {
                CoreUIMsg::Sending => {
                    self.send_status = SendStatus::Sending;
                    Command::none()
                }
                CoreUIMsg::SendSuccess(params) => {
                    info!("Send success: {params:?}");
                    self.send_status = SendStatus::Idle;
                    Command::none()
                }
                CoreUIMsg::SendFailure(reason) => {
                    self.send_status = SendStatus::Idle;
                    self.send_failure_reason = Some(reason);
                    Command::none()
                }
                CoreUIMsg::ReceiveSuccess(params) => {
                    info!("Receive success: {params:?}");
                    self.receive_status = ReceiveStatus::Idle;
                    Command::none()
                }
                CoreUIMsg::ReceiveFailed(reason) => {
                    self.receive_status = ReceiveStatus::Idle;
                    self.receive_failure_reason = Some(reason);
                    Command::none()
                }
                CoreUIMsg::BalanceUpdated(balance) => {
                    self.balance = balance;
                    Command::none()
                }
                CoreUIMsg::ReceiveInvoiceGenerating => {
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
                CoreUIMsg::Unlocking => {
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
