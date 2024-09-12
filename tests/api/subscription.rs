use crate::helpers::spawn_app;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[actix_web::test]
async fn subscriptions_valid_request_ret200() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.to_string()).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[actix_web::test]
async fn subscriptions_invalid_request_ret400() {
    let test_cases = [
        ("email=ursula_le_guin%40gmail.com", "missing name field"),
        ("name=le%20guin", "missing email field"),
        ("", "missing all fields"),
        (
            "email=ursula_le_guin%40gmail.com&name=%20",
            "whitespace for name",
        ),
        (
            "email=this-aint-it-chief&name=things",
            "invalid email field",
        ),
        ("email=ursula_le_guin%40gmail.com&name=", "Empty name"),
        ("email=&name=le%20guin", "Empty email"),
    ];

    let app = spawn_app().await;
    for (body, case) in test_cases {
        let response = app.post_subscriptions(body.into()).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "The api did not fail with code 400 when payload was {}",
            case
        );
    }
}

#[actix_web::test]
async fn subscription_returns_200_for_valid_data() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    dbg!(&body);

    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);

        links[0].as_str().to_owned()
    };

    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());

    assert_eq!(html_link, text_link)
}

#[actix_web::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let subscription = sqlx::query!("SELECT email,name,status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();

    assert_eq!(subscription.email, "ursula_le_guin@gmail.com");
    assert_eq!(subscription.name, "le guin");
    assert_eq!(subscription.status, "pending_confirmation")
}