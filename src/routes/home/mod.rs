use actix_web::HttpResponse;

pub async fn home() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(actix_web::http::header::ContentType::html())
        .body(include_str!("home.html"))
}
