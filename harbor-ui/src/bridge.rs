use crate::Message;
use crate::config::read_config;
use crate::keyring::{save_to_keyring, try_get_keyring_password};
use harbor_client::bitcoin::Network;
use harbor_client::cashu_client::TorMintConnector;
use harbor_client::cdk::mint_url::MintUrl;
use harbor_client::cdk::nuts::CurrencyUnit;
use harbor_client::cdk::wallet::WalletBuilder;
use harbor_client::cdk_redb::WalletRedbDatabase;
use harbor_client::db::{DBConnection, check_password, setup_db};
use harbor_client::fedimint_client::{FederationInviteOrId, FedimintClient};
use harbor_client::fedimint_core::config::FederationId;
use harbor_client::metadata::FederationMeta;
use harbor_client::{
    CoreUIMsg, CoreUIMsgPacket, HarborCore, MintIdentifier, UICoreMsg, UICoreMsgPacket, data_dir,
};
use iced::futures::channel::mpsc::Sender;
use iced::futures::{SinkExt, Stream, StreamExt};
use log::{LevelFilter, error, info, warn};
use simplelog::WriteLogger;
use simplelog::{CombinedLogger, TermLogger, TerminalMode};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::task::spawn_blocking;
use uuid::Uuid;

pub const HARBOR_FILE_NAME: &str = "harbor.sqlite";
pub const LOG_FILE_NAME: &str = "harbor.log";

#[derive(Debug)]
pub struct UIHandle {
    ui_to_core_tx: mpsc::Sender<UICoreMsgPacket>,
}

#[derive(Debug, Clone)]
pub enum BridgeError {
    SendFailed,
    Unknown,
}

impl UIHandle {
    pub async fn send_msg(&self, id: Uuid, msg: UICoreMsg) {
        self.ui_to_core_tx
            .send(UICoreMsgPacket { msg, id })
            .await
            .unwrap();
    }
}

#[derive(Debug)]
pub struct CoreHandle {
    core_from_ui_rx: mpsc::Receiver<UICoreMsgPacket>,
}

impl CoreHandle {
    pub async fn recv(&mut self) -> Option<UICoreMsgPacket> {
        self.core_from_ui_rx.recv().await
    }
}

pub fn create_handles() -> (UIHandle, CoreHandle) {
    let (ui_to_core_tx, core_from_ui_rx) = mpsc::channel::<UICoreMsgPacket>(50);

    let ui_handle = UIHandle { ui_to_core_tx };

    let core_handle = CoreHandle { core_from_ui_rx };

    (ui_handle, core_handle)
}

/// Common setup function for creating a HarborCore instance
async fn setup_harbor_core(
    data_dir: PathBuf,
    db_path: &str,
    password: &str,
    network: Network,
    tx: &mut Sender<Message>,
) -> Option<HarborCore> {
    // Setup database
    let db_path = db_path.to_string();
    let password = password.to_string();
    let db = spawn_blocking(move || setup_db(&db_path, password))
        .await
        .expect("Could not create join handle")
        .ok()?;

    // Retrieve mnemonic
    let profile = db
        .get_profile()
        .ok()
        .flatten()
        .expect("Could not get profile from db");
    let mnemonic = profile.mnemonic();

    // Create stop signal
    let stop = Arc::new(AtomicBool::new(false));

    // Setup federation clients
    let federation_ids = db
        .list_federations()
        .expect("should load initial fedimints");
    let mut clients = HashMap::with_capacity(federation_ids.len());
    for f in federation_ids {
        let client = FedimintClient::new(
            db.clone(),
            FederationInviteOrId::Id(
                FederationId::from_str(&f).expect("should parse federation id"),
            ),
            &mnemonic,
            network,
            stop.clone(),
        )
        .await
        .expect("Could not create fedimint client");

        clients.insert(client.federation_id(), client);
    }

    let cashu_db_path = data_dir.join("cashu.redb");
    if !cashu_db_path.exists() {
        File::create_new(&cashu_db_path).expect("could not create cashu db");
    }
    let cashu_db = Arc::new(
        WalletRedbDatabase::new(&cashu_db_path).expect("Could not create cashu WalletRedbDatabase"),
    );

    // Setup cashu clients
    let mint_urls = db
        .list_cashu_mints()
        .expect("should load initial fedimints");
    let mut cashu_clients = HashMap::with_capacity(mint_urls.len());
    for url in mint_urls {
        let seed = mnemonic.to_seed_normalized("");

        let mint_url = MintUrl::from_str(&url).expect("Could not create MintUrl");

        let builder = WalletBuilder::new()
            .mint_url(mint_url.clone())
            .unit(CurrencyUnit::Sat)
            .localstore(cashu_db.clone())
            .seed(&seed);

        let builder = if profile.tor_enabled() {
            builder.client(TorMintConnector::new(
                mint_url,
                Arc::new(AtomicBool::new(false)),
            ))
        } else {
            builder
        };

        let wallet = builder.build().expect("Could not create cashu client");

        cashu_clients.insert(wallet.mint_url.clone(), wallet);
    }

    // Setup core message channel
    let (core_tx, mut core_rx) = iced::futures::channel::mpsc::channel::<CoreUIMsgPacket>(128);
    let mut tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            let next_result = core_rx.next().await;
            match next_result {
                Some(rev) => {
                    let send_result = tx_clone.send(Message::CoreMessage(rev)).await;
                    send_result.expect("should send");
                }
                None => break,
            }
        }
    });

    // Create and return HarborCore
    Some(
        HarborCore::new(
            network,
            mnemonic,
            data_dir,
            core_tx,
            Arc::new(RwLock::new(clients)),
            Arc::new(RwLock::new(cashu_clients)),
            db,
            cashu_db,
            stop.clone(),
            Arc::new(AtomicBool::new(profile.tor_enabled())),
        )
        .await
        .expect("Failed to build harbor core"),
    )
}

