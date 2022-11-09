use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseConfiguration};
use zero2prod::startup::{Application, get_connection_pool};
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
    pub db_pool: PgPool,
}

const CREATE_TEMP_DB: bool = false;

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        if CREATE_TEMP_DB {
            // Use a different database for each test case
            c.database.database_name = Uuid::new_v4().to_string();
        }
        // Use a random OS port
        c.application.port = 0;
        c
    };

    configure_database(&configuration.database, CREATE_TEMP_DB).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        // How do we get these?
        address,
        db_pool: get_connection_pool(&configuration.database)
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

pub async fn cleanup_database(db_pool: PgPool, email: String) {
    sqlx::query!("DELETE FROM subscriptions WHERE email = $1", email)
        .execute(&db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
}
