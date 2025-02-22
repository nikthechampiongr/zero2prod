use std::time::Duration;

use fake::{
    Fake,
    faker::{internet::en::SafeEmail, name::en::Name},
};
use wiremock::{
    Mock, MockBuilder, ResponseTemplate,
    matchers::{any, method, path},
};

use crate::helpers::{ConfirmationLinks, TestApp, assert_is_redirect_to, spawn_app};

#[actix_web::test]
async fn newsletters_are_not_delivered_for_unconfirmed_subscribers() {
    let app = spawn_app().await;

    create_unconfirmed_subscriber(&app).await;

    app.test_user.login(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = app.post_newsletters(newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>The newsletter has been published!</i></p>"));
    app.dispatch_all_pending_emails().await;
}

#[actix_web::test]
async fn requests_from_unauthenticated_users_are_rejected() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = app.post_newsletters(newsletter_request_body).await;

    assert_is_redirect_to(&response, "/login");
    app.dispatch_all_pending_emails().await;
}

#[actix_web::test]
async fn newsletters_get_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.test_user.login(&app).await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>",
    "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = app.post_newsletters(newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>The newsletter has been published!</i></p>"));

    app.dispatch_all_pending_emails().await;
}

#[actix_web::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.test_user.login(&app).await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>The newsletter has been published!</i></p>"));

    // Do it again
    let response = app.post_newsletters(newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html = app.get_newsletters_html().await;
    assert!(html.contains("<p><i>The newsletter has been published!</i></p>"));

    app.dispatch_all_pending_emails().await;
    // Mock should verify that it has been sent more than once on drop.
}

#[actix_web::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.test_user.login(&app).await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response1 = app.post_newsletters(&newsletter_request_body);
    let response2 = app.post_newsletters(&newsletter_request_body);

    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );

    app.dispatch_all_pending_emails().await;

    // Mock should verify that it has been sent more than once on drop.
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();

    let _guard = when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .named("Create unconfirmed subscriber")
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.to_string())
        .await
        .error_for_status()
        .unwrap();

    let response = app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    ConfirmationLinks::get_confirmation_link(&response, app.port)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let links = create_unconfirmed_subscriber(app).await;

    reqwest::get(links.plain_link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}
