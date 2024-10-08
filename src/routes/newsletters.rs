use std::fmt::{Debug, Display};

use actix_web::{http::header::HeaderMap, HttpResponse};
use anyhow::anyhow;
use anyhow::Context;
use base64::Engine;
use reqwest::StatusCode;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::error;

use crate::authentication::validate_credentials;
use crate::authentication::AuthError;
use crate::authentication::Credentials;
use crate::{domain::SubscriberEmail, email_client::EmailClient};

use super::error_chain_fmt;

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[tracing::instrument(
    name = "Publish newsletter to confirmed subscriber",
    skip(email_client, pg_pool, body),
    fields(username=tracing::field::Empty, userid=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: actix_web::web::Json<BodyData>,
    email_client: actix_web::web::Data<EmailClient>,
    pg_pool: actix_web::web::Data<PgPool>,
    request: actix_web::HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    tracing::Span::current().record(
        "user_id",
        tracing::field::display(validate_credentials(&pg_pool, credentials).await.map_err(
            |e| match e {
                AuthError::AuthError(_) => PublishError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
            },
        )?),
    );

    let confirmed_subscribers = get_confirmed_subscribers(pg_pool.as_ref()).await?;

    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(email) => {
                email_client
                    .send_email(
                        &email.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| format!("Failed to send newsletter issue to {}", email))?;
            }
            Err(e) => tracing::warn!(error.cause_chain = ?e,
                "Skipping a confirmed subscriber\
                Their stored contact information is invalid"),
        }
    }
    Ok(HttpResponse::Ok().finish())
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let encoded_auth = headers
        .get("Authorization")
        .with_context(|| "The Authorization header was missing".to_string())?
        .to_str()
        .context("Authorization header content was not a valid UTF-8 string")?
        .strip_prefix("Basic ")
        .context("The Authorization scheme was not basic")?;

    let decoded = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(encoded_auth)
            .context("Could not decode base64 credentials")?,
    )
    .context("Decoded header content is not valid utf-8")?;

    let mut split = decoded.splitn(2, ':');

    let (username, password) = (
        split
            .next()
            .ok_or_else(|| anyhow!("Basic Auth must contain a username header"))?
            .to_string(),
        split
            .next()
            .ok_or_else(|| anyhow!("Basic auth must contain a password header"))?
            .to_string(),
    );

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

struct Row {
    email: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

impl Display for ConfirmedSubscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.email.fmt(f)
    }
}

#[tracing::instrument(name = "Get list of all confirmed subscribers", skip(pg_pool))]
async fn get_confirmed_subscribers(
    pg_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, PublishError> {
    let confirmed_subscribers = sqlx::query_as!(
        Row,
        "SELECT email FROM subscriptions WHERE status = 'confirmed'"
    )
    .fetch_all(pg_pool)
    .await
    .context("Failed to get confirmed subscribers from database")?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(e) => Err(anyhow!(e)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        if let Self::AuthError(_) = self {
            let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
            let header_value =
                actix_web::http::header::HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();

            response
                .headers_mut()
                .append(actix_web::http::header::WWW_AUTHENTICATE, header_value);

            response
        } else {
            HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
