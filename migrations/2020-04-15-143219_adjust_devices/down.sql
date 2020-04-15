-- This file should undo anything in `up.sql`
ALTER TABLE devices
    ALTER COLUMN owner SET NOT NULL;

ALTER TABLE devices
    ALTER COLUMN id DROP DEFAULT;