use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn unauthenticated_request_to_send_newsletter_should_bounce() {
    let app = spawn_app().await;

    let response = app.get_admin_send_newsletters().await;

    assert_is_redirect_to(&response, "/login");
}
