#![windows_subsystem = "windows"]

use crate::bridge::run_core;
use crate::components::confirm_modal::{BasicModalState, ConfirmModalState};
use crate::components::focus_input_id;
use crate::components::{Toast, ToastManager, ToastStatus};
use crate::config::{Config, write_config};
use components::{MUTINY_GREEN, MUTINY_RED};
use harbor_client::Bolt11Invoice;
use harbor_client::bip39::Mnemonic;
use harbor_client::bitcoin::{Address, Network};
use harbor_client::cdk::mint_url::MintUrl;
use harbor_client::db_models::MintItem;
use harbor_client::db_models::transaction_item::TransactionItem;
use harbor_client::fedimint_core::Amount;
use harbor_client::fedimint_core::core::ModuleKind;
use harbor_client::fedimint_core::invite_code::InviteCode;
use harbor_client::lightning_address::parse_lnurl;
use harbor_client::{
    CoreUIMsg, CoreUIMsgPacket, MintIdentifier, ReceiveSuccessMsg, SendSuccessMsg, UICoreMsg,
    data_dir,
};
use iced::Font;
use iced::Subscription;
use iced::Task;
use iced::widget::qr_code::Data;
use iced::widget::row;
use iced::{Color, clipboard};
use iced::{Element, window};
use log::{debug, error, info, trace};
use routes::Route;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub mod bridge;
pub mod components;
mod config;
pub mod keyring;
pub mod lock;
pub mod routes;

