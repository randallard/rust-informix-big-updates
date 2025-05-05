use odbc_api::buffers::TextRowSet;

// Helper function to find column index by position (for key field)
pub fn find_column_index_by_position(batch: &TextRowSet, default_position: usize) -> usize {
    // Return the default position, but make sure it's within the valid range
    let num_cols = batch.num_cols();
    if default_position < num_cols {
        default_position
    } else {
        0 // Fallback to first column if default is out of range
    }
}

// Find column index based on field name and query structure
pub fn find_column_index_by_name(query: &str, field_name: &str) -> usize {
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
pub fn find_column_index_by_pattern(batch: &TextRowSet, field_name: &str, default_position: usize) -> usize {
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
pub fn extract_table_name(query: &str) -> String {
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
pub fn find_column_index(batch: &TextRowSet, column_name: &str) -> Option<usize> {
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