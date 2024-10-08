use anyhow::{anyhow, Context};
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
                return Err(AuthError::AuthError(anyhow!("Invalid password")))
            }
            _ => return Err(anyhow::Error::new(e).into()),
        }
    }
    Ok(())
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
