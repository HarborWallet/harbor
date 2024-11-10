use anyhow::anyhow;
use bip39::Mnemonic;
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Network};
use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::invite_code::InviteCode;
use fedimint_core::Amount;
use fedimint_ln_client::{LightningClientModule, PayType};
use fedimint_ln_common::config::FeeToAmount;
use fedimint_ln_common::lightning_invoice::{Bolt11Invoice, Bolt11InvoiceDescription, Description};
use fedimint_wallet_client::WalletClientModule;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use futures::{channel::mpsc::Sender, SinkExt};
use log::{error, trace};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db::DBConnection;
use crate::fedimint_client::{
    select_gateway, spawn_internal_payment_subscription, spawn_invoice_payment_subscription,
    spawn_invoice_receive_subscription, spawn_onchain_payment_subscription,
    spawn_onchain_receive_subscription, FederationInviteOrId, FedimintClient,
};
use crate::{CoreUIMsg, CoreUIMsgPacket, FederationItem};

#[derive(Clone)]
pub struct HarborCore {
    pub network: Network,
    pub mnemonic: Mnemonic,
    pub tx: Sender<CoreUIMsgPacket>,
    pub clients: Arc<RwLock<HashMap<FederationId, FedimintClient>>>,
    pub storage: Arc<dyn DBConnection + Send + Sync>,
    pub stop: Arc<AtomicBool>,
}

impl HarborCore {
    pub async fn msg(&self, id: Option<Uuid>, msg: CoreUIMsg) {
        self.tx
            .clone()
            .send(CoreUIMsgPacket { id, msg })
            .await
            .unwrap();
    }

    // Sends updates to the UI to refelect the initial state
    pub async fn init_ui_state(&self) {
        for client in self.clients.read().await.values() {
            let fed_balance = client.fedimint_client.get_balance().await;
            self.msg(
                None,
                CoreUIMsg::FederationBalanceUpdated {
                    id: client.fedimint_client.federation_id(),
                    balance: fed_balance,
                },
            )
            .await;
        }

        let history = self.storage.get_transaction_history().unwrap();
        self.msg(None, CoreUIMsg::TransactionHistoryUpdated(history))
            .await;

        let federation_items = self.get_federation_items().await;
        self.msg(None, CoreUIMsg::FederationListUpdated(federation_items))
            .await;
    }

    async fn get_client(&self, federation_id: FederationId) -> FedimintClient {
        let clients = self.clients.read().await;
        clients
            .get(&federation_id)
            .expect("No client found for federation")
            .clone()
    }

    pub async fn send_lightning(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        invoice: Bolt11Invoice,
    ) -> anyhow::Result<()> {
        if invoice.amount_milli_satoshis().is_none() {
            return Err(anyhow!("Invoice must have an amount"));
        }
        let amount = Amount::from_msats(invoice.amount_milli_satoshis().expect("must have amount"));

        // todo go through all clients and select the first one that has enough balance
        let client = self.get_client(federation_id).await.fedimint_client;
        let lightning_module = client
            .get_first_module::<LightningClientModule>()
            .expect("must have ln module");

        let gateway = select_gateway(&client)
            .await
            .ok_or(anyhow!("Internal error: No gateway found for federation"))?;

        let fees = gateway.fees.to_amount(&amount);

        log::info!("Sending lightning invoice: {invoice}, paying fees: {fees}");

        let outgoing = lightning_module
            .pay_bolt11_invoice(Some(gateway), invoice.clone(), ())
            .await?;

        self.storage.create_lightning_payment(
            outgoing.payment_type.operation_id(),
            client.federation_id(),
            invoice,
            amount,
            fees,
        )?;

        match outgoing.payment_type {
            PayType::Internal(op_id) => {
                let sub = lightning_module.subscribe_internal_pay(op_id).await?;
                spawn_internal_payment_subscription(
                    self.tx.clone(),
                    client.clone(),
                    self.storage.clone(),
                    op_id,
                    msg_id,
                    sub,
                )
                .await;
            }
            PayType::Lightning(op_id) => {
                let sub = lightning_module.subscribe_ln_pay(op_id).await?;
                spawn_invoice_payment_subscription(
                    self.tx.clone(),
                    client.clone(),
                    self.storage.clone(),
                    op_id,
                    msg_id,
                    sub,
                )
                .await;
            }
        }

        log::info!("Invoice sent");

        Ok(())
    }

    pub async fn receive_lightning(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        amount: Amount,
    ) -> anyhow::Result<Bolt11Invoice> {
        let client = self.get_client(federation_id).await.fedimint_client;
        let lightning_module = client
            .get_first_module::<LightningClientModule>()
            .expect("must have ln module");

        let gateway = select_gateway(&client)
            .await
            .ok_or(anyhow!("Internal error: No gateway found for federation"))?;

        let desc = Description::new(String::new()).expect("empty string is valid");
        let (op_id, invoice, preimage) = lightning_module
            .create_bolt11_invoice(
                amount,
                Bolt11InvoiceDescription::Direct(&desc),
                None,
                (),
                Some(gateway),
            )
            .await?;

        log::info!("Invoice created: {invoice}");

        self.storage.create_ln_receive(
            op_id,
            client.federation_id(),
            invoice.clone(),
            amount,
            Amount::ZERO, // todo one day there will be receive fees
            preimage,
        )?;

        // Create subscription to operation if it exists
        if let Ok(subscription) = lightning_module.subscribe_ln_receive(op_id).await {
            spawn_invoice_receive_subscription(
                self.tx.clone(),
                client.clone(),
                self.storage.clone(),
                op_id,
                msg_id,
                subscription,
            )
            .await;
        } else {
            error!("Could not create subscription to lightning receive");
        }

        Ok(invoice)
    }

