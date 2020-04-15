-- This file should undo anything in `up.sql`
ALTER TABLE users
    ALTER COLUMN id DROP DEFAULT;
DROP EXTENSION pgcrypto;