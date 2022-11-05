#[derive(serde::Deserialize)]
pub struct Configuration {
    pub web_server: WebServerConfiguration,
    pub database: DatabaseConfiguration,
}

#[derive(serde::Deserialize)]
pub struct WebServerConfiguration {
    pub port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseConfiguration {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    // Initialise our configuration reader
    let configuration = config::Config::builder()
        // Add configuration values from a file named `configuration.yaml`.
        .add_source(config::File::new("configuration.yaml", config::FileFormat::Yaml))
        .build()?;
    // Try to convert the configuration values it read into our configuration type
    configuration.try_deserialize::<Configuration>()
}

impl DatabaseConfiguration {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }

    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}