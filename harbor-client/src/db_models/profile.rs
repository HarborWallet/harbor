use crate::db_models::schema::profile;
use bip39::Mnemonic;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(
    QueryableByName, Queryable, AsChangeset, Serialize, Deserialize, Debug, Clone, PartialEq,
)]
#[diesel(table_name = profile)]
pub struct Profile {
    pub id: String,
    pub seed_words: String,
    onchain_receive_enabled: i32,
    tor_enabled: i32,
}

impl Profile {
    pub fn get_first(conn: &mut SqliteConnection) -> anyhow::Result<Option<Profile>> {
        Ok(profile::table.first::<Profile>(conn).optional()?)
    }

    pub fn set_onchain_receive_enabled(
        conn: &mut SqliteConnection,
        enabled: bool,
    ) -> anyhow::Result<()> {
        log::debug!(
            "Updating on-chain receive enabled setting in database to: {}",
            enabled
        );
        diesel::update(profile::table)
            .set(profile::onchain_receive_enabled.eq(i32::from(enabled)))
            .execute(conn)?;
        log::debug!("Successfully updated on-chain receive enabled setting in database");
        Ok(())
    }

    pub fn onchain_receive_enabled(&self) -> bool {
        self.onchain_receive_enabled == 1
    }

    pub fn set_tor_enabled(conn: &mut SqliteConnection, enabled: bool) -> anyhow::Result<()> {
        log::debug!("Updating Tor enabled setting in database to: {}", enabled);
        diesel::update(profile::table)
            .set(profile::tor_enabled.eq(i32::from(enabled)))
            .execute(conn)?;
        log::debug!("Successfully updated Tor enabled setting in database");
        Ok(())
    }

    pub fn mnemonic(&self) -> Mnemonic {
        Mnemonic::from_str(self.seed_words.as_str()).expect("valid mnemonic")
    }

    pub fn tor_enabled(&self) -> bool {
        self.tor_enabled == 1
    }
}

#[derive(Insertable)]
#[diesel(table_name = profile)]
pub struct NewProfile {
    pub id: String,
    pub seed_words: String,
}

impl From<&NewProfile> for Profile {
    fn from(new_profile: &NewProfile) -> Self {
        Profile {
            id: new_profile.id.clone(),
            seed_words: new_profile.seed_words.clone(),
            onchain_receive_enabled: 0,
            tor_enabled: 1,
        }
    }
}

impl NewProfile {
    pub fn insert(&self, conn: &mut SqliteConnection) -> anyhow::Result<Profile> {
        let _ = diesel::insert_into(profile::table)
            .values(self)
            .execute(conn)?;

        Ok(self.into())
    }
}
