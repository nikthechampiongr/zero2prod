use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};
use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid email address");

        let timeout = configuration.email_client.timeout();

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let db_pool = PgPoolOptions::new()
            .idle_timeout(std::time::Duration::from_secs(2))
            .connect_lazy_with(configuration.database.with_db());

        let listener = std::net::TcpListener::bind(address).expect("Failed to bind to port");
        let port = listener.local_addr().unwrap().port();

        Ok(Self {port, server: run(listener, db_pool, email_client)? })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool (configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

pub fn run(
    address: std::net::TcpListener,
    db_pool: sqlx::PgPool,
    email_client: EmailClient,
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
