use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{Secret, ExposeSecret};

pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailRequest<'a> {
    from_email: &'a str,
    recipients: Vec<Recipient<'a>>,
    subject: &'a str,
    #[serde(rename(serialize = "Html-part"))]
    html_part: &'a str,
    #[serde(rename(serialize = "Text-part"))]
    text_part: &'a str,
}

#[derive(serde::Serialize)]
pub struct Recipient<'a> {
    email: &'a str
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap();
        Self {
            http_client: http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/send", self.base_url);
        let request_body = SendEmailRequest {
            from_email: self.sender.as_ref(),
            recipients: vec![
                Recipient { email: recipient.as_ref() },
            ],
            subject: subject,
            html_part: html_content,
            text_part: text_content,
        };

        let authorization_header = format!("Basic {}", self.authorization_token.expose_secret());
        self
            .http_client
            .post(&url)
            .header("Authorization", authorization_header.to_owned())
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claim::assert_err;
    use claim::assert_ok;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::any;
    use wiremock::matchers::{header, header_exists, path, method};
    use wiremock::Request;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> =
                serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("FromEmail").is_some()
                && body.get("Recipients").is_some()
                && body.get("Subject").is_some()
                && body.get("Html-part").is_some()
                && body.get("Text-part").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        email_client_with_secret(base_url, Faker.fake())
    }

    fn email_client_with_secret(base_url: String, secret: String) -> EmailClient {
        EmailClient::new(
            base_url, 
            email(), 
            Secret::new(secret), 
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // Arrange
        let mock_server = MockServer::start().await;
        let authorization_token = Faker.fake::<String>();
        let email_client = email_client_with_secret(mock_server.uri(), authorization_token.clone());

        let authorization_header_value = format!("Basic {}", authorization_token);
        let authorization_header_value = authorization_header_value.as_str();
        Mock::given(header_exists("Authorization"))
            .and(header("Authorization", authorization_header_value))
            .and(header("Content-Type", "application/json"))
            .and(path("/send"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        
        // Act
        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
        
        // Assert
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // We do not copy in all the matchers we have in the other test.
        // The purpose of this test is not to assert on the request we 
        // are sending out! 
        // We add the bare minimum needed to trigger the path we want
        // to test in `send_email`.
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            // Not a 200 anymore!
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200)
            // 3 minutes!
            .set_delay(std::time::Duration::from_secs(60));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }
}