-- Your SQL goes here
CREATE EXTENSION pgcrypto;
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identity_key bytea NOT NULL,
    signed_prekey bytea NOT NULL,
    prekey_signature bytea NOT NULL,
    bio text,
    profile_thumb bytea,
    email text UNIQUE NOT NULL,
    email_verified bool NOT NULL DEFAULT false,
    nickname text,
    last_seen timestamp
)