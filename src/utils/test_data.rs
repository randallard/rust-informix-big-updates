// src/utils/test_data.rs

use crate::zip_county_map::{load_zip_county_map, ZipCountyInfo};
use odbc_api::{Connection, Environment};
use rand::prelude::*;
use std::error::Error;

pub fn generate_test_data(conn: &Connection, count: usize) -> Result<(), Box<dyn Error>> {
    // Load the zip-county mapping data
    let zip_county_map = load_zip_county_map();
    let zip_codes: Vec<String> = zip_county_map.keys().cloned().collect();
    
    if zip_codes.is_empty() {
        return Err("No zip codes found in mapping".into());
    }
    
    println!("Generating {} test records...", count);
    
    // Setup random number generator
    let mut rng = rand::thread_rng();
    
    for i in 0..count {
        // Select a random zip code
        let idx = rng.gen_range(0..zip_codes.len());
        let zip_code = &zip_codes[idx];
        
        // Get the corresponding county info
        let county_info = zip_county_map.get(zip_code).unwrap();
        
        // Create a unique key for this record
        let key = format!("testkey_{}", i + 1);
        
        // Generate some random data for other fields
        let field1 = format!("value_{}", rng.gen_range(1000..9999));
        let field2 = format!("data_{}", rng.gen_range(100..999));
        let condition = if rng.gen_bool(0.8) { "t" } else { "f" };
        
        // Format the zip code to be 10 characters (add a random 4-digit extension)
        let extended_zip = format!("{}-{:04}", zip_code, rng.gen_range(0..9999));
        
        // Execute the insert statement
        let insert_query = format!(
            "INSERT INTO table_name (key_field, field1, field2, condition, county, zip_code) 
             VALUES ('{}', '{}', '{}', '{}', '{}', '{}')",
            key, field1, field2, condition, county_info.fips_code, extended_zip
        );
        
        match conn.execute(&insert_query, ()) {
            Ok(_) => {
                if i % 50 == 0 {
                    println!("Inserted {} records...", i + 1);
                }
            },
            Err(e) => {
                eprintln!("Error inserting record {}: {:?}", key, e);
                // Continue with other records even if one fails
            }
        }
    }
    
    println!("Successfully generated {} test records", count);
    Ok(())
}

// Function to clean all test data
pub fn clean_test_data(conn: &Connection) -> Result<(), Box<dyn Error>> {
    println!("Cleaning test data...");
    
    let delete_query = "DELETE FROM table_name WHERE key_field LIKE 'testkey_%'";
    
    match conn.execute(delete_query, ()) {
        Ok(_) => println!("Successfully cleaned test data"),
        Err(e) => return Err(format!("Error cleaning test data: {:?}", e).into()),
    }
    
    Ok(())
}