use crate::routes::{health_check, subscribe};
use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use tracing_actix_web::TracingLogger;
use crate::email_client::EmailClient;

pub fn run(
    address: std::net::TcpListener,
    db_pool: sqlx::PgPool,
    email_client: EmailClient
) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let email_client = Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscribe", web::post().to(subscribe))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(address)?
    .run();
    Ok(server)
}
