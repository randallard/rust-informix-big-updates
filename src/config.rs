use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use config::{Config, ConfigError, File, Environment};
use std::convert::TryFrom;

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    // Database connection parameters
    #[serde(default = "default_empty_string")]
    pub odbc_dsn: String,
    #[serde(default = "default_empty_string")]
    pub db_username: String,
    #[serde(default = "default_empty_string")]
    pub db_password: String,
    
    // Query parameters
    #[serde(default = "default_selection_query")]
    pub selection_query: String,
    #[serde(default = "default_update_query_template")]
    pub update_query_template: String,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    
    // File paths and other settings
    #[serde(default = "default_data_path")]
    pub data_path: String,
    #[serde(default = "default_check_again_after")]
    pub check_again_after: u64,

    // Field name mappings (new fields)
    #[serde(default = "default_key_field_name")]
    pub key_field_name: String,
    #[serde(default = "default_zip_field_name")]
    pub zip_field_name: String,
    #[serde(default = "default_county_field_name")]
    pub county_field_name: String,
}

// Default function implementations
fn default_empty_string() -> String {
    "".to_string()
}

fn default_selection_query() -> String {
    "SELECT key_field, field1, field2 FROM table_name WHERE condition = 't'".to_string()
}

fn default_update_query_template() -> String {
    "UPDATE table_name SET field1 = 'new_value' WHERE key_field = '{{key}}'".to_string()
}

fn default_batch_size() -> usize {
    100
}

fn default_timeout_seconds() -> u64 {
    30
}

fn default_data_path() -> String {
    "processed_records.json".to_string()
}

fn default_check_again_after() -> u64 {
    1800 // 30 minutes in seconds
}

fn default_key_field_name() -> String {
    "key_field".to_string()
}

fn default_zip_field_name() -> String {
    "zip_code".to_string()
}

fn default_county_field_name() -> String {
    "county".to_string()
}

impl AppConfig {
    pub fn from_env_or_file() -> Result<Self, ConfigError> {
        let mut config = Config::default();

        // Try to load from a config file (e.g., config.toml)
        config.merge(File::with_name("config").required(false))?;

        // Override with environment variables if they exist
        config.merge(Environment::with_prefix("IBP"))?;

        // Parse the config into the AppConfig struct
        let app_config: AppConfig = config.try_deserialize()?;
        
        Ok(app_config)
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