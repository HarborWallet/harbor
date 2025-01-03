use crate::db_models::transaction_item::TransactionItem;
use crate::db_models::{
    Fedimint, LightningPayment, LightningReceive, NewFedimint, NewProfile, OnChainPayment,
    OnChainReceive, Profile,
};
use bitcoin::address::NetworkUnchecked;
use bitcoin::{Address, Txid};
use diesel::{
    connection::SimpleConnection,
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use fedimint_core::config::FederationId;
use fedimint_core::core::OperationId;
use fedimint_core::Amount;
use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
use rusqlite::{Connection, OpenFlags};
use std::{sync::Arc, time::Duration};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn check_password(url: &str, password: &str) -> anyhow::Result<()> {
    let conn = Connection::open_with_flags(
        url,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )?;

    // Set the key for the encrypted database
    let password = normalize_password(password);
    conn.execute_batch(&format!("PRAGMA key = '{password}';"))?;

    // Try to prepare a query to verify if the key is correct
    let res = conn.prepare("SELECT name FROM sqlite_master WHERE type='table';");

    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            if e.to_string() == "file is not a database" {
                Err(anyhow::anyhow!("Invalid password"))
            } else {
                Err(anyhow::anyhow!("Could not open database: {e}"))
            }
        }
    }
}

pub fn setup_db(url: &str, password: String) -> anyhow::Result<Arc<SQLConnection>> {
    let manager = ConnectionManager::<SqliteConnection>::new(url);

    let pool = Pool::builder()
        .max_size(50)
        .connection_customizer(Box::new(ConnectionOptions {
            key: password,
            enable_wal: true,
            enable_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(15)),
        }))
        .test_on_check_out(true)
        .build(manager)?;
    Ok(Arc::new(SQLConnection { db: pool }))
}

pub trait DBConnection {
    // Gets a seed from the first profile in the DB or returns None
    fn get_seed(&self) -> anyhow::Result<Option<String>>;

    // Inserts a new profile into the DB
    fn insert_new_profile(&self, new_profile: NewProfile) -> anyhow::Result<Profile>;

    // Inserts a new federation into the DB
    fn insert_new_federation(&self, f: NewFedimint) -> anyhow::Result<Fedimint>;

    // Removes a federation from the DB
    fn remove_federation(&self, f: FederationId) -> anyhow::Result<()>;

    // gets the federation data for a specific federation
    fn get_federation_value(&self, id: String) -> anyhow::Result<Option<Vec<u8>>>;

    // gets the federation data for a specific federation
    fn list_federations(&self) -> anyhow::Result<Vec<String>>;

    // updates the federation data
    fn update_fedimint_data(&self, id: String, value: Vec<u8>) -> anyhow::Result<()>;

    fn create_ln_receive(
        &self,
        operation_id: OperationId,
        fedimint_id: FederationId,
        bolt11: Bolt11Invoice,
        amount: Amount,
        fee: Amount,
        preimage: [u8; 32],
    ) -> anyhow::Result<()>;

    fn mark_ln_receive_as_success(&self, operation_id: OperationId) -> anyhow::Result<()>;

    fn mark_ln_receive_as_failed(&self, operation_id: OperationId) -> anyhow::Result<()>;

    fn create_lightning_payment(
        &self,
        operation_id: OperationId,
        fedimint_id: FederationId,
        bolt11: Bolt11Invoice,
        amount: Amount,
        fee: Amount,
    ) -> anyhow::Result<()>;

    fn set_lightning_payment_preimage(
        &self,
        operation_id: OperationId,
        preimage: [u8; 32],
    ) -> anyhow::Result<()>;

    fn mark_lightning_payment_as_failed(&self, operation_id: OperationId) -> anyhow::Result<()>;

    fn create_onchain_payment(
        &self,
        operation_id: OperationId,
        fedimint_id: FederationId,
        address: Address<NetworkUnchecked>,
        amount_sats: u64,
        fee_sats: u64,
    ) -> anyhow::Result<()>;

    fn set_onchain_payment_txid(&self, operation_id: OperationId, txid: Txid)
        -> anyhow::Result<()>;

    fn mark_onchain_payment_as_failed(&self, operation_id: OperationId) -> anyhow::Result<()>;

    fn create_onchain_receive(
        &self,
        operation_id: OperationId,
        fedimint_id: FederationId,
        address: Address,
    ) -> anyhow::Result<()>;

    fn mark_onchain_receive_as_failed(&self, operation_id: OperationId) -> anyhow::Result<()>;

