use odbc_api::{Connection, Environment};
use std::error::Error;
use std::sync::Arc;

use crate::config::AppConfig;

// Use a global static environment to ensure it lives for the entire program
lazy_static::lazy_static! {
    static ref ENVIRONMENT: Arc<Environment> = Arc::new(Environment::new().expect("Failed to create ODBC environment"));
}

pub fn create_connection(config: &AppConfig) -> Result<Connection<'static>, Box<dyn Error>> {
    // Use the global environment
    let connection = ENVIRONMENT.connect(
        &config.get_odbc_dsn(),
        &config.get_db_username(),
        &config.get_db_password(),
    )?;
    
    log::info!("Successfully connected to the database");
    
    Ok(connection)
}

pub fn test_connection(config: &AppConfig) -> Result<(), Box<dyn Error>> {
    let connection = create_connection(config)?;
    
    // Test a simple query
    let query_result = connection.execute("SELECT 1 FROM systables WHERE tabid = 1", ())?;
    
    if query_result.is_some() {
        log::info!("Database connection test successful");
        println!("Database connection test successful");
    } else {
        return Err("Database connection test failed".into());
    }
    
    Ok(())
}