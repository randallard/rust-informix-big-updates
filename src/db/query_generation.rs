use odbc_api::{buffers::TextRowSet, Connection, Cursor};
use indicatif::ProgressBar;
use std::error::Error;
use std::collections::HashMap;

use crate::config::AppConfig;
use crate::db::query_types::QueryRecord;
use crate::files::json_handler::save_query_file;
use crate::ui;

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
            let mut values = HashMap::new();
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
                status: crate::db::query_types::QueryStatus::Pending,
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