    fn set_onchain_receive_txid(
        &self,
        operation_id: OperationId,
        txid: Txid,
        amount_sats: u64,
        fee_sats: u64,
    ) -> anyhow::Result<()>;

    fn mark_onchain_receive_as_confirmed(&self, operation_id: OperationId) -> anyhow::Result<()>;

    fn get_transaction_history(&self) -> anyhow::Result<Vec<TransactionItem>>;
}

pub struct SQLConnection {
    db: Pool<ConnectionManager<SqliteConnection>>,
}

impl DBConnection for SQLConnection {
    fn get_seed(&self) -> anyhow::Result<Option<String>> {
        let conn = &mut self.db.get()?;
        match Profile::get_first(conn)? {
            Some(p) => Ok(Some(p.seed_words)),
            None => Ok(None),
        }
    }

    fn insert_new_profile(&self, new_profile: NewProfile) -> anyhow::Result<Profile> {
        let conn = &mut self.db.get()?;
        new_profile.insert(conn)
    }

    fn get_federation_value(&self, id: String) -> anyhow::Result<Option<Vec<u8>>> {
        let conn = &mut self.db.get()?;
        Fedimint::get_value(conn, id)
    }

    fn list_federations(&self) -> anyhow::Result<Vec<String>> {
        let conn = &mut self.db.get()?;
        Fedimint::get_ids(conn)
    }

    fn insert_new_federation(&self, f: NewFedimint) -> anyhow::Result<Fedimint> {
        let conn = &mut self.db.get()?;
        f.insert(conn)
    }

    fn update_fedimint_data(&self, id: String, value: Vec<u8>) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;
        let f = Fedimint {
            id,
            value,
            active: 1,
        };
        f.update(conn)
    }

    fn create_ln_receive(
        &self,
        operation_id: OperationId,
        fedimint_id: FederationId,
        bolt11: Bolt11Invoice,
        amount: Amount,
        fee: Amount,
        preimage: [u8; 32],
    ) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        LightningReceive::create(
            conn,
            operation_id,
            fedimint_id,
            bolt11,
            amount,
            fee,
            preimage,
        )?;

        Ok(())
    }

    fn mark_ln_receive_as_success(&self, operation_id: OperationId) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        LightningReceive::mark_as_success(conn, operation_id)?;

        Ok(())
    }

    fn mark_ln_receive_as_failed(&self, operation_id: OperationId) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        LightningReceive::mark_as_failed(conn, operation_id)?;

        Ok(())
    }

    fn create_lightning_payment(
        &self,
        operation_id: OperationId,
        fedimint_id: FederationId,
        bolt11: Bolt11Invoice,
        amount: Amount,
        fee: Amount,
    ) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        LightningPayment::create(conn, operation_id, fedimint_id, bolt11, amount, fee)?;

        Ok(())
    }

    fn set_lightning_payment_preimage(
        &self,
        operation_id: OperationId,
        preimage: [u8; 32],
    ) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        LightningPayment::set_preimage(conn, operation_id, preimage)?;

        Ok(())
    }

    fn mark_lightning_payment_as_failed(&self, operation_id: OperationId) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        LightningPayment::mark_as_failed(conn, operation_id)?;

        Ok(())
    }

    fn create_onchain_receive(
        &self,
        operation_id: OperationId,
        fedimint_id: FederationId,
        address: Address,
    ) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        OnChainReceive::create(conn, operation_id, fedimint_id, address)?;

        Ok(())
    }

    fn create_onchain_payment(
        &self,
        operation_id: OperationId,
        fedimint_id: FederationId,
        address: Address<NetworkUnchecked>,
        amount_sats: u64,
        fee_sats: u64,
    ) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        OnChainPayment::create(
            conn,
            operation_id,
            fedimint_id,
            address,
            amount_sats,
            fee_sats,
        )?;

        Ok(())
    }

    fn set_onchain_payment_txid(
        &self,
        operation_id: OperationId,
        txid: Txid,
    ) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        OnChainPayment::set_txid(conn, operation_id, txid)?;

        Ok(())
    }

    fn mark_onchain_payment_as_failed(&self, operation_id: OperationId) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        OnChainPayment::mark_as_failed(conn, operation_id)?;

        Ok(())
    }

    fn mark_onchain_receive_as_failed(&self, operation_id: OperationId) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        OnChainReceive::mark_as_failed(conn, operation_id)?;

        Ok(())
    }

    fn set_onchain_receive_txid(
        &self,
        operation_id: OperationId,
        txid: Txid,
        amount_sats: u64,
        fee_sats: u64,
    ) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        OnChainReceive::set_txid(conn, operation_id, txid, amount_sats, fee_sats)?;

        Ok(())
    }

    fn mark_onchain_receive_as_confirmed(&self, operation_id: OperationId) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;

        OnChainReceive::mark_as_confirmed(conn, operation_id)?;

        Ok(())
    }

    fn get_transaction_history(&self) -> anyhow::Result<Vec<TransactionItem>> {
        let conn = &mut self.db.get()?;

        let onchain_payments = OnChainPayment::get_history(conn)?;
        let onchain_receives = OnChainReceive::get_history(conn)?;
        let lightning_payments = LightningPayment::get_history(conn)?;
        let lightning_receives = LightningReceive::get_history(conn)?;

        let mut items: Vec<TransactionItem> = Vec::with_capacity(
            onchain_payments.len()
                + onchain_receives.len()
                + lightning_payments.len()
                + lightning_receives.len(),
        );

        for onchain_payment in onchain_payments {
            items.push(onchain_payment.into());
        }

        for onchain_receive in onchain_receives {
            items.push(onchain_receive.into());
        }

        for lightning_payment in lightning_payments {
            items.push(lightning_payment.into());
        }

        for lightning_receive in lightning_receives {
            items.push(lightning_receive.into());
        }

        // sort by timestamp so that the most recent items are at the top
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(items)
    }

    fn remove_federation(&self, f: FederationId) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;
        Fedimint::remove_federation(conn, f.to_string())?;
        Ok(())
    }
}

