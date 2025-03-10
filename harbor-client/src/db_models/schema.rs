// @generated automatically by Diesel CLI.

diesel::table! {
    cashu_mint (mint_url) {
        mint_url -> Text,
        active -> Integer,
    }
}

diesel::table! {
    fedimint (id) {
        id -> Text,
        invite_code -> Text,
        value -> Binary,
        active -> Integer,
    }
}

diesel::table! {
    lightning_payments (operation_id) {
        operation_id -> Text,
        fedimint_id -> Nullable<Text>,
        cashu_mint_url -> Nullable<Text>,
        payment_hash -> Text,
        bolt11 -> Text,
        amount_msats -> BigInt,
        fee_msats -> BigInt,
        preimage -> Nullable<Text>,
        status -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    lightning_receives (operation_id) {
        operation_id -> Text,
        fedimint_id -> Nullable<Text>,
        cashu_mint_url -> Nullable<Text>,
        payment_hash -> Text,
        bolt11 -> Text,
        amount_msats -> BigInt,
        fee_msats -> BigInt,
        preimage -> Text,
        status -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    mint_metadata (id) {
        id -> Text,
        name -> Nullable<Text>,
        welcome_message -> Nullable<Text>,
        federation_expiry_timestamp -> Nullable<Timestamp>,
        preview_message -> Nullable<Text>,
        popup_end_timestamp -> Nullable<Timestamp>,
        popup_countdown_message -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    on_chain_payments (operation_id) {
        operation_id -> Text,
        fedimint_id -> Nullable<Text>,
        cashu_mint_url -> Nullable<Text>,
        address -> Text,
        amount_sats -> BigInt,
        fee_sats -> BigInt,
        txid -> Nullable<Text>,
        status -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    on_chain_receives (operation_id) {
        operation_id -> Text,
        fedimint_id -> Nullable<Text>,
        cashu_mint_url -> Nullable<Text>,
        address -> Text,
        amount_sats -> Nullable<BigInt>,
        fee_sats -> Nullable<BigInt>,
        txid -> Nullable<Text>,
        status -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    profile (id) {
        id -> Text,
        seed_words -> Text,
        onchain_receive_enabled -> Integer,
        tor_enabled -> Integer,
    }
}

diesel::joinable!(lightning_payments -> cashu_mint (cashu_mint_url));
diesel::joinable!(lightning_payments -> fedimint (fedimint_id));
diesel::joinable!(lightning_receives -> cashu_mint (cashu_mint_url));
diesel::joinable!(lightning_receives -> fedimint (fedimint_id));
diesel::joinable!(on_chain_payments -> cashu_mint (cashu_mint_url));
diesel::joinable!(on_chain_payments -> fedimint (fedimint_id));
diesel::joinable!(on_chain_receives -> cashu_mint (cashu_mint_url));
diesel::joinable!(on_chain_receives -> fedimint (fedimint_id));

diesel::allow_tables_to_appear_in_same_query!(
    cashu_mint,
    fedimint,
    lightning_payments,
    lightning_receives,
    mint_metadata,
    on_chain_payments,
    on_chain_receives,
    profile,
);
