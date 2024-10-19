use actix_web::HttpResponse;
use actix_web_flash_messages::{IncomingFlashMessages, Level};

pub async fn login_form(received: IncomingFlashMessages) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(actix_web::http::header::ContentType::html())
        .body(format!(
            include_str!("login.html"),
            received
                .iter()
                .filter(|p| p.level() == Level::Error)
                .map(|c| c.content())
                .next()
                .unwrap_or_default()
        ))
}
