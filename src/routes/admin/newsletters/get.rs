use std::fmt::Write;

use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

pub async fn get_newsletters(received: IncomingFlashMessages) -> HttpResponse {
    let mut messages = String::new();
    for msg in received.iter() {
        // This should never throw an error
        write!(&mut messages, "{}", msg.content()).unwrap();
    }
    HttpResponse::Ok()
        .content_type(actix_web::http::header::ContentType::html())
        .body(format!(
            include_str!("newsletters.html"),
            messages,
            uuid::Uuid::new_v4() // idempotency key
        ))
}
