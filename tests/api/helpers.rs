use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::{get_configuration, DatabaseConfiguration};
use zero2prod::startup::{get_connection_pool, Application};
use zero2prod::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber_name = "test".to_string();
    let default_filter_level = "info".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});
pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn cleanup_subscriptinos(&self, email: String) {
        sqlx::query!("delete from subscription_tokens st using subscriptions s where st.subscriber_id = s.id and s.email = $1", email)
            .execute(&self.db_pool)
            .await
            .expect("Failed to delete subscription_token.");
        sqlx::query!("DELETE FROM subscriptions WHERE email = $1", email)
            .execute(&self.db_pool)
            .await
            .expect("Failed to delete subscription.");
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["Html-part"].as_str().unwrap());
        let plain_text = get_link(&body["Text-part"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }
}

const CREATE_TEMP_DB: bool = false;

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        if CREATE_TEMP_DB {
            // Use a different database for each test case
            c.database.database_name = Uuid::new_v4().to_string();
        }
        // Use a random OS port
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database, CREATE_TEMP_DB).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let application_port = application.port();
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        // How do we get these?
        address,
        port: application_port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
    }
}

async fn configure_database(config: &DatabaseConfiguration, create_temp_db: bool) -> PgPool {
    // Create database
    if create_temp_db {
        let mut connection = PgConnection::connect_with(&config.without_db())
            .await
            .expect("Failed to connect to Postgres");
        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
            .await
            .expect("Failed to create database.");
    }

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    if create_temp_db {
        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .expect("Failed to migrate the database");
    }

    connection_pool
}
