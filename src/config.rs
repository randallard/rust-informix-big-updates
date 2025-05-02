use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use config::{Config, ConfigError, File, Environment};
use std::convert::TryFrom;

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    // Database connection parameters
    pub odbc_dsn: String,
    pub db_username: String,
    pub db_password: String,
    
    // Query parameters
    pub selection_query: String,
    pub update_query_template: String,
    pub batch_size: usize,
    pub timeout_seconds: u64,
    
    // File paths and other settings
    pub data_path: String,
    pub check_again_after: u64,
}

impl AppConfig {
    pub fn from_env_or_file() -> Result<Self, ConfigError> {
        let mut config = Config::default();

        // Try to load from a config file (e.g., config.toml)
        config.merge(File::with_name("config").required(false))?;

        // Override with environment variables if they exist
        config.merge(Environment::with_prefix("IBP"))?;

        // Parse the config into the AppConfig struct
        config.try_deserialize()
    }
    
    // Get the ODBC DSN, preferring the config file, then environment, and failing if neither
    pub fn get_odbc_dsn(&self) -> String {
        if !self.odbc_dsn.is_empty() {
            self.odbc_dsn.clone()
        } else {
            env::var("ODBC_DSN").expect("No ODBC_DSN found in config or environment")
        }
    }

    // Get the database username, preferring the config file, then environment, and failing if neither
    pub fn get_db_username(&self) -> String {
        if !self.db_username.is_empty() {
            self.db_username.clone()
        } else {
            env::var("DB_USERNAME").expect("No DB_USERNAME found in config or environment")
        }
    }

    // Get the database password, preferring the config file, then environment, and failing if neither
    pub fn get_db_password(&self) -> String {
        if !self.db_password.is_empty() {
            self.db_password.clone()
        } else {
            env::var("DB_PASSWORD").expect("No DB_PASSWORD found in config or environment")
        }
    }
}