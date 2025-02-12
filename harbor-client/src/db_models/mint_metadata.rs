#![allow(deprecated)]

use crate::db_models::schema::mint_metadata;
use crate::metadata::FederationMeta;
use diesel::prelude::*;
use fedimint_core::config::FederationId;

#[derive(QueryableByName, Queryable, AsChangeset, Debug, Clone, PartialEq)]
#[diesel(table_name = mint_metadata)]
pub struct MintMetadata {
    pub id: String,
    pub name: Option<String>,
    pub welcome_message: Option<String>,
    pub federation_expiry_timestamp: Option<chrono::NaiveDateTime>,
    pub preview_message: Option<String>,
    pub popup_end_timestamp: Option<chrono::NaiveDateTime>,
    pub popup_countdown_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl MintMetadata {
    pub fn from(id: FederationId, meta: FederationMeta) -> Self {
        MintMetadata {
            id: id.to_string(),
            federation_expiry_timestamp: meta
                .federation_expiry_timestamp()
                .map(|s| chrono::NaiveDateTime::from_timestamp(s as i64, 0)),
            popup_end_timestamp: meta
                .popup_end_timestamp()
                .map(|s| chrono::NaiveDateTime::from_timestamp(s as i64, 0)),
            name: meta.federation_name,
            welcome_message: meta.welcome_message,
            preview_message: meta.preview_message,
            popup_countdown_message: meta.popup_countdown_message,
            created_at: Default::default(),
            updated_at: Default::default(),
        }
    }

    pub fn get(conn: &mut SqliteConnection, id: String) -> anyhow::Result<Option<MintMetadata>> {
        Ok(mint_metadata::table
            .filter(mint_metadata::id.eq(id))
            .first::<MintMetadata>(conn)
            .optional()?)
    }

    pub fn upsert(&self, conn: &mut SqliteConnection) -> anyhow::Result<()> {
        diesel::insert_into(mint_metadata::table)
            .values((
                mint_metadata::id.eq(&self.id),
                mint_metadata::name.eq(&self.name),
                mint_metadata::welcome_message.eq(&self.welcome_message),
                mint_metadata::federation_expiry_timestamp.eq(&self.federation_expiry_timestamp),
                mint_metadata::preview_message.eq(&self.preview_message),
                mint_metadata::popup_end_timestamp.eq(&self.popup_end_timestamp),
                mint_metadata::popup_countdown_message.eq(&self.popup_countdown_message),
            ))
            .on_conflict(mint_metadata::id)
            .do_update()
            .set((
                mint_metadata::name.eq(&self.name),
                mint_metadata::welcome_message.eq(&self.welcome_message),
                mint_metadata::federation_expiry_timestamp.eq(&self.federation_expiry_timestamp),
                mint_metadata::preview_message.eq(&self.preview_message),
                mint_metadata::popup_end_timestamp.eq(&self.popup_end_timestamp),
                mint_metadata::popup_countdown_message.eq(&self.popup_countdown_message),
            ))
            .execute(conn)?;

        Ok(())
    }
}

impl From<MintMetadata> for FederationMeta {
    fn from(value: MintMetadata) -> FederationMeta {
        FederationMeta {
            federation_name: value.name,
            federation_expiry_timestamp: value.federation_expiry_timestamp.map(|f| f.to_string()),
            welcome_message: value.welcome_message,
            vetted_gateways: None,
            federation_icon_url: None,
            meta_external_url: None,
            preview_message: value.preview_message,
            popup_end_timestamp: value.popup_end_timestamp.map(|f| f.to_string()),
            popup_countdown_message: value.popup_countdown_message,
        }
    }
}
