use actix_web::{
    web::{self, Query},
    HttpResponse,
};
use sqlx::{query, PgPool};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a subscription", skip(parameters))]
pub async fn confirm(parameters: Query<Parameters>, pool: web::Data<sqlx::PgPool>) -> HttpResponse {
    let subscriber_id =
        match get_subscriber_id_from_token(pool.as_ref(), &parameters.subscription_token).await {
            Ok(Some(uuid)) => uuid,
            Ok(None) => return HttpResponse::Unauthorized().finish(),
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    if let Err(_) = confirm_subscriber(pool.as_ref(), subscriber_id).await {
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Confirm subscriber", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let subscription = query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;

    Ok(subscription.map(|r| r.subscriber_id))
}
