table! {
    devices (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        missed_messages -> Int4,
        public_key -> Bytea,
    }
}

table! {
    mailbox (id) {
        device_id -> Uuid,
        message_id -> Uuid,
        id -> Int4,
    }
}

table! {
    messages (id) {
        id -> Uuid,
        recipient -> Uuid,
        sender -> Uuid,
        reception_time -> Timestamptz,
        message_type -> Text,
        payload -> Json,
    }
}

table! {
    onetimekeys (id) {
        id -> Int4,
        user_id -> Nullable<Uuid>,
        prekey -> Bytea,
    }
}

table! {
    sessions (id) {
        id -> Int4,
        nonce -> Bytea,
        expires -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Uuid,
        identity_key -> Bytea,
        signed_prekey -> Bytea,
        prekey_signature -> Bytea,
        bio -> Nullable<Text>,
        profile_thumb -> Nullable<Bytea>,
        email -> Text,
        email_verified -> Bool,
        nickname -> Nullable<Text>,
        last_seen -> Nullable<Timestamp>,
    }
}

joinable!(devices -> users (user_id));
joinable!(mailbox -> devices (device_id));
joinable!(mailbox -> messages (message_id));
joinable!(onetimekeys -> users (user_id));

allow_tables_to_appear_in_same_query!(
    devices,
    mailbox,
    messages,
    onetimekeys,
    sessions,
    users,
);
