-- Create users table for authentication
-- I know the book wants to start you off with plain text password to learn and
-- improve but no. I will just start with a hashed one and go on from there.
CREATE TABLE Users(
    user_id uuid PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
)

