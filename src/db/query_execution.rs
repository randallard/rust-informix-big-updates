use odbc_api::Connection;
use indicatif::ProgressBar;
use std::error::Error;
use std::fs;
use chrono::prelude::*;

use crate::db::query_types::{QueryRecord, QueryStatus, ErrorRecord};
use crate::files::json_handler::{save_query_file, read_query_files, save_error_file};
use crate::ui;

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
                // Success case - we assume rows were affected
                query_record.status = QueryStatus::Completed;
                query_record.result = Some("success - operation completed".to_string());
                query_record.timestamp = Some(current_time.clone());
                success_count += 1;
                
                log::info!("Query execution successful for key {}", query_record.key);
            },
            Ok(None) => {
                // No cursor returned, but no error - could be 0 rows affected
                // This is still considered a success, not an error
                query_record.status = QueryStatus::Completed;
                query_record.result = Some("success".to_string());
                query_record.timestamp = Some(current_time.clone());
                success_count += 1;
                
                // Just log as info, not as error
                log::info!("Query execution completed for key {} but no rows were affected", query_record.key);
            },
            Err(err) => {
                // Only this case is a true error - when ODBC returns an error
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
                
                // Only log actual ODBC errors
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