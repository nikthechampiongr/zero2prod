pub mod configuration;
mod routes;
pub mod startup;
use std::net::TcpListener;

use configuration::DatabaseSettings;
use serde::Deserialize;
use sqlx::Connection;
use sqlx::Executor;

use crate::{configuration::get_configuration, startup::run};

#[derive(Deserialize)]
pub struct Subscription {
    name: String,
    email: String,
}

pub struct TestApp {
    pub address: String,
    pub db_pool: sqlx::PgPool,
}

pub async fn spawn_app() -> TestApp {
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = uuid::Uuid::new_v4().to_string();
    let db_pool = configure_database(&configuration.database).await;
    let listener = TcpListener::bind("localhost:0").expect("Failed to bind to a port");
    let port = listener.local_addr().unwrap().port();
    let server = run(listener, db_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);
    let address = format!("http://localhost:{port}");
    TestApp { address, db_pool }
}

async fn configure_database(config: &DatabaseSettings) -> sqlx::PgPool {
    let mut connection = sqlx::PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to postgres instance");
    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, config.database_name,).as_str())
        .await
        .expect("Failed to create new database");
    let db_pool = sqlx::PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to database");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");
    db_pool
}
