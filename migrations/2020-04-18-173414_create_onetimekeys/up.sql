-- Your SQL goes here
CREATE TABLE onetimekeys (
    id serial PRIMARY KEY,
    user_id uuid REFERENCES users,
    prekey bytea NOT NULL
)