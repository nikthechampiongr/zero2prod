use crate::helpers::{assert_is_redirect_to, spawn_app};

#[actix_web::test]
async fn dashboard_redirects_to_login_when_user_is_unauthenticated() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login")
}

#[actix_web::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });

    let response = app.post_login(login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let page = app.get_login_html().await;
    dbg!(&page);
    assert!(page.contains("<p><i>You have successfuly logged out.</i></p>"));

    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
