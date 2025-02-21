use std::fmt::{Debug, Display};

use actix_web::web;
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;
use anyhow::anyhow;
use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::idempotency::save_response;
use crate::idempotency::try_processing;
use crate::idempotency::IdempotencyKey;
use crate::idempotency::NextAction;
use crate::util::e400;
use crate::util::e500;
use crate::util::get_username;
use crate::util::see_other;
use crate::{domain::SubscriberEmail, email_client::EmailClient};

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    html: String,
    text: String,
    idempotency_key: String,
}

#[tracing::instrument(
    name = "Publish newsletter to confirmed subscriber",
    skip(email_client, pg_pool, body),
    fields(username=tracing::field::Empty, userid=tracing::field::Empty)
)]
pub async fn post_newsletters(
    body: web::Form<BodyData>,
    email_client: web::Data<EmailClient>,
    pg_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    let BodyData {
        title,
        html,
        text,
        idempotency_key,
    } = body.0;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let transaction = match try_processing(&pg_pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(res) => {
            success();
            return Ok(res);
        }
    };

    let username = get_username(&pg_pool, *user_id).await.map_err(e500)?;
    tracing::Span::current().record("username", tracing::field::display(username));

    tracing::Span::current().record("user_id", tracing::field::display(*user_id));

    let confirmed_subscribers = get_confirmed_subscribers(pg_pool.as_ref())
        .await
        .map_err(e500)?;

    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(email) => {
                email_client
                    .send_email(&email.email, &title, &html, &text)
                    .await
                    .with_context(|| format!("Failed to send newsletter issue to {}", email))
                    .map_err(e500)?;
            }
            Err(e) => tracing::warn!(error.cause_chain = ?e,
                "Skipping a confirmed subscriber\
                Their stored contact information is invalid"),
        }
    }
    let response = see_other("/admin/newsletters");
    success();
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

pub fn success() {
    FlashMessage::info("The newsletter has been published!".to_string()).send();
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
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
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
