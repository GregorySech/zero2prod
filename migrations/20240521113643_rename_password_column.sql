-- Let's store the password hash instead of a clear-text password.
ALTER TABLE users RENAME password TO password_hash;
