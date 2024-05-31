use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let app = spawn_app().await;

    let response = app.get_change_password().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let response = app.post_change_password(&serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": &new_password,
        "new_password_check": &new_password,
    })).await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_provide_matching_passwords_to_change_your_password() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();
    let new_different_password = Uuid::new_v4().to_string();

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });

    let change_password_body = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &new_different_password,
    });

    app.post_login(&login_body).await;

    let response = app.post_change_password(&change_password_body).await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("You entered two different new passwords - \
    the field values must match."));
}

#[tokio::test]
async fn you_must_provide_the_correct_password_to_change_your_password() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });

    let change_password_body = serde_json::json!({
        "current_password": &wrong_password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    });


    app.post_login(&login_body).await;

    let response = app.post_change_password(&change_password_body).await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("The current password is incorrect."));
}