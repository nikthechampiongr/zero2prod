use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[actix_web::test]
async fn user_must_be_authenticated_to_get_change_password_form() {
    let app = spawn_app().await;

    let response = app.get_change_password().await;

    assert_is_redirect_to(&response, "/login");
}

#[actix_web::test]
async fn user_must_be_authenticated_to_change_password() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4();

    let login_body = serde_json::json!({
        "current_password": app.test_user.username,
        "new_password": new_password,
        "confirm_password": new_password
    });

    let response = app.post_change_password(login_body).await;

    assert_is_redirect_to(&response, "/login");
}

#[actix_web::test]
async fn password_fields_must_match() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4();
    let new_password2 = Uuid::new_v4();

    let password_change_body = serde_json::json!({
        "current_password": app.test_user.username,
        "new_password": new_password,
        "confirm_password": new_password2
    });

    // Need to login to change password
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password
    });
    app.post_login(&login_body).await;

    let response = app.post_change_password(password_change_body).await;

    assert_is_redirect_to(&response, "/admin/change_password");

    let page = app.get_change_password_html().await;

    assert!(page.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ));
}

#[actix_web::test]
async fn current_password_must_be_valid_to_change_password() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4();
    let bad_password = Uuid::new_v4();

    let password_change_body = serde_json::json!({
        "current_password": bad_password,
        "new_password": new_password,
        "confirm_password": new_password
    });

    // Need to login to change password
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password
    });
    app.post_login(&login_body).await;

    let response = app.post_change_password(password_change_body).await;

    assert_is_redirect_to(&response, "/admin/change_password");

    let page = app.get_change_password_html().await;

    assert!(page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[actix_web::test]
async fn changing_password_works() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4();

    // Need to login to change password
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let password_change_body = serde_json::json!({
        "current_password": app.test_user.password,
        "new_password": new_password,
        "confirm_password": new_password
    });

    let response = app.post_change_password(&password_change_body).await;
    assert_is_redirect_to(&response, "/admin/change-password");

    let page = app.get_change_password_html().await;
    assert!(page.contains("<p><i>Your password has been changed.</i></p>"));

    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let page = app.get_login_html().await;
    assert!(page.contains("<p><i>You have successfuly logged out.</i></p>"));

    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": new_password
    });

    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");
}
