use odbc_api::{buffers::TextRowSet, Connection, Cursor, IntoParameter};
use indicatif::ProgressBar;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::files::json_handler::{save_query_file, read_query_files, save_error_file};
use crate::ui;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRecord {
    pub key: String,
    pub query: String,
    pub status: QueryStatus,
    pub result: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorRecord {
    pub key: String,
    pub file: String,
    pub error: String,
    pub timestamp: String,
}

pub fn prompt_user(question: &str) -> String {
    print!("{} (Y/N): ", question);
    io::stdout().flush().expect("Failed to flush stdout");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read input");
    input.trim().to_string()
}

pub fn generate_queries(
    conn: &Connection,
    config: &AppConfig,
    results_dir: &str,
    progress_bar: &ProgressBar,
) -> Result<usize, Box<dyn Error>> {
    ui::progress::print_with_progress(progress_bar, "Finding records requiring updates...");
    
    // Execute the selection query to find records requiring updates
    let cursor = match conn.execute(&config.selection_query, ())? {
        Some(cursor) => cursor,
        None => {
            ui::progress::print_with_progress(progress_bar, "No records found requiring updates.");
            log::warn!("Selection query returned no results");
            return Ok(0);
        }
    };
    
    // Set up buffer for fetching rows
    let mut buffers = TextRowSet::for_cursor(config.batch_size, &cursor, Some(4096))?;
    let mut row_set_cursor = cursor.bind_buffer(&mut buffers)?;
    
    let mut count = 0;
    let mut total_records = 0;
    
    ui::progress::print_with_progress(progress_bar, "Generating update queries for all matching records...");
    
    // Process each batch of rows
    while let Some(batch) = row_set_cursor.fetch()? {
        total_records += batch.num_rows();
        progress_bar.set_length(total_records as u64);
        
        for row_index in 0..batch.num_rows() {
            progress_bar.set_position(count as u64);
            
            // Get key field value (assuming first column is key)
            let key_field = String::from_utf8_lossy(batch.at(0, row_index).unwrap_or(&[])).to_string();
            
            // Update progress bar message but don't print to console
            ui::progress::update_message(progress_bar, format!("Generating query for key: {}", key_field));
            
            // Create a map of values for template substitution
            let mut values = std::collections::HashMap::new();
            values.insert("key".to_string(), key_field.clone());
            
            // Add all other columns to the values map
            for col_index in 1..batch.num_cols() {
                let col_name = format!("field{}", col_index);
                let value = String::from_utf8_lossy(batch.at(col_index, row_index).unwrap_or(&[])).to_string();
                values.insert(col_name, value);
            }
            
            // Generate update query by replacing template placeholders
            let mut query = config.update_query_template.clone();
            for (key, value) in &values {
                let placeholder = format!("{{{{{}}}}}", key);
                query = query.replace(&placeholder, value);
            }
            
            // Create query record
            let query_record = QueryRecord {
                key: key_field.clone(),
                query,
                status: QueryStatus::Pending,
                result: None,
                timestamp: None,
            };
            
            // Save query to file
            let file_path = format!("{}/{}.json", results_dir, key_field);
            save_query_file(&file_path, &query_record)?;
            
            count += 1;
        }
    }
    
    // Only print the summary at the end
    let summary = format!("Generated {} update queries", count);
    ui::progress::print_with_progress(progress_bar, &format!("\x1b[32m{}\x1b[0m", summary));
    log::info!("{}", summary);
    
    Ok(count)
}

pub fn test_queries(
    conn: &Connection,
    results_dir: &str,
    progress_bar: &ProgressBar,
) -> Result<(usize, usize), Box<dyn Error>> {
    ui::progress::print_with_progress(progress_bar, "Testing queries for syntax errors without executing them...");
    
    // Find all query files in the results directory
    let query_files = read_query_files(results_dir)?;
    let total_files = query_files.len();
    
    if total_files == 0 {
        ui::progress::print_with_progress(progress_bar, "No queries found to test. Run the 'generate' command first.");
        return Ok((0, 0));
    }
    
    progress_bar.set_length(total_files as u64);
    
    let mut valid_count = 0;
    let mut invalid_count = 0;
    
    // No longer using transactions for testing, just test each query independently
    for (index, file_path) in query_files.iter().enumerate() {
        progress_bar.set_position(index as u64);
        
        // Read query record from file
        let file_content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to read file {}: {}", file_path.display(), e);
                invalid_count += 1;
                continue;
            }
        };
            
        let query_record: QueryRecord = match serde_json::from_str(&file_content) {
            Ok(record) => record,
            Err(e) => {
                log::error!("Failed to parse JSON from file {}: {}", file_path.display(), e);
                invalid_count += 1;
                continue;
            }
        };
        
        let key = &query_record.key;
        let query = &query_record.query;
        
        // Update progress message but don't print to console
        ui::progress::update_message(progress_bar, format!("Testing query for key: {}", key));
        
        // Very basic SQL syntax validation without using ODBC
        let is_valid = basic_sql_validation(query);
        
        if is_valid {
            log::info!("Query syntax looks valid for key: {}", key);
            valid_count += 1;
        } else {
            log::error!("Query syntax error for key: {}", key);
            log::error!("Query: {}", query);
            invalid_count += 1;
        }
    }
    
    // Print summary only at the end
    let summary = format!("Tested {} queries: {} valid, {} invalid", total_files, valid_count, invalid_count);
    ui::progress::print_with_progress(progress_bar, &summary);
    log::info!("{}", summary);
    
    Ok((valid_count, invalid_count))
}

