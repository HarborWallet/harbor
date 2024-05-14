// @generated automatically by Diesel CLI.

diesel::table! {
    fedimint (id) {
        id -> Text,
        value -> Binary,
    }
}

diesel::table! {
    profile (id) {
        id -> Text,
        seed_words -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    fedimint,
    profile,
);
