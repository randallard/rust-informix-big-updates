use std::error::Error;
use std::fs;
use std::path::Path;

/// Setup the necessary directory structure
pub fn setup_directories(results_dir: &str, clean: bool) -> Result<(), Box<dyn Error>> {
    let results_path = Path::new(results_dir);
    
    // If clean flag is set, remove the directory if it exists
    if clean && results_path.exists() {
        fs::remove_dir_all(results_path)?;
        log::info!("Cleaned existing results directory: {}", results_dir);
    }
    
    // Create the results directory if it doesn't exist
    if !results_path.exists() {
        fs::create_dir_all(results_path)?;
        log::info!("Created results directory: {}", results_dir);
    }
    
    Ok(())
}

/// Check if a file exists
pub fn file_exists(file_path: &str) -> bool {
    Path::new(file_path).exists()
}

/// Count number of files in a directory with specific extension
pub fn count_files(dir_path: &str, extension: &str) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;
    
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && 
           path.extension().map_or(false, |ext| ext == extension) {
            count += 1;
        }
    }
    
    Ok(count)
}