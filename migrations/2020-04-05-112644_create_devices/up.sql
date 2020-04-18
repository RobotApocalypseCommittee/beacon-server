-- Your SQL goes here
CREATE TABLE devices (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid REFERENCES users,
    missed_messages integer NOT NULL DEFAULT 0,
    public_key bytea NOT NULL
)