use crate::authentication::reject_anonymous_users;
use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::admin::{get_newsletters, post_newsletters};
use crate::routes::{
    admin_dashboard, change_password, change_password_form, confirm, health_check, home, log_out,
    login, login_form, subscription,
};
use actix_session::SessionMiddleware;
use actix_session::storage::RedisSessionStore;
use actix_web::cookie::Key;
use actix_web::{
    App, HttpServer,
    dev::Server,
    web::{self, Data},
};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
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

        Ok(Self {
            port,
            server: run(
                listener,
                db_pool,
                email_client,
                configuration.application.base_url,
                configuration.application.hmac_secret,
                configuration.redis_uri,
            )
            .await?,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub struct ApplicationBaseUrl(pub String);

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

pub async fn run(
    address: std::net::TcpListener,
    db_pool: sqlx::PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    let db_pool = Data::new(db_pool);
    let email_client = Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let hmac_secret = Data::new(HmacSecret(hmac_secret));
    let secret_key = Key::from(hmac_secret.0.expose_secret().as_bytes());
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let message_store = actix_web_flash_messages::storage::CookieMessageStore::builder(
        actix_web::cookie::Key::from(hmac_secret.0.expose_secret().as_bytes()),
    )
    .build();
    let message_framework =
        actix_web_flash_messages::FlashMessagesFramework::builder(message_store).build();

    let server = HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscription", web::post().to(subscription))
            .route("/subscription/confirm", web::get().to(confirm))
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .service(
                actix_web::web::scope("/admin")
                    .wrap(actix_web::middleware::from_fn(reject_anonymous_users))
                    .route("/newsletters", web::get().to(get_newsletters))
                    .route("/newsletters", web::post().to(post_newsletters))
                    .route("/dashboard", web::get().to(admin_dashboard))
                    .route("/change_password", web::get().to(change_password_form))
                    .route("/change_password", web::post().to(change_password))
                    .route("/logout", web::post().to(log_out)),
            )
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(hmac_secret.clone())
    })
    .listen(address)?
    .run();
    Ok(server)
}

pub struct HmacSecret(pub Secret<String>);
