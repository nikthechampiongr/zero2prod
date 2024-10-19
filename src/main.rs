use zero2prod::startup::Application;
use zero2prod::{configuration::get_configuration, telemetry::*};

#[actix_web::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    Application::build(configuration)
        .await?
        .run_until_stopped()
        .await?;
    Ok(())
}
