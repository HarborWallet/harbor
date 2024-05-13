use core::run_core;
use std::sync::Arc;
use fedimint_core::Amount;

use bridge::CoreUIMsg;
use components::{h_button, lighten, sidebar_button, SvgIcon};
use iced::subscription::Subscription;
use iced::widget::container::Style;
use iced::widget::{
    center, column, container, row, scrollable, text, text_input, vertical_space, Svg,
};
use iced::{program, Border, Color, Shadow};
use iced::{Alignment, Element, Length};
use iced::{Command, Font};

pub mod bridge;
pub mod components;
pub mod core;
pub mod conf;
mod fedimint_client;

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
struct HarborWallet {
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

fn home(harbor: &HarborWallet) -> Element<Message> {
    let balance = text(format!("{} sats", harbor.balance.sats_round_down())).size(64);
    let send_button = h_button("Send", SvgIcon::UpRight).on_press(Message::Send(100));
    let receive_button = h_button("Receive", SvgIcon::DownLeft).on_press(Message::Noop);
    let buttons = row![send_button, receive_button].spacing(32);

    let failure_message = harbor
        .send_failure_reason
        .as_ref()
        .map(|r| text(r).size(50).color(Color::from_rgb(255., 0., 0.)));

    let column = if let Some(failure_message) = failure_message {
        column![balance, failure_message, buttons]
    } else {
        column![balance, buttons]
    };
    container(center(column.spacing(32).align_items(Alignment::Center))).into()
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
    async fn async_send(ui_handle: Option<Arc<bridge::UIHandle>>, amount: u64) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().send(amount).await;
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
        let sidebar = container(
            column![
                Svg::from_path("assets/harbor_logo.svg").width(167),
                sidebar_button("Home", SvgIcon::Home, Route::Home, self.active_route)
                    .on_press(Message::Navigate(Route::Home)),
                sidebar_button("Mints", SvgIcon::People, Route::Mints, self.active_route)
                    .on_press(Message::Navigate(Route::Mints)),
                sidebar_button(
                    "Transfer",
                    SvgIcon::LeftRight,
                    Route::Transfer,
                    self.active_route
                )
                .on_press(Message::Navigate(Route::Transfer)),
                sidebar_button(
                    "History",
                    SvgIcon::Squirrel,
                    Route::History,
                    self.active_route
                )
                .on_press(Message::Navigate(Route::History)),
                vertical_space(),
                sidebar_button(
                    "Settings",
                    SvgIcon::Settings,
                    Route::Settings,
                    self.active_route
                )
                .on_press(Message::Navigate(Route::Settings)),
            ]
            .spacing(8)
            .align_items(Alignment::Start),
        )
        .padding(8)
        .style(|theme| -> Style {
            Style {
                text_color: None,
                background: Some(lighten(theme.palette().background, 0.05).into()),
                border: Border::default(),
                shadow: Shadow::default(),
            }
        });

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