fn normalize_password(password: &str) -> String {
    password.replace("'", "''")
}

#[derive(Debug)]
pub struct ConnectionOptions {
    pub key: String,
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error>
    for ConnectionOptions
{
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            let password = normalize_password(&self.key);
            conn.batch_execute(&format!("PRAGMA key='{password}'"))?;
            if self.enable_wal {
                conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            }
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }

            conn.run_pending_migrations(MIGRATIONS)
                .expect("Migration has to run successfully");

            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_models::{
        LightningPayment, LightningReceive, OnChainPayment, OnChainReceive, PaymentStatus,
    };
    use bip39::{Language, Mnemonic};
    use bitcoin::hashes::Hash;
    use bitcoin::secp256k1::ThirtyTwoByteHash;
    use bitcoin::{Address, Txid};
    use fedimint_core::config::FederationId;
    use fedimint_core::core::OperationId;
    use fedimint_core::Amount;
    use fedimint_ln_common::lightning_invoice::Bolt11Invoice;
    use std::str::FromStr;
    use tempdir::TempDir;

    const DEFAULT_PASSWORD: &str = "p.a$$w0rd!'x";
    const FEDERATION_ID: &str = "c8d423964c7ad944d30f57359b6e5b260e211dcfdb945140e28d4df51fd572d2";

    fn setup_test_db() -> Arc<SQLConnection> {
        let tmp_dir = TempDir::new("harbor").expect("Could not create temp dir");
        let url = format!("sqlite://{}/harbor.sqlite", tmp_dir.path().display());

        setup_db(&url, DEFAULT_PASSWORD.to_string()).expect("Could not setup db")
    }

    fn setup_test_db_with_data() -> Arc<SQLConnection> {
        let db = setup_test_db();

        let seed_words = Mnemonic::generate_in(Language::English, 12)
            .unwrap()
            .to_string();

        let new_profile = NewProfile {
            id: uuid::Uuid::new_v4().to_string(),
            seed_words,
        };
        db.insert_new_profile(new_profile).unwrap();

        let new_fedimint = NewFedimint {
            id: FEDERATION_ID.to_string(),
            value: vec![],
        };
        db.insert_new_federation(new_fedimint).unwrap();

        db
    }

    #[test]
    fn test_seed() {
        let db = setup_test_db();

        let seed = db.get_seed().unwrap();
        assert!(seed.is_none());

        let new_profile = NewProfile {
            id: uuid::Uuid::new_v4().to_string(),
            seed_words: Mnemonic::generate_in(Language::English, 12)
                .unwrap()
                .to_string(),
        };
        let p = db.insert_new_profile(new_profile).unwrap();

        let seed = db.get_seed().unwrap();
        assert_eq!(seed.unwrap(), p.seed_words);
    }

