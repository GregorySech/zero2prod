-- For their own good.
ALTER TABLE users ADD COLUMN salt TEXT NOT NULL;