// A simple SQL validator that doesn't use the ODBC API at all
fn basic_sql_validation(query: &str) -> bool {
    let query = query.trim().to_uppercase();
    
    // Basic CHECK: SQL must not be empty
    if query.is_empty() {
        return false;
    }
    
    // Basic CHECK: Must start with a valid SQL command for this application
    if !query.starts_with("UPDATE") && !query.starts_with("INSERT") && !query.starts_with("DELETE") {
        return false;
    }
    
    // Basic CHECK: For UPDATE statements, must contain SET and WHERE
    if query.starts_with("UPDATE") && (!query.contains(" SET ") || !query.contains(" WHERE ")) {
        return false;
    }
    
    // Basic CHECK: For INSERT statements, must contain VALUES or SELECT
    if query.starts_with("INSERT") && !query.contains(" VALUES ") && !query.contains(" SELECT ") {
        return false;
    }
    
    // Basic CHECK: For DELETE statements, must contain FROM
    if query.starts_with("DELETE") && !query.contains(" FROM ") {
        return false;
    }
    
    // Basic CHECK: Must have balanced quotes
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    
    for c in query.chars() {
        match c {
            '\'' => {
                if !in_double_quote {
                    in_single_quote = !in_single_quote;
                }
            },
            '"' => {
                if !in_single_quote {
                    in_double_quote = !in_double_quote;
                }
            },
            _ => {}
        }
    }
    
    if in_single_quote || in_double_quote {
        return false;  // Unbalanced quotes
    }
    
    // Basic CHECK: Must have balanced parentheses
    let mut paren_count = 0;
    
    for c in query.chars() {
        match c {
            '(' => paren_count += 1,
            ')' => paren_count -= 1,
            _ => {}
        }
        
        if paren_count < 0 {
            return false;  // More closing than opening parentheses
        }
    }
    
    if paren_count != 0 {
        return false;  // Unbalanced parentheses
    }
    
    // If all checks pass, consider the query valid
    true
}

