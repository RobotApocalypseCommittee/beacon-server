-- Your SQL goes here
CREATE TABLE users (
    id UUID PRIMARY KEY,
    pubkey bytea NOT NULL,
    bio text,
    profile_thumb bytea,
    email text UNIQUE NOT NULL,
    email_verified bool NOT NULL DEFAULT false,
    nickname text,
    last_seen timestamp
)