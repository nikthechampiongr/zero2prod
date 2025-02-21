use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

use crate::helpers::{ConfirmationLinks, spawn_app};

#[actix_web::test]
async fn confirmation_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(format!("{}/subscription/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[actix_web::test]
async fn returned_confirmation_link_returns_200() {
    let app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.to_string()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let link = ConfirmationLinks::get_confirmation_link(email_request, app.port).plain_link;

    let response = reqwest::get(link).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
async fn clicking_confirmation_link_confirms_subscriber() {
    let app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.to_string()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let link = ConfirmationLinks::get_confirmation_link(email_request, app.port).plain_link;

    reqwest::get(link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let status = sqlx::query!("SELECT email,name,status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();

    assert_eq!(status.name, "le guin");
    assert_eq!(status.email, "ursula_le_guin@gmail.com");
    assert_eq!(status.status, "confirmed");
}