pub fn execute_queries(
    conn: &Connection,
    results_dir: &str,
    progress_bar: &ProgressBar,
) -> Result<(usize, usize), Box<dyn Error>> {
    ui::progress::print_with_progress(progress_bar, "Executing update queries...");
    
    // Find all query files in the results directory
    let query_files = read_query_files(results_dir)?;
    let total_files = query_files.len();
    
    if total_files == 0 {
        ui::progress::print_with_progress(progress_bar, "No queries found to execute.");
        return Ok((0, 0));
    }
    
    progress_bar.set_length(total_files as u64);
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for (index, file_path) in query_files.iter().enumerate() {
        progress_bar.set_position(index as u64);
        
        // Read query record from file
        let file_content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to read file {}: {}", file_path.display(), e);
                error_count += 1;
                continue;
            }
        };
            
        let mut query_record: QueryRecord = match serde_json::from_str(&file_content) {
            Ok(record) => record,
            Err(e) => {
                log::error!("Failed to parse JSON from file {}: {}", file_path.display(), e);
                error_count += 1;
                continue;
            }
        };
            
        // Skip already executed queries
        if query_record.status == QueryStatus::Completed {
            ui::progress::update_message(progress_bar, format!("Skipping already completed query for key: {}", query_record.key));
            continue;
        }
        
        // Update progress bar message
        ui::progress::update_message(progress_bar, format!("Executing query for key: {}", query_record.key));
        
        // Execute the query directly (no transaction)
        let current_time = Utc::now().to_rfc3339();
        match conn.execute(&query_record.query, ()) {
            Ok(_) => {
                // Update query record with success result
                query_record.status = QueryStatus::Completed;
                query_record.result = Some("success".to_string());
                query_record.timestamp = Some(current_time.clone());
                success_count += 1;
            },
            Err(err) => {
                // Update query record with error result
                query_record.status = QueryStatus::Failed;
                query_record.result = Some(format!("error: {:?}", err));
                query_record.timestamp = Some(current_time.clone());
                
                // Add to error log
                let error_record = ErrorRecord {
                    key: query_record.key.clone(),
                    file: file_path.file_name().unwrap().to_string_lossy().to_string(),
                    error: format!("{:?}", err),
                    timestamp: current_time,
                };
                
                save_error_file(&format!("{}/errors.json", results_dir), &error_record)?;
                error_count += 1;
                
                // Only print errors to log, not console
                log::error!("Query execution failed for key {}: {:?}", query_record.key, err);
            }
        }
        
        // Save updated query record
        save_query_file(file_path, &query_record)?;
    }
    
    // Print summary at the end
    let summary = format!("Executed {} queries: {} successful, {} failed", total_files, success_count, error_count);
    ui::progress::print_with_progress(progress_bar, &summary);
    log::info!("{}", summary);
    
    Ok((success_count, error_count))
}

// Add this function to db/query.rs

