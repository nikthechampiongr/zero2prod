use uuid::Uuid;

use actix_web::{
    web::{self, Form},
    HttpResponse,
};

use tracing::error;

use crate::Subscription;

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, db)
    fields(         
           subscriber_email = %form.email,
           subscriber_name = %form.name
           )
)]
pub async fn subscribe(
    Form(form): Form<Subscription>,
    db: web::Data<sqlx::PgPool>,
) -> HttpResponse {
    match insert_subscriber(db.as_ref(), form).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => actix_web::HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Saving new subscriber to databse", skip(db, form))]
async fn insert_subscriber(
    db: &sqlx::PgPool,
    form: Subscription,
) -> Result<(), sqlx::error::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions(id,email,name,subscribed_at)
                 VALUES($1, $2, $3,$4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
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
