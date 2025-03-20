use crate::db_models::schema::cashu_mint;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Insertable,
    QueryableByName,
    Queryable,
    AsChangeset,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
)]
#[diesel(table_name = cashu_mint)]
pub struct CashuMint {
    pub mint_url: String,
    pub active: i32,
}

impl CashuMint {
    pub fn get(conn: &mut SqliteConnection, url: String) -> anyhow::Result<Option<CashuMint>> {
        Ok(cashu_mint::table
            .filter(cashu_mint::mint_url.eq(url))
            .first::<CashuMint>(conn)
            .optional()?)
    }

    pub fn remove_mint(conn: &mut SqliteConnection, url: String) -> anyhow::Result<()> {
        // First check if the federation exists and is active
        let exists = cashu_mint::table
            .filter(cashu_mint::mint_url.eq(&url))
            .filter(cashu_mint::active.eq(1))
            .first::<CashuMint>(conn)
            .optional()?
            .is_some();

        if !exists {
            return Err(anyhow::anyhow!("Mint not found or already inactive"));
        }

        // Mark the federation as inactive
        diesel::update(cashu_mint::table)
            .filter(cashu_mint::mint_url.eq(&url))
            .set(cashu_mint::active.eq(0))
            .execute(conn)?;

        Ok(())
    }

    pub fn set_active(conn: &mut SqliteConnection, url: &String) -> anyhow::Result<()> {
        diesel::update(cashu_mint::table)
            .filter(cashu_mint::mint_url.eq(url))
            .set(cashu_mint::active.eq(1))
            .execute(conn)?;
        Ok(())
    }

    pub fn get_mints(conn: &mut SqliteConnection) -> anyhow::Result<Vec<String>> {
        Ok(cashu_mint::table
            .filter(cashu_mint::active.eq(1))
            .load::<Self>(conn)?
            .into_iter()
            .map(|f| f.mint_url)
            .collect())
    }

    pub fn get_archived_mints(conn: &mut SqliteConnection) -> anyhow::Result<Vec<String>> {
        Ok(cashu_mint::table
            .filter(cashu_mint::active.eq(0))
            .load::<Self>(conn)?
            .into_iter()
            .map(|f| f.mint_url)
            .collect())
    }

    pub fn insert(conn: &mut SqliteConnection, mint_url: String) -> anyhow::Result<()> {
        // First check if the federation exists and is active
        let exists = cashu_mint::table
            .filter(cashu_mint::mint_url.eq(&mint_url))
            .first::<CashuMint>(conn)
            .optional()?
            .is_some();

        if exists {
            Self::set_active(conn, &mint_url)?;
        }

        let mint = CashuMint {
            mint_url,
            active: 1,
        };

        diesel::insert_into(cashu_mint::table)
            .values(mint)
            .on_conflict_do_nothing()
            .execute(conn)?;

        Ok(())
    }
}
