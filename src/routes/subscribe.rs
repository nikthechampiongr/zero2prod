use uuid::Uuid;

use actix_web::{
    web::{self, Form},
    HttpResponse,
};

use crate::Subscription;

pub async fn subscribe(
    Form(form): Form<Subscription>,
    db: web::Data<sqlx::PgPool>,
) -> HttpResponse {
    match sqlx::query!(
        r#"INSERT INTO subscriptions(id,email,name,subscribed_at) 
                 VALUES($1, $2, $3,$4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        chrono::Utc::now()
    )
    .execute(db.as_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            println!("Failed to execute query: {e}");
            actix_web::HttpResponse::InternalServerError().finish()
        }
    }
}
