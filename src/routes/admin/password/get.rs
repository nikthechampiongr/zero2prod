use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;

use crate::authentication::UserId;

pub async fn change_password_form(
    received: IncomingFlashMessages,
    _: actix_web::web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            include_str!("password_form.html"),
            received
                .iter()
                .filter(|p| p.level() == actix_web_flash_messages::Level::Error)
                .map(|c| c.content())
                .next()
                .unwrap_or_default()
        )))
}
