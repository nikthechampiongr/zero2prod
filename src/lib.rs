pub mod configuration;
pub mod domain;
pub mod routes;
pub mod startup;
pub mod telemetry;
pub mod email_client;

#[derive(serde::Deserialize)]
pub struct Subscription {
    name: String,
    email: String,
}
