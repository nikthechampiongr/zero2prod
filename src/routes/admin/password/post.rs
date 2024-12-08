use crate::authentication::{validate_credentials, AuthError, Credentials, UserId};
use crate::routes::error_chain_fmt;
use actix_web::error::InternalError;
use actix_web::{web::Form, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use unicode_segmentation::UnicodeSegmentation;

use crate::util::get_username;
use crate::util::see_other;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    confirm_password: Secret<String>,
}

#[tracing::instrument(name = "Change password", skip(form, user_id))]
pub async fn change_password(
    form: Form<FormData>,
    db_pool: actix_web::web::Data<PgPool>,
    user_id: actix_web::web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    if form.new_password.expose_secret() != form.confirm_password.expose_secret() {
        return Err(password_change_err(
            PasswordChangeError::MismatchedPasswords,
        ));
    };

    validate_password_strength(&form.current_password).map_err(password_change_err)?;

    let username = get_username(db_pool.as_ref(), *user_id)
        .await
        .map_err(|e| password_change_err(PasswordChangeError::UnexpectedError(e)))?;

    let creds = Credentials {
        username,
        password: form.0.current_password,
    };

    let uuid = match validate_credentials(db_pool.as_ref(), creds).await {
        Ok(uuid) => uuid,
        Err(AuthError::AuthError(e)) => {
            return Err(password_change_err(PasswordChangeError::AuthError(e)))
        }
        Err(AuthError::UnexpectedError(e)) => {
            return Err(password_change_err(PasswordChangeError::UnexpectedError(e)))
        }
    };

    crate::authentication::change_password(uuid, form.0.new_password, db_pool.as_ref())
        .await
        .map_err(|e| password_change_err(PasswordChangeError::UnexpectedError(e)))?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/change-password"))
}

fn validate_password_strength(password: &Secret<String>) -> Result<(), PasswordChangeError> {
    if password.expose_secret().graphemes(true).count() > 128 {
        return Err(PasswordChangeError::PasswordTooLong);
    }

    let combined_spaces_password = {
        let mut s = String::new();
        let mut last_whitespace = false;

        //OWASP wants spaces combined. Sure.
        for c in password.expose_secret().chars() {
            if !c.is_whitespace() {
                last_whitespace = false;
            } else {
                if last_whitespace {
                    continue;
                }
                last_whitespace = true;
            }
            s.push(c);
        }
        Secret::new(s)
    };

    if combined_spaces_password
        .expose_secret()
        .graphemes(true)
        .count()
        < 12
    {
        return Err(PasswordChangeError::PasswordTooShort);
    }
    Ok(())
}

fn password_change_err(e: PasswordChangeError) -> actix_web::Error {
    FlashMessage::error(e.to_string()).send();
    let response = see_other("/admin/change_password");
    InternalError::from_response(e, response).into()
}

#[derive(thiserror::Error)]
pub enum PasswordChangeError {
    #[error("The current password is incorrect.")]
    AuthError(#[source] anyhow::Error),
    #[error("An unexpected error occurred.")]
    UnexpectedError(#[source] anyhow::Error),
    #[error("You entered two different new passwords - the field values must match.")]
    MismatchedPasswords,
    #[error("The new password you entered must be at least 12 characters long.")]
    PasswordTooShort,
    #[error("The new password you entered must be at most 128 characters long.")]
    PasswordTooLong,
    #[error("The user has not logged in")]
    AnonymousUser,
}

impl std::fmt::Debug for PasswordChangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
