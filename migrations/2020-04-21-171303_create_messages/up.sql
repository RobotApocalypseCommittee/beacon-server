-- Your SQL goes here
CREATE TABLE messages (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    recipient uuid NOT NULL references users,
    sender uuid NOT NULL references users,
    reception_time timestamptz NOT NULL DEFAULT now(),
    message_type text NOT NULL,
    payload json NOT NULL
)