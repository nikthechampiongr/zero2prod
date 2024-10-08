use actix_web::HttpResponse;
use hmac::Mac;
use secrecy::ExposeSecret;

use crate::startup::HmacSecret;

pub async fn login_form(
    query_params: Option<actix_web::web::Query<QueryParams>>,
    secret: actix_web::web::Data<HmacSecret>,
) -> HttpResponse {
    let error_message = match query_params {
        Some(query_params) => match query_params.0.verify(&secret) {
            Ok(error) => htmlescape::encode_minimal(&error),
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify query parameters"
                );
                "".to_string()
            }
        },
        None => "".to_string(),
    };
    HttpResponse::Ok()
        .content_type(actix_web::http::header::ContentType::html())
        .body(format!(include_str!("login.html"), error_message))
}

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    pub fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac =
            hmac::Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes())
                .unwrap();
        mac.update(query_string.as_bytes());
        mac.verify(tag.as_slice().into())?;
        Ok(self.error)
    }
}
