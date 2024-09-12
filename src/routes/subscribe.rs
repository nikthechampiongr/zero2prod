use crate::domain::NewSubscriber;
use uuid::Uuid;

use actix_web::{
    web::{self, Form},
    HttpResponse,
};

use crate::email_client::EmailClient;
use crate::Subscription;
use tracing::error;

#[tracing::instrument(
    name = "Add a new subscriber",
    skip(form, db, email_client)
    fields(
           subscriber_email = %form.email,
           subscriber_name = %form.name
           )
)]
pub async fn subscribe(
    Form(form): Form<Subscription>,
    db: web::Data<sqlx::PgPool>,
    email_client: web::Data<EmailClient>,
) -> HttpResponse {
    let sub = match form.try_into() {
        Ok(sub) => sub,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    if insert_subscriber(db.as_ref(), &sub).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_email(&email_client, sub).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Save new subscriber to database", skip(db, form))]
async fn insert_subscriber(
    db: &sqlx::PgPool,
    form: &NewSubscriber,
) -> Result<(), sqlx::error::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions(id,email,name,subscribed_at, status)
                 VALUES($1, $2, $3,$4, 'pending_confirmation')"#,
        Uuid::new_v4(),
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
    Ok(())
}

#[tracing::instrument(
    name = "Send confirmation email to new subscriber",
    skip(email_client, sub)
)]
async fn send_email(email_client: &EmailClient, sub: NewSubscriber) -> Result<(), reqwest::Error> {
    let confirmation_link = "http://www.example.com/subscriptions/confirm";
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
