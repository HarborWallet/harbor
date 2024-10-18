use crate::db_models::schema::profile;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    QueryableByName, Queryable, AsChangeset, Serialize, Deserialize, Debug, Clone, PartialEq,
)]
#[diesel(table_name = profile)]
pub struct Profile {
    pub id: String,
    pub seed_words: String,
}

impl Profile {
    pub fn get_first(conn: &mut SqliteConnection) -> anyhow::Result<Option<Profile>> {
        Ok(profile::table.first::<Profile>(conn).optional()?)
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