/// Attempts to auto-unlock the wallet using a password from the environment or keyring.
/// Returns Some(HarborCore) if successful, None if unsuccessful or no password found.
async fn try_auto_unlock(
    path: &Path,
    network: Network,
    tx: &mut Sender<Message>,
) -> Option<HarborCore> {
    let db_path = path.join(HARBOR_FILE_NAME);
    let db_path_str = db_path.to_str().unwrap().to_string();

    // First try to get password from keyring
    match try_get_keyring_password().await {
        Some(password) => {
            log::info!("Found password in keyring, attempting auto-unlock");

            if check_password(&db_path_str, &password).is_ok() {
                log::info!("Successfully unlocked wallet with keyring password");
                let core =
                    setup_harbor_core(path.to_path_buf(), &db_path_str, &password, network, tx)
                        .await?;
                tx.send(Message::core_msg(None, CoreUIMsg::UnlockSuccess))
                    .await
                    .expect("should send");
                return Some(core);
            } else {
                log::warn!("Password from keyring is invalid");
            }
        }
        _ => {
            log::info!("No password found in keyring or keyring not available");
        }
    }

    // Fall back to environment variable if keyring fails
    if let Ok(password) = std::env::var("WALLET_PASSWORD") {
        log::info!("Found password in environment, attempting auto-unlock");

        if check_password(&db_path_str, &password).is_ok() {
            log::info!("Successfully unlocked wallet with environment password");
            let core =
                setup_harbor_core(path.to_path_buf(), &db_path_str, &password, network, tx).await?;
            tx.send(Message::core_msg(None, CoreUIMsg::UnlockSuccess))
                .await
                .expect("should send");
            return Some(core);
        } else {
            log::warn!("Password from environment is invalid");
        }
    } else {
        log::info!("No password found in environment");
    }

    // If we get here, neither keyring nor environment password worked
    log::info!("Auto-unlock failed, falling back to manual unlock");
    None
}

