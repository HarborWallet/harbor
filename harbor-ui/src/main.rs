use crate::bridge::run_core;
use crate::components::focus_input_id;
use bitcoin::Address;
use components::{Toast, ToastManager, ToastStatus};
use fedimint_core::config::FederationId;
use fedimint_core::core::ModuleKind;
use fedimint_core::invite_code::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use harbor_client::db_models::transaction_item::TransactionItem;
use harbor_client::db_models::FederationItem;
use harbor_client::{CoreUIMsg, CoreUIMsgPacket, ReceiveSuccessMsg, SendSuccessMsg, UICoreMsg};
use iced::widget::qr_code::Data;
use iced::widget::row;
use iced::Element;
use iced::Font;
use iced::Subscription;
use iced::Task;
use iced::{clipboard, Color};
use log::{error, info};
use routes::Route;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub mod bridge;
pub mod components;
pub mod routes;

// This starts the program. Importantly, it registers the update and view methods, along with a subscription.
// We can also run logic during load if we need to.
pub fn main() -> iced::Result {
    pretty_env_logger::init();
    iced::application("Harbor", HarborWallet::update, HarborWallet::view)
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
    SetTransferFrom(String),
    SetTransferTo(String),
    TransferAmountInputChanged(String),
    UrlClicked(String),
    // Async commands we fire from the UI to core
    Noop,
    Send(String),
    GenerateInvoice,
    GenerateAddress,
    Unlock(String),
    Init(String), // TODO add seed option
    AddFederation(String),
    PeekFederation(String),
    RemoveFederation(FederationId),
    ChangeFederation(FederationId),
    Donate,
    SetOnchainReceiveEnabled(bool),
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
    federation_balances: HashMap<FederationId, Amount>,
    transaction_history: Vec<TransactionItem>,
    federation_list: Vec<FederationItem>,
    active_federation: Option<FederationItem>,
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
    peek_federation_item: Option<FederationItem>,
    mint_invite_code_str: String,
    // Transfer
    transfer_from_federation_selection: Option<String>,
    transfer_to_federation_selection: Option<String>,
    transfer_amount_input_str: String,
    // Donate
    donate_amount_str: String,
    // Settings
    settings_show_seed_words: bool,
    seed_words: Option<String>,
    current_send_id: Option<Uuid>,
    current_receive_id: Option<Uuid>,
    peek_status: PeekStatus,
    add_federation_status: AddFederationStatus,
    // Onboarding
    show_add_a_mint_cta: bool,
    has_navigated_to_mints: bool,
    onchain_receive_enabled: bool,
}

impl HarborWallet {
    fn balance_sats(&self) -> u64 {
        let mut amount = Amount::ZERO;
        for balance in self.federation_balances.values() {
            amount += *balance;
        }
        amount.sats_round_down()
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
        if let Some(ui_handle) = ui_handle {
            f(ui_handle).await;
        } else {
            panic!("UI handle is None");
        }
    }

