use crate::authentication::{AuthError, Credentials};
use crate::routes::error_chain_fmt;
use crate::session_state::TypedSession;
use actix_web::error::InternalError;
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use sqlx::PgPool;

#[tracing::instrument(name = "Login", skip(form, pg_pool, session), fields(username=tracing::field::Empty, user_id=tracing::field::Empty) )]
pub async fn login(
    form: actix_web::web::Form<LoginForm>,
    pg_pool: actix_web::web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    tracing::Span::current().record("username", tracing::field::display(&form.username));
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    match crate::authentication::validate_credentials(&pg_pool, credentials).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            session.renew();
            session
                .insert_user_id(user_id)
                .map_err(|e| login_err(LoginError::UnknownError(e.into())))?;
            Ok(HttpResponse::SeeOther()
                .insert_header((actix_web::http::header::LOCATION, "/admin/dashboard"))
                .finish())
        }
        Err(e) => match e {
            AuthError::AuthError(e) => Err(login_err(LoginError::AuthError(e))),
            AuthError::UnexpectedError(e) => Err(login_err(LoginError::UnknownError(e))),
        },
    }
}

fn login_err(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    let response = HttpResponse::SeeOther()
        .insert_header((actix_web::http::header::LOCATION, "/login"))
        .finish();
    tracing::error!("{response:?}");
    InternalError::from_response(e, response)
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Invalid login credentials")]
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
