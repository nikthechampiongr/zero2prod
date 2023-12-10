use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
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
    let db_pool = PgPoolOptions::new()
        .idle_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(configuration.database.connection_string().expose_secret())
        .expect("Failed to create database connection");
    run(
        std::net::TcpListener::bind(address).expect("Failed to bind to port"),
        db_pool,
    )?
    .await?;
    Ok(())
}
