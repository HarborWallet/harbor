use core::run_core;
use std::sync::Arc;

use bridge::CoreUIMsg;
use iced::subscription::Subscription;
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::Command;
use iced::{program, Color};
use iced::{Alignment, Element, Length};

pub mod bridge;
pub mod core;

// This starts the program. Importantly, it registers the update and view methods, along with a subscription.
// We can also run logic during load if we need to.
pub fn main() -> iced::Result {
    program("Harbor", HarborWallet::update, HarborWallet::view)
        // .load(HarborWallet::load)
        .subscription(HarborWallet::subscription)
        .run()
}

// This is the UI state. It should only contain data that is directly rendered by the UI
// More complicated state should be in Core, and bridged to the UI in a UI-friendly format.
struct HarborWallet {
    ui_handle: Option<Arc<bridge::UIHandle>>,
    balance: u64,
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

#[derive(Default, Debug, Clone, Copy)]
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
    // Core messages we get from core
    CoreMessage(CoreUIMsg),
}

fn home(harbor: &HarborWallet) -> Element<Message> {
    // TODO: figure out a way to only optionally show this
    let failure_message = if let Some(r) = &harbor.send_failure_reason {
        text(r).size(50).color(Color::from_rgb(255., 0., 0.))
    } else {
        text("")
    };
    container(
        scrollable(
            column![
                "Home",
                text(harbor.balance).size(50),
                text(format!("{:?}", harbor.send_status)).size(50),
                failure_message,
                row![
                    button("Send").on_press(Message::Send(100)),
                    button("Receive").on_press(Message::Noop)
                ]
                .spacing(16)
            ]
            .spacing(32)
            .align_items(Alignment::Center)
            .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}

fn mints(_harbor: &HarborWallet) -> Element<Message> {
    container(
        scrollable(
            column!["These are the mints!",]
                .spacing(32)
                .align_items(Alignment::Center)
                .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}

fn transfer(harbor: &HarborWallet) -> Element<Message> {
    container(
        scrollable(
            column![
                "Let's transfer some ecash!",
                text_input("how much?", &harbor.transfer_amount_str)
                    .on_input(Message::TransferAmountChanged,)
            ]
            .spacing(32)
            .align_items(Alignment::Center)
            .width(Length::Fill),
        )
        .height(Length::Fill),
    )
    .into()
}

impl HarborWallet {
    fn new() -> Self {
        Self {
            ui_handle: None,
            balance: 0,
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
    async fn async_send(ui_handle: Option<Arc<bridge::UIHandle>>, amount: u64) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().send(amount).await;
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
            Message::Send(amount) => match self.send_status {
                SendStatus::Sending => Command::none(),
                _ => {
                    self.send_failure_reason = None;
                    Command::perform(Self::async_send(self.ui_handle.clone(), amount), |_| {
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
                CoreUIMsg::BalanceUpdated(balance) => {
                    self.balance = balance;
                    Command::none()
                }
            },
        }
    }

    fn view(&self) -> Element<Message> {
        let sidebar = container(
            column![
                button("Home").on_press(Message::Navigate(Route::Home)),
                button("Mints").on_press(Message::Navigate(Route::Mints)),
                button("Transfer").on_press(Message::Navigate(Route::Transfer)),
                button("History").on_press(Message::Navigate(Route::History)),
                button("Settings").on_press(Message::Navigate(Route::Settings)),
            ]
            .spacing(16),
        );

        let home_content = home(self);
        let mints_content = mints(self);
        let transfer_content = transfer(self);

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
}
