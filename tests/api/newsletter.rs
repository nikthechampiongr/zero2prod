use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

#[actix_web::test]
async fn newsletter_returns_400_for_bad_data() {
    let app = spawn_app().await;

    let test_cases = [
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>" ,
                }
            }),
            "Missing title",
        ),
        (
            serde_json::json!({
            "title": "Newsletter title"}),
            "Missing content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "content": {
                    "html": "<p>Newsletter body as HTML</p>" ,
                }
            }),
            "Missing text",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "content": {
                    "text": "Newsletter body as plain text",
                }
            }),
            "Missing html",
        ),
    ];

    for (invalid_body, reason) in test_cases {
        let response = app.post_newsletter(invalid_body).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API payload did not return 400 Bad Request when Payload was: {}",
            reason
        );
    }
}

#[actix_web::test]
async fn newsletters_are_not_delivered_for_unconfirmed_subscribers() {
    let app = spawn_app().await;

    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    }
    });

    let response = app.post_newsletter(newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 200);
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

    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    }
    });

    let response = app.post_newsletter(newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
async fn requests_missing_authentication_header_are_rejected() {
    let app = spawn_app().await;
    let body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    }
    });

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}

#[actix_web::test]
async fn non_existing_users_are_rejected() {
    let app = spawn_app().await;
    let body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    }
    });

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .json(&body)
        // All users generated for tests are v4 UUIDs so something like this should never be
        // generated
        .basic_auth("NonExisting", None::<String>)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 401);
}

#[actix_web::test]
async fn invalid_passwords_are_rejected() {
    let app = spawn_app().await;
    let body = serde_json::json!({
    "title": "Newsletter title",
    "content": {
    "text": "Newsletter body as plain text",
    "html": "<p>Newsletter body as HTML</p>" ,
    }
    });

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .json(&body)
        // All passwords generated for tests are v4 UUIDs so something like this should never be
        // generated
        .basic_auth(app.test_user.username, Some("Invalid password"))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 401);
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
