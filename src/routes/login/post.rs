use crate::authentication::{AuthError, Credentials};
use crate::routes::error_chain_fmt;
use crate::startup::HmacSecret;
use actix_web::error::InternalError;
use actix_web::HttpResponse;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::PgPool;

#[tracing::instrument(name = "Login", skip(form, pg_pool, secret), fields(username=tracing::field::Empty, user_id=tracing::field::Empty) )]
pub async fn login(
    form: actix_web::web::Form<LoginForm>,
    pg_pool: actix_web::web::Data<PgPool>,
    secret: actix_web::web::Data<HmacSecret>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    tracing::Span::current().record("username", tracing::field::display(&form.username));
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    match crate::authentication::validate_credentials(&pg_pool, credentials).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((actix_web::http::header::LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::AuthError(e) => LoginError::AuthError(e),
                AuthError::UnexpectedError(e) => LoginError::UnknownError(e),
            };
            let encoded_error = format!("error={}", urlencoding::Encoded::new(e.to_string()));
            let hmac_tag = {
                let mut mac =
                    Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes())
                        .unwrap();
                mac.update(encoded_error.to_string().as_bytes());
                mac.finalize().into_bytes()
            };

            let response = HttpResponse::SeeOther()
                .insert_header((
                    actix_web::http::header::LOCATION,
                    format!("/login?{encoded_error}&tag={hmac_tag:x}"),
                ))
                .finish();
            Err(InternalError::from_response(e, response))
        }
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Invalid login crdentials")]
    AuthError(#[source] anyhow::Error),
    #[error("An unexpected error occurred while trying to authenticate")]
    UnknownError(#[source] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    username: String,
    password: Secret<String>,
}
