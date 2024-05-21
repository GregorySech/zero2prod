-- PHC doesn't need a dedicated salt column!
ALTER TABLE users DROP COLUMN salt;
