-- Your SQL goes here
CREATE TABLE messages (
    body bytea NOT NULL,
    dh_pubkey bytea,
    sender uuid REFERENCES users,
    recipient uuid REFERENCES users,
    chain_number integer NOT NULL,
    message_number integer NOT NULL,
    signature bytea NOT NULL,
    channel bigint NOT NULL,
    PRIMARY KEY (channel, chain_number, message_number)
)