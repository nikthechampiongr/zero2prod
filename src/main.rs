use std::fmt::Debug;
use std::fmt::Display;

use tokio::task::JoinError;
use zero2prod::idempotency;
use zero2prod::issue_delivery_workers::run_workers_until_stopped;
use zero2prod::startup::Application;
use zero2prod::{configuration::get_configuration, telemetry::*};

#[actix_web::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let email_worker = run_workers_until_stopped(configuration.clone());
    let idempotency_cleanup_worker =
        idempotency::expiry_workers::run_until_stopped(configuration.clone());
    let app = Application::build(configuration).await?.run_until_stopped();
    let app = tokio::spawn(app);
    let email_worker = tokio::spawn(email_worker);
    let idempotency_cleanup_worker = tokio::spawn(idempotency_cleanup_worker);

    tokio::select! {
        o = app => report_exit("API", o),
        o = email_worker => report_exit("Email Background worker", o),
        o = idempotency_cleanup_worker => report_exit("Idempotency Cleanup Background Worker", o)
    }

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{task_name} has exited"),
        Ok(Err(e)) => {
            tracing::error!(error.cause_chain = ?e, error.message = %e, "{task_name} failed");
        }
        Err(e) => {
            tracing::error!(error.cause_chain = ?e, error.message = %e, "{task_name} failed to complete");
        }
    }
}
