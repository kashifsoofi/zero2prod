use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_request() {
    // Arrange
    let app = spawn_app().await;
    let body = r#"
        { "name":"le guin", "email":"ursula_le_guin@gmail.com" }
    "#;
    Mock::given(path("/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    let response = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
    app.cleanup_subscriptinos("ursula_le_guin@gmail.com".into())
        .await;
    app.cleanup_user().await;
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // Arrange
    let app = spawn_app().await;
    let body = r#"
        { "name":"le guin", "email":"ursula_le_guin1@gmail.com" }
    "#;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    let saved = sqlx::query!(
        "SELECT email, name, status FROM subscriptions WHERE email = $1",
        "ursula_le_guin1@gmail.com"
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved subscription.");
    app.cleanup_subscriptinos("ursula_le_guin1@gmail.com".into())
        .await;

    assert_eq!(saved.email, "ursula_le_guin1@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
    app.cleanup_user().await;
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("{ \"name\":\"le guin\" }", "missing the email"),
        (
            "{ \"email\":\"ursula_le_guin@gmail.com\" }",
            "missing the name",
        ),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions(invalid_body.into()).await;
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
    app.cleanup_user().await;
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            "{ \"name\":\"\", \"email\":\"ursula_le_guin@gmail.com\" }",
            "empty name",
        ),
        ("{ \"name\":\"Ursula\", \"email\":\"\" }", "empty email"),
        (
            "{ \"name\":\"\", \"email\":\"definitely-not-an-email\" }",
            "invalid email",
        ),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
    app.cleanup_user().await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // Arrange
    let app = spawn_app().await;
    let body = r#"
        { "name":"le guin2", "email":"ursula_le_guin2@gmail.com" }
    "#;

    Mock::given(path("/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    // Mock asserts on drop
    app.cleanup_subscriptinos("ursula_le_guin2@gmail.com".into())
        .await;
    app.cleanup_user().await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Arrange
    let app = spawn_app().await;
    let body = r#"
        { "name":"le guin3", "email":"ursula_le_guin3@gmail.com" }
    "#;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // We are not setting an expectation here anymore
        // The test is focused on another aspect of the app
        // behaviour.
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    app.cleanup_subscriptinos("ursula_le_guin3@gmail.com".into())
        .await;

    // Get the first intercepted request
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
    app.cleanup_user().await;
}
