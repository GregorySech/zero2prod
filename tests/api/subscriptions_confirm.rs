use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    // Arrange app
    let app = spawn_app().await;

    // Act: send request to subscription/confirm endpoint.
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400, "Confirmation requests without tokens are bad requests!");
}
