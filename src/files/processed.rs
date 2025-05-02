use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

/// Struct to track processed records to avoid reprocessing
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ProcessedRecords {
    pub processed: Vec<ProcessedRecord>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProcessedRecord {
    pub key: String,
    pub timestamp: String,
    pub action: String, // "skipped" or "updated"
}

impl ProcessedRecords {
    /// Load processed records from file
    pub fn load(file_path: &str) -> Self {
        if Path::new(file_path).exists() {
            match fs::read_to_string(file_path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(records) => records,
                        Err(e) => {
                            eprintln!("Error parsing processed records file: {}", e);
                            Self::default()
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Error reading processed records file: {}", e);
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }

    /// Save processed records to file
    pub fn save(&self, file_path: &str) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self).expect("Failed to serialize processed records");
        fs::write(file_path, json)
    }

    /// Add a processed record
    pub fn add(&mut self, key: String, timestamp: String, action: String) {
        let record = ProcessedRecord {
            key,
            timestamp,
            action,
        };
        
        if !self.processed.contains(&record) {
            self.processed.push(record);
        }
    }

    /// Check if a record has been processed
    pub fn is_processed(&self, key: &str) -> bool {
        self.processed.iter().any(|r| r.key == key)
    }
    
    /// Get action for a record if it has been processed
    pub fn get_action(&self, key: &str) -> Option<String> {
        self.processed.iter()
            .find(|r| r.key == key)
            .map(|r| r.action.clone())
    }
}