    fn clear_add_federation_state(&mut self) {
        self.peek_federation_item = None;
        self.mint_invite_code_str = String::new();
        self.peek_status = PeekStatus::Idle;
        self.add_federation_status = AddFederationStatus::Idle;
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

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Setup
            Message::UIHandlerLoaded(ui_handle) => {
                self.ui_handle = Some(ui_handle);
                println!("Core loaded");
                Task::none()
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
                    _ => match route {
                        Route::Mints(_) => {
                            // Hide the add a mint cta when navigating to mints
                            self.show_add_a_mint_cta = false;
                            self.has_navigated_to_mints = true;
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
                self.send_failure_reason = None;
                self.send_success_msg = None;
                self.send_dest_input_str = String::new();
                self.send_amount_input_str = String::new();
                self.is_max = false;
                self.send_status = SendStatus::Idle;
                Task::none()
            }
            Message::ReceiveStateReset => {
                self.receive_failure_reason = None;
                self.receive_amount_str = String::new();
                self.receive_invoice = None;
                self.receive_success_msg = None;
                self.receive_address = None;
                self.receive_qr_data = None;
                self.receive_status = ReceiveStatus::Idle;
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
                self.toasts.remove(index);
                Task::none()
            }
            Message::CancelAddFederation => {
                self.clear_add_federation_state();
                self.active_route = Route::Mints(routes::MintSubroute::List);

                Task::none()
            }
            Message::SetTransferFrom(s) => {
                self.transfer_from_federation_selection = Some(s);
                Task::none()
            }
            Message::SetTransferTo(s) => {
                self.transfer_to_federation_selection = Some(s);
                Task::none()
            }
            Message::TransferAmountInputChanged(input) => {
                self.transfer_amount_input_str = input;
                Task::none()
            }
            // Async commands we fire from the UI to core
            Message::Noop => Task::none(),
            Message::Send(invoice_str) => match self.send_status {
                SendStatus::Sending => Task::none(),
                _ => {
                    self.send_failure_reason = None;
                    let federation_id = match self.active_federation.as_ref() {
                        Some(f) => f.id,
                        None => {
                            // todo show error
                            error!("No active federation");
                            return Task::none();
                        }
                    };

                    if let Ok(invoice) = Bolt11Invoice::from_str(&invoice_str) {
                        let (id, task) = self.send_from_ui(UICoreMsg::SendLightning {
                            federation_id,
                            invoice,
                        });
                        self.current_send_id = Some(id);
                        task
                    } else if let Ok(address) = Address::from_str(&invoice_str) {
                        let amount = if self.is_max {
                            None
                        } else {
                            match self.send_amount_input_str.parse::<u64>() {
                                Ok(amount) => Some(amount),
                                Err(e) => {
                                    eprintln!("Error parsing amount: {e}");
                                    self.send_failure_reason = Some(e.to_string());
                                    return Task::none();
                                }
                            }
                        };
                        let (id, task) = self.send_from_ui(UICoreMsg::SendOnChain {
                            federation_id,
                            address,
                            amount_sats: amount,
                        });
                        self.current_send_id = Some(id);
                        task
                    } else {
                        error!("Invalid invoice or address");
                        self.current_send_id = None;
                        Task::none()
                    }
                }
            },
            Message::GenerateInvoice => match self.receive_status {
                ReceiveStatus::Generating => Task::none(),
                _ => {
                    let federation_id = match self.active_federation.as_ref() {
                        Some(f) => f.id,
                        None => {
                            // todo show error
                            error!("No active federation");
                            return Task::none();
                        }
                    };
                    match self.receive_amount_str.parse::<u64>() {
                        Ok(amount) => {
                            let (id, task) = self.send_from_ui(UICoreMsg::ReceiveLightning {
                                federation_id,
                                amount: Amount::from_sats(amount),
                            });
                            self.current_receive_id = Some(id);
                            self.receive_failure_reason = None;
                            task
                        }
                        Err(e) => {
                            self.receive_amount_str = String::new();
                            eprintln!("Error parsing amount: {e}");
                            Task::none()
                        }
                    }
                }
            },
            Message::GenerateAddress => match self.receive_status {
                ReceiveStatus::Generating => Task::none(),
                _ => {
                    let federation_id = match self.active_federation.as_ref() {
                        Some(f) => f.id,
                        None => {
                            // todo show error
                            error!("No active federation");
                            return Task::none();
                        }
                    };
                    let (id, task) = self.send_from_ui(UICoreMsg::ReceiveOnChain { federation_id });
                    self.current_receive_id = Some(id);
                    self.receive_failure_reason = None;
                    task
                }
            },
            Message::Donate => match self.donate_amount_str.parse::<u64>() {
                Ok(amount) => {
                    let federation_id = match self.active_federation.as_ref() {
                        Some(f) => f.id,
                        None => {
                            // todo show error
                            error!("No active federation");
                            return Task::none();
                        }
                    };

                    // TODO: don't hardcode this!
                    let hardcoded_donation_address = "tb1qd28npep0s8frcm3y7dxqajkcy2m40eysplyr9v";
                    let address = Address::from_str(hardcoded_donation_address).unwrap();
                    let (id, task) = self.send_from_ui(UICoreMsg::SendOnChain {
                        federation_id,
                        address,
                        amount_sats: Some(amount),
                    });
                    self.current_send_id = Some(id);
                    task
                }
                Err(e) => {
                    self.receive_amount_str = String::new();
                    eprintln!("Error parsing amount: {e}");
                    Task::none()
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
            Message::Init(password) => match self.unlock_status {
                UnlockStatus::Unlocking => Task::none(),
                _ => {
                    self.unlock_failure_reason = None;
                    let (_, task) = self.send_from_ui(UICoreMsg::Init {
                        password,
                        seed: None, // FIXME: Use this
                    });
                    task
                }
            },
            Message::AddFederation(invite_code) => {
                let invite = InviteCode::from_str(&invite_code);
                if let Ok(invite) = invite {
                    self.add_federation_status = AddFederationStatus::Adding;
                    let (_, task) = self.send_from_ui(UICoreMsg::AddFederation(invite));
                    task
                } else {
                    Task::perform(async {}, move |_| {
                        Message::AddToast(Toast {
                            title: "Failed to join mint".to_string(),
                            body: "Invalid invite code".to_string(),
                            status: ToastStatus::Bad,
                        })
                    })
                }
            }
            Message::PeekFederation(invite_code) => {
                let invite = InviteCode::from_str(&invite_code);
                if let Ok(invite) = invite {
                    self.peek_status = PeekStatus::Peeking;
                    let (_, task) = self.send_from_ui(UICoreMsg::GetFederationInfo(invite));
                    task
                } else {
                    Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Failed to preview mint".to_string(),
                            body: "Invalid invite code".to_string(),
                            status: ToastStatus::Bad,
                        })
                    })
                }
            }
            Message::RemoveFederation(federation_id) => {
                let (_, task) = self.send_from_ui(UICoreMsg::RemoveFederation(federation_id));
                task
            }
            Message::ChangeFederation(id) => {
                let federation = self
                    .federation_list
                    .iter()
                    .find(|f| f.id == id)
                    .expect("federation not found");
                self.active_federation = Some(federation.clone());
                Task::none()
            }
            Message::CopyToClipboard(s) => Task::batch([
                clipboard::write(s),
                Task::perform(async {}, |_| {
                    Message::AddToast(Toast {
                        title: "Copied to clipboard".to_string(),
                        body: "...".to_string(),
                        status: ToastStatus::Neutral,
                    })
                }),
            ]),
            Message::ShowSeedWords(show) => {
                if show {
                    let (_, task) = self.send_from_ui(UICoreMsg::GetSeedWords);
                    task
                } else {
                    self.settings_show_seed_words = false;
                    Task::none()
                }
            }
            // TODO: we might want an intermediate modal
            // To warn people that this will open their browser
            Message::UrlClicked(url) => {
                log::info!("Url clicked: {}", url);
                if let Err(e) = webbrowser::open(&url) {
                    log::error!("Failed to open URL: {}", e);
                }
                Task::none()
            }
            Message::SetOnchainReceiveEnabled(enabled) => {
                let (_, task) = self.send_from_ui(UICoreMsg::SetOnchainReceiveEnabled(enabled));
                task
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
                    }
                    Task::none()
                }
                CoreUIMsg::SendFailure(reason) => {
                    if self.current_send_id == msg.id {
                        self.send_status = SendStatus::Idle;
                        self.send_failure_reason = Some(reason);
                        self.current_send_id = None;
                    }
                    Task::none()
                }
                CoreUIMsg::ReceiveSuccess(params) => {
                    info!("Receive success: {params:?}");
                    if self.current_receive_id == msg.id {
                        self.receive_success_msg = Some(params);
                        self.current_receive_id = None;
                    }
                    Task::none()
                }
                CoreUIMsg::ReceiveFailed(reason) => {
                    if self.current_receive_id == msg.id {
                        self.receive_status = ReceiveStatus::Idle;
                        self.receive_failure_reason = Some(reason);
                        self.current_receive_id = None;
                    }
                    Task::none()
                }
                CoreUIMsg::TransactionHistoryUpdated(history) => {
                    self.transaction_history = history;
                    Task::none()
                }
                CoreUIMsg::FederationBalanceUpdated { id, balance } => {
                    self.federation_balances.insert(id, balance);
                    Task::none()
                }
                CoreUIMsg::ReceiveGenerating => {
                    self.receive_status = ReceiveStatus::Generating;
                    Task::none()
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
                    Task::none()
                }
                CoreUIMsg::AddFederationFailed(reason) => {
                    let reason = reason.clone();
                    self.clear_add_federation_state();
                    Task::perform(async {}, move |_| {
                        Message::AddToast(Toast {
                            title: "Failed to join mint".to_string(),
                            body: reason.clone(),
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
                            body: reason.clone(),
                            status: ToastStatus::Bad,
                        })
                    })
                }
                CoreUIMsg::FederationInfo(config) => {
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

                    let item = FederationItem {
                        id,
                        name,
                        balance: 0,
                        guardians: Some(guardians),
                        module_kinds: Some(module_kinds),
                    };

                    self.peek_federation_item = Some(item);
                    self.peek_status = PeekStatus::Idle;
                    Task::none()
                }
                CoreUIMsg::AddFederationSuccess => {
                    self.clear_add_federation_state();
                    // Route to the mints list
                    self.active_route = Route::Mints(routes::MintSubroute::List);
                    Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Mint added".to_string(),
                            // TODO: maybe we should make body optional
                            body: "...".to_string(),
                            status: ToastStatus::Neutral,
                        })
                    })
                }
                CoreUIMsg::RemoveFederationSuccess => {
                    self.clear_add_federation_state();
                    // Route to the mints list
                    self.active_route = Route::Mints(routes::MintSubroute::List);
                    Task::perform(async {}, |_| {
                        Message::AddToast(Toast {
                            title: "Mint removed".to_string(),
                            // TODO: maybe we should make body optional
                            body: "...".to_string(),
                            status: ToastStatus::Neutral,
                        })
                    })
                }
                CoreUIMsg::FederationListUpdated(list) => {
                    // if we don't have an active federation, set it to the first one
                    if self.active_federation.is_none() {
                        self.active_federation = list.first().cloned();
                    }

                    // Show the CTA if we have no federations and we haven't navigated to the mints page yet
                    self.show_add_a_mint_cta = list.is_empty() && !self.has_navigated_to_mints;

                    self.federation_list = list;
                    Task::none()
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
                    self.seed_words = Some(words);
                    self.settings_show_seed_words = true;
                    Task::none()
                }
                CoreUIMsg::OnchainReceiveEnabled(enabled) => {
                    self.onchain_receive_enabled = enabled;
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
