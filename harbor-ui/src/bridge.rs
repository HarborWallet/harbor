use crate::conf::{generate_mnemonic, retrieve_mnemonic};
use crate::{conf, Message};
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Network};
use fedimint_core::config::FederationId;
use fedimint_core::invite_code::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use harbor_client::db::{check_password, setup_db, DBConnection};
use harbor_client::fedimint_client::{FederationInviteOrId, FedimintClient};
use harbor_client::HarborCore;
use harbor_client::{CoreUIMsg, CoreUIMsgPacket, UICoreMsg, UICoreMsgPacket};
use iced::futures::channel::mpsc::Sender;
use iced::futures::{SinkExt, Stream, StreamExt};
use log::{error, warn};
use std::collections::HashMap;
use std::path::PathBuf;
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
    pub async fn msg_send(&self, msg: UICoreMsgPacket) {
        self.ui_to_core_tx.send(msg).await.unwrap();
    }

    pub async fn send_lightning(
        &self,
        id: Uuid,
        federation_id: FederationId,
        invoice: Bolt11Invoice,
    ) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::SendLightning {
                federation_id,
                invoice,
            },
            id,
        })
        .await;
    }

    pub async fn send_onchain(
        &self,
        id: Uuid,
        federation_id: FederationId,
        address: Address<NetworkUnchecked>,
        amount_sats: Option<u64>,
    ) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::SendOnChain {
                federation_id,
                address,
                amount_sats,
            },
            id,
        })
        .await;
    }

    pub async fn receive(&self, id: Uuid, federation_id: FederationId, amount: u64) {
        let amount = Amount::from_sats(amount);
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::ReceiveLightning {
                federation_id,
                amount,
            },
            id,
        })
        .await;
    }

    pub async fn receive_onchain(&self, id: Uuid, federation_id: FederationId) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::ReceiveOnChain { federation_id },
            id,
        })
        .await;
    }

    pub async fn unlock(&self, id: Uuid, password: String) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::Unlock(password),
            id,
        })
        .await;
    }

    pub async fn init(&self, id: Uuid, password: String) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::Init {
                password,
                seed: None, // FIXME: Use this
            },
            id,
        })
        .await;
    }

    pub async fn add_federation(&self, id: Uuid, invite: InviteCode) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::AddFederation(invite),
            id,
        })
        .await;
    }

    pub async fn peek_federation(&self, id: Uuid, invite: InviteCode) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::GetFederationInfo(invite),
            id,
        })
        .await;
    }

    pub async fn get_seed_words(&self, id: Uuid) {
        self.msg_send(UICoreMsgPacket {
            msg: UICoreMsg::GetSeedWords,
            id,
        })
        .await;
    }
}

impl CoreHandle {
    pub async fn recv(&mut self) -> Option<UICoreMsgPacket> {
        self.core_from_ui_rx.recv().await
    }
}

#[derive(Debug)]
pub struct CoreHandle {
    core_from_ui_rx: mpsc::Receiver<UICoreMsgPacket>,
}

pub fn create_handles() -> (UIHandle, CoreHandle) {
    let (ui_to_core_tx, core_from_ui_rx) = mpsc::channel::<UICoreMsgPacket>(50);

    let ui_handle = UIHandle { ui_to_core_tx };

    let core_handle = CoreHandle { core_from_ui_rx };

    (ui_handle, core_handle)
}

