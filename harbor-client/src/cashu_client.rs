use crate::db::DBConnection;
use crate::fedimint_client::update_history;
use crate::{CoreUIMsg, CoreUIMsgPacket, MintIdentifier, ReceiveSuccessMsg, SendSuccessMsg};
use bitcoin::hex::FromHex;
use cdk::Wallet;
use cdk::amount::SplitTarget;
use cdk::nuts::MintQuoteState;
use cdk::util::unix_time;
use cdk::wallet::{MeltQuote, MintQuote};
use fedimint_core::Amount;
use futures::SinkExt;
use futures::channel::mpsc::Sender;
use log::error;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use uuid::Uuid;

pub fn spawn_lightning_payment_thread(
    mut sender: Sender<CoreUIMsgPacket>,
    client: Wallet,
    storage: Arc<dyn DBConnection + Send + Sync>,
    quote: MeltQuote,
    msg_id: Uuid,
    is_transfer: bool,
) {
    spawn(async move {
        match client.melt(&quote.id).await {
            Ok(outgoing) => {
                log::info!(
                    "Payment completed: {}, preimage: {:?}",
                    quote.id,
                    outgoing.preimage
                );
                let preimage: [u8; 32] = FromHex::from_hex(&outgoing.preimage.unwrap())
                    .expect("preimage must be valid hex");
                let params = if is_transfer {
                    SendSuccessMsg::Transfer
                } else {
                    SendSuccessMsg::Lightning { preimage }
                };
                sender
                    .send(CoreUIMsgPacket {
                        id: Some(msg_id),
                        msg: CoreUIMsg::SendSuccess(params),
                    })
                    .await
                    .unwrap();

                let bal: u64 = client.total_balance().await.unwrap().into();
                sender
                    .send(CoreUIMsgPacket {
                        id: Some(msg_id),
                        msg: CoreUIMsg::MintBalanceUpdated {
                            id: MintIdentifier::Cashu(client.mint_url.clone()),
                            balance: Amount::from_sats(bal),
                        },
                    })
                    .await
                    .unwrap();

                if let Err(e) = storage.set_lightning_payment_preimage(quote.id, preimage) {
                    error!("Could not set preimage for lightning payment: {e}");
                }

                update_history(storage, msg_id, &mut sender).await;
            }
            Err(e) => {
                log::error!("Payment failed: {e}");
                sender
                    .send(CoreUIMsgPacket {
                        id: Some(msg_id),
                        msg: if is_transfer {
                            CoreUIMsg::TransferFailure(e.to_string())
                        } else {
                            CoreUIMsg::SendFailure(e.to_string())
                        },
                    })
                    .await
                    .unwrap();

                if let Err(e) = storage.mark_lightning_payment_as_failed(quote.id) {
                    error!("Could not mark lightning payment as failed: {e}");
                }
            }
        }
    });
}

pub fn spawn_lightning_receive_thread(
    mut sender: Sender<CoreUIMsgPacket>,
    client: Wallet,
    storage: Arc<dyn DBConnection + Send + Sync>,
    quote: MintQuote,
    msg_id: Uuid,
    is_transfer: bool,
) {
    spawn(async move {
        loop {
            let mint_quote_response = client
                .mint_quote_state(&quote.id)
                .await
                .expect("Failed to get mint quote state");

            if mint_quote_response.state == MintQuoteState::Paid {
                client
                    .mint(&quote.id, SplitTarget::default(), None)
                    .await
                    .expect("Failed to mint receive tokens");

                let params = if is_transfer {
                    ReceiveSuccessMsg::Transfer
                } else {
                    ReceiveSuccessMsg::Lightning
                };
                sender
                    .send(CoreUIMsgPacket {
                        id: Some(msg_id),
                        msg: CoreUIMsg::ReceiveSuccess(params),
                    })
                    .await
                    .unwrap();

                if let Err(e) = storage.mark_ln_receive_as_success(quote.id) {
                    error!("Could not mark lightning receive as success: {e}");
                }

                let new_balance = client.total_balance().await.expect("Failed to get balance");
                sender
                    .send(CoreUIMsgPacket {
                        id: Some(msg_id),
                        msg: CoreUIMsg::MintBalanceUpdated {
                            id: MintIdentifier::Cashu(client.mint_url.clone()),
                            balance: Amount::from_sats(new_balance.into()),
                        },
                    })
                    .await
                    .unwrap();

                update_history(storage, msg_id, &mut sender).await;

                break;
            } else if quote.expiry.le(&unix_time()) {
                client
                    .localstore
                    .remove_mint_quote(&quote.id)
                    .await
                    .expect("Failed to remove mint quote");

                sender
                    .send(CoreUIMsgPacket {
                        id: Some(msg_id),
                        msg: CoreUIMsg::ReceiveFailed("Expired".to_string()),
                    })
                    .await
                    .unwrap();

                if let Err(e) = storage.mark_ln_receive_as_failed(quote.id) {
                    error!("Could not mark lightning receive as failed: {e}");
                }

                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}
