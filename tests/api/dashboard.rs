use crate::helpers::{assert_is_redirect_to, spawn_app};

#[actix_web::test]
async fn dashboard_redirects_to_login_when_user_is_unauthenticated() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login")
}