pub fn update_county_by_zip(
    conn: &Connection,
    config: &AppConfig,
    results_dir: &str,
    progress_bar: &ProgressBar,
) -> Result<(usize, usize), Box<dyn Error>> {
    ui::progress::print_with_progress(progress_bar, "Finding records with mismatched county and zip codes...");
    
    // Load the zip-county mapping
    let zip_county_map = crate::zip_county_map::load_zip_county_map();
    
    // Query to find records with zip codes but potentially incorrect county codes
    let selection_query = "SELECT key_field, zip_code, county FROM table_name WHERE zip_code IS NOT NULL";
    
    // Execute the selection query
    let cursor = match conn.execute(selection_query, ())? {
        Some(cursor) => cursor,
        None => {
            ui::progress::print_with_progress(progress_bar, "No records found with zip codes.");
            log::warn!("Selection query returned no results");
            return Ok((0, 0));
        }
    };
    
    // Set up buffer for fetching rows
    let mut buffers = TextRowSet::for_cursor(config.batch_size, &cursor, Some(4096))?;
    let mut row_set_cursor = cursor.bind_buffer(&mut buffers)?;
    
    let mut count = 0;
    let mut mismatch_count = 0;
    let mut total_records = 0;
    
    ui::progress::print_with_progress(progress_bar, "Generating update queries for records with mismatched county codes...");
    
    // Process each batch of rows
    while let Some(batch) = row_set_cursor.fetch()? {
        total_records += batch.num_rows();
        progress_bar.set_length(total_records as u64);
        
        for row_index in 0..batch.num_rows() {
            progress_bar.set_position(count as u64);
            
            // Get key field value
            let key_field = String::from_utf8_lossy(batch.at(0, row_index).unwrap_or(&[])).to_string();
            
            // Get zip code
            let zip_code = String::from_utf8_lossy(batch.at(1, row_index).unwrap_or(&[])).to_string();
            
            // Get current county code
            let current_county = String::from_utf8_lossy(batch.at(2, row_index).unwrap_or(&[])).to_string();
            
            // Extract 5-digit zip from zip+4 if needed
            let zip5 = if zip_code.contains('-') {
                zip_code.split('-').next().unwrap_or("").to_string()
            } else if zip_code.len() >= 5 {
                zip_code[0..5].to_string()
            } else {
                zip_code.clone()
            };
            
            // Look up the correct FIPS code for this zip
            if let Some(zip_info) = zip_county_map.get(&zip5) {
                let correct_fips = &zip_info.fips_code;
                
                // Update progress bar message but don't print to console
                ui::progress::update_message(progress_bar, format!("Checking key: {}, zip: {}, county: {}", key_field, zip5, current_county));
                
                // Only generate update query if county code doesn't match the correct FIPS code
                if current_county != *correct_fips {
                    mismatch_count += 1;
                    
                    // Generate update query
                    let query = format!(
                        "UPDATE table_name SET county = '{}' WHERE key_field = '{}'",
                        correct_fips, key_field
                    );
                    
                    // Create query record
                    let query_record = QueryRecord {
                        key: key_field.clone(),
                        query,
                        status: QueryStatus::Pending,
                        result: None,
                        timestamp: None,
                    };
                    
                    // Save query to file
                    let file_path = format!("{}/{}.json", results_dir, key_field);
                    save_query_file(&file_path, &query_record)?;
                    
                    log::info!("Generated update query for key: {}, changing county from '{}' to '{}'", 
                               key_field, current_county, correct_fips);
                }
            }
            
            count += 1;
        }
    }
    
    // Print summary
    let summary = format!("Checked {} records, found {} with mismatched county codes", count, mismatch_count);
    ui::progress::print_with_progress(progress_bar, &format!("\x1b[32m{}\x1b[0m", summary));
    log::info!("{}", summary);
    
    Ok((count, mismatch_count))
}

// Add this function to db/query.rs

