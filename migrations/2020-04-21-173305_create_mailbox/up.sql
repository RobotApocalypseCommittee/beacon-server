-- Your SQL goes here
CREATE TABLE mailbox (
    device_id uuid NOT NULL references devices,
    message_id uuid NOT NULL references messages,
    id serial PRIMARY KEY
)