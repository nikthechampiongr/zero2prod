use crate::routes::{health_check, subscribe};
use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use tracing_actix_web::TracingLogger;

pub fn run(
    address: std::net::TcpListener,
    db_pool: sqlx::PgPool,
) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscribe", web::post().to(subscribe))
            .app_data(db_pool.clone())
    })
    .listen(address)?
    .run();
    Ok(server)
}