pub fn update_county_by_zip_2digit(
    conn: &Connection,
    config: &AppConfig,
    results_dir: &str,
    progress_bar: &ProgressBar,
) -> Result<(usize, usize), Box<dyn Error>> {
    ui::progress::print_with_progress(progress_bar, "Finding records with zip codes to update county codes...");
    
    // Load the zip-county mapping
    let zip_county_map = crate::zip_county_map::load_zip_county_map();
    
    // Use the selection query from config
    let selection_query = &config.selection_query;
    
    // Execute the selection query
    let cursor = match conn.execute(selection_query, ())? {
        Some(cursor) => cursor,
        None => {
            ui::progress::print_with_progress(progress_bar, "No records found matching the selection query.");
            log::warn!("Selection query returned no results");
            return Ok((0, 0));
        }
    };
    
    // Set up buffer for fetching rows
    let mut buffers = TextRowSet::for_cursor(config.batch_size, &cursor, Some(4096))?;
    let mut row_set_cursor = cursor.bind_buffer(&mut buffers)?;
    
    let mut count = 0;
    let mut updated_count = 0;
    let mut total_records = 0;
    
    ui::progress::print_with_progress(progress_bar, "Generating update queries for county codes...");
    
    // Process each batch of rows
    while let Some(batch) = row_set_cursor.fetch()? {
        total_records += batch.num_rows();
        progress_bar.set_length(total_records as u64);
        
        for row_index in 0..batch.num_rows() {
            progress_bar.set_position(count as u64);
            
            // Find the column index for key_field and zip_code
            let key_field_idx = find_column_index(&batch, "key_field").unwrap_or(0);
            let zip_code_idx = find_column_index(&batch, "zip_code");
            
            // Get key field value
            let key_field = String::from_utf8_lossy(batch.at(key_field_idx, row_index).unwrap_or(&[])).to_string();
            
            // Update progress bar message but don't print to console
            ui::progress::update_message(progress_bar, format!("Processing key: {}", key_field));
            
            // Only proceed if zip_code column was found
            if let Some(zip_idx) = zip_code_idx {
                // Get zip code
                let zip_code = String::from_utf8_lossy(batch.at(zip_idx, row_index).unwrap_or(&[])).to_string();
                
                // Extract 5-digit zip from zip+4 if needed
                let zip5 = if zip_code.contains('-') {
                    zip_code.split('-').next().unwrap_or("").to_string()
                } else if zip_code.len() >= 5 {
                    zip_code[0..5].to_string()
                } else {
                    zip_code.clone()
                };
                
                // Look up the county code for this zip
                if let Some(zip_info) = zip_county_map.get(&zip5) {
                    // Get the two-digit county code
                    let county_code = &zip_info.county_code;
                    
                    // Generate update query
                    let query = format!(
                        "UPDATE table_name SET county = '{}' WHERE key_field = '{}'",
                        county_code, key_field
                    );
                    
                    // Create query record
                    let query_record = QueryRecord {
                        key: key_field.clone(),
                        query,
                        status: QueryStatus::Pending,
                        result: None,
                        timestamp: None,
                    };
                    
                    // Save query to file
                    let file_path = format!("{}/{}.json", results_dir, key_field);
                    save_query_file(&file_path, &query_record)?;
                    
                    updated_count += 1;
                    log::info!("Generated update query for key: {}, setting county code to '{}' based on zip code {}", 
                               key_field, county_code, zip5);
                } else {
                    log::warn!("No county code mapping found for zip code: {}", zip5);
                }
            } else {
                log::warn!("No zip_code column found in the result set");
            }
            
            count += 1;
        }
    }
    
    // Print summary
    let summary = format!("Processed {} records, generated {} county code update queries", count, updated_count);
    ui::progress::print_with_progress(progress_bar, &format!("\x1b[32m{}\x1b[0m", summary));
    log::info!("{}", summary);
    
    Ok((count, updated_count))
}

// Helper function to find column index by name - more robust approach
fn find_column_index(batch: &TextRowSet, column_name: &str) -> Option<usize> {
    // Get the number of columns in the result set
    let num_cols = batch.num_cols();
    
    // We can't directly access column names, so we'll need an alternative approach
    // First, check if we're looking for specific columns we know we need
    if column_name == "zip" || column_name == "zip_code" {
        // Try to identify the zip column by looking at a sample row
        for i in 0..num_cols {
            // Look at the first row (if available)
            if batch.num_rows() > 0 {
                let value = String::from_utf8_lossy(batch.at(i, 0).unwrap_or(&[])).to_string();
                // Check if this looks like a zip code (5 digits, possibly followed by a dash and 4 digits)
                if value.len() >= 5 && value.chars().take(5).all(|c| c.is_digit(10)) {
                    return Some(i);
                }
            }
        }
    } else if column_name == "county" || column_name == "county_code" {
        // Try to identify the county column by looking at a sample row
        for i in 0..num_cols {
            // Look at the first row (if available)
            if batch.num_rows() > 0 {
                let value = String::from_utf8_lossy(batch.at(i, 0).unwrap_or(&[])).to_string();
                // Check if this looks like a county code (2 or 3 digits)
                if value.len() <= 3 && value.chars().all(|c| c.is_digit(10)) {
                    return Some(i);
                }
            }
        }
    } else if column_name == "key_field" {
        // Assuming key_field is the first column
        return Some(0);
    }
    
    // If we couldn't find a match through pattern recognition,
    // we'll fall back to a more generic approach
    None
}