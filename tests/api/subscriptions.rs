use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app: TestApp = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // Arrange
    let app: TestApp = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    app.post_subscriptions(body.into()).await;

    let db_pool = app.db_pool;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(
        saved.status, "pending_confirmation",
        "Created subscription should have pending_confirmation status."
    );
}

/// The subscribe endpoint should return 200 even when the user is still
/// pending confirmation. No testing assumptions on the database state or email
/// sending.
#[tokio::test]
async fn subscribing_twice_returns_200() {
    // Arrange
    let app: TestApp = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&app.email_server)
        .await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response1 = app.post_subscriptions(body.into()).await;
    let response2 = app.post_subscriptions(body.into()).await;

    assert_eq!(response1.status().as_u16(), 200);
    assert_eq!(response2.status().as_u16(), 200);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!(
        "
    ALTER TABLE subscriptions
    DROP COLUMN email;
    ",
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn subscribing_twice_persists_one_new_subscriber() {
    // Arrange
    let app: TestApp = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&app.email_server)
        .await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _ = app.post_subscriptions(body.into()).await;
    let _ = app.post_subscriptions(body.into()).await;

    let db_pool = app.db_pool;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_all(&db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.len(), 1, "only one record");
    let record = saved.first().unwrap();
    assert_eq!(record.email, "ursula_le_guin@gmail.com");
    assert_eq!(record.name, "le guin");
    assert_eq!(
        record.status, "pending_confirmation",
        "Created subscription should have pending_confirmation status."
    );
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitley-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return 400 Bad Request when the payload was {}",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 when payload was {}.",
            error_message
        )
    }
}

/// Checking if a confirmation email is sent when the subscription endpoint is
/// hit with a valid body.
#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
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

    let get_link = |s: &str| {
        let links: Vec<linkify::Link<'_>> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1, "There should be at least one link!");
        links.first().unwrap().as_str().to_owned()
    };

    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());
    assert_eq!(html_link, text_link);
}
