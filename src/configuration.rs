use crate::email_client::EmailClient;
use config::Environment;
use secrecy::ExposeSecret;
use secrecy::Secret;
use serde_aux::prelude::deserialize_number_from_string;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgSslMode;
use sqlx::ConnectOptions;
use std::env;

use crate::domain::SubscriberEmail;

#[derive(Clone, serde::Deserialize)]
pub struct Configuration {
    pub application: ApplicationConfiguration,
    pub database: DatabaseConfiguration,
    pub email_client: EmailClientConfiguration,
    pub redis_uri: Secret<String>,
}

#[derive(Clone, serde::Deserialize)]
pub struct ApplicationConfiguration {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
    pub hmac_secret: Secret<String>,
}

#[derive(Clone, serde::Deserialize)]
pub struct DatabaseConfiguration {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(Clone, serde::Deserialize)]
pub struct EmailClientConfiguration {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timeout_milliseconds: u64,
}

pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    let current_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = current_path.join("configuration");

    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into());

    // Initialise our configuration reader
    let configuration = config::Config::builder()
        // Add configuration values from a file named `base.yaml`.
        .add_source(config::File::from(
            configuration_directory.join("base.yaml"),
        ))
        .add_source(
            config::File::from(configuration_directory.join(format!("{}.yaml", environment)))
                .required(false),
        )
        // Add in a local configuration file
        // This file shouldn't be checked in to git or source control
        .add_source(config::File::from(configuration_directory.join("local.yaml")).required(false))
        .add_source(Environment::default().separator("_"))
        .build()?;
    // Try to convert the configuration values it read into our configuration type
    configuration.try_deserialize::<Configuration>()
}

impl DatabaseConfiguration {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // Try an encrypted connection, fallback to unencrypted if it fails
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        let mut options = self.without_db().database(&self.database_name);
        options.log_statements(tracing::log::LevelFilter::Trace);
        options
    }
}

impl EmailClientConfiguration {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }

    pub fn client(self) -> EmailClient {
        let sender_email = self.sender().expect("Invalid sender email address.");
        let timeout = self.timeout();
        EmailClient::new(
            self.base_url,
            sender_email,
            self.authorization_token,
            timeout,
        )
    }
}
