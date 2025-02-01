use crate::config::read_config;
use crate::Message;
use bitcoin::Network;
use fedimint_core::config::FederationId;
use harbor_client::db::{check_password, setup_db, DBConnection};
use harbor_client::fedimint_client::{FederationInviteOrId, FedimintClient};
use harbor_client::{data_dir, CoreUIMsg, CoreUIMsgPacket, HarborCore, UICoreMsg, UICoreMsgPacket};
use iced::futures::channel::mpsc::Sender;
use iced::futures::{SinkExt, Stream, StreamExt};
use log::{error, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::task::spawn_blocking;
use uuid::Uuid;

pub const HARBOR_FILE_NAME: &str = "harbor.sqlite";

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
    let mnemonic = db.retrieve_mnemonic().expect("should get seed");

    // Create stop signal
    let stop = Arc::new(AtomicBool::new(false));

    // Setup federation clients
    let mut clients = HashMap::new();
    let federation_ids = db
        .list_federations()
        .expect("should load initial fedimints");
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

    // Setup core message channel
    let (core_tx, mut core_rx) = iced::futures::channel::mpsc::channel::<CoreUIMsgPacket>(128);
    let mut tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(rev) = core_rx.next().await {
            tx_clone
                .send(Message::CoreMessage(rev))
                .await
                .expect("should send");
        }
    });

    // Create and return HarborCore
    Some(HarborCore {
        network,
        mnemonic,
        tx: core_tx,
        clients: Arc::new(RwLock::new(clients)),
        storage: db,
        stop: stop.clone(),
    })
}

/// Attempts to auto-unlock the wallet using a password from the environment.
/// Returns Some(HarborCore) if successful, None if unsuccessful or no password found.
async fn try_auto_unlock(
    path: &Path,
    network: Network,
    tx: &mut Sender<Message>,
) -> Option<HarborCore> {
    let password = std::env::var("WALLET_PASSWORD").ok()?;
    log::info!("Found password in environment, attempting auto-unlock");

    let db_path = path.join(HARBOR_FILE_NAME);
    let db_path = db_path.to_str().unwrap().to_string();

    if check_password(&db_path, &password).is_err() {
        return None;
    }

    let core = setup_harbor_core(&db_path, &password, network, tx).await?;
    tx.send(Message::core_msg(None, CoreUIMsg::UnlockSuccess))
        .await
        .expect("should send");
    Some(core)
}

