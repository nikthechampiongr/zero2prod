use sqlx::postgres::PgPoolOptions;
use zero2prod::email_client::EmailClient;
use zero2prod::{configuration::get_configuration, startup::run, telemetry::*};

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
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
        timeout
    );

    let db_pool = PgPoolOptions::new()
        .idle_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());
    run(
        std::net::TcpListener::bind(address).expect("Failed to bind to port"),
        db_pool,
        email_client,
    )?
    .await?;
    Ok(())
}