    /// Sends a given amount of sats to a given address, if the amount is None, send all funds
    pub async fn send_onchain(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
        address: Address<NetworkUnchecked>,
        sats: Option<u64>,
    ) -> anyhow::Result<()> {
        // todo go through all clients and select the first one that has enough balance
        let client = self.get_client(federation_id).await.fedimint_client;
        let onchain = client
            .get_first_module::<WalletClientModule>()
            .expect("must have wallet module");

        // todo add manual fee selection
        let (fees, amount) = match sats {
            Some(sats) => {
                let amount = bitcoin::Amount::from_sat(sats);
                let fees = onchain.get_withdraw_fees(address.clone(), amount).await?;
                (fees, amount)
            }
            None => {
                let balance = client.get_balance().await;

                if balance.sats_round_down() == 0 {
                    return Err(anyhow!("No funds in wallet"));
                }

                // get fees for the entire balance
                let fees = onchain
                    .get_withdraw_fees(
                        address.clone(),
                        bitcoin::Amount::from_sat(balance.sats_round_down()),
                    )
                    .await?;

                let fees_paid = Amount::from_sats(fees.amount().to_sat());
                let amount = balance.saturating_sub(fees_paid);

                if amount.sats_round_down() < 546 {
                    return Err(anyhow!("Not enough funds to send"));
                }

                (fees, bitcoin::Amount::from_sat(amount.sats_round_down()))
            }
        };

        let op_id = onchain.withdraw(address.clone(), amount, fees, ()).await?;

        self.storage.create_onchain_payment(
            op_id,
            client.federation_id(),
            address,
            amount.to_sat(),
            fees.amount().to_sat(),
        )?;

        let sub = onchain.subscribe_withdraw_updates(op_id).await?;

        spawn_onchain_payment_subscription(
            self.tx.clone(),
            client.clone(),
            self.storage.clone(),
            op_id,
            msg_id,
            sub,
        )
        .await;

        Ok(())
    }

    pub async fn receive_onchain(
        &self,
        msg_id: Uuid,
        federation_id: FederationId,
    ) -> anyhow::Result<Address> {
        let client = self.get_client(federation_id).await.fedimint_client;
        let onchain = client
            .get_first_module::<WalletClientModule>()
            .expect("must have wallet module");

        let (op_id, address, _) = onchain.allocate_deposit_address_expert_only(()).await?;

        self.storage
            .create_onchain_receive(op_id, client.federation_id(), address.clone())?;

        let sub = onchain.subscribe_deposit(op_id).await?;

        spawn_onchain_receive_subscription(
            self.tx.clone(),
            client.clone(),
            self.storage.clone(),
            op_id,
            msg_id,
            sub,
        )
        .await;

        Ok(address)
    }

    pub async fn get_federation_info(
        &self,
        invite_code: InviteCode,
    ) -> anyhow::Result<ClientConfig> {
        let download = Instant::now();
        let config = fedimint_api_client::api::net::Connector::Tor
            .download_from_invite_code(&invite_code)
            .await
            .map_err(|e| {
                error!("Could not download federation info: {e}");
                e
            })?;
        trace!(
            "Downloaded federation info in: {}ms",
            download.elapsed().as_millis()
        );

        Ok(config)
    }

    pub async fn add_federation(&self, invite_code: InviteCode) -> anyhow::Result<()> {
        let id = invite_code.federation_id();

        let mut clients = self.clients.write().await;
        if clients.get(&id).is_some() {
            return Err(anyhow!("Federation already added"));
        }

        let client = FedimintClient::new(
            self.storage.clone(),
            FederationInviteOrId::Invite(invite_code),
            &self.mnemonic,
            self.network,
            self.stop.clone(),
        )
        .await?;

        clients.insert(client.fedimint_client.federation_id(), client);

        Ok(())
    }

    pub async fn get_federation_items(&self) -> Vec<FederationItem> {
        let clients = self.clients.read().await;

        // Tell the UI about any clients we have
        clients
            .values()
            .map(|c| FederationItem {
                id: c.fedimint_client.federation_id(),
                name: c
                    .fedimint_client
                    .get_meta("federation_name")
                    .unwrap_or("Unknown".to_string()),
                // TODO: get the balance per fedimint
                balance: 420,
                guardians: None,
                module_kinds: None,
            })
            .collect::<Vec<FederationItem>>()
    }

    pub async fn get_seed_words(&self) -> String {
        self.mnemonic.to_string()
    }
}
