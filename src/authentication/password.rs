use anyhow::{Context, anyhow};
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_async;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid Credentials")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, db_pool))]
pub async fn validate_credentials(
    db_pool: &PgPool,
    credentials: Credentials,
) -> Result<uuid::Uuid, AuthError> {
    // This is just a random password hash so that we can still do work even if the user does not
    // exist.
    let mut password_hash = Secret::new("$argon2id$v=19$m=19456,t=2,p=1$vNXfNE0l0bV2e1R7vDhL8w$uvhiLTsidsTzLUUYFHJbjZ5AKEMDEySRhwVZFcehFWs".to_owned());
    let mut user_id = None;

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(db_pool, &credentials.username).await?
    {
        user_id = Some(stored_user_id);
        password_hash = stored_password_hash
    }

    spawn_blocking_with_async(|| verify_password_hash(password_hash, credentials.password))
        .await
        .context("Failed to spawn blocking task")??;

    // This should only be some if the username was found in the database.
    // We will never get to this point anyways unless the password given is somehow the random
    // password used above.
    user_id
        .ok_or_else(|| anyhow!("Invalid username"))
        .map_err(AuthError::AuthError)
}

#[tracing::instrument(name = "Verify password hash", skip(expected_hash, given_password))]
fn verify_password_hash(
    expected_hash: Secret<String>,
    given_password: Secret<String>,
) -> Result<(), AuthError> {
    let argon2 = argon2::Argon2::default();

    let hash = argon2::password_hash::PasswordHash::new(expected_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    if let Err(e) = argon2.verify_password(given_password.expose_secret().as_bytes(), &hash) {
        match e {
            argon2::password_hash::Error::Password => {
                return Err(AuthError::AuthError(anyhow!("Invalid password")));
            }
            _ => return Err(anyhow::Error::new(e).into()),
        }
    }
    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let argon2 = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(19 * 1024, 2, 1, None).unwrap(),
    );

    let salt = argon2::password_hash::SaltString::generate(rand::thread_rng());

    let hash = Secret::new(
        argon2
            .hash_password(password.expose_secret().as_bytes(), &salt)?
            .to_string(),
    );
    Ok(hash)
}

#[tracing::instrument(name = "Get stored crdentials", skip(username, db_pool))]
async fn get_stored_credentials(
    db_pool: &PgPool,
    username: &str,
) -> Result<Option<(Uuid, Secret<String>)>, AuthError> {
    let row = sqlx::query!(
        "SELECT user_id, password_hash FROM users WHERE username = $1",
        username,
    )
    .fetch_optional(db_pool)
    .await
    .context("Failed to retrieve stored credentials from database")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));

    Ok(row)
}

#[tracing::instrument(name = "Change password", skip(new_password, pool))]
pub async fn change_password(
    user_id: Uuid,
    new_password: Secret<String>,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let hash = spawn_blocking_with_async(move || compute_password_hash(new_password))
        .await?
        .context("Failed to has password")?;
    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE user_id = $2",
        hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database")?;
    Ok(())
}
