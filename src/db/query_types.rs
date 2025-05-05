use serde::{Deserialize, Serialize};
use std::io::{self, Write};

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

// Utility function for user prompts
pub fn prompt_user(question: &str) -> String {
    print!("{} (Y/N): ", question);
    io::stdout().flush().expect("Failed to flush stdout");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read input");
    input.trim().to_string()
}