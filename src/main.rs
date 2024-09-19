use bitcoin::address::NetworkUnchecked;
use bitcoin::Address;
use components::{FederationItem, Toast, ToastManager, ToastStatus, TransactionItem};
use core::run_core;
use fedimint_core::core::ModuleKind;
use fedimint_core::invite_code::InviteCode;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use iced::widget::qr_code::Data;
use routes::Route;
use std::str::FromStr;
use std::sync::Arc;

use bridge::{CoreUIMsg, CoreUIMsgPacket, ReceiveSuccessMsg, SendSuccessMsg};
use iced::subscription::Subscription;
use iced::widget::row;
use iced::Element;
use iced::{clipboard, program, Color};
use iced::{Command, Font};
use log::{error, info};
use uuid::Uuid;

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
enum WelcomeStatus {
    #[default]
    Loading,
    NeedsInit,
    Inited,
    Initing,
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
    SetIsMax(bool),
    SendStateReset,
    PasswordInputChanged(String),
    SeedInputChanged(String),
    MintInviteCodeInputChanged(String),
    DonateAmountChanged(String),
    CopyToClipboard(String),
    ReceiveMethodChanged(ReceiveMethod),
    ShowSeedWords(bool),
    AddToast(Toast),
    CloseToast(usize),
    CancelAddFederation,
    // Async commands we fire from the UI to core
    Noop,
    Send(String),
    GenerateInvoice,
    GenerateAddress,
    Unlock(String),
    Init(String), // TODO add seed option
    AddFederation(String),
    PeekFederation(String),
    Donate,
    // Core messages we get from core
    CoreMessage(CoreUIMsgPacket),
}

impl Message {
    pub fn core_msg(id: Option<Uuid>, msg: CoreUIMsg) -> Self {
        Self::CoreMessage(CoreUIMsgPacket { id, msg })
    }
}

// This is the UI state. It should only contain data that is directly rendered by the UI
// More complicated state should be in Core, and bridged to the UI in a UI-friendly format.
#[derive(Default, Debug)]
pub struct HarborWallet {
    ui_handle: Option<Arc<bridge::UIHandle>>,
    active_route: Route,
    toasts: Vec<Toast>,
    // Globals
    balance_sats: u64,
    transaction_history: Vec<TransactionItem>,
    federation_list: Vec<FederationItem>,
    // Welcome screen
    init_status: WelcomeStatus,
    seed_input_str: String,
    init_failure_reason: Option<String>,
    // Lock screen
    password_input_str: String,
    unlock_status: UnlockStatus,
    unlock_failure_reason: Option<String>,
    // Send
    send_status: SendStatus,
    send_failure_reason: Option<String>,
    send_success_msg: Option<SendSuccessMsg>,
    send_dest_input_str: String,
    send_amount_input_str: String,
    is_max: bool,
    // Receive
    receive_failure_reason: Option<String>,
    receive_success_msg: Option<ReceiveSuccessMsg>,
    receive_status: ReceiveStatus,
    receive_amount_str: String,
    receive_invoice: Option<Bolt11Invoice>,
    receive_address: Option<Address>,
    receive_qr_data: Option<Data>,
    receive_method: ReceiveMethod,
    // Mints
    peek_federation_failure_reason: Option<String>,
    peek_federation_item: Option<FederationItem>,
    mint_invite_code_str: String,
    add_federation_failure_reason: Option<String>,
    // Donate
    donate_amount_str: String,
    // Settings
    settings_show_seed_words: bool,
    seed_words: Option<String>,
    current_send_id: Option<Uuid>,
    current_receive_id: Option<Uuid>,
}

impl HarborWallet {
    fn subscription(&self) -> Subscription<Message> {
        run_core()
    }

