use crate::{domain::NewSubscriber, startup::ApplicationBaseUrl};
use rand::{distributions::Alphanumeric, Rng};
use uuid::Uuid;

use actix_web::{
    web::{self, Form},
    HttpResponse,
};

use crate::email_client::EmailClient;
use crate::Subscription;
use tracing::error;

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
) -> HttpResponse {
    let sub = match form.try_into() {
        Ok(sub) => sub,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let subscriber_id = match insert_subscriber(db.as_ref(), &sub).await {
        Ok(uuid) => uuid,
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    };

    let token = get_subscription_token();

    if store_token(db.as_ref(), subscriber_id, &token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if send_email(&email_client, sub, &base_url.0, &token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Save new subscriber to database", skip(db, form))]
async fn insert_subscriber(
    db: &sqlx::PgPool,
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
    .execute(db)
    .await
    .map_err(|e| {
        error!("Failed to insert subscriber to database: {e:?}");
        e
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
        .send_email(sub.email, "welcome!", &html_body, &plain_body)
        .await
}

#[tracing::instrument(name = "Store subscription token in the database", skip(token))]
async fn store_token(
    pool: &sqlx::PgPool,
    subscriber_id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscription_tokens(subscriber_id, subscription_token) VALUES($1, $2)",
        subscriber_id,
        token
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;

    Ok(())
}
