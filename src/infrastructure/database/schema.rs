// @generated automatically by Diesel CLI.

diesel::table! {
    incoming_transactions (id) {
        id -> Int8,
        wallet_id -> Int8,
        #[max_length = 128]
        tx_hash -> Varchar,
        block_number -> Nullable<Int8>,
        #[max_length = 64]
        from_address -> Varchar,
        #[max_length = 64]
        to_address -> Varchar,
        amount -> Numeric,
        #[max_length = 16]
        status -> Varchar,
        error_message -> Nullable<Text>,
        detected_at -> Timestamptz,
        confirmed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    outgoing_transfers (id) {
        id -> Int8,
        from_wallet_id -> Int8,
        #[max_length = 64]
        to_address -> Varchar,
        amount -> Numeric,
        #[max_length = 16]
        status -> Varchar,
        #[max_length = 128]
        tx_hash -> Nullable<Varchar>,
        #[max_length = 128]
        reference_id -> Nullable<Varchar>,
        error_message -> Nullable<Text>,
        created_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    wallets (id) {
        id -> Int8,
        #[max_length = 64]
        address -> Varchar,
        #[max_length = 64]
        hex_address -> Varchar,
        #[max_length = 128]
        private_key -> Varchar,
        #[max_length = 255]
        owner_id -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(incoming_transactions -> wallets (wallet_id));
diesel::joinable!(outgoing_transfers -> wallets (from_wallet_id));

diesel::allow_tables_to_appear_in_same_query!(
    incoming_transactions,
    outgoing_transfers,
    wallets,
);