    async fn async_send_lightning(
        ui_handle: Option<Arc<bridge::UIHandle>>,
        id: Uuid,
        invoice: Bolt11Invoice,
    ) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.send_lightning(id, invoice).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_send_onchain(
        ui_handle: Option<Arc<bridge::UIHandle>>,
        id: Uuid,
        address: Address<NetworkUnchecked>,
        amount_sats: Option<u64>,
    ) {
        println!("Got to async_send");
        if let Some(ui_handle) = ui_handle {
            ui_handle.send_onchain(id, address, amount_sats).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_receive(ui_handle: Option<Arc<bridge::UIHandle>>, id: Uuid, amount: u64) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.receive(id, amount).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_receive_onchain(ui_handle: Option<Arc<bridge::UIHandle>>, id: Uuid) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.receive_onchain(id).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_unlock(ui_handle: Option<Arc<bridge::UIHandle>>, id: Uuid, password: String) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.unlock(id, password).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_init(ui_handle: Option<Arc<bridge::UIHandle>>, id: Uuid, password: String) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.clone().init(id, password).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_add_federation(
        ui_handle: Option<Arc<bridge::UIHandle>>,
        id: Uuid,
        invite: InviteCode,
    ) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.add_federation(id, invite).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_peek_federation(
        ui_handle: Option<Arc<bridge::UIHandle>>,
        id: Uuid,
        invite: InviteCode,
    ) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.peek_federation(id, invite).await;
        } else {
            panic!("UI handle is None");
        }
    }

    async fn async_get_seed_words(ui_handle: Option<Arc<bridge::UIHandle>>, id: Uuid) {
        if let Some(ui_handle) = ui_handle {
            ui_handle.get_seed_words(id).await;
        } else {
            panic!("UI handle is None");
        }
    }

