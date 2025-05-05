use odbc_api::{buffers::TextRowSet, Connection, Cursor, IntoParameter};
use indicatif::ProgressBar;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::io::{self, Write};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::db::connection::create_connection;
use crate::files::json_handler::{save_query_file, read_query_files, save_error_file};
use crate::ui;
use crate::ui::progress::create_progress_bar;

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
        
        // Execute the query
        let current_time = Utc::now().to_rfc3339();
        match conn.execute(&query_record.query, ()) {
            Ok(Some(_cursor)) => {
                // For UPDATE, INSERT, DELETE, assume success if we got a cursor without error
                // Success case - we assume 1 row was affected
                query_record.status = QueryStatus::Completed;
                query_record.result = Some("success - operation completed".to_string());
                query_record.timestamp = Some(current_time.clone());
                success_count += 1;
                
                log::info!("Query execution successful for key {}", query_record.key);
            },
            Ok(None) => {
                // No cursor returned, but no error - could be 0 rows affected
                query_record.status = QueryStatus::Failed;
                query_record.result = Some("error: No rows updated".to_string());
                query_record.timestamp = Some(current_time.clone());
                
                // Add to error log
                let error_record = ErrorRecord {
                    key: query_record.key.clone(),
                    file: file_path.file_name().unwrap().to_string_lossy().to_string(),
                    error: "Expected 1 row to be updated, but 0 rows were affected".to_string(),
                    timestamp: current_time.clone(),
                };
                
                save_error_file(&format!("{}/errors.json", results_dir), &error_record)?;
                error_count += 1;
                
                log::error!("Query execution failed for key {}: No rows updated", query_record.key);
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
                    timestamp: current_time.clone(),
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

pub fn update_county_code_from_countyfp(
    conn: &Connection,
    config: &AppConfig,
    results_dir: &str,
    progress_bar: &ProgressBar,
) -> Result<(usize, usize), Box<dyn Error>> {
    ui::progress::update_message(progress_bar, "Finding records with county codes to update...");
    
    // Load the zip-county mapping
    let zip_county_map = crate::zip_county_map::load_zip_county_map();
    
    // Query to find records with zip codes but potentially incorrect county codes
    let selection_query = &config.selection_query;
    
    // Execute the selection query
    let cursor = match conn.execute(selection_query, ())? {
        Some(cursor) => cursor,
        None => {
            ui::progress::print_with_progress(progress_bar, "No records found with selection query.");
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
    
    ui::progress::print_with_progress(progress_bar, "Generating update queries for records with county codes...");
    
    // Process each batch of rows
    while let Some(batch) = row_set_cursor.fetch()? {
        total_records += batch.num_rows();
        progress_bar.set_length(total_records as u64);
        
        // Parse column indices from the selection query
        let key_col_idx = find_column_index_by_name(&config.selection_query, &config.key_field_name);
        let county_col_idx = find_column_index_by_name(&config.selection_query, &config.county_field_name);
        let zip_col_idx = find_column_index_by_name(&config.selection_query, &config.zip_field_name);
        
        log::info!("Using column indices: key_field={}, county_field={}, zip_field={}",
                  key_col_idx, county_col_idx, zip_col_idx);
        
        for row_index in 0..batch.num_rows() {
            progress_bar.set_position(count as u64);
            
            // Get key field value (always use the first column as the key)
            let key_field = String::from_utf8_lossy(batch.at(key_col_idx, row_index).unwrap_or(&[])).to_string();
            
            // Get zip code and current county code from the determined column indices
            let zip_code = String::from_utf8_lossy(batch.at(zip_col_idx, row_index).unwrap_or(&[])).to_string();
            let current_county = String::from_utf8_lossy(batch.at(county_col_idx, row_index).unwrap_or(&[])).to_string();
            
            // Skip if no zip code
            if zip_code.is_empty() {
                continue;
            }
            
            // Extract 5-digit zip from zip+4 if needed
            let zip5 = if zip_code.contains('-') {
                zip_code.split('-').next().unwrap_or("").to_string()
            } else if zip_code.len() >= 5 {
                zip_code[0..5].to_string()
            } else {
                zip_code.clone()
            };
            
            // Update progress bar message but don't print to console
            ui::progress::update_message(progress_bar, 
                format!("Checking key: {}, zip: {}, county: {}", key_field, zip5, current_county));
            
            // Look up the correct county code for this zip
            if let Some(zip_info) = zip_county_map.get(&zip5) {
                let correct_county_code = &zip_info.county_code;
                
                // Only generate update query if county code doesn't match the correct county code
                if current_county != *correct_county_code {
                    mismatch_count += 1;
                    
                    // Generate update query using the table name from the selection_query
                    let table_name = extract_table_name(&config.selection_query);
                    
                    // Generate update query with the correct field names from config
                    let query = format!(
                        "UPDATE {} SET {} = '{}' WHERE {} = '{}'",
                        table_name, 
                        config.county_field_name,
                        correct_county_code, 
                        config.key_field_name, 
                        key_field
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
                    
                    log::info!("Generated update query for key: {}, changing county from '{}' to '{}' where zip starts with '{}'", 
                              key_field, current_county, correct_county_code, zip5);
                }
            }
            
            count += 1;
        }
    }
    
    // Print summary
    let summary = format!("Checked {} records, found {} with county codes to update", count, mismatch_count);
    ui::progress::print_with_progress(progress_bar, &format!("\x1b[32m{}\x1b[0m", summary));
    log::info!("{}", summary);
    
    Ok((count, mismatch_count))
}

// Helper function to find column index by position (for key field)
fn find_column_index_by_position(batch: &TextRowSet, default_position: usize) -> usize {
    // Return the default position, but make sure it's within the valid range
    let num_cols = batch.num_cols();
    if default_position < num_cols {
        default_position
    } else {
        0 // Fallback to first column if default is out of range
    }
}

// Find column index based on field name and query structure
fn find_column_index_by_name(query: &str, field_name: &str) -> usize {
    // Parse the SELECT statement to extract column names
    if let Some(select_pos) = query.to_uppercase().find("SELECT ") {
        if let Some(from_pos) = query.to_uppercase().find(" FROM ") {
            let columns_str = &query[(select_pos + 7)..from_pos];
            let columns: Vec<&str> = columns_str.split(',').map(|s| s.trim()).collect();
            
            // Find the position of the field name in the columns list
            for (i, col) in columns.iter().enumerate() {
                // Check if the column name exactly matches the field name
                // or if it ends with the field name (e.g., "table.field_name")
                if col.to_lowercase() == field_name.to_lowercase() || 
                   col.to_lowercase().ends_with(&format!(".{}", field_name.to_lowercase())) {
                    return i;
                }
            }
            
            // If we got here, the field name wasn't found in the columns
            panic!("Field name '{}' not found in SELECT statement: {}", field_name, columns_str);
        } else {
            panic!("Invalid SELECT statement: FROM clause not found in query: {}", query);
        }
    } else {
        panic!("Invalid query: SELECT statement not found in query: {}", query);
    }
}

// Helper function to find column index by data pattern or position
fn find_column_index_by_pattern(batch: &TextRowSet, field_name: &str, default_position: usize) -> usize {
    let num_cols = batch.num_cols();
    
    // If the default position is valid, use it as a fallback
    let fallback = if default_position < num_cols { default_position } else { 0 };
    
    // For zip fields, look for a column that contains zip-like data (5 digits, maybe a dash)
    if field_name.contains("zip") && batch.num_rows() > 0 {
        for i in 0..num_cols {
            let value = String::from_utf8_lossy(batch.at(i, 0).unwrap_or(&[])).to_string();
            if value.len() >= 5 && value.chars().take(5).all(|c| c.is_digit(10)) {
                return i;
            }
        }
    }
    
    // For county fields, look for a column with values that match county code patterns (2-3 digits)
    if field_name.contains("county") && batch.num_rows() > 0 {
        for i in 0..num_cols {
            let value = String::from_utf8_lossy(batch.at(i, 0).unwrap_or(&[])).to_string();
            if (value.len() == 2 || value.len() == 3) && value.chars().all(|c| c.is_digit(10)) {
                return i;
            }
        }
    }
    
    // If we couldn't identify the column by pattern, return the fallback position
    fallback
}

// Extract table name from SQL query
fn extract_table_name(query: &str) -> String {
    // Simple implementation to extract table name from SELECT query
    // Example: "SELECT field1, field2 FROM table_name WHERE condition"
    let query = query.trim().to_uppercase();
    
    if let Some(from_pos) = query.find(" FROM ") {
        let after_from = &query[from_pos + 6..];
        if let Some(where_pos) = after_from.find(" WHERE ") {
            return after_from[..where_pos].trim().to_string();
        } else if let Some(limit_pos) = after_from.find(" LIMIT ") {
            return after_from[..limit_pos].trim().to_string();
        } else {
            return after_from.trim().to_string();
        }
    }
    
    // Fallback to "table_name" if we can't extract it
    "table_name".to_string()
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