use std::fmt::{Debug, Display};

use actix_web::HttpResponse;
use anyhow::Context;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;

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
    skip(email_client, pg_pool, body)
)]
pub async fn publish_newsletter(
    body: actix_web::web::Json<BodyData>,
    email_client: actix_web::web::Data<EmailClient>,
    pg_pool: actix_web::web::Data<PgPool>,
) -> Result<HttpResponse, PublishError> {
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
    .context("Failed to get confirmed subscribers from database")
    .map_err(PublishError::UnexpectedError)?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(e) => Err(anyhow::anyhow!(e)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