    #[test]
    fn test_insert_new_federation() {
        let db = setup_test_db();

        let seed_words = Mnemonic::generate_in(Language::English, 12)
            .unwrap()
            .to_string();

        let new_profile = NewProfile {
            id: uuid::Uuid::new_v4().to_string(),
            seed_words,
        };
        db.insert_new_profile(new_profile).unwrap();

        let new_fedimint = NewFedimint {
            id: FEDERATION_ID.to_string(),
            value: vec![],
        };
        db.insert_new_federation(new_fedimint.clone()).unwrap();

        let federation = db.get_federation_value(FEDERATION_ID.to_string()).unwrap();
        assert!(federation.is_some());
        assert_eq!(federation.unwrap(), new_fedimint.value);
    }

    #[test]
    fn test_lightning_payment_db() {
        let db = setup_test_db_with_data();
        let pool = db.db.clone();
        let mut conn = pool.get().unwrap();

        let operation_id = OperationId::new_random();
        let invoice = Bolt11Invoice::from_str("lntbs10u1pny86cupp52lkv666juacc9evu0fpfmduac6l6qp0qypxr0yk9wfpze2u5sngshp57t8sp5tcchfv0y29yg46nqujktk2ufwcjcc7zvyd8rteadd7rjyscqzzsxqyz5vqsp5nnhtrhvyfh077g6rdfrs7ml9hqks4mj6f0e50nyeejc73ee7gl3q9qyyssq3urmp6hy3c95rtddevae0djrfn8au0rumgd05zvddzshg8krwupzc4htl38kqufp27el5ev5l8ea4736y3a3rpq5cewxwftsdk2v52cp9w25a0").unwrap();

        LightningPayment::create(
            &mut conn,
            operation_id,
            FederationId::from_str(FEDERATION_ID).unwrap(),
            invoice.clone(),
            Amount::from_sats(1_000),
            Amount::from_sats(1),
        )
        .unwrap();

        let payment = LightningPayment::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(payment.operation_id(), operation_id);
        assert_eq!(
            payment.fedimint_id(),
            FederationId::from_str(FEDERATION_ID).unwrap()
        );
        assert_eq!(payment.payment_hash(), invoice.payment_hash().into_32());
        assert_eq!(payment.bolt11(), invoice);
        assert_eq!(payment.amount(), Amount::from_sats(1_000));
        assert_eq!(payment.fee(), Amount::from_sats(1));
        assert_eq!(payment.preimage(), None);
        assert_eq!(payment.status(), PaymentStatus::Pending);

        // sleep for a second to make sure the timestamps are different
        std::thread::sleep(Duration::from_secs(1));

        LightningPayment::mark_as_failed(&mut conn, operation_id).unwrap();

        let failed = LightningPayment::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(failed.status(), PaymentStatus::Failed);
        assert_eq!(failed.preimage(), None);
        assert_ne!(failed.updated_at, failed.created_at);
        assert_ne!(failed.updated_at, payment.updated_at);
    }

    #[test]
    fn test_lightning_receive_db() {
        let db = setup_test_db_with_data();
        let pool = db.db.clone();
        let mut conn = pool.get().unwrap();

        let operation_id = OperationId::new_random();
        let invoice = Bolt11Invoice::from_str("lntbs10u1pny86cupp52lkv666juacc9evu0fpfmduac6l6qp0qypxr0yk9wfpze2u5sngshp57t8sp5tcchfv0y29yg46nqujktk2ufwcjcc7zvyd8rteadd7rjyscqzzsxqyz5vqsp5nnhtrhvyfh077g6rdfrs7ml9hqks4mj6f0e50nyeejc73ee7gl3q9qyyssq3urmp6hy3c95rtddevae0djrfn8au0rumgd05zvddzshg8krwupzc4htl38kqufp27el5ev5l8ea4736y3a3rpq5cewxwftsdk2v52cp9w25a0").unwrap();
        let preimage: [u8; 32] = [0; 32];

        LightningReceive::create(
            &mut conn,
            operation_id,
            FederationId::from_str(FEDERATION_ID).unwrap(),
            invoice.clone(),
            Amount::from_sats(1_000),
            Amount::from_sats(1),
            preimage,
        )
        .unwrap();

        let receive = LightningReceive::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(receive.operation_id(), operation_id);
        assert_eq!(
            receive.fedimint_id(),
            FederationId::from_str(FEDERATION_ID).unwrap()
        );
        assert_eq!(receive.payment_hash(), invoice.payment_hash().into_32());
        assert_eq!(receive.bolt11(), invoice);
        assert_eq!(receive.amount(), Amount::from_sats(1_000));
        assert_eq!(receive.fee(), Amount::from_sats(1));
        assert_eq!(receive.preimage(), preimage);
        assert_eq!(receive.status(), PaymentStatus::Pending);

        // sleep for a second to make sure the timestamps are different
        std::thread::sleep(Duration::from_secs(1));

        LightningReceive::mark_as_failed(&mut conn, operation_id).unwrap();

        let failed = LightningReceive::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(failed.status(), PaymentStatus::Failed);
        assert_eq!(failed.preimage(), preimage);
        assert_ne!(failed.updated_at, failed.created_at);
        assert_ne!(failed.updated_at, receive.updated_at);
    }

