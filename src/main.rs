use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;

use bridge::BridgeError;
use iced::futures::channel::mpsc::Sender;
use iced::futures::SinkExt;
use iced::mouse;
use iced::program;
use iced::subscription::{self, Subscription};
use iced::widget::{
    button, canvas, center, checkbox, column, container, horizontal_space, pick_list, row,
    scrollable, text, text_input,
};
use iced::Command;
use iced::Program;
use iced::{Alignment, Element, Length};
use tokio::time::sleep;

pub mod bridge;
pub mod core;

use crate::bridge::UICoreMsg;

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
    balance: u64,
    active_route: Route,
    transfer_amount_str: String,
    send_status: SendStatus,
    ui_handle: Option<Arc<bridge::UIHandle>>,
    test_message: String,
}

impl Default for HarborWallet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug, Clone, Copy)]
enum Route {
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
    Sent,
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Route),
    TransferAmountChanged(String),
    Send(u64),
    Receive(u64),
    CoreLoaded(Arc<bridge::UIHandle>),
    SetBalance(u64),
    SetIsSending,
    SetIsDoneSending,
    SetSendResult(Result<(), BridgeError>),
}

fn home(harbor: &HarborWallet) -> Element<Message> {
    container(
        scrollable(
            column![
                "Home",
                text(harbor.balance).size(50),
                text(format!("{:?}", harbor.send_status)).size(50),
                row![
                    button("Send").on_press(Message::Send(100)),
                    button("Receive").on_press(Message::Receive(100))
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

fn mints(harbor: &HarborWallet) -> Element<Message> {
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
            balance: 0,
            active_route: Route::Home,
            transfer_amount_str: String::new(),
            send_status: SendStatus::Idle,
            ui_handle: None,
            test_message: String::new(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        struct Connect;
        subscription::channel(
            std::any::TypeId::of::<Connect>(),
            100,
            |mut tx: Sender<Message>| async move {
                enum State {
                    NeedsInit,
                    Running,
                }
                let mut state = State::NeedsInit;

                let handles = bridge::create_handles();
                let (ui_handle, mut core_handle) = handles;
                let arc_ui_handle = Arc::new(ui_handle);
                let mut balance = 4000;

                loop {
                    match &mut state {
                        State::NeedsInit => {
                            tx.send(Message::CoreLoaded(arc_ui_handle.clone()))
                                .await
                                .unwrap();
                            tx.send(Message::SetBalance(balance)).await.unwrap();
                            state = State::Running;
                        }
                        State::Running => {
                            let msg = core_handle.recv().await;
                            if let Some(msg) = msg {
                                match msg {
                                    UICoreMsg::Test(counter) => {
                                        println!("{counter}");
                                    }
                                    UICoreMsg::Send(amount) => {
                                        tx.send(Message::SetIsSending).await.unwrap();
                                        sleep(Duration::from_secs(1)).await;
                                        println!("Sending {amount}");
                                        if let Some(b) = balance.checked_sub(amount) {
                                            balance = b;
                                            tx.send(Message::SetBalance(balance)).await.unwrap();
                                        }
                                        tx.send(Message::SetSendResult(Ok(()))).await.unwrap();
                                    }
                                }
                            }
                        }
                    }
                }
            },
        )
    }

    async fn send(&self, amount: u64) -> Result<(), BridgeError> {
        self.ui_handle.as_ref().unwrap().clone().send(amount).await;
        Ok(())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // Internal app state stuff like navigation and text inputs
            Message::Navigate(route) => {
                self.active_route = route;
                Command::none()
            }
            Message::TransferAmountChanged(amount) => {
                self.transfer_amount_str = amount;
                Command::none()
            }
            Message::Send(amount) => match self.send_status {
                SendStatus::Sending => Command::none(),
                _ => {
                    // let ui_handle = self.ui_handle.as_ref().unwrap().clone();
                    Command::perform(self.send(amount), |_| Message::SetIsDoneSending)
                }
            },
            Message::CoreLoaded(ui_handle) => {
                self.ui_handle = Some(ui_handle);
                Command::none()
            }
            // Send
            Message::SetIsSending => {
                self.send_status = SendStatus::Sending;
                Command::none()
            }
            Message::SetIsDoneSending => {
                self.send_status = SendStatus::Idle;
                Command::none()
            }
            Message::SetBalance(balance) => {
                self.balance = balance;
                Command::none()
            }
            Message::SetSendResult(result) => {
                self.send_status = SendStatus::Idle;
                match result {
                    Ok(_) => {}
                    Err(e) => {}
                }
                Command::none()
            }
            Message::Receive(amount) => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        let sidebar = container(
            column![
                text(self.test_message.clone()).size(50),
                "Sidebar!",
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
