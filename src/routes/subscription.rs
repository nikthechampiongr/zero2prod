use std::fmt::{self, Debug};

use crate::{domain::NewSubscriber, startup::ApplicationBaseUrl};
use anyhow::Context;
use rand::{distributions::Alphanumeric, Rng};
use sqlx::Postgres;
use uuid::Uuid;

use actix_web::{
    http::StatusCode,
    web::{self, Form},
    HttpResponse, ResponseError,
};

use crate::email_client::EmailClient;
use crate::Subscription;

fn get_subscription_token() -> String {
    let mut rng = rand::thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Add a new subscriber",
    skip(form, db, email_client, base_url)
    fields(
           subscriber_email = %form.email,
           subscriber_name = %form.name
           )
)]
pub async fn subscription(
    Form(form): Form<Subscription>,
    db: web::Data<sqlx::PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let sub = form.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = db
        .as_ref()
        .begin()
        .await
        .context("Failed to get Postgres connection from Pool.")?;

    let subscriber_id = insert_subscriber(&mut transaction, &sub)
        .await
        .context("Failed to insert new subscriber into database.")?;

    let token = get_subscription_token();

    store_token(&mut transaction, subscriber_id, &token)
        .await
        .context("Failed to store confirmation token into database")?;

    transaction
        .commit()
        .await
        .context("Failed to commit new subscriber into database.")?;

    send_email(&email_client, sub, &base_url.0, &token)
        .await
        .context("Failed to send confirmation email to new subscriber.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Save new subscriber to database", skip(db, form))]
async fn insert_subscriber(
    db: &mut sqlx::Transaction<'_, Postgres>,
    form: &NewSubscriber,
) -> Result<Uuid, sqlx::error::Error> {
    let uuid = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO subscriptions(id,email,name,subscribed_at, status)
                 VALUES($1, $2, $3,$4, 'pending_confirmation')"#,
        uuid,
        form.email.as_ref(),
        form.name.as_ref(),
        chrono::Utc::now()
    )
    .execute(&mut **db)
    .await
    .inspect_err(|e| {
        tracing::error!("Failed to insert subscriber to database: {e:?}");
    })?;
    Ok(uuid)
}

#[tracing::instrument(
    name = "Send confirmation email to new subscriber",
    skip(email_client, sub)
)]
async fn send_email(
    email_client: &EmailClient,
    sub: NewSubscriber,
    base_url: &str,
    token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{base_url}/subscription/confirm?subscription_token={token}");

    let html_body = format!(
        "welcome to our newsletter!<br/> \
         Click <a href=\"{confirmation_link}\">"
    );
    let plain_body = format!(
        "welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription"
    );

    email_client
        .send_email(&sub.email, "welcome!", &html_body, &plain_body)
        .await
}

#[tracing::instrument(name = "Store subscription token in the database", skip(token))]
async fn store_token(
    pool: &mut sqlx::Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscription_tokens(subscriber_id, subscription_token) VALUES($1, $2)",
        subscriber_id,
        token
    )
    .execute(&mut **pool)
    .await?;

    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for SubscribeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        if let Self::ValidationError(_) = self {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;

    let mut current = e.source();

    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
