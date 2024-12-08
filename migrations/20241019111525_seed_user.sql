-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES (
    '32e6df6c-b27c-4445-a108-497867b9aa83',
    'admin',
    '$argon2id$v=19$m=19456,t=2,p=1$WU5JT2lhMG1OVXFnUm5IUw$1pInWTCJhvwKLjV/l86b8kP737mSZE4iA4k1k3oYIZk'
)