    #[test]
    fn test_onchain_payment_db() {
        let db = setup_test_db_with_data();
        let pool = db.db.clone();
        let mut conn = pool.get().unwrap();

        let operation_id = OperationId::new_random();
        let address = Address::from_str("tb1qd28npep0s8frcm3y7dxqajkcy2m40eysplyr9v").unwrap();

        let amount: u64 = 10_000;
        let fee: u64 = 200;

        OnChainPayment::create(
            &mut conn,
            operation_id,
            FederationId::from_str(FEDERATION_ID).unwrap(),
            address.clone(),
            amount,
            fee,
        )
        .unwrap();

        let payment = OnChainPayment::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(payment.operation_id(), operation_id);
        assert_eq!(
            payment.fedimint_id(),
            FederationId::from_str(FEDERATION_ID).unwrap()
        );
        assert_eq!(payment.address(), address);
        assert_eq!(payment.amount_sats as u64, amount);
        assert_eq!(payment.fee_sats as u64, fee);
        assert_eq!(payment.txid(), None);
        assert_eq!(payment.status(), PaymentStatus::Pending);

        // sleep for a second to make sure the timestamps are different
        std::thread::sleep(Duration::from_secs(1));

        OnChainPayment::set_txid(&mut conn, operation_id, Txid::all_zeros()).unwrap();

        let with_txid = OnChainPayment::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(with_txid.status(), PaymentStatus::Success);
        assert_eq!(with_txid.txid(), Some(Txid::all_zeros()));
        assert_ne!(with_txid.updated_at, with_txid.created_at);
        assert_ne!(with_txid.updated_at, payment.updated_at);
    }

    #[test]
    fn test_onchain_receive_db() {
        let db = setup_test_db_with_data();
        let pool = db.db.clone();
        let mut conn = pool.get().unwrap();

        let operation_id = OperationId::new_random();
        let address = Address::from_str("tb1qd28npep0s8frcm3y7dxqajkcy2m40eysplyr9v")
            .unwrap()
            .assume_checked();

        let amount: u64 = 10_000;
        let fee: u64 = 200;

        OnChainReceive::create(
            &mut conn,
            operation_id,
            FederationId::from_str(FEDERATION_ID).unwrap(),
            address.clone(),
        )
        .unwrap();

        let payment = OnChainReceive::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(payment.operation_id(), operation_id);
        assert_eq!(
            payment.fedimint_id(),
            FederationId::from_str(FEDERATION_ID).unwrap()
        );
        assert_eq!(payment.address().assume_checked(), address);
        assert!(payment.amount_sats.is_none());
        assert!(payment.fee_sats.is_none());
        assert_eq!(payment.txid(), None);
        assert_eq!(payment.status(), PaymentStatus::Pending);

        // sleep for a second to make sure the timestamps are different
        std::thread::sleep(Duration::from_secs(1));

        OnChainReceive::set_txid(&mut conn, operation_id, Txid::all_zeros(), amount, fee).unwrap();

        let with_txid = OnChainReceive::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(with_txid.status(), PaymentStatus::WaitingConfirmation);
        assert_eq!(with_txid.txid(), Some(Txid::all_zeros()));
        assert_eq!(with_txid.amount_sats, Some(amount as i64));
        assert_eq!(with_txid.fee_sats, Some(fee as i64));
        assert_ne!(with_txid.updated_at, with_txid.created_at);
        assert_ne!(with_txid.updated_at, payment.updated_at);

        // sleep for a second to make sure the timestamps are different
        std::thread::sleep(Duration::from_secs(1));

        OnChainReceive::mark_as_confirmed(&mut conn, operation_id).unwrap();

        let confirmed = OnChainReceive::get_by_operation_id(&mut conn, operation_id)
            .unwrap()
            .unwrap();

        assert_eq!(confirmed.status(), PaymentStatus::Success);
        assert_eq!(confirmed.txid(), Some(Txid::all_zeros()));
        assert_eq!(with_txid.amount_sats, Some(amount as i64));
        assert_eq!(with_txid.fee_sats, Some(fee as i64));
        assert_ne!(confirmed.updated_at, confirmed.created_at);
        assert_ne!(confirmed.updated_at, with_txid.updated_at);
    }
}
