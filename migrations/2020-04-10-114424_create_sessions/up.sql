-- Your SQL goes here
CREATE TABLE sessions (
    id SERIAL PRIMARY KEY,
    nonce bytea NOT NULL UNIQUE,
    expires timestamp NOT NULL
)