pub fn run_core() -> impl Stream<Item = Message> {
    iced::stream::channel(100, |mut tx: Sender<Message>| async move {
        let config = match read_config() {
            Ok(config) => config,
            Err(e) => {
                // In case the config file is malformed we can tell the UI about it
                tx.send(Message::InitError(e.to_string()))
                    .await
                    .expect("should send");
                tx.send(Message::core_msg(None, CoreUIMsg::NeedsInit))
                    .await
                    .expect("should send");
                return;
            }
        };
        let network = config.network;

        // Create the network-specific datadir if it doesn't exist
        let path = PathBuf::from(&data_dir(Some(network)));
        std::fs::create_dir_all(&path).expect("Could not create datadir");
        log::info!("Using datadir: {path:?}");

        let log_file_path = path.join(LOG_FILE_NAME);
        let log_file = if log_file_path.exists() {
            File::open(log_file_path).expect("Could not open log file")
        } else {
            File::create(log_file_path).expect("Could not create log file")
        };

        let log_config = simplelog::ConfigBuilder::new()
            // ignore spammy UI logs
            .add_filter_ignore_str("wgpu_hal")
            .add_filter_ignore_str("wgpu_core")
            .add_filter_ignore_str("iced")
            .add_filter_ignore_str("naga")
            .add_filter_ignore_str("cosmic_text")
            .add_filter_ignore_str("rustls")
            // spammy trace logs
            .add_filter_ignore_str("calloop")
            .add_filter_ignore_str("soketto")
            .build();
        CombinedLogger::init(vec![
            TermLogger::new(
                LevelFilter::Info,
                log_config.clone(),
                TerminalMode::Mixed,
                simplelog::ColorChoice::Auto,
            ),
            WriteLogger::new(LevelFilter::Debug, log_config, log_file),
        ])
        .expect("Could not initialize logger");

        // Setup UI Handle
        let (ui_handle, mut core_handle) = create_handles();
        let arc_ui_handle = Arc::new(ui_handle);
        tx.send(Message::UIHandlerLoaded(arc_ui_handle))
            .await
            .expect("should send");

        tx.send(Message::ConfigLoaded(config))
            .await
            .expect("should send");

        // FIXME: Artificial sleep because it loads too fast
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Check if the database file exists already, if so tell UI to unlock
        if std::fs::metadata(path.join(HARBOR_FILE_NAME)).is_ok() {
            // Try auto-unlock first, fall back to manual unlock if it fails
            if let Some(core) = try_auto_unlock(&path, network, &mut tx).await {
                let mut core_handle = core_handle;
                tokio::spawn(async move {
                    process_core(&mut core_handle, &core).await;
                });
                return;
            }
            tx.send(Message::core_msg(None, CoreUIMsg::Locked))
                .await
                .expect("should send");
        } else {
            tx.send(Message::core_msg(None, CoreUIMsg::NeedsInit))
                .await
                .expect("should send");
        }

        loop {
            let msg = core_handle.recv().await;

            let id = msg.as_ref().map(|m| m.id);

            // Watch for either Unlock or Init, ignore everything else until started
            match msg.map(|m| m.msg) {
                Some(UICoreMsg::Unlock(password)) => {
                    log::info!("Sending unlock message");
                    tx.send(Message::core_msg(id, CoreUIMsg::Unlocking))
                        .await
                        .expect("should send");

                    let db_path = path.join(HARBOR_FILE_NAME);
                    let db_path = db_path.to_str().unwrap().to_string();

                    // if the db file doesn't exist, error out to go through init flow
                    if !std::path::Path::new(&db_path).exists() {
                        error!("Database does not exist, new wallet is required");

                        tx.send(Message::core_msg(
                            id,
                            CoreUIMsg::UnlockFailed(
                                "Database does not exist, new wallet is required".to_string(),
                            ),
                        ))
                        .await
                        .expect("should send");

                        continue;
                    }

                    if let Err(e) = check_password(&db_path, &password) {
                        // probably invalid password
                        error!("error using password: {e}");

                        tx.send(Message::core_msg(
                            id,
                            CoreUIMsg::UnlockFailed(e.to_string()),
                        ))
                        .await
                        .expect("should send");

                        continue;
                    }

                    // Save password to keyring when successfully unlocked
                    save_to_keyring(&password).await;

                    match setup_harbor_core(
                        path.to_path_buf(),
                        &db_path,
                        &password,
                        network,
                        &mut tx,
                    )
                    .await
                    {
                        Some(core) => {
                            tx.send(Message::core_msg(id, CoreUIMsg::UnlockSuccess))
                                .await
                                .expect("should send");
                            process_core(&mut core_handle, &core).await;
                        }
                        _ => {
                            tx.send(Message::core_msg(
                                id,
                                CoreUIMsg::UnlockFailed("Failed to setup wallet".to_string()),
                            ))
                            .await
                            .expect("should send");
                        }
                    }
                }
                Some(UICoreMsg::Init { password, seed }) => {
                    log::info!("Sending init message");
                    tx.send(Message::core_msg(id, CoreUIMsg::Initing))
                        .await
                        .expect("should send");

                    // Save password to keyring during initial setup
                    save_to_keyring(&password).await;

                    // set up the DB with the provided password
                    let db_path = path.join(HARBOR_FILE_NAME);
                    let db = spawn_blocking(move || setup_db(db_path.to_str().unwrap(), password))
                        .await
                        .expect("Could not create join handle");

                    if let Err(e) = db {
                        error!("error creating DB: {e}");

                        tx.send(Message::core_msg(id, CoreUIMsg::InitFailed(e.to_string())))
                            .await
                            .expect("should send");

                        continue;
                    }
                    let db = db.expect("no error");

                    let cashu_db_path = path.join("cashu.redb");
                    if !cashu_db_path.exists() {
                        File::create_new(&cashu_db_path).expect("could not create cashu db");
                    }
                    let cashu_db = Arc::new(
                        WalletRedbDatabase::new(&cashu_db_path)
                            .expect("Could not create cashu WalletRedbDatabase"),
                    );

                    let (core_tx, mut core_rx) =
                        iced::futures::channel::mpsc::channel::<CoreUIMsgPacket>(128);

                    let mut tx_clone = tx.clone();
                    tokio::spawn(async move {
                        loop {
                            let next_result = core_rx.next().await;
                            match next_result {
                                Some(rev) => {
                                    let send_result =
                                        tx_clone.send(Message::CoreMessage(rev)).await;
                                    send_result.expect("should send");
                                }
                                None => break,
                            }
                        }
                    });

                    let core = HarborCore::new(
                        network,
                        db.generate_mnemonic(seed).expect("should generate words"),
                        path.to_path_buf(),
                        core_tx,
                        Arc::new(RwLock::new(HashMap::new())),
                        Arc::new(RwLock::new(HashMap::new())),
                        db.clone(),
                        cashu_db,
                        Arc::new(AtomicBool::new(false)), // stop
                        Arc::new(AtomicBool::new(true)),  // tor enabled
                    )
                    .await
                    .expect("Failed to build harbor core");

                    tx.send(Message::core_msg(id, CoreUIMsg::InitSuccess))
                        .await
                        .expect("should send");

                    process_core(&mut core_handle, &core).await;
                }

                _ => {
                    warn!("Ignoring unrelated message to locked core")
                }
            }
        }
    })
}

