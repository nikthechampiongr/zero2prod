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
    name = "Adding a new subscriber",
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
    if  insert_subscriber(db.as_ref(), &sub).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if email_client
        .send_email(
            sub.email,
            "welcome!",
            "welcome to our newsletter!",
            "welcome to our newsletter!",
        )
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Saving new subscriber to database", skip(db, form))]
async fn insert_subscriber(
    db: &sqlx::PgPool,
    form: &NewSubscriber,
) -> Result<(), sqlx::error::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions(id,email,name,subscribed_at, status)
                 VALUES($1, $2, $3,$4, 'confirmed')"#,
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
