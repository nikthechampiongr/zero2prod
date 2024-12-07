use actix_web::{http::header::ContentType, HttpResponse};

use sqlx::PgPool;

use crate::{
    session_state::TypedSession,
    util::{e500, get_username, see_other},
};

pub async fn admin_dashboard(
    session: TypedSession,
    db_pool: actix_web::web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(uuid) = session.get_user_id()? {
        get_username(db_pool.as_ref(), uuid).await.map_err(e500)?
    } else {
        return Ok(see_other("/login"));
    };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("dashboard.html"), username)))
}