pub fn run_core() -> impl Stream<Item = Message> {
    iced::stream::channel(100, |mut tx: Sender<Message>| async move {
        // Setup UI Handle
        let (ui_handle, mut core_handle) = create_handles();
        let arc_ui_handle = Arc::new(ui_handle);
        tx.send(Message::UIHandlerLoaded(arc_ui_handle))
            .await
            .expect("should send");

        // todo make configurable
        let network = Network::Signet;

        // Create the datadir if it doesn't exist
        let path = PathBuf::from(&conf::data_dir(network));
        std::fs::create_dir_all(path.clone()).expect("Could not create datadir");
        log::info!("Using datadir: {path:?}");

        // FIXME: Artificial sleep because it loads too fast
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Check if the database file exists already, if so tell UI to unlock
        if std::fs::metadata(path.join(HARBOR_FILE_NAME)).is_ok() {
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

                    // attempting to unlock
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

                    log::info!("Correct password");

                    let db = spawn_blocking(move || setup_db(&db_path, password))
                        .await
                        .expect("Could not create join handle");

                    if let Err(e) = db {
                        error!("error opening database: {e}");

                        tx.send(Message::core_msg(
                            id,
                            CoreUIMsg::UnlockFailed(e.to_string()),
                        ))
                        .await
                        .expect("should send");
                        continue;
                    }
                    let db = db.expect("no error");

                    let mnemonic = retrieve_mnemonic(db.clone()).expect("should get seed");

                    let stop = Arc::new(AtomicBool::new(false));

                    // check db for fedimints
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
                        mnemonic,
                        network,
                        clients: Arc::new(RwLock::new(clients)),
                        stop,
                    };

                    tx.send(Message::core_msg(id, CoreUIMsg::UnlockSuccess))
                        .await
                        .expect("should send");

                    process_core(&mut core_handle, &core).await;
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
                        mnemonic: generate_mnemonic(db.clone(), seed)
                            .expect("should generate words"),
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
    core.init_ui_state().await;

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
                        core.msg(Some(msg.id), CoreUIMsg::Sending).await;
                        if let Err(e) = core.send_lightning(msg.id, federation_id, invoice).await {
                            error!("Error sending: {e}");
                            core.msg(Some(msg.id), CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveLightning {
                        federation_id,
                        amount,
                    } => {
                        core.msg(Some(msg.id), CoreUIMsg::ReceiveGenerating).await;
                        match core.receive_lightning(msg.id, federation_id, amount).await {
                            Err(e) => {
                                core.msg(Some(msg.id), CoreUIMsg::ReceiveFailed(e.to_string()))
                                    .await;
                            }
                            Ok(invoice) => {
                                core.msg(Some(msg.id), CoreUIMsg::ReceiveInvoiceGenerated(invoice))
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
                        core.msg(Some(msg.id), CoreUIMsg::Sending).await;
                        if let Err(e) = core
                            .send_onchain(msg.id, federation_id, address, amount_sats)
                            .await
                        {
                            error!("Error sending: {e}");
                            core.msg(Some(msg.id), CoreUIMsg::SendFailure(e.to_string()))
                                .await;
                        }
                    }
                    UICoreMsg::ReceiveOnChain { federation_id } => {
                        core.msg(Some(msg.id), CoreUIMsg::ReceiveGenerating).await;
                        match core.receive_onchain(msg.id, federation_id).await {
                            Err(e) => {
                                core.msg(Some(msg.id), CoreUIMsg::ReceiveFailed(e.to_string()))
                                    .await;
                            }
                            Ok(address) => {
                                core.msg(Some(msg.id), CoreUIMsg::ReceiveAddressGenerated(address))
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::GetFederationInfo(invite_code) => {
                        match core.get_federation_info(invite_code).await {
                            Err(e) => {
                                error!("Error getting federation info: {e}");
                                core.msg(
                                    Some(msg.id),
                                    CoreUIMsg::AddFederationFailed(e.to_string()),
                                )
                                .await;
                            }
                            Ok(config) => {
                                core.msg(Some(msg.id), CoreUIMsg::FederationInfo(config))
                                    .await;
                            }
                        }
                    }
                    UICoreMsg::AddFederation(invite_code) => {
                        if let Err(e) = core.add_federation(invite_code).await {
                            error!("Error adding federation: {e}");
                            core.msg(Some(msg.id), CoreUIMsg::AddFederationFailed(e.to_string()))
                                .await;
                        } else {
                            core.msg(Some(msg.id), CoreUIMsg::AddFederationSuccess)
                                .await;
                            let new_federation_list = core.get_federation_items().await;
                            core.msg(
                                Some(msg.id),
                                CoreUIMsg::FederationListUpdated(new_federation_list),
                            )
                            .await;
                        }
                    }
                    UICoreMsg::Unlock(_password) => {
                        unreachable!("should already be unlocked")
                    }
                    UICoreMsg::Init { .. } => {
                        unreachable!("should already be inited")
                    }
                    UICoreMsg::GetSeedWords => {
                        let seed_words = core.get_seed_words().await;
                        core.msg(Some(msg.id), CoreUIMsg::SeedWords(seed_words))
                            .await;
                    }
                }
            }
        });
    }
}
