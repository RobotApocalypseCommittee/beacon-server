-- Your SQL goes here
CREATE TABLE devices (
    owner uuid NOT NULL REFERENCES users,
    id uuid PRIMARY KEY,
    missed_messages integer NOT NULL DEFAULT 0,
    public_key bytea NOT NULL
)