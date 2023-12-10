use sqlx::{Connection, Executor};
use std::sync::OnceLock;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::run,
    telemetry,
};

pub struct TestApp {
    pub address: String,
    pub db_pool: sqlx::PgPool,
}

static TRACING: OnceLock<()> = OnceLock::new();

#[actix_web::test]
async fn health_check_works() {
    let address = spawn_app().await.address;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request.");
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

pub async fn spawn_app() -> TestApp {
    TRACING.get_or_init(|| {
        if std::env::var("TEST_LOG").is_ok() {
            let subscriber =
                telemetry::get_subscriber("test".into(), "debug".into(), std::io::stdout);
            telemetry::init_subscriber(subscriber);
        } else {
            let subscriber =
                telemetry::get_subscriber("test".into(), "debug".into(), std::io::sink);
            telemetry::init_subscriber(subscriber);
        }
    });

    let mut configuration = get_configuration().expect("Failed to read configuration.");

    configuration.database.database_name = uuid::Uuid::new_v4().to_string();
    let db_pool = configure_database(&configuration.database).await;

    let listener = std::net::TcpListener::bind("localhost:0").expect("Failed to bind to a port");
    let port = listener.local_addr().unwrap().port();
    let server = run(listener, db_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);

    let address = format!("http://localhost:{port}");
    TestApp { address, db_pool }
}

async fn configure_database(config: &DatabaseSettings) -> sqlx::PgPool {
    let mut connection = sqlx::PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to postgres instance");

    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, config.database_name,).as_str())
        .await
        .expect("Failed to create new database");

    let db_pool = sqlx::PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to database");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");
    db_pool
}
