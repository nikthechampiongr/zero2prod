use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;

use crate::{
    session_state::TypedSession,
    util::{e500, see_other},
};

pub async fn change_password_form(
    session: TypedSession,
    received: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_some() {
        return Ok(HttpResponse::Ok()
            .content_type(ContentType::html())
            .body(format!(
                include_str!("password_form.html"),
                received
                    .iter()
                    .filter(|p| p.level() == actix_web_flash_messages::Level::Error)
                    .map(|c| c.content())
                    .next()
                    .unwrap_or_default()
            )));
    }

    Ok(see_other("/login"))
}
