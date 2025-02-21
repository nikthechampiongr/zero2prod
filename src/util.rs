use actix_web::HttpResponse;
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

// Return 500 with the error preserved
pub fn e500<T: std::fmt::Display + std::fmt::Debug + 'static>(e: T) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(e)
}

// Returns 400 for a validation error with the human readable error as the body.
pub fn e400<T: std::fmt::Display + std::fmt::Debug + 'static>(e: T) -> actix_web::Error {
    actix_web::error::ErrorBadRequest(e)
}

pub fn see_other(path: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((actix_web::http::header::LOCATION, path))
        .finish()
}

#[tracing::instrument(name = "Get username", skip(db_pool))]
pub async fn get_username(db_pool: &PgPool, uuid: Uuid) -> Result<String, anyhow::Error> {
    let name = sqlx::query!("SELECT username FROM users WHERE user_id = $1", uuid)
        .fetch_one(db_pool)
        .await
        .context("Failed to perform a query to fetch a username from the databse")?;

    Ok(name.username)
}
