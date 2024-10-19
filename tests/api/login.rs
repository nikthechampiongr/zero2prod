use crate::helpers::{assert_is_redirect_to, spawn_app};

#[actix_web::test]
async fn an_error_flash_message_is_sent_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = app.post_login(&login_body).await;

    assert_is_redirect_to(&response, "/login");
    assert!(response.cookies().any(|cookie| cookie.name() == "_flash"));

    let login_page = app.get_login_form().await;
    assert!(login_page.contains(r#"<p><i>Invalid login credentials</i></p>"#));

    let login_page = app.get_login_form().await;
    assert!(!login_page.contains(r#"<p><i>Invalid login credentials</i></p>"#));
}

#[actix_web::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}
