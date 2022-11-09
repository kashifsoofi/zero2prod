use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_request() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let body = r#"
        { "name":"le guin", "email":"ursula_le_guin@gmail.com" }
    "#.to_string();
    let response = app.post_subscriptions(body).await;

    // Assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    
    app.cleanup_subscriptinos(saved.email).await
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("{ \"name\":\"le guin\" }".to_string(), "missing the email"),
        (
            "{ \"email\":\"ursula_le_guin@gmail.com\" }".to_string(),
            "missing the name",
        ),
        ("".to_string(), "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions(invalid_body).await;
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            "{ \"name\":\"\", \"email\":\"ursula_le_guin@gmail.com\" }".to_string(),
            "empty name",
        ),
        ("{ \"name\":\"Ursula\", \"email\":\"\" }".to_string(), "empty email"),
        (
            "{ \"name\":\"\", \"email\":\"definitely-not-an-email\" }".to_string(),
            "invalid email",
        ),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions(body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}
