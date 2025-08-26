use crate::db_models::schema::fedimint;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    QueryableByName, Queryable, AsChangeset, Serialize, Deserialize, Debug, Clone, PartialEq,
)]
#[diesel(table_name = fedimint)]
pub struct Fedimint {
    pub id: String,
    pub invite_code: String,
    pub value: Vec<u8>,
    pub active: i32,
}

impl Fedimint {
    pub fn get_value(conn: &mut SqliteConnection, id: String) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(fedimint::table
            .filter(fedimint::id.eq(id))
            .first::<Self>(conn)
            .optional()?
            .map(|v| v.value))
    }

    pub fn get(conn: &mut SqliteConnection, id: String) -> anyhow::Result<Option<Self>> {
        Ok(fedimint::table
            .filter(fedimint::id.eq(id))
            .first::<Self>(conn)
            .optional()?)
    }

    pub fn remove_federation(conn: &mut SqliteConnection, id: String) -> anyhow::Result<()> {
        // First check if the federation exists and is active
        let exists = fedimint::table
            .filter(fedimint::id.eq(&id))
            .filter(fedimint::active.eq(1))
            .first::<Self>(conn)
            .optional()?
            .is_some();

        if !exists {
            return Err(anyhow::anyhow!("Federation not found or already inactive"));
        }

        // Mark the federation as inactive
        diesel::update(fedimint::table)
            .filter(fedimint::id.eq(&id))
            .set(fedimint::active.eq(0))
            .execute(conn)?;

        Ok(())
    }

    pub fn set_active(conn: &mut SqliteConnection, id: String) -> anyhow::Result<()> {
        diesel::update(fedimint::table)
            .filter(fedimint::id.eq(id))
            .set(fedimint::active.eq(1))
            .execute(conn)?;
        Ok(())
    }

    pub fn get_ids(conn: &mut SqliteConnection) -> anyhow::Result<Vec<String>> {
        Ok(fedimint::table
            .filter(fedimint::active.eq(1))
            .load::<Self>(conn)?
            .into_iter()
            .map(|f| f.id)
            .collect())
    }

    pub fn get_archived_ids(conn: &mut SqliteConnection) -> anyhow::Result<Vec<String>> {
        Ok(fedimint::table
            .filter(fedimint::active.eq(0))
            .load::<Self>(conn)?
            .into_iter()
            .map(|f| f.id)
            .collect())
    }

    pub fn update_value(
        conn: &mut SqliteConnection,
        id: String,
        value: Vec<u8>,
    ) -> anyhow::Result<()> {
        let _ = diesel::update(fedimint::table)
            .filter(fedimint::id.eq(&id))
            .set((fedimint::value.eq(&value),))
            .execute(conn)?;

        Ok(())
    }
}

#[derive(Insertable, Clone)]
#[diesel(table_name = fedimint)]
pub struct NewFedimint {
    pub id: String,
    pub invite_code: String,
    pub value: Vec<u8>,
}

impl From<&NewFedimint> for Fedimint {
    fn from(new_fedimint: &NewFedimint) -> Self {
        Self {
            id: new_fedimint.id.clone(),
            invite_code: new_fedimint.invite_code.clone(),
            value: new_fedimint.value.clone(),
            active: 1,
        }
    }
}

impl NewFedimint {
    pub fn insert(&self, conn: &mut SqliteConnection) -> anyhow::Result<Fedimint> {
        let _ = diesel::insert_into(fedimint::table)
            .values(self)
            .execute(conn)?;

        Ok(self.into())
    }
}
