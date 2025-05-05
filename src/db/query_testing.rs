use odbc_api::Connection;
use indicatif::ProgressBar;
use std::error::Error;
use std::fs;

use crate::db::query_types::QueryRecord;
use crate::files::json_handler::read_query_files;
use crate::ui;

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
pub fn basic_sql_validation(query: &str) -> bool {
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