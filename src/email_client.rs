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
pub struct SendEmailRequest {
    from_email: String,
    recipients: Vec<Recipient>,
    subject: String,
    #[serde(rename(serialize = "Html-part"))]
    html_part: String,
    #[serde(rename(serialize = "Text-part"))]
    text_part: String,
}

#[derive(serde::Serialize)]
pub struct Recipient {
    email: String
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
    ) -> Self {
        Self {
            http_client: Client::new(),
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
            from_email: self.sender.as_ref().to_owned(),
            recipients: vec![
                Recipient { email: recipient.as_ref().to_owned() },
            ],
            subject: subject.to_owned(),
            html_part: html_content.to_owned(),
            text_part: text_content.to_owned(),
        };

        let authorization_header = format!("Basic {}", self.authorization_token.expose_secret());
        self
            .http_client
            .post(&url)
            .header("Authorization", authorization_header.to_owned())
            .json(&request_body)
            .send()
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
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

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let authorization_token = Faker.fake::<String>();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Secret::new(authorization_token.clone()),
        );

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
        
        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        // Act
        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
        
        // Assert
    }
}