use odbc_api::{buffers::TextRowSet, Connection, Cursor};
use indicatif::ProgressBar;
use std::error::Error;

use crate::config::AppConfig;
use crate::db::query_types::{QueryRecord, QueryStatus};
use crate::db::sql_helpers::{find_column_index_by_name, extract_table_name};
use crate::files::json_handler::save_query_file;
use crate::ui;

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