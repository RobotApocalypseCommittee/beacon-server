-- Your SQL goes here
CREATE EXTENSION pgcrypto;
ALTER TABLE users
    ALTER COLUMN id SET DEFAULT gen_random_uuid();