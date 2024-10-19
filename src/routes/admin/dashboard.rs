use actix_web::{
    http::header::{ContentType, LOCATION},
    HttpResponse,
};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::session_state::TypedSession;

pub async fn admin_dashboard(
    session: TypedSession,
    db_pool: actix_web::web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(uuid) = session.get_user_id()? {
        get_username(db_pool.as_ref(), uuid).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("dashboard.html"), username)))
}

// Return 500 with the error preserved
fn e500<T: std::fmt::Display + std::fmt::Debug + 'static>(e: T) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(e)
}

#[tracing::instrument(name = "Get username", skip(db_pool))]
async fn get_username(db_pool: &PgPool, uuid: Uuid) -> Result<String, anyhow::Error> {
    let name = sqlx::query!("SELECT username FROM users WHERE user_id = $1", uuid)
        .fetch_one(db_pool)
        .await
        .context("Failed to perform a query to fetch a username from the databse")?;

    Ok(name.username)
}
