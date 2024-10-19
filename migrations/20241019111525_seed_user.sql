-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES (
    '32e6df6c-b27c-4445-a108-497867b9aa83',
    'admin',
    '$argon2id$v=19$m=19456,t=2,p=1$dE1kU3dMVnpaazc3NW5WOQ$X1/jO+R3k7kosXVO7x8EjXGdECW+Zqmw530ZCFCpGq0'
)
