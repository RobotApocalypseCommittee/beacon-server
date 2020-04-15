-- Your SQL goes here
ALTER TABLE devices
    ALTER COLUMN owner DROP NOT NULL;

ALTER TABLE devices
    ALTER COLUMN id SET DEFAULT gen_random_uuid();