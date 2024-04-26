use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app  = spawn_app().await;
    let bound_address = app.address;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", &bound_address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
