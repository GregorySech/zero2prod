-- Seeding the first admin user!
-- username: admin
-- password: admin_password
INSERT INTO users (user_id, username, password_hash)
VALUES (
    '169cfc7c-74c9-4ae1-870f-8ea0397d2c8f',
    'admin',
    '$argon2id$v=19$m=15000,t=2,p=1$ebHcL7T2mvxGsBWVOGmCsw$3gYd6cCSiEU9wuhVZ5YmVK+pzh3pMqGgNCp3KyaeW7Y'
);