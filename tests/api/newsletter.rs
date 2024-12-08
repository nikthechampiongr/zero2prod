use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};

#[actix_web::test]
async fn newsletters_are_not_delivered_for_unconfirmed_subscribers() {
    let app = spawn_app().await;

    create_unconfirmed_subscriber(&app).await;

    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password
    });

    let response = app.post_login(login_body).await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    });

    let response = app.post_newsletter(newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
async fn requests_from_unauthenticated_users_are_rejected() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    });

    let response = app.post_newsletter(newsletter_request_body).await;

    assert_is_redirect_to(&response, "/login");
}

#[actix_web::test]
async fn newsletters_get_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password
    });

    let response = app.post_login(login_body).await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    });

    let response = app.post_newsletter(newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200);
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _guard = Mock::given(path("/email"))
        .and(method("POST"))
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
