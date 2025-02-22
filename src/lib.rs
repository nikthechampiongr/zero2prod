pub mod authentication;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod idempotency;
pub mod issue_delivery_workers;
pub mod routes;
pub mod session_state;
pub mod startup;
pub mod telemetry;
pub mod util;

#[derive(serde::Deserialize)]
pub struct Subscription {
    name: String,
    email: String,
}