// This starts the program. Importantly, it registers the update and view methods, along with a subscription.
// We can also run logic during load if we need to.
pub fn main() -> iced::Result {
    // Acquire the app lock - this prevents multiple instances from running
    if let Err(e) = lock::AppLock::acquire() {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    #[cfg(target_os = "macos")]
    let window_settings = window::Settings {
        platform_specific: window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..Default::default()
    };

    // If not macos, use default window settings
    #[cfg(not(target_os = "macos"))]
    let window_settings = window::Settings::default();

    iced::application("Harbor", HarborWallet::update, HarborWallet::view)
        .font(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-Bold.ttf").as_slice())
        .theme(HarborWallet::theme)
        .window(window_settings)
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

#[derive(Default, Debug, Clone, PartialEq)]
pub enum PeekStatus {
    #[default]
    Idle,
    Peeking,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum AddFederationStatus {
    #[default]
    Idle,
    Adding,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Setup
    UIHandlerLoaded(Arc<bridge::UIHandle>),
    ConfigLoaded(Config),
    InitError(String),
    // Local state changes
    Navigate(Route),
    SetConfirmModal(Option<ConfirmModalState>),
    SetBasicModal(Option<BasicModalState>),
    ReceiveAmountChanged(String),
    ReceiveStateReset,
    SendDestInputChanged(String),
    SendAmountInputChanged(String),
    SetIsMax(bool),
    SendStateReset,
    TransferStateReset,
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
    SetTransferFrom(String),
    SetTransferTo(String),
    TransferAmountInputChanged(String),
    UrlClicked(String),
    OpenUrl(String),
    SelectTransaction(Option<TransactionItem>),
    OpenDataDirectory,
    TestStatusUpdates,
    // Batch multiple messages together
    Batch(Vec<Message>),
    // Config commands
    ChangeNetwork(Network),
    SetTorEnabled(bool),
    // Async commands we fire from the UI to core
    Noop,
    Send(String),
    Transfer,
    GenerateInvoice,
    GenerateAddress,
    Unlock(String),
    Init {
        password: String,
        seed: Option<String>,
    },
    AddMint(String),
    RejoinMint(MintIdentifier),
    PeekMint(String),
    RemoveMint(MintIdentifier),
    ChangeMint(MintIdentifier),
    Donate,
    SetOnchainReceiveEnabled(bool),
    // Core messages we get from core
    CoreMessage(CoreUIMsgPacket),
    CancelReceiveGeneration,
}

impl Message {
    pub fn core_msg(id: Option<Uuid>, msg: CoreUIMsg) -> Self {
        Self::CoreMessage(CoreUIMsgPacket { id, msg })
    }
}

// This is the UI state. It should only contain data that is directly rendered by the UI
// More complicated state should be in Core, and bridged to the UI in a UI-friendly format.
#[derive(Debug, Clone)]
pub struct OperationStatus {
    pub message: String,
}

#[derive(Default, Debug)]
pub struct HarborWallet {
    ui_handle: Option<Arc<bridge::UIHandle>>,
    config: Config,
    active_route: Route,
    toasts: Vec<Toast>,
    // Globals
    transaction_history: Vec<TransactionItem>,
    selected_transaction: Option<TransactionItem>,
    mint_list: Vec<MintItem>,
    active_mint: Option<MintIdentifier>,
    // Modal
    confirm_modal: Option<ConfirmModalState>,
    basic_modal: Option<BasicModalState>,
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
    input_has_amount: bool,
    current_send_id: Option<Uuid>,
    current_receive_id: Option<Uuid>,
    current_transfer_id: Option<Uuid>,
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
    peek_federation_item: Option<MintItem>,
    mint_invite_code_str: String,
    peek_status: PeekStatus,
    add_federation_status: AddFederationStatus,
    current_peek_id: Option<Uuid>,
    current_add_id: Option<Uuid>,
    current_rejoin_id: Option<MintIdentifier>,
    // Transfer
    transfer_from_federation_selection: Option<String>,
    transfer_to_federation_selection: Option<String>,
    transfer_amount_input_str: String,
    transfer_status: SendStatus,
    // Donate
    donate_amount_str: String,
    // Settings
    settings_show_seed_words: bool,
    seed_words: Option<String>,
    tor_enabled: bool,
    // Onboarding
    show_add_a_mint_cta: bool,
    has_navigated_to_mints: bool,
    onchain_receive_enabled: bool,
    /// Tracks ongoing operations and their status
    operation_status: HashMap<Uuid, OperationStatus>,
}

impl HarborWallet {
    fn active_federation(&self) -> Option<&MintItem> {
        self.active_mint
            .as_ref()
            .and_then(|id| self.mint_list.iter().find(|f| &f.id == id))
    }

    fn next_federation(&self, name: &str) -> MintItem {
        let fed = self
            .mint_list
            .iter()
            .find(|f| f.name == name)
            .expect("Federation not found");
        self.mint_list
            .iter()
            .find(|f| f.id != fed.id && fed.active)
            .expect("No next federation found")
            .clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(run_core)
    }

    // Helper function to handle common UI handle pattern
    async fn with_ui_handle<F, Fut>(ui_handle: Option<Arc<bridge::UIHandle>>, f: F)
    where
        F: FnOnce(Arc<bridge::UIHandle>) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        match ui_handle {
            Some(ui_handle) => {
                f(ui_handle).await;
            }
            _ => {
                panic!("UI handle is None");
            }
        }
    }

    fn clear_add_federation_state(&mut self) {
        self.peek_federation_item = None;
        self.mint_invite_code_str = String::new();
        self.peek_status = PeekStatus::Idle;
        self.add_federation_status = AddFederationStatus::Idle;
        self.current_peek_id = None;
        self.current_add_id = None;
        self.current_rejoin_id = None;
    }

    fn clear_receive_state(&mut self) {
        self.receive_failure_reason = None;
        self.receive_status = ReceiveStatus::Idle;
        self.receive_amount_str = String::new();
        self.receive_invoice = None;
        self.receive_address = None;
        self.receive_qr_data = None;
        self.receive_method = ReceiveMethod::Lightning;
        // We dont' clear the success msg so the history screen can show the most recent
        // transaction
    }

    fn clear_send_state(&mut self) {
        self.send_failure_reason = None;
        self.send_status = SendStatus::Idle;
        self.send_dest_input_str = String::new();
        self.send_amount_input_str = String::new();
        self.is_max = false;
        self.input_has_amount = false;
        self.confirm_modal = None;
        self.current_send_id = None;
        // We dont' clear the success msg so the history screen can show the most recent
        // transaction
    }

    fn clear_transfer_state(&mut self) {
        self.transfer_amount_input_str = String::new();
        self.transfer_to_federation_selection = None;
        self.transfer_from_federation_selection = None;
        self.transfer_status = SendStatus::Idle;
        self.confirm_modal = None;
        self.current_transfer_id = None;
    }

    fn send_from_ui(&self, msg: UICoreMsg) -> (Uuid, Task<Message>) {
        let id = Uuid::new_v4();
        let task = Task::perform(
            Self::with_ui_handle(self.ui_handle.clone(), move |h| async move {
                h.send_msg(id, msg).await
            }),
            |_| Message::Noop,
        );
        (id, task)
    }

    // Helper function to safely remove a toast by index
    fn remove_toast(&mut self, index: usize) {
        if index < self.toasts.len() {
            self.toasts.remove(index);
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Setup
            Message::UIHandlerLoaded(ui_handle) => {
                self.ui_handle = Some(ui_handle);
                Task::none()
            }
            Message::ConfigLoaded(config) => {
                self.config = config;
                Task::none()
            }
            Message::InitError(error) => {
                self.init_failure_reason = Some(error);
                Task::none()
            }
            Message::ChangeNetwork(network) => {
                if self.config.network == network {
                    return Task::none();
                }

                let mut new_config = self.config.clone();
                new_config.network = network;

                write_config(&new_config).expect("Failed to write config");

                // Relaunch the app with the new network
                lock::restart_app();
                Task::none()
            }
            Message::Batch(messages) => {
                Task::batch(messages.into_iter().map(|msg| self.update(msg)))
            }
            // Internal app state stuff like navigation and text inputs
            Message::Navigate(route) => {
                // Hide the add_a_mint_cta as soon as the user navs anywhere
                self.show_add_a_mint_cta = false;

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
                    _ => match route {
                        Route::Mints(_) => {
                            // Hide the add a mint cta when navigating to mints
                            self.has_navigated_to_mints = true;
                            self.active_route = route;
                        }
                        Route::Transfer => {
                            // Set default federation selections if they're not already set
                            if self.transfer_from_federation_selection.is_none()
                                || self.transfer_to_federation_selection.is_none()
                            {
                                // Get first two federation names
                                let fed_names: Vec<String> = self
                                    .mint_list
                                    .iter()
                                    .filter(|f| f.active)
                                    .map(|f| f.name.clone())
                                    .collect();
                                if fed_names.len() >= 2 {
                                    // Only set source if it's not already set
                                    if self.transfer_from_federation_selection.is_none() {
                                        self.transfer_from_federation_selection =
                                            Some(fed_names[0].clone());
                                    }
                                    // Only set destination if it's not already set
                                    if self.transfer_to_federation_selection.is_none() {
                                        self.transfer_to_federation_selection =
                                            Some(fed_names[1].clone());
                                    }
                                }
                            }
                            self.active_route = route;
                        }
                        _ => self.active_route = route,
                    },
                }
                Task::none()
            }
            Message::ReceiveAmountChanged(amount) => {
                self.receive_amount_str = amount;
                Task::none()
            }
            Message::SendDestInputChanged(input) => {
                let msats = Bolt11Invoice::from_str(&input)
                    .ok()
                    .and_then(|i| i.amount_milli_satoshis());
                self.input_has_amount = msats.is_some();
                if let Some(amt) = msats {
                    self.send_amount_input_str = (amt / 1_000).to_string();
                } else {
                    self.send_amount_input_str = String::from("");
                }
                self.send_dest_input_str = input;
                Task::none()
            }
            Message::SendAmountInputChanged(input) => {
                self.send_amount_input_str = input;
                Task::none()
            }
            Message::SetIsMax(is_max) => {
                self.is_max = is_max;
                Task::none()
            }
            Message::PasswordInputChanged(input) => {
                self.password_input_str = input;
                Task::none()
            }
            Message::SeedInputChanged(input) => {
                self.seed_input_str = input;
                Task::none()
            }
            Message::MintInviteCodeInputChanged(input) => {
                self.mint_invite_code_str = input;
                Task::none()
            }
            Message::DonateAmountChanged(input) => {
                self.donate_amount_str = input;
                Task::none()
            }
            Message::SendStateReset => {
                self.clear_send_state();
                Task::none()
            }
            Message::TransferStateReset => {
                self.clear_transfer_state();
                Task::none()
            }
            Message::ReceiveStateReset => {
                self.clear_receive_state();
                Task::none()
            }
            Message::ReceiveMethodChanged(method) => {
                self.receive_method = method;
                Task::none()
            }
            Message::AddToast(toast) => {
                self.toasts.push(toast);
                Task::none()
            }
            Message::CloseToast(index) => {
                self.remove_toast(index);
                Task::none()
            }
            Message::RejoinMint(id) => {
                info!("Rejoining mint: {id:?}");
                self.add_federation_status = AddFederationStatus::Adding;
                let (_, task) = self.send_from_ui(UICoreMsg::RejoinMint(id.clone()));
                // We need to know which federation we're rejoining so we use the federation id
                self.current_rejoin_id = Some(id);
                task
            }
            Message::CancelAddFederation => {
                self.clear_add_federation_state();
                self.active_route = Route::Mints(routes::MintSubroute::List);

                Task::none()
            }
            Message::SetTransferFrom(s) => {
                self.transfer_from_federation_selection = Some(s.clone());
                // If the to_federation is the same as the from_federation, we need to change it
                if self.transfer_to_federation_selection == self.transfer_from_federation_selection
                {
                    let fed = self.next_federation(&s);
                    self.transfer_to_federation_selection = Some(fed.name.clone());
                }
                Task::none()
            }
            Message::SetTransferTo(s) => {
                self.transfer_to_federation_selection = Some(s.clone());
                // If the from_federation is the same as the to_federation, we need to change it
                if self.transfer_from_federation_selection == self.transfer_to_federation_selection
                {
                    let fed = self.next_federation(&s);
                    self.transfer_from_federation_selection = Some(fed.name.clone());
                }
                Task::none()
            }
            Message::TransferAmountInputChanged(input) => {
                self.transfer_amount_input_str = input;
                Task::none()
            }
            Message::OpenDataDirectory => {
                let network = self.config.network;
                let dir = PathBuf::from(&data_dir(Some(network)));
                opener::reveal(&dir).expect("Failed to open data directory");
                Task::none()
            }
            Message::TestStatusUpdates => {
                let (_id, task) = self.send_from_ui(UICoreMsg::TestStatusUpdates);
                task
            }
            // Async commands we fire from the UI to core
            Message::Noop => Task::none(),
            Message::Send(invoice_str) => match self.send_status {
                SendStatus::Sending => Task::none(),
                _ => {
                    self.send_failure_reason = None;
                    let mint = match self.active_mint.clone() {
                        Some(f) => f,
                        None => {
                            error!("No active mint");
                            return Task::perform(async {}, |_| {
                                Message::AddToast(Toast {
                                    title: "Cannot send".to_string(),
                                    body: Some("No active mint selected".to_string()),
                                    status: ToastStatus::Bad,
                                })
                            });
                        }
                    };

                    if let Ok(invoice) = Bolt11Invoice::from_str(&invoice_str) {
                        let (id, task) =
                            self.send_from_ui(UICoreMsg::SendLightning { mint, invoice });
                        self.current_send_id = Some(id);
                        task
                    } else {
                        match parse_lnurl(&invoice_str) {
                            Ok(lnurl) => {
                                // TODO: can we handle is_max somehow?
                                let amount = if self.is_max {
                                    return Task::perform(async {}, |_| {
                                        Message::AddToast(Toast {
                                            title: "Cannot send max with Lightning Address"
                                                .to_string(),
                                            body: Some(
                                                "Please enter a specific amount".to_string(),
                                            ),
                                            status: ToastStatus::Bad,
                                        })
                                    });
                                } else {
                                    match self.send_amount_input_str.parse::<u64>() {
                                        Ok(amount) => amount,
                                        Err(e) => {
                                            error!("Error parsing amount: {e}");
                                            self.send_failure_reason = Some(e.to_string());
                                            return Task::none();
                                        }
                                    }
                                };
                                let (id, task) = self.send_from_ui(UICoreMsg::SendLnurlPay {
                                    mint,
                                    lnurl,
                                    amount_sats: amount,
                                });
                                self.current_send_id = Some(id);
                                task
                            }
                            _ => {
                                if let Ok(address) = Address::from_str(&invoice_str) {
                                    let amount = if self.is_max {
                                        None
                                    } else {
                                        match self.send_amount_input_str.parse::<u64>() {
                                            Ok(amount) => Some(amount),
                                            Err(e) => {
                                                error!("Error parsing amount: {e}");
                                                self.send_failure_reason = Some(e.to_string());
                                                return Task::none();
                                            }
                                        }
                                    };
                                    let (id, task) = self.send_from_ui(UICoreMsg::SendOnChain {
                                        mint,
                                        address,
                                        amount_sats: amount,
                                    });
                                    self.current_send_id = Some(id);
                                    task
                                } else {
                                    error!("Invalid invoice or address");
                                    self.current_send_id = None;
                                    Task::perform(async {}, |_| {
                                        Message::AddToast(Toast {
                                            title: "Failed to send".to_string(),
                                            body: Some("Invalid invoice or address".to_string()),
                                            status: ToastStatus::Bad,
                                        })
                                    })
                                }
                            }
                        }
                    }
                }
            },
            Message::Transfer => {
                let from = if let Some(name) = &self.transfer_from_federation_selection {
                    self.mint_list
                        .iter()
                        .find(|f| &f.name == name)
                        .unwrap()
                        .id
                        .clone()
                } else {
                    error!("No source federation selected");
                    return Task::none();
                };
                let to = if let Some(name) = &self.transfer_to_federation_selection {
                    self.mint_list
                        .iter()
                        .find(|f| &f.name == name)
                        .unwrap()
                        .id
                        .clone()
                } else {
                    error!("No destination federation selected");
                    return Task::none();
                };

                if from == to {
                    error!("Cannot transfer to same federation");
                    return Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Cannot transfer".to_string(),
                            body: Some("Cannot transfer to the same mint".to_string()),
                            status: ToastStatus::Bad,
                        })
                    });
                }

                let amount = match self.transfer_amount_input_str.parse::<u64>() {
                    Ok(a) => a,
                    Err(_) => {
                        error!("Invalid amount");
                        return Task::perform(async {}, |_| {
                            Message::AddToast(Toast {
                                title: "Invalid amount".to_string(),
                                body: Some("Please enter a valid number of sats".to_string()),
                                status: ToastStatus::Bad,
                            })
                        });
                    }
                };

                let (id, task) = self.send_from_ui(UICoreMsg::Transfer {
                    from,
                    to,
                    amount: Amount::from_sats(amount),
                });
                self.current_transfer_id = Some(id);
                self.transfer_status = SendStatus::Sending;
                task
            }
            Message::GenerateInvoice => match self.receive_status {
                ReceiveStatus::Generating => Task::none(),
                _ => {
                    let mint = match self.active_mint.clone() {
                        Some(f) => f,
                        None => {
                            // This should be unreachable yeah?
                            panic!("No active federation, but we're trying to generate an invoice");
                        }
                    };
                    match self.receive_amount_str.parse::<u64>() {
                        Ok(amount) => {
                            let (id, task) = self.send_from_ui(UICoreMsg::ReceiveLightning {
                                mint,
                                amount: Amount::from_sats(amount),
                            });
                            self.current_receive_id = Some(id);
                            self.receive_failure_reason = None;
                            task
                        }
                        Err(e) => {
                            self.receive_amount_str = String::new();
                            error!("Error parsing amount: {e}");
                            Task::perform(async {}, move |_| {
                                Message::AddToast(Toast {
                                    title: "Failed to generate invoice".to_string(),
                                    body: Some(e.to_string()),
                                    status: ToastStatus::Bad,
                                })
                            })
                        }
                    }
                }
            },
            Message::GenerateAddress => match self.receive_status {
                ReceiveStatus::Generating => Task::none(),
                _ => {
                    let mint = match self.active_mint.clone() {
                        Some(f) => f,
                        None => {
                            // todo show error
                            error!("No active federation");
                            return Task::none();
                        }
                    };
                    let (id, task) = self.send_from_ui(UICoreMsg::ReceiveOnChain { mint });
                    self.current_receive_id = Some(id);
                    self.receive_failure_reason = None;
                    task
                }
            },
            Message::Donate => match self.donate_amount_str.parse::<u64>() {
                Ok(amount_sats) => {
                    // Check if we have an active mint
                    if self.active_mint.is_none() {
                        return Task::perform(async {}, |_| {
                            Message::AddToast(Toast {
                                title: "Cannot donate".to_string(),
                                body: Some("No active mint selected".to_string()),
                                status: ToastStatus::Bad,
                            })
                        });
                    }

                    // Clear any existing send state
                    self.clear_send_state();

                    // Prepopulate the send screen with donation details
                    self.send_dest_input_str = "hrf@btcpay.hrf.org".to_string();
                    self.send_amount_input_str = amount_sats.to_string();

                    // Clear the donate amount
                    self.donate_amount_str = String::new();

                    // Navigate to the send screen
                    self.active_route = Route::Send;

                    Task::none()
                }
                Err(e) => {
                    error!("Error parsing donate amount: {e}");
                    Task::perform(async {}, move |_| {
                        Message::AddToast(Toast {
                            title: "Invalid donation amount".to_string(),
                            body: Some("Please enter a valid number of sats".to_string()),
                            status: ToastStatus::Bad,
                        })
                    })
                }
            },
            Message::Unlock(password) => match self.unlock_status {
                UnlockStatus::Unlocking => Task::none(),
                _ => {
                    self.unlock_failure_reason = None;
                    let (_, task) = self.send_from_ui(UICoreMsg::Unlock(password));
                    task
                }
            },
            Message::Init { password, seed } => match self.unlock_status {
                UnlockStatus::Unlocking => Task::none(),
                _ => {
                    if password.is_empty() {
                        Task::perform(async {}, |_| {
                            Message::AddToast(Toast {
                                title: "Error".to_string(),
                                body: Some("Password cannot be empty".to_string()),
                                status: ToastStatus::Bad,
                            })
                        })
                    } else {
                        self.unlock_failure_reason = None;

                        let seed = match seed {
                            None => None,
                            Some(seed) => match Mnemonic::from_str(&seed) {
                                Ok(seed) => Some(seed),
                                Err(_) => {
                                    return Task::perform(async {}, |_| {
                                        Message::AddToast(Toast {
                                            title: "Error".to_string(),
                                            body: Some("Invalid seed words".to_string()),
                                            status: ToastStatus::Bad,
                                        })
                                    });
                                }
                            },
                        };

                        let (_, task) = self.send_from_ui(UICoreMsg::Init { password, seed });
                        task
                    }
                }
            },
            Message::AddMint(string) => match InviteCode::from_str(&string) {
                Ok(invite) => {
                    self.add_federation_status = AddFederationStatus::Adding;
                    let (id, task) = self.send_from_ui(UICoreMsg::AddFederation(invite));
                    self.current_add_id = Some(id);
                    task
                }
                Err(_) => match MintUrl::from_str(&string) {
                    Ok(mint_url) => {
                        self.add_federation_status = AddFederationStatus::Adding;
                        let (id, task) = self.send_from_ui(UICoreMsg::AddCashuMint(mint_url));
                        self.current_add_id = Some(id);
                        task
                    }
                    Err(_) => Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Can't add mint".to_string(),
                            body: Some("Invalid invite code".to_string()),
                            status: ToastStatus::Bad,
                        })
                    }),
                },
            },
            Message::PeekMint(string) => match InviteCode::from_str(&string) {
                Ok(invite) => {
                    if self.mint_list.iter().any(|m| {
                        m.id.federation_id()
                            .is_some_and(|f| f == invite.federation_id())
                    }) {
                        return Task::perform(async {}, |_| {
                            Message::AddToast(Toast {
                                title: "Mint already added".to_string(),
                                body: None,
                                status: ToastStatus::Bad,
                            })
                        });
                    }

                    self.peek_status = PeekStatus::Peeking;
                    let (id, task) = self.send_from_ui(UICoreMsg::GetFederationInfo(invite));
                    self.current_peek_id = Some(id);
                    task
                }
                Err(_) => match MintUrl::from_str(&string) {
                    Ok(mint) => {
                        if self
                            .mint_list
                            .iter()
                            .any(|m| m.id.mint_url().is_some_and(|u| u == mint))
                        {
                            return Task::perform(async {}, |_| {
                                Message::AddToast(Toast {
                                    title: "Mint already added".to_string(),
                                    body: None,
                                    status: ToastStatus::Bad,
                                })
                            });
                        }
                        self.peek_status = PeekStatus::Peeking;
                        let (id, task) = self.send_from_ui(UICoreMsg::GetCashuMintInfo(mint));
                        self.current_peek_id = Some(id);
                        task
                    }
                    Err(_) => Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Can't preview mint".to_string(),
                            body: Some("Invalid invite code".to_string()),
                            status: ToastStatus::Bad,
                        })
                    }),
                },
            },
            Message::RemoveMint(mint) => {
                // Check if the federation still exists before trying to remove it
                if !self.mint_list.iter().any(|f| f.id == mint) {
                    return Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Federation already removed".to_string(),
                            body: None,
                            status: ToastStatus::Neutral,
                        })
                    });
                }
                let (_, task) = self.send_from_ui(UICoreMsg::RemoveMint(mint));
                task
            }
            Message::ChangeMint(mint) => {
                self.active_mint = Some(mint);
                self.clear_receive_state();
                self.clear_send_state();
                self.clear_transfer_state();
                Task::none()
            }
            Message::CopyToClipboard(s) => Task::batch([
                clipboard::write(s),
                Task::perform(async {}, |_| {
                    Message::AddToast(Toast {
                        title: "Copied to clipboard".to_string(),
                        body: None,
                        status: ToastStatus::Neutral,
                    })
                }),
            ]),
            Message::ShowSeedWords(show) => {
                if show {
                    let (_, task) = self.send_from_ui(UICoreMsg::GetSeedWords);
                    task
                } else {
                    self.basic_modal = None;
                    Task::none()
                }
            }
            Message::UrlClicked(url) => {
                log::info!("Url clicked: {}", url);
                self.confirm_modal = Some(ConfirmModalState {
                    title: "Open External Link?".to_string(),
                    description: format!("This will open {} in your default browser.", url),
                    confirm_action: Box::new(Message::OpenUrl(url)),
                    cancel_action: Box::new(Message::SetConfirmModal(None)),
                    confirm_button_text: "Open Link".to_string(),
                });
                Task::none()
            }
            Message::OpenUrl(url) => {
                if let Err(e) = opener::open(&url) {
                    log::error!("Failed to open URL: {}", e);
                }
                self.confirm_modal = None;
                Task::none()
            }
            Message::SetOnchainReceiveEnabled(enabled) => {
                let (_, task) = self.send_from_ui(UICoreMsg::SetOnchainReceiveEnabled(enabled));
                self.confirm_modal = None;
                task
            }
            Message::SetTorEnabled(enabled) => {
                // Just send the request to update Tor setting
                let (_, task) = self.send_from_ui(UICoreMsg::SetTorEnabled(enabled));
                task
            }
            Message::SelectTransaction(transaction) => {
                self.selected_transaction = transaction;
                Task::none()
            }
            Message::SetConfirmModal(modal_state) => {
                self.confirm_modal = modal_state;
                Task::none()
            }
            Message::SetBasicModal(modal_state) => {
                self.basic_modal = modal_state;
                Task::none()
            }
            Message::CancelReceiveGeneration => {
                // Cancel any ongoing metadata fetch
                self.receive_status = ReceiveStatus::Idle;
                self.receive_failure_reason = None;
                self.current_receive_id = None;
                Task::none()
            }
            // Handle any messages we get from core
            Message::CoreMessage(msg) => match msg.msg {
                CoreUIMsg::Sending => {
                    if self.current_send_id == msg.id {
                        self.send_status = SendStatus::Sending;
                    }
                    Task::none()
                }
                CoreUIMsg::SendSuccess(params) => {
                    info!("Send success: {params:?}");
                    if self.current_send_id == msg.id {
                        self.send_success_msg = Some(params);
                        self.current_send_id = None;

                        // Navigate to the history screen
                        self.active_route = Route::History;
                        self.clear_send_state();
                    }
                    // Toast success
                    if params != SendSuccessMsg::Transfer {
                        Task::perform(async {}, move |_| {
                            Message::AddToast(Toast {
                                title: "Payment sent".to_string(),
                                body: None,
                                status: ToastStatus::Good,
                            })
                        })
                    } else {
                        Task::none()
                    }
                }
                CoreUIMsg::SendFailure(reason) => {
                    if self.current_send_id == msg.id {
                        self.send_status = SendStatus::Idle;
                        self.current_send_id = None;
                        // We don't clear the send state here because maybe they want to try again
                    }
                    Task::perform(async {}, move |_| {
                        Message::AddToast(Toast {
                            title: "Failed to send".to_string(),
                            body: Some(reason.clone()),
                            status: ToastStatus::Bad,
                        })
                    })
                }
                CoreUIMsg::ReceiveSuccess(params) => {
                    info!("Receive success: {params:?}");
                    if self.current_receive_id == msg.id {
                        self.receive_success_msg = Some(params);
                        self.current_receive_id = None;

                        // Navigate to the history screen
                        self.active_route = Route::History;
                        self.clear_receive_state();
                    } else if self.current_transfer_id == msg.id && msg.id.is_some() {
                        self.current_transfer_id = None;

                        // Navigate to the history screen
                        self.active_route = Route::History;
                        self.clear_transfer_state();
                    }
                    if params != ReceiveSuccessMsg::Transfer {
                        // Toast success
                        Task::perform(async {}, move |_| {
                            Message::AddToast(Toast {
                                title: "Payment received".to_string(),
                                body: None,
                                status: ToastStatus::Good,
                            })
                        })
                    } else {
                        Task::perform(async {}, move |_| {
                            Message::AddToast(Toast {
                                title: "Transfer complete".to_string(),
                                body: None,
                                status: ToastStatus::Good,
                            })
                        })
                    }
                }
                CoreUIMsg::ReceiveFailed(reason) => {
                    if self.current_receive_id == msg.id {
                        self.receive_status = ReceiveStatus::Idle;
                        self.receive_failure_reason = Some(reason.clone());
                        self.current_receive_id = None;
                        self.clear_receive_state();
                    }
                    Task::perform(async {}, move |_| {
                        Message::AddToast(Toast {
                            title: "Failed to receive".to_string(),
                            body: Some(reason.clone()),
                            status: ToastStatus::Bad,
                        })
                    })
                }
                CoreUIMsg::TransferFailure(reason) => {
                    if self.current_transfer_id == msg.id {
                        self.transfer_status = SendStatus::Idle;
                    }
                    error!("Transfer failed: {reason}");
                    Task::perform(async {}, move |_| {
                        Message::AddToast(Toast {
                            title: "Failed to transfer".to_string(),
                            body: Some(reason.clone()),
                            status: ToastStatus::Bad,
                        })
                    })
                }
                CoreUIMsg::TransactionHistoryUpdated(history) => {
                    self.transaction_history = history;
                    Task::none()
                }
                CoreUIMsg::MintBalanceUpdated { id, balance } => {
                    debug!(
                        "Balance update received - ID: {:?}, Balance: {:?}",
                        id, balance
                    );

                    // Update the balance in the federation list
                    if let Some(federation) = self.mint_list.iter_mut().find(|f| f.id == id) {
                        federation.balance = balance.sats_round_down();
                    }

                    Task::none()
                }
                CoreUIMsg::ReceiveGenerating => {
                    self.receive_status = ReceiveStatus::Generating;
                    Task::none()
                }
                CoreUIMsg::ReceiveInvoiceGenerated(invoice) => {
                    self.receive_status = ReceiveStatus::WaitingToReceive;
                    debug!("Received invoice: {invoice}");
                    self.receive_qr_data = Some(
                        Data::with_error_correction(
                            format!("lightning:{invoice}"),
                            iced::widget::qr_code::ErrorCorrection::Low,
                        )
                        .unwrap(),
                    );
                    self.receive_invoice = Some(invoice);
                    Task::none()
                }
                CoreUIMsg::AddMintFailed(reason) => {
                    let reason = reason.clone();
                    self.clear_add_federation_state();
                    Task::perform(async {}, move |_| {
                        Message::AddToast(Toast {
                            title: "Failed to join mint".to_string(),
                            body: Some(reason.clone()),
                            status: ToastStatus::Bad,
                        })
                    })
                }
                CoreUIMsg::RemoveFederationFailed(reason) => {
                    let reason = reason.clone();
                    self.clear_add_federation_state();
                    Task::perform(async {}, move |_| {
                        Message::AddToast(Toast {
                            title: "Failed to remove mint".to_string(),
                            body: Some(reason.clone()),
                            status: ToastStatus::Bad,
                        })
                    })
                }
                CoreUIMsg::FederationListNeedsUpdate => {
                    let (_, task) = self.send_from_ui(UICoreMsg::FederationListNeedsUpdate);
                    task
                }
                CoreUIMsg::MintInfo {
                    id,
                    config,
                    metadata,
                } => {
                    let name = metadata.federation_name.clone().unwrap_or_else(|| {
                        match config.as_ref().map(|c| c.meta::<String>("federation_name")) {
                            Some(Ok(Some(n))) => n,
                            _ => "Unknown".to_string(),
                        }
                    });

                    let (guardians, module_kinds) = match config {
                        None => (vec![], vec![]),
                        Some(config) => {
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

                            (guardians, module_kinds)
                        }
                    };

                    let item = MintItem {
                        id,
                        name,
                        balance: 0,
                        guardians: Some(guardians),
                        module_kinds: Some(module_kinds),
                        metadata,
                        on_chain_supported: false,
                        active: true,
                    };

                    self.peek_federation_item = Some(item);
                    self.peek_status = PeekStatus::Idle;
                    Task::none()
                }
                CoreUIMsg::AddMintSuccess(id) => {
                    self.clear_add_federation_state();
                    // Route to the mints list
                    self.active_route = Route::Mints(routes::MintSubroute::List);
                    self.active_mint = Some(id);
                    Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Mint added".to_string(),
                            body: None,
                            status: ToastStatus::Neutral,
                        })
                    })
                }
                CoreUIMsg::RemoveFederationSuccess => {
                    self.clear_add_federation_state();
                    // Route to the mints list
                    self.active_route = Route::Mints(routes::MintSubroute::List);
                    // We probably got here because of a modal so we should close the modal
                    self.confirm_modal = None;
                    Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Mint removed".to_string(),
                            body: None,
                            status: ToastStatus::Neutral,
                        })
                    })
                }
                CoreUIMsg::MintListUpdated(mut list) => {
                    list.sort();
                    trace!("Updated federation list: {:#?}", list);

                    // if we don't have an active federation, set it to the first one
                    if self.active_mint.is_none() {
                        self.active_mint = list.iter().find(|f| f.active).map(|f| f.id.clone());
                    }

                    // Show the CTA if we have no federations and we haven't navigated to the mints page yet
                    self.show_add_a_mint_cta = list.is_empty() && !self.has_navigated_to_mints;

                    self.mint_list = list;
                    Task::none()
                }
                CoreUIMsg::ReceiveAddressGenerated(address) => {
                    self.receive_status = ReceiveStatus::WaitingToReceive;
                    debug!("Received address: {address}");
                    self.receive_qr_data = Some(
                        Data::with_error_correction(
                            format!("bitcoin:{address}"),
                            iced::widget::qr_code::ErrorCorrection::Low,
                        )
                        .unwrap(),
                    );
                    self.receive_address = Some(address);
                    Task::none()
                }
                CoreUIMsg::NeedsInit => {
                    info!("Got init message");
                    self.init_status = WelcomeStatus::NeedsInit;
                    focus_input_id("password_init_input")
                }
                CoreUIMsg::Initing => {
                    self.init_status = WelcomeStatus::Initing;
                    Task::none()
                }
                CoreUIMsg::InitSuccess => {
                    self.init_status = WelcomeStatus::Inited;
                    self.active_route = Route::Home;
                    Task::none()
                }
                CoreUIMsg::InitFailed(reason) => {
                    self.init_status = WelcomeStatus::NeedsInit;
                    self.init_failure_reason = Some(reason);
                    Task::none()
                }
                CoreUIMsg::Locked => {
                    info!("Got locked message");
                    self.active_route = Route::Unlock;
                    focus_input_id("password_unlock_input")
                }
                CoreUIMsg::Unlocking => {
                    info!("Got unlocking message");
                    self.unlock_status = UnlockStatus::Unlocking;
                    Task::none()
                }
                CoreUIMsg::UnlockSuccess => {
                    self.unlock_status = UnlockStatus::Unlocked;
                    self.active_route = Route::Home;
                    Task::none()
                }
                CoreUIMsg::UnlockFailed(reason) => {
                    self.unlock_status = UnlockStatus::Locked;
                    self.unlock_failure_reason = Some(reason);
                    Task::none()
                }
                CoreUIMsg::SeedWords(words) => {
                    self.seed_words = Some(words.clone());

                    // Create a BasicModalState to display the seed words
                    self.basic_modal = Some(BasicModalState {
                        title: "Your Seed Words".to_string(),
                        description:
                            "Keep these 12 words safe. They are the only way to recover your funds."
                                .to_string(),
                        close_action: Box::new(Message::ShowSeedWords(false)),
                        content_renderer: Some(routes::settings::render_seed_words),
                        content_data: Some(words),
                    });

                    Task::none()
                }
                CoreUIMsg::OnchainReceiveEnabled(enabled) => {
                    self.onchain_receive_enabled = enabled;
                    Task::perform(async {}, |_| Message::Noop)
                }
                CoreUIMsg::TorEnabled(enabled) => {
                    self.tor_enabled = enabled;

                    // After getting confirmation of the Tor setting change, restart the app
                    Task::perform(async {}, move |_| {
                        lock::restart_app();
                        Message::Noop
                    })
                }
                CoreUIMsg::InitialProfile {
                    seed_words,
                    onchain_receive_enabled,
                    tor_enabled,
                } => {
                    self.seed_words = Some(seed_words);
                    self.onchain_receive_enabled = onchain_receive_enabled;
                    self.tor_enabled = tor_enabled;
                    Task::none()
                }
                CoreUIMsg::StatusUpdate {
                    message,
                    operation_id,
                } => {
                    if let Some(id) = operation_id {
                        self.operation_status.insert(
                            id,
                            OperationStatus {
                                message: message.clone(),
                            },
                        );
                    }
                    Task::none()
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
            Route::Restore => crate::routes::restore(self),
        };

        // First wrap the content in the confirm modal
        let content_with_confirm =
            crate::components::confirm_modal(active_route, self.confirm_modal.as_ref());

        // Then wrap it in the basic modal
        let content_with_modals =
            crate::components::basic_modal(content_with_confirm, self.basic_modal.as_ref());

        ToastManager::new(content_with_modals, &self.toasts, Message::CloseToast).into()
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::custom(
            String::from("Custom"),
            iced::theme::Palette {
                background: Color::from_rgb8(23, 23, 25),
                primary: MUTINY_RED,
                text: Color::WHITE,
                success: MUTINY_GREEN,
                danger: MUTINY_RED,
                // TODO: do we need a warning yellow?
                warning: Color::from_rgb8(255, 165, 0),
            },
        )
    }
}
