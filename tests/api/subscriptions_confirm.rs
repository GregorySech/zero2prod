use reqwest::StatusCode;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};
use zero2prod::routes::generate_subscription_token;

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    // Arrange app
    let app = spawn_app().await;

    // Act: send request to subscription/confirm endpoint.
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(
        response.status().as_u16(),
        400,
        "Confirmation requests without tokens are bad requests!"
    );
}

#[tokio::test]
async fn confirmation_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!("ALTER TABLE subscriptions_tokens DROP COLUMN subscriber_id",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
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
    let confirmation_links = app.get_confirmation_links(email_request);

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
    assert_eq!(
        confirmation_links.html.host_str().unwrap(),
        app.base_url.host_str().unwrap(),
        "No random APIs."
    );

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(
        response.status().as_u16(),
        200,
        "Confirmation link should return OK on request."
    );
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_the_user() {
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
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let db_pool = app.db_pool;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(
        saved.status, "confirmed",
        "After following confirmation link user should have confirmed status."
    );
}

/// Confirming the user status should happen once.
/// After the first time the confirmation should no longer be available.
#[tokio::test]
async fn confirmation_link_should_be_gone_for_confirmed_users() {
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
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html.clone()).await.unwrap();

    assert_eq!(response.status().as_u16(), StatusCode::OK);

    // Second click
    let response = reqwest::get(confirmation_links.html.clone()).await.unwrap();

    assert_eq!(response.status().as_u16(), StatusCode::GONE);
}

#[tokio::test]
async fn confirming_a_subscription_with_an_unexisting_token_is_unauthorized() {
    let app = spawn_app().await;
    let fake_token = generate_subscription_token();
    let response = app.confirm_token(fake_token).await;
    assert_eq!(response.status().as_u16(), StatusCode::UNAUTHORIZED);
}
