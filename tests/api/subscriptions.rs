use crate::check_health::spawn_app;

#[actix_web::test]
async fn subscriptions_valid_request_ret200() {
    let crate::check_health::TestApp { address, db_pool } = spawn_app().await;
    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{}/subscribe", address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute Request");
    assert_eq!(response.status().as_u16(), 200);
    let sub = sqlx::query!("SELECT email,name FROM SUBSCRIPTIONS")
        .fetch_one(&db_pool)
        .await
        .expect("Failed to execute request");
    assert_eq!("ursula_le_guin@gmail.com", sub.email);
    assert_eq!("le guin", sub.name);
}

#[actix_web::test]
async fn subscriptions_invalid_request_ret400() {
    let test_cases = [
        ("email=ursula_le_guin%40gmail.com", "missing name field"),
        ("name=le%20guin", "missing mail field"),
        ("", "missing all fields"),
    ];

    let address = spawn_app().await.address;
    let client = reqwest::Client::new();
    for (body, case) in test_cases {
        let response = client
            .post(format!("{}/subscribe", address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute Request");
        assert_eq!(
            response.status().as_u16(),
            400,
            "The api did not fail with code 400 when payload was {}",
            case
        );
    }
}