async fn process_core(core_handle: &mut CoreHandle, core: &HarborCore) {
    // Initialize the ui's state
    core.init_ui_state().await.expect("Could not init ui state");

    loop {
        let msg = core_handle.recv().await;

        let core = core.clone();
        tokio::spawn(async move {
            if let Some(msg) = msg {
                match msg.msg {
                    UICoreMsg::SendLightning { mint, invoice } => {
                        log::info!("Got UICoreMsg::Send");
                        core.msg(msg.id, CoreUIMsg::Sending).await;
                        if let Err(e) = core.send_lightning(msg.id, mint, invoice, false).await {
                            error!("Error sending: {e}");
                            core.msg(msg.id, CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveLightning { mint, amount } => {
                        core.msg(msg.id, CoreUIMsg::ReceiveGenerating).await;
                        match core.receive_lightning(msg.id, mint, amount, false).await {
                            Err(e) => {
                                core.msg(msg.id, CoreUIMsg::ReceiveFailed(e.to_string()))
                                    .await;
                            }
                            Ok(invoice) => {
                                core.msg(msg.id, CoreUIMsg::ReceiveInvoiceGenerated(invoice))
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::SendLnurlPay {
                        mint,
                        lnurl,
                        amount_sats,
                    } => {
                        log::info!("Got UICoreMsg::SendLnurlPay");
                        core.msg(msg.id, CoreUIMsg::Sending).await;
                        if let Err(e) = core.send_lnurl_pay(msg.id, mint, lnurl, amount_sats).await
                        {
                            error!("Error sending: {e}");
                            core.msg(msg.id, CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::SendOnChain {
                        mint,
                        address,
                        amount_sats,
                    } => {
                        log::info!("Got UICoreMsg::SendOnChain");
                        core.msg(msg.id, CoreUIMsg::Sending).await;
                        let federation_id = match mint {
                            MintIdentifier::Cashu(_) => panic!("should not receive cashu"), // todo
                            MintIdentifier::Fedimint(mint) => mint,
                        };
                        if let Err(e) = core
                            .send_onchain(msg.id, federation_id, address, amount_sats)
                            .await
                        {
                            error!("Error sending: {e}");
                            core.msg(msg.id, CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveOnChain { mint } => {
                        core.msg(msg.id, CoreUIMsg::ReceiveGenerating).await;
                        let federation_id = match mint {
                            MintIdentifier::Cashu(_) => panic!("should not receive cashu"), // todo
                            MintIdentifier::Fedimint(mint) => mint,
                        };

                        match core.receive_onchain(msg.id, federation_id).await {
                            Err(e) => {
                                core.msg(msg.id, CoreUIMsg::ReceiveFailed(e.to_string()))
                                    .await;
                            }
                            Ok(address) => {
                                core.msg(msg.id, CoreUIMsg::ReceiveAddressGenerated(address))
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::Transfer { to, from, amount } => {
                        if let Err(e) = core.transfer(msg.id, to, from, amount).await {
                            error!("Error transferring: {e}");
                            core.msg(msg.id, CoreUIMsg::TransferFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::GetFederationInfo(invite_code) => {
                        match core.get_federation_info(msg.id, invite_code).await {
                            Err(e) => {
                                error!("Error getting federation info: {e}");
                                core.msg(msg.id, CoreUIMsg::AddMintFailed(e.to_string()))
                                    .await;
                            }
                            Ok((config, metadata)) => {
                                core.msg(
                                    msg.id,
                                    CoreUIMsg::MintInfo {
                                        id: MintIdentifier::Fedimint(
                                            config.calculate_federation_id(),
                                        ),
                                        config: Some(config),
                                        metadata,
                                    },
                                )
                                .await;
                            }
                        }
                    }
                    UICoreMsg::GetCashuMintInfo(mint_url) => {
                        match core.get_cashu_mint_info(msg.id, mint_url.clone()).await {
                            Err(e) => {
                                error!("Error getting cashu mint info: {e}");
                                core.msg(msg.id, CoreUIMsg::AddMintFailed(e.to_string()))
                                    .await;
                            }
                            Ok(info) => {
                                let metadata = FederationMeta {
                                    federation_name: info
                                        .as_ref()
                                        .and_then(|i| i.name.clone())
                                        .or(Some(mint_url.to_string())),
                                    federation_expiry_timestamp: None,
                                    welcome_message: None,
                                    vetted_gateways: None,
                                    federation_icon_url: info
                                        .as_ref()
                                        .and_then(|i| i.icon_url.clone()),
                                    meta_external_url: None,
                                    preview_message: info.and_then(|i| i.description),
                                    popup_end_timestamp: None,
                                    popup_countdown_message: None,
                                };
                                core.msg(
                                    msg.id,
                                    CoreUIMsg::MintInfo {
                                        id: MintIdentifier::Cashu(mint_url),
                                        config: None,
                                        metadata,
                                    },
                                )
                                .await;
                            }
                        }
                    }
                    UICoreMsg::AddFederation(invite_code) => {
                        let id = invite_code.federation_id();
                        match core.add_federation(msg.id, invite_code).await {
                            Err(e) => {
                                error!("Error adding federation: {e}");
                                core.msg(msg.id, CoreUIMsg::AddMintFailed(e.to_string()))
                                    .await;
                            }
                            Ok(_) => {
                                if let Ok(new_federation_list) = core.get_mint_items().await {
                                    core.msg(
                                        msg.id,
                                        CoreUIMsg::MintListUpdated(new_federation_list),
                                    )
                                    .await;
                                }
                                core.msg(
                                    msg.id,
                                    CoreUIMsg::AddMintSuccess(MintIdentifier::Fedimint(id)),
                                )
                                .await;
                            }
                        }
                    }
                    UICoreMsg::AddCashuMint(url) => match core
                        .add_cashu_mint(msg.id, url.clone())
                        .await
                    {
                        Err(e) => {
                            error!("Error adding mint: {e}");
                            core.msg(msg.id, CoreUIMsg::AddMintFailed(e.to_string()))
                                .await;
                        }
                        Ok(_) => {
                            if let Ok(new_federation_list) = core.get_mint_items().await {
                                core.msg(msg.id, CoreUIMsg::MintListUpdated(new_federation_list))
                                    .await;
                            }
                            core.msg(
                                msg.id,
                                CoreUIMsg::AddMintSuccess(MintIdentifier::Cashu(url)),
                            )
                            .await;
                        }
                    },
                    UICoreMsg::RemoveMint(id) => {
                        // Send status update before attempting removal
                        core.msg(
                            msg.id,
                            CoreUIMsg::StatusUpdate {
                                message: "Removing mint...".to_string(),
                                operation_id: Some(msg.id),
                            },
                        )
                        .await;

                        match id {
                            MintIdentifier::Fedimint(id) => {
                                match core.remove_federation(msg.id, id).await {
                                    Err(e) => {
                                        error!("Error removing federation: {e}");
                                        core.msg(
                                            msg.id,
                                            CoreUIMsg::RemoveFederationFailed(e.to_string()),
                                        )
                                        .await;
                                    }
                                    Ok(_) => {
                                        log::info!("Removed federation: {id}");
                                        if let Ok(new_federation_list) = core.get_mint_items().await
                                        {
                                            core.msg(
                                                msg.id,
                                                CoreUIMsg::MintListUpdated(new_federation_list),
                                            )
                                            .await;
                                        }
                                        core.msg(msg.id, CoreUIMsg::RemoveFederationSuccess).await;
                                    }
                                }
                            }
                            MintIdentifier::Cashu(url) => {
                                match core.remove_cashu_mint(msg.id, &url).await {
                                    Err(e) => {
                                        error!("Error removing cashu mint: {e}");
                                        core.msg(
                                            msg.id,
                                            CoreUIMsg::RemoveFederationFailed(e.to_string()),
                                        )
                                        .await;
                                    }
                                    Ok(_) => {
                                        log::info!("Removed cashu mint: {url}");
                                        if let Ok(new_federation_list) = core.get_mint_items().await
                                        {
                                            core.msg(
                                                msg.id,
                                                CoreUIMsg::MintListUpdated(new_federation_list),
                                            )
                                            .await;
                                        }
                                        core.msg(msg.id, CoreUIMsg::RemoveFederationSuccess).await;
                                    }
                                }
                            }
                        }
                    }
                    UICoreMsg::RejoinMint(mint) => match mint {
                        MintIdentifier::Fedimint(id) => {
                            if let Ok(Some(invite_code)) =
                                core.storage.get_federation_invite_code(id)
                            {
                                match core.add_federation(msg.id, invite_code).await {
                                    Err(e) => {
                                        error!("Error adding federation: {e}");
                                        core.msg(msg.id, CoreUIMsg::AddMintFailed(e.to_string()))
                                            .await;
                                    }
                                    Ok(_) => {
                                        if let Ok(new_federation_list) = core.get_mint_items().await
                                        {
                                            core.msg(
                                                msg.id,
                                                CoreUIMsg::MintListUpdated(new_federation_list),
                                            )
                                            .await;
                                        }
                                        core.msg(msg.id, CoreUIMsg::AddMintSuccess(mint)).await;
                                        info!("Rejoined federation: {id}");
                                    }
                                }
                            }
                        }
                        MintIdentifier::Cashu(ref mint_url) => {
                            match core.add_cashu_mint(msg.id, mint_url.clone()).await {
                                Err(e) => {
                                    error!("Error adding cashu mint: {e}");
                                    core.msg(msg.id, CoreUIMsg::AddMintFailed(e.to_string()))
                                        .await;
                                }
                                Ok(_) => {
                                    if let Ok(new_list) = core.get_mint_items().await {
                                        core.msg(msg.id, CoreUIMsg::MintListUpdated(new_list))
                                            .await;
                                    }
                                    info!("Rejoined cashu mint: {mint_url}");
                                    core.msg(msg.id, CoreUIMsg::AddMintSuccess(mint)).await;
                                }
                            }
                        }
                    },
                    UICoreMsg::FederationListNeedsUpdate => {
                        if let Ok(new_federation_list) = core.get_mint_items().await {
                            core.msg(msg.id, CoreUIMsg::MintListUpdated(new_federation_list))
                                .await;
                        }
                    }
                    UICoreMsg::GetSeedWords => {
                        let seed_words = core.get_seed_words().await;
                        core.msg(msg.id, CoreUIMsg::SeedWords(seed_words)).await;
                    }
                    UICoreMsg::SetOnchainReceiveEnabled(enabled) => {
                        match core.set_onchain_receive_enabled(enabled).await {
                            Err(e) => {
                                error!("error setting onchain receive enabled: {e}");
                            }
                            _ => {
                                core.msg(msg.id, CoreUIMsg::OnchainReceiveEnabled(enabled))
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::SetTorEnabled(enabled) => {
                        match core.set_tor_enabled(enabled).await {
                            Err(e) => {
                                error!("error setting tor enabled: {e}");
                            }
                            _ => {
                                core.msg(msg.id, CoreUIMsg::TorEnabled(enabled)).await;
                            }
                        }
                    }
                    UICoreMsg::TestStatusUpdates => {
                        core.test_status_updates(msg.id).await;
                    }
                    UICoreMsg::Unlock(_password) => {
                        unreachable!("should already be unlocked")
                    }
                    UICoreMsg::Init { .. } => {
                        unreachable!("should already be inited")
                    }
                }
            }
        });
    }
}