pub fn run_core() -> impl Stream<Item = Message> {
    iced::stream::channel(100, |mut tx: Sender<Message>| async move {
        // Setup UI Handle
        let (ui_handle, mut core_handle) = create_handles();
        let arc_ui_handle = Arc::new(ui_handle);
        tx.send(Message::UIHandlerLoaded(arc_ui_handle))
            .await
            .expect("should send");

        let config = read_config().expect("could not read config");
        let network = config.network;

        tx.send(Message::ConfigLoaded(config))
            .await
            .expect("should send");

        // Create the datadir if it doesn't exist
        let path = PathBuf::from(&data_dir(network));
        std::fs::create_dir_all(&path).expect("Could not create datadir");
        log::info!("Using datadir: {path:?}");

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

                    if let Some(core) =
                        setup_harbor_core(&db_path, &password, network, &mut tx).await
                    {
                        tx.send(Message::core_msg(id, CoreUIMsg::UnlockSuccess))
                            .await
                            .expect("should send");
                        process_core(&mut core_handle, &core).await;
                    } else {
                        tx.send(Message::core_msg(
                            id,
                            CoreUIMsg::UnlockFailed("Failed to setup wallet".to_string()),
                        ))
                        .await
                        .expect("should send");
                    }
                }
                Some(UICoreMsg::Init { password, seed }) => {
                    log::info!("Sending init message");
                    tx.send(Message::core_msg(id, CoreUIMsg::Initing))
                        .await
                        .expect("should send");

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

                    let (core_tx, mut core_rx) =
                        iced::futures::channel::mpsc::channel::<CoreUIMsgPacket>(128);

                    let mut tx_clone = tx.clone();
                    tokio::spawn(async move {
                        while let Some(rev) = core_rx.next().await {
                            tx_clone
                                .send(Message::CoreMessage(rev))
                                .await
                                .expect("should send");
                        }
                    });

                    let core = HarborCore {
                        storage: db.clone(),
                        tx: core_tx,
                        mnemonic: db.generate_mnemonic(seed).expect("should generate words"),
                        network,
                        clients: Arc::new(RwLock::new(HashMap::new())),
                        stop: Arc::new(AtomicBool::new(false)),
                    };

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
                    UICoreMsg::SendLightning {
                        federation_id,
                        invoice,
                    } => {
                        log::info!("Got UICoreMsg::Send");
                        core.msg(msg.id, CoreUIMsg::Sending).await;
                        if let Err(e) = core
                            .send_lightning(msg.id, federation_id, invoice, false)
                            .await
                        {
                            error!("Error sending: {e}");
                            core.msg(msg.id, CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveLightning {
                        federation_id,
                        amount,
                    } => {
                        core.msg(msg.id, CoreUIMsg::ReceiveGenerating).await;
                        match core
                            .receive_lightning(msg.id, federation_id, amount, false)
                            .await
                        {
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
                    UICoreMsg::SendOnChain {
                        federation_id,
                        address,
                        amount_sats,
                    } => {
                        log::info!("Got UICoreMsg::SendOnChain");
                        core.msg(msg.id, CoreUIMsg::Sending).await;
                        if let Err(e) = core
                            .send_onchain(msg.id, federation_id, address, amount_sats)
                            .await
                        {
                            error!("Error sending: {e}");
                            core.msg(msg.id, CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveOnChain { federation_id } => {
                        core.msg(msg.id, CoreUIMsg::ReceiveGenerating).await;
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
                        match core.get_federation_info(invite_code).await {
                            Err(e) => {
                                error!("Error getting federation info: {e}");
                                core.msg(msg.id, CoreUIMsg::AddFederationFailed(e.to_string()))
                                    .await;
                            }
                            Ok((config, metadata)) => {
                                core.msg(msg.id, CoreUIMsg::FederationInfo { config, metadata })
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::AddFederation(invite_code) => {
                        if let Err(e) = core.add_federation(invite_code).await {
                            error!("Error adding federation: {e}");
                            core.msg(msg.id, CoreUIMsg::AddFederationFailed(e.to_string()))
                                .await;
                        } else {
                            let new_federation_list = core.get_federation_items().await;
                            core.msg(
                                msg.id,
                                CoreUIMsg::FederationListUpdated(new_federation_list),
                            )
                            .await;
                            core.msg(msg.id, CoreUIMsg::AddFederationSuccess).await;
                        }
                    }
                    UICoreMsg::RemoveFederation(id) => {
                        if let Err(e) = core.remove_federation(id).await {
                            error!("Error removing federation: {e}");
                            core.msg(msg.id, CoreUIMsg::RemoveFederationFailed(e.to_string()))
                                .await;
                        } else {
                            let new_federation_list = core.get_federation_items().await;
                            core.msg(
                                msg.id,
                                CoreUIMsg::FederationListUpdated(new_federation_list),
                            )
                            .await;
                            core.msg(msg.id, CoreUIMsg::RemoveFederationSuccess).await;
                        }
                    }
                    UICoreMsg::FederationListNeedsUpdate => {
                        let new_federation_list = core.get_federation_items().await;
                        core.msg(
                            msg.id,
                            CoreUIMsg::FederationListUpdated(new_federation_list),
                        )
                        .await;
                    }
                    UICoreMsg::GetSeedWords => {
                        let seed_words = core.get_seed_words().await;
                        core.msg(msg.id, CoreUIMsg::SeedWords(seed_words)).await;
                    }
                    UICoreMsg::SetOnchainReceiveEnabled(enabled) => {
                        if let Err(e) = core.set_onchain_receive_enabled(enabled).await {
                            error!("Error setting onchain receive enabled: {e}");
                        } else {
                            core.msg(msg.id, CoreUIMsg::OnchainReceiveEnabled(enabled))
                                .await;
                        }
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
