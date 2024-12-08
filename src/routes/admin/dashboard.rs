use actix_web::{http::header::ContentType, HttpResponse};

use sqlx::PgPool;

use crate::{
    authentication::UserId,
    util::{e500, get_username},
};

pub async fn admin_dashboard(
    db_pool: actix_web::web::Data<PgPool>,
    user_id: actix_web::web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(db_pool.as_ref(), *user_id)
        .await
        .map_err(e500)?;
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("dashboard.html"), username)))
}
