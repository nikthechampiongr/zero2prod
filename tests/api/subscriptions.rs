use crate::helpers::spawn_app;

#[actix_web::test]
async fn subscriptions_valid_request_ret200() {
    let app  = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_subscriptions(body.to_string()).await;
    assert_eq!(response.status().as_u16(), 200);

    let sub = sqlx::query!("SELECT email,name FROM SUBSCRIPTIONS")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to execute request");
    assert_eq!("ursula_le_guin@gmail.com", sub.email);
    assert_eq!("le guin", sub.name);
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