use crate::db_models::schema::fedimint;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    QueryableByName, Queryable, AsChangeset, Serialize, Deserialize, Debug, Clone, PartialEq,
)]
#[diesel(table_name = fedimint)]
pub struct Fedimint {
    pub id: String,
    pub value: Vec<u8>,
}

impl Fedimint {
    pub fn get_value(conn: &mut SqliteConnection, id: String) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(fedimint::table
            .filter(fedimint::id.eq(id))
            .first::<Fedimint>(conn)
            .optional()?
            .map(|v| v.value))
    }

    pub fn update(&self, conn: &mut SqliteConnection) -> anyhow::Result<()> {
        let _ = diesel::update(fedimint::table)
            .filter(fedimint::id.eq(self.id.clone()))
            .set(fedimint::value.eq(self.value.clone()))
            .execute(conn)?;

        Ok(())
    }
}

#[derive(Insertable)]
#[diesel(table_name = fedimint)]
pub struct NewFedimint {
    pub id: String,
    pub value: Vec<u8>,
}

impl From<&NewFedimint> for Fedimint {
    fn from(new_fedimint: &NewFedimint) -> Self {
        Fedimint {
            id: new_fedimint.id.clone(),
            value: new_fedimint.value.clone(),
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