    fn clear_add_federation_state(&mut self) {
        self.add_federation_failure_reason = None;
        self.peek_federation_failure_reason = None;
        self.peek_federation_item = None;
        self.mint_invite_code_str = String::new();
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // Setup
            Message::UIHandlerLoaded(ui_handle) => {
                self.ui_handle = Some(ui_handle);
                println!("Core loaded");
                Command::none()
            }
            // Internal app state stuff like navigation and text inputs
            Message::Navigate(route) => {
                match self.active_route {
                    // Reset the seed words state when we leave the settings screen
                    Route::Settings => {
                        self.settings_show_seed_words = false;
                        self.active_route = route;
                    }
                    // Reset the add federation state when leaving mints
                    Route::Mints(_) => match route {
                        // Staying in mints, don't reset
                        Route::Mints(_) => {
                            self.active_route = route;
                        }
                        _ => {
                            self.clear_add_federation_state();
                            self.active_route = route;
                        }
                    },
                    _ => self.active_route = route,
                }
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
            Message::SetIsMax(is_max) => {
                self.is_max = is_max;
                Command::none()
            }
            Message::PasswordInputChanged(input) => {
                self.password_input_str = input;
                Command::none()
            }
            Message::SeedInputChanged(input) => {
                self.seed_input_str = input;
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
                self.is_max = false;
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
            Message::AddToast(toast) => {
                self.toasts.push(toast);
                Command::none()
            }
            Message::CloseToast(index) => {
                self.toasts.remove(index);
                Command::none()
            }
            Message::CancelAddFederation => {
                self.clear_add_federation_state();
                self.active_route = Route::Mints(routes::MintSubroute::List);

                Command::none()
            }
            // Async commands we fire from the UI to core
            Message::Noop => Command::none(),
            Message::Send(invoice_str) => match self.send_status {
                SendStatus::Sending => Command::none(),
                _ => {
                    self.send_failure_reason = None;
                    let id = Uuid::new_v4();
                    self.current_send_id = Some(id);
                    if let Ok(invoice) = Bolt11Invoice::from_str(&invoice_str) {
                        Command::perform(
                            Self::async_send_lightning(self.ui_handle.clone(), id, invoice),
                            |_| Message::Noop,
                        )
                    } else if let Ok(address) = Address::from_str(&invoice_str) {
                        let amount = if self.is_max {
                            None
                        } else {
                            // TODO: error handling
                            Some(self.send_amount_input_str.parse::<u64>().unwrap())
                        };
                        Command::perform(
                            Self::async_send_onchain(self.ui_handle.clone(), id, address, amount),
                            |_| Message::Noop,
                        )
                    } else {
                        error!("Invalid invoice or address");
                        self.current_send_id = None;
                        Command::none()
                    }
                }
            },
            Message::GenerateInvoice => match self.receive_status {
                ReceiveStatus::Generating => Command::none(),
                _ => {
                    let id = Uuid::new_v4();
                    self.current_receive_id = Some(id);
                    self.receive_failure_reason = None;
                    match self.receive_amount_str.parse::<u64>() {
                        Ok(amount) => Command::perform(
                            Self::async_receive(self.ui_handle.clone(), id, amount),
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
                    let id = Uuid::new_v4();
                    self.current_receive_id = Some(id);
                    self.receive_failure_reason = None;
                    Command::perform(
                        Self::async_receive_onchain(self.ui_handle.clone(), id),
                        |_| Message::Noop,
                    )
                }
            },
            Message::Donate => match self.donate_amount_str.parse::<u64>() {
                Ok(amount) => {
                    // TODO: don't hardcode this!
                    let hardcoded_donation_address = "tb1qd28npep0s8frcm3y7dxqajkcy2m40eysplyr9v";
                    let address = Address::from_str(hardcoded_donation_address).unwrap();
                    let id = Uuid::new_v4();
                    self.current_send_id = Some(id);

                    Command::perform(
                        Self::async_send_onchain(self.ui_handle.clone(), id, address, Some(amount)),
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
                    let id = Uuid::new_v4(); // todo use this id somewhere
                    Command::perform(
                        Self::async_unlock(self.ui_handle.clone(), id, password),
                        |_| Message::Noop,
                    )
                }
            },
            Message::Init(password) => match self.unlock_status {
                UnlockStatus::Unlocking => Command::none(),
                _ => {
                    self.unlock_failure_reason = None;
                    let id = Uuid::new_v4(); // todo use this id somewhere
                    Command::perform(
                        Self::async_init(self.ui_handle.clone(), id, password),
                        |_| Message::Noop,
                    )
                }
            },
            Message::AddFederation(invite_code) => {
                let invite = InviteCode::from_str(&invite_code);
                if let Ok(invite) = invite {
                    let id = Uuid::new_v4(); // todo use this id somewhere
                    Command::perform(
                        Self::async_add_federation(self.ui_handle.clone(), id, invite),
                        |_| Message::Noop,
                    )
                } else {
                    self.add_federation_failure_reason = Some("Invalid invite code".to_string());
                    Command::none()
                }
            }
            Message::PeekFederation(invite_code) => {
                let invite = InviteCode::from_str(&invite_code);
                if let Ok(invite) = invite {
                    self.add_federation_failure_reason = None;
                    let id = Uuid::new_v4(); // todo use this id somewhere
                    Command::perform(
                        Self::async_peek_federation(self.ui_handle.clone(), id, invite),
                        |_| Message::Noop,
                    )
                } else {
                    self.peek_federation_failure_reason = Some("Invalid invite code".to_string());
                    Command::none()
                }
            }
            Message::CopyToClipboard(s) => Command::batch([
                clipboard::write(s),
                Command::perform(async {}, |_| {
                    Message::AddToast(Toast {
                        title: "Copied to clipboard".to_string(),
                        body: "...".to_string(),
                        status: ToastStatus::Neutral,
                    })
                }),
            ]),
            Message::ShowSeedWords(show) => {
                if show {
                    let id = Uuid::new_v4(); // todo use this id somewhere
                    Command::perform(
                        Self::async_get_seed_words(self.ui_handle.clone(), id),
                        |_| Message::Noop,
                    )
                } else {
                    self.settings_show_seed_words = false;
                    Command::none()
                }
            }
            // Handle any messages we get from core
            Message::CoreMessage(msg) => match msg.msg {
                CoreUIMsg::Sending => {
                    if self.current_send_id == msg.id {
                        self.send_status = SendStatus::Sending;
                    }
                    Command::none()
                }
                CoreUIMsg::SendSuccess(params) => {
                    info!("Send success: {params:?}");
                    if self.current_send_id == msg.id {
                        self.send_success_msg = Some(params);
                        self.current_send_id = None;
                    }
                    Command::none()
                }
                CoreUIMsg::SendFailure(reason) => {
                    if self.current_send_id == msg.id {
                        self.send_status = SendStatus::Idle;
                        self.send_failure_reason = Some(reason);
                        self.current_send_id = None;
                    }
                    Command::none()
                }
                CoreUIMsg::ReceiveSuccess(params) => {
                    info!("Receive success: {params:?}");
                    if self.current_receive_id == msg.id {
                        self.receive_success_msg = Some(params);
                        self.current_receive_id = None;
                    }
                    Command::none()
                }
                CoreUIMsg::ReceiveFailed(reason) => {
                    if self.current_receive_id == msg.id {
                        self.receive_status = ReceiveStatus::Idle;
                        self.receive_failure_reason = Some(reason);
                        self.current_receive_id = None;
                    }
                    Command::none()
                }
                CoreUIMsg::BalanceUpdated(balance) => {
                    self.balance_sats = balance.sats_round_down();
                    Command::none()
                }
                CoreUIMsg::TransactionHistoryUpdated(history) => {
                    self.transaction_history = history;
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
                    self.peek_federation_item = None;
                    Command::none()
                }
                CoreUIMsg::FederationInfo(config) => {
                    // todo update the UI with the new config
                    let id = config.calculate_federation_id();
                    let name = config.meta::<String>("federation_name");
                    let guardians: Vec<String> = config
                        .global
                        .api_endpoints
                        .values()
                        .map(|url| url.name.clone())
                        .collect();

                    let module_kinds = config
                        .modules
                        .into_values()
                        .map(|module_config| module_config.kind().to_owned())
                        .collect::<Vec<ModuleKind>>();

                    let name = match name {
                        Ok(Some(n)) => n,
                        _ => "Unknown".to_string(),
                    };

                    // TODO: what to do about balance in this case? Maybe it should be Option<u64>?
                    let item = FederationItem {
                        id,
                        name,
                        balance: 0,
                        guardians: Some(guardians),
                        module_kinds: Some(module_kinds),
                    };

                    self.peek_federation_item = Some(item);

                    Command::none()
                }
                CoreUIMsg::AddFederationSuccess => {
                    self.mint_invite_code_str = String::new();
                    self.active_route = Route::Mints(routes::MintSubroute::List);
                    self.peek_federation_item = None;
                    Command::none()
                }
                CoreUIMsg::FederationListUpdated(list) => {
                    self.federation_list = list;
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
                CoreUIMsg::NeedsInit => {
                    info!("Got init message");
                    self.init_status = WelcomeStatus::NeedsInit;
                    focus_input_id("password_init_input")
                }
                CoreUIMsg::Initing => {
                    self.init_status = WelcomeStatus::Initing;
                    Command::none()
                }
                CoreUIMsg::InitSuccess => {
                    self.init_status = WelcomeStatus::Inited;
                    self.active_route = Route::Home;
                    Command::none()
                }
                CoreUIMsg::InitFailed(reason) => {
                    self.init_status = WelcomeStatus::NeedsInit;
                    self.init_failure_reason = Some(reason);
                    Command::none()
                }
                CoreUIMsg::Locked => {
                    info!("Got locked message");
                    self.active_route = Route::Unlock;
                    focus_input_id("password_unlock_input")
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
                CoreUIMsg::SeedWords(words) => {
                    self.seed_words = Some(words);
                    self.settings_show_seed_words = true;
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
            Route::Mints(_) => row![sidebar, crate::routes::mints(self)].into(),
            Route::Donate => row![sidebar, crate::routes::donate(self)].into(),
            Route::History => row![sidebar, crate::routes::history(self)].into(),
            Route::Transfer => row![sidebar, crate::routes::transfer(self)].into(),
            Route::Settings => row![sidebar, crate::routes::settings(self)].into(),
            Route::Welcome => crate::routes::welcome(self),
        };

        ToastManager::new(active_route, &self.toasts, Message::CloseToast).into()
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
