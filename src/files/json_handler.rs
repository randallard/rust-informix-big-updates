use serde::Serialize;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use crate::db::query::{QueryRecord, ErrorRecord};

/// Save a query record to a JSON file
pub fn save_query_file<P: AsRef<Path>>(file_path: P, query_record: &QueryRecord) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(query_record)?;
    
    let mut file = File::create(file_path)?;
    file.write_all(json.as_bytes())?;
    
    Ok(())
}

/// Read all query files from a directory
pub fn read_query_files(dir_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut query_files = Vec::new();
    
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && 
           path.extension().map_or(false, |ext| ext == "json") && 
           path.file_name().map_or(false, |name| name != "errors.json") {
            query_files.push(path);
        }
    }
    
    Ok(query_files)
}

/// Save an error record to the errors.json file
pub fn save_error_file<P: AsRef<Path>>(file_path: P, error_record: &ErrorRecord) -> Result<(), Box<dyn Error>> {
    // Create or open the errors file
    let file_path = file_path.as_ref();
    
    // If the file exists, read existing errors
    let mut errors: Vec<ErrorRecord> = if file_path.exists() {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)?
    } else {
        Vec::new()
    };
    
    // Now that ErrorRecord derives Clone, we can simply clone it
    errors.push(error_record.clone());
    
    // Write back to file
    let json = serde_json::to_string_pretty(&errors)?;
    
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;
        
    file.write_all(json.as_bytes())?;
    
    Ok(())
}

/// Read a query record from a file
pub fn read_query_file<P: AsRef<Path>>(file_path: P) -> Result<QueryRecord, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let query_record = serde_json::from_reader(reader)?;
    
    Ok(query_record)
}