use core::run_core;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use std::str::FromStr;
use std::sync::Arc;

use bridge::CoreUIMsg;
use iced::subscription::Subscription;
use iced::widget::row;
use iced::{program, Color};
use iced::{Alignment, Element};
use iced::{Command, Font};

pub mod bridge;
pub mod components;
pub mod conf;
pub mod core;
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
}

impl Default for HarborWallet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub enum Route {
    #[default]
    Home,
    Mints,
    Transfer,
    History,
    Settings,
}

#[derive(Default, Debug, Clone)]
enum SendStatus {
    #[default]
    Idle,
    Sending,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Setup
    CoreLoaded(Arc<bridge::UIHandle>),
    // Local state changes
    Navigate(Route),
    TransferAmountChanged(String),
    // Async commands we fire from the UI to core
    Noop,
    Send(u64),
    Receive(u64),
    // Core messages we get from core
    CoreMessage(CoreUIMsg),
}

impl HarborWallet {
    fn new() -> Self {
        Self {
            ui_handle: None,
            balance: Amount::ZERO,
            active_route: Route::Home,
            transfer_amount_str: String::new(),
            send_status: SendStatus::Idle,
            send_failure_reason: None,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        run_core()
    }

    // We can't use self in these async functions because lifetimes are hard
    #[allow(dead_code)] // TODO: remove
    async fn async_fake_send(ui_handle: Option<Arc<bridge::UIHandle>>, amount: u64) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().fake_send(amount).await;
        } else {
            panic!("UI handle is None");
        }
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

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // Setup
            Message::CoreLoaded(ui_handle) => {
                self.ui_handle = Some(ui_handle);
                println!("Core loaded");
                Command::none()
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
            // Async commands we fire from the UI to core
            Message::Noop => Command::none(),
            Message::Send(_amount) => match self.send_status {
                SendStatus::Sending => Command::none(),
                _ => {
                    self.send_failure_reason = None;
                    // todo get invoice from user
                    let invoice = Bolt11Invoice::from_str("lntbs900n1pnyylm5pp57p3w5ll63xpc5zw4sff87vcgr46xnxnftkyye44q4e3px6uf97vshp57t8sp5tcchfv0y29yg46nqujktk2ufwcjcc7zvyd8rteadd7rjyscqzzsxqyz5vqsp5npd8xwtuwz80ppvrpfps0eyzw3y80h5vymf86mxkyw8psaaxkcnq9qyyssqs6ylfjhcpyx5epj80ynzw56c6wcrckl57jtt6uzf83wjd8uw7mypyl9qf7h6gfehkh08vy0kq7ktzfjds859jfh0eafpflz9j8vgnusp0fj0np").unwrap();
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
            // Handle any messages we get from core
            Message::CoreMessage(msg) => match msg {
                CoreUIMsg::Sending => {
                    self.send_status = SendStatus::Sending;
                    Command::none()
                }
                CoreUIMsg::SendSuccess => {
                    self.send_status = SendStatus::Idle;
                    Command::none()
                }
                CoreUIMsg::SendFailure(reason) => {
                    self.send_status = SendStatus::Idle;
                    self.send_failure_reason = Some(reason);
                    Command::none()
                }
                CoreUIMsg::ReceiveSuccess => Command::none(),
                CoreUIMsg::ReceiveFailed(reason) => {
                    // todo use receive failure reason
                    self.send_status = SendStatus::Idle;
                    self.send_failure_reason = Some(reason);
                    Command::none()
                }
                CoreUIMsg::BalanceUpdated(balance) => {
                    self.balance = balance;
                    Command::none()
                }
            },
        }
    }

    fn view(&self) -> Element<Message> {
        let sidebar = crate::components::sidebar(self);

        let home_content = crate::routes::home(self);
        let mints_content = crate::routes::mints(self);
        let transfer_content = crate::routes::transfer(self);

        let active_route = match self.active_route {
            Route::Home => home_content,
            Route::Mints => mints_content,
            Route::Transfer => transfer_content,
            _ => home_content,
        };

        row![sidebar, active_route]
            .align_items(Alignment::Center)
            .into()
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
