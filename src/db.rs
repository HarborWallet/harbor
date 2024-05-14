use crate::db_models::{Fedimint, NewFedimint, NewProfile, Profile};
use diesel::{
    connection::SimpleConnection,
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::{sync::Arc, time::Duration};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub(crate) fn setup_db(url: &str, password: String) -> Arc<dyn DBConnection + Send + Sync> {
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
        .build(manager)
        .expect("Unable to build DB connection pool");
    Arc::new(SQLConnection { db: pool })
}

pub trait DBConnection {
    // Gets a seed from the first profile in the DB or returns None
    fn get_seed(&self) -> anyhow::Result<Option<String>>;

    // Inserts a new profile into the DB
    fn insert_new_profile(&self, new_profile: NewProfile) -> anyhow::Result<Profile>;

    // Inserts a new federation into the DB
    fn insert_new_federation(&self, f: NewFedimint) -> anyhow::Result<Fedimint>;

    // gets the federation data for a specific federation
    fn get_federation_value(&self, id: String) -> anyhow::Result<Option<Vec<u8>>>;

    // updates the federation data
    fn update_fedimint_data(&self, id: String, value: Vec<u8>) -> anyhow::Result<()>;
}

pub(crate) struct SQLConnection {
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

    fn insert_new_federation(&self, f: NewFedimint) -> anyhow::Result<Fedimint> {
        let conn = &mut self.db.get()?;
        f.insert(conn)
    }

    fn update_fedimint_data(&self, id: String, value: Vec<u8>) -> anyhow::Result<()> {
        let conn = &mut self.db.get()?;
        let f = Fedimint { id, value };
        f.update(conn)
    }
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
            // FIXME: Special characters might fuck up
            conn.batch_execute(&format!("PRAGMA key={}", self.key))?;
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
