use zero2prod::{configuration::get_configuration, startup::run};

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let address = format!("localhost:{}", configuration.application_port);
    let connection_string = configuration.database.connection_string();
    let db_pool = sqlx::PgPool::connect(&connection_string)
        .await
        .expect("Failed to connect to database");
    run(
        std::net::TcpListener::bind(address).expect("Failed to bind to port 8000"),
        db_pool,
    )?
    .await?;
    Ok(())
}
