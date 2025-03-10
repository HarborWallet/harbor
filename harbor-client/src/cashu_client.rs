use crate::db::DBConnection;
use crate::fedimint_client::update_history;
use crate::http::{make_get_request_tor, make_tor_request};
use crate::{CoreUIMsg, CoreUIMsgPacket, MintIdentifier, ReceiveSuccessMsg, SendSuccessMsg};
use async_trait::async_trait;
use bitcoin::hex::FromHex;
use cdk::amount::SplitTarget;
use cdk::mint_url::MintUrl;
use cdk::nuts::{
    CheckStateRequest, CheckStateResponse, Id, KeySet, KeysResponse, KeysetResponse,
    MeltBolt11Request, MeltQuoteBolt11Request, MeltQuoteBolt11Response, MintBolt11Request,
    MintBolt11Response, MintInfo, MintQuoteBolt11Request, MintQuoteBolt11Response, MintQuoteState,
    RestoreRequest, RestoreResponse, SwapRequest, SwapResponse,
};
use cdk::util::unix_time;
use cdk::wallet::{MeltQuote, MintConnector, MintQuote};
use cdk::{Error, Wallet};
use fedimint_core::Amount;
use futures::SinkExt;
use futures::channel::mpsc::Sender;
use log::error;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tokio::spawn;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TorMintConnector {
    mint_url: MintUrl,
    cancel_handle: Arc<AtomicBool>,
}

impl TorMintConnector {
    pub fn new(mint_url: MintUrl, cancel_handle: Arc<AtomicBool>) -> Self {
        Self {
            mint_url,
            cancel_handle,
        }
    }

    async fn http_get<R: DeserializeOwned + Send + 'static>(&self, url: Url) -> Result<R, Error> {
        let res: R = make_get_request_tor(url.as_str(), self.cancel_handle.clone())
            .await
            .map_err(|e| Error::Custom(e.to_string()))?;
        Ok(res)
    }

    #[inline]
    async fn http_post<P: Serialize + ?Sized, R: DeserializeOwned + Send + 'static>(
        &self,
        url: Url,
        payload: &P,
    ) -> Result<R, Error> {
        let res: R = make_tor_request(url.as_str(), Some(payload), self.cancel_handle.clone())
            .await
            .map_err(|e| Error::Custom(e.to_string()))?;
        Ok(res)
    }
}

#[async_trait]
impl MintConnector for TorMintConnector {
    /// Get Active Mint Keys [NUT-01]
    async fn get_mint_keys(&self) -> Result<Vec<KeySet>, Error> {
        let url = self.mint_url.join_paths(&["v1", "keys"])?;
        Ok(self.http_get::<KeysResponse>(url).await?.keysets)
    }

    /// Get Keyset Keys [NUT-01]
    async fn get_mint_keyset(&self, keyset_id: Id) -> Result<KeySet, Error> {
        let url = self
            .mint_url
            .join_paths(&["v1", "keys", &keyset_id.to_string()])?;
        self.http_get::<KeysResponse>(url)
            .await?
            .keysets
            .drain(0..1)
            .next()
            .ok_or_else(|| Error::UnknownKeySet)
    }

    /// Get Keysets [NUT-02]
    async fn get_mint_keysets(&self) -> Result<KeysetResponse, Error> {
        let url = self.mint_url.join_paths(&["v1", "keysets"])?;
        self.http_get(url).await
    }

    /// Mint Quote [NUT-04]
    async fn post_mint_quote(
        &self,
        request: MintQuoteBolt11Request,
    ) -> Result<MintQuoteBolt11Response<String>, Error> {
        let url = self
            .mint_url
            .join_paths(&["v1", "mint", "quote", "bolt11"])?;
        self.http_post(url, &request).await
    }

    /// Mint Quote status
    async fn get_mint_quote_status(
        &self,
        quote_id: &str,
    ) -> Result<MintQuoteBolt11Response<String>, Error> {
        let url = self
            .mint_url
            .join_paths(&["v1", "mint", "quote", "bolt11", quote_id])?;

        self.http_get(url).await
    }

    /// Mint Tokens [NUT-04]
    async fn post_mint(
        &self,
        request: MintBolt11Request<String>,
    ) -> Result<MintBolt11Response, Error> {
        let url = self.mint_url.join_paths(&["v1", "mint", "bolt11"])?;
        self.http_post(url, &request).await
    }

    /// Melt Quote [NUT-05]
    async fn post_melt_quote(
        &self,
        request: MeltQuoteBolt11Request,
    ) -> Result<MeltQuoteBolt11Response<String>, Error> {
        let url = self
            .mint_url
            .join_paths(&["v1", "melt", "quote", "bolt11"])?;
        self.http_post(url, &request).await
    }

    /// Melt Quote Status
    async fn get_melt_quote_status(
        &self,
        quote_id: &str,
    ) -> Result<MeltQuoteBolt11Response<String>, Error> {
        let url = self
            .mint_url
            .join_paths(&["v1", "melt", "quote", "bolt11", quote_id])?;

        self.http_get(url).await
    }

    /// Melt [NUT-05]
    /// [Nut-08] Lightning fee return if outputs defined
    async fn post_melt(
        &self,
        request: MeltBolt11Request<String>,
    ) -> Result<MeltQuoteBolt11Response<String>, Error> {
        let url = self.mint_url.join_paths(&["v1", "melt", "bolt11"])?;
        self.http_post(url, &request).await
    }

    /// Swap Token [NUT-03]
    async fn post_swap(&self, swap_request: SwapRequest) -> Result<SwapResponse, Error> {
        let url = self.mint_url.join_paths(&["v1", "swap"])?;
        self.http_post(url, &swap_request).await
    }

    /// Get Mint Info [NUT-06]
    async fn get_mint_info(&self) -> Result<MintInfo, Error> {
        let url = self.mint_url.join_paths(&["v1", "info"])?;
        self.http_get(url).await
    }

    /// Spendable check [NUT-07]
    async fn post_check_state(
        &self,
        request: CheckStateRequest,
    ) -> Result<CheckStateResponse, Error> {
        let url = self.mint_url.join_paths(&["v1", "checkstate"])?;
        self.http_post(url, &request).await
    }

    /// Restore request [NUT-13]
    async fn post_restore(&self, request: RestoreRequest) -> Result<RestoreResponse, Error> {
        let url = self.mint_url.join_paths(&["v1", "restore"])?;
        self.http_post(url, &request).await
    }
}

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

                if let Err(e) = storage.mark_ln_receive_as_failed(quote.id) {
                    error!("Could not mark lightning receive as failed: {e}");
                }

                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}
