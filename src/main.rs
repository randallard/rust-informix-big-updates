mod db;
mod config;
mod files;
mod ui;
mod utils;
mod zip_county_map;

use clap::{Parser, Subcommand};
use db::query::prompt_user;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crossterm::event::{self, Event, KeyCode};
use std::io::{stdout, Write};
use std::fs::File;
use indicatif::ProgressBar;
use chrono::prelude::*;

use crate::config::AppConfig;
use crate::db::connection::create_connection;
use crate::db::query::{generate_queries, execute_queries};
use crate::files::file_manager::setup_directories;
use crate::files::processed::ProcessedRecords;
use crate::ui::progress::create_progress_bar;

#[derive(Parser)]
#[clap(author, version, about = "Informix Batch Processor CLI")]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,

    /// Clean existing query files before starting
    #[clap(short, long)]
    clean: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate SQL queries based on selection criteria
    Generate,
    
    /// Execute previously generated queries
    Execute,
    
    /// Test queries for syntax errors without executing them
    Test,
    
    /// Run both generation and execution phases
    Run,
    
    /// Setup test data with county and zip code mappings
    SetupTest {
        /// Number of test records to generate
        #[clap(short, long, default_value = "1000")]
        count: usize,
    },
    
    /// Clean test data
    CleanTest,
    
    /// Update county codes based on zip codes (using 3-digit FIPS codes)
    UpdateCountyCodes,
    
    /// Update county codes based on zip codes (using 2-digit county codes)
    UpdateCountyCodeFromCountyfp,
}

fn setup_logger(log_file: &str) -> Result<(), Box<dyn Error>> {
    // Create log file and directory if it doesn't exist
    let log_path = std::path::Path::new(log_file);
    
    // Create directory if needed
    if let Some(parent) = log_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    
    // Open log file with append mode
    let file = File::create(log_path)?;
    
    // Configure env_logger to use the file
    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(Box::new(file)))
        .filter_level(log::LevelFilter::Info) // Set default log level
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] {} - {}: {}",
                Local::now().format("%Y-%m-%dT%H:%M:%SZ"),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();
    
    log::info!("Logger initialized");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Create timestamp for result directory
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    
    let results_dir = format!("results_{}", timestamp);
    
    // Setup log file in the results directory
    let log_file = format!("{}/batch_process.log", results_dir);
    
    // Setup directories and clean if requested
    setup_directories(&results_dir, cli.clean)?;
    
    // Setup logger after directory is created
    setup_logger(&log_file)?;
    
    log::info!("Starting Informix Batch Processor");
    
    // Load configuration
    let app_config = AppConfig::from_env_or_file()
        .expect("Failed to load configuration");
    
    // Determine which command to run - default to Test command if none specified
    let command = cli.command.unwrap_or(Commands::Test);
    
    match command {
        Commands::Generate => {
            generate_query_phase(&app_config, &results_dir)?;
        },
        Commands::Execute => {
            execute_query_phase(&app_config, &results_dir)?;
        },
        Commands::Test => {
            // Run the generation phase first, then test
            generate_query_phase(&app_config, &results_dir)?;
            test_query_phase(&app_config, &results_dir)?;
        },
        Commands::Run => {
            run_continuous_mode(&app_config, &results_dir)?;
        },
        Commands::SetupTest { count } => {
            setup_test_data(&app_config, count)?;
        },
        Commands::CleanTest => {
            clean_test_data(&app_config)?;
        },
        Commands::UpdateCountyCodes => {
            update_county_codes(&app_config, &results_dir)?;
        },
        Commands::UpdateCountyCodeFromCountyfp => {  
            update_county_code_from_countyfp(&app_config, &results_dir)?;  
        },
    }

    log::info!("Batch processing completed successfully");
    println!("Batch processing completed successfully");
    
    Ok(())
}

fn generate_query_phase(config: &AppConfig, results_dir: &str) -> Result<(), Box<dyn Error>> {
    println!("Starting Query Generation Phase");
    log::info!("Starting Query Generation Phase");
    
    // Load processed records
    let mut processed_records = ProcessedRecords::load(&config.data_path);
    
    // Create database connection
    let connection = create_connection(config)?;
    
    // Create progress bar for query generation
    let progress_bar = create_progress_bar("Generating Queries");
    
    // Generate queries
    let count = generate_queries(&connection, config, results_dir, &progress_bar)?;
    
    // Save processed records
    processed_records.save(&config.data_path)?;
    
    progress_bar.finish_with_message(format!("Generated {} queries", count));
    
    Ok(())
}

fn execute_query_phase(config: &AppConfig, results_dir: &str) -> Result<(), Box<dyn Error>> {
    println!("Starting Query Execution Phase");
    log::info!("Starting Query Execution Phase");
    
    // Load processed records
    let mut processed_records = ProcessedRecords::load(&config.data_path);
    
    // Create database connection
    let connection = create_connection(config)?;
    
    // Create progress bar for query execution
    let progress_bar = create_progress_bar("Executing Queries");
    
    // Execute queries
    let (success_count, error_count) = execute_queries(&connection, results_dir, &progress_bar)?;
    
    // Save processed records
    processed_records.save(&config.data_path)?;
    
    progress_bar.finish_with_message(
        format!("Executed {} queries ({} successful, {} failed)", 
                success_count + error_count, success_count, error_count)
    );
    
    println!("Executed {} queries ({} successful, {} failed)", 
             success_count + error_count, success_count, error_count);
    
    Ok(())
}

fn test_query_phase(config: &AppConfig, results_dir: &str) -> Result<(), Box<dyn Error>> {
    println!("Starting Query Test Phase");
    log::info!("Starting Query Test Phase");
    
    // Create database connection
    let connection = create_connection(config)?;
    
    // Test the queries
    let progress_bar = create_progress_bar("Testing Queries");
    
    // Call the test_queries function that we'll create in db/query.rs
    let (valid_count, invalid_count) = db::query::test_queries(&connection, results_dir, &progress_bar)?;
    
    progress_bar.finish_with_message(
        format!("Tested {} queries ({} valid, {} invalid)", 
                valid_count + invalid_count, valid_count, invalid_count)
    );
    
    println!("Tested {} queries ({} valid, {} invalid)", 
             valid_count + invalid_count, valid_count, invalid_count);
    
    Ok(())
}

fn run_continuous_mode(config: &AppConfig, results_dir: &str) -> Result<(), Box<dyn Error>> {
    loop {
        // Run both phases
        generate_query_phase(config, results_dir)?;
        execute_query_phase(config, results_dir)?;
        
        // Disconnect from the database (will be reconnected in the next phase)
        
        let next_check_time = SystemTime::now() + Duration::from_secs(config.check_again_after);
        let datetime = chrono::DateTime::<chrono::Local>::from(next_check_time);
        println!(
            "Batch processing complete, checking again at: {}",
            datetime.format("%Y-%m-%d %H:%M:%S")
        );
        println!("(press 'R' to check again now)");

        // Sleep or listen for manual trigger (key press)
        let sleep_duration = Duration::from_secs(config.check_again_after);
        let mut time_passed = Duration::from_secs(0);
        let interval = Duration::from_millis(100); // Polling interval for keypress

        while time_passed < sleep_duration {
            if event::poll(interval)? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.code == KeyCode::Char('r') || key_event.code == KeyCode::Char('R') {
                        println!("Manual check triggered by key press...");
                        break; // Exit the sleep loop and run the check immediately
                    }
                }
            }
            time_passed += interval;
        }

        println!("Reconnecting to the database...");
    }
}

// Add to main.rs

fn setup_test_data(config: &AppConfig, count: usize) -> Result<(), Box<dyn Error>> {
    println!("Setting up test data...");
    log::info!("Setting up test data");
    
    // Create database connection
    let connection = create_connection(config)?;
    
    // Call the test data generator
    utils::test_data::generate_test_data(&connection, count)?;
    
    log::info!("Test data setup completed successfully");
    println!("Test data setup completed successfully");
    
    Ok(())
}

fn clean_test_data(config: &AppConfig) -> Result<(), Box<dyn Error>> {
    println!("Cleaning test data...");
    log::info!("Cleaning test data");
    
    // Create database connection
    let connection = create_connection(config)?;
    
    // Call the test data cleaner
    utils::test_data::clean_test_data(&connection)?;
    
    log::info!("Test data cleaned successfully");
    println!("Test data cleaned successfully");
    
    Ok(())
}

fn update_county_codes(config: &AppConfig, results_dir: &str) -> Result<(), Box<dyn Error>> {
    println!("Starting County Code Update Phase");
    log::info!("Starting County Code Update Phase");
    
    // Create database connection
    let connection = create_connection(config)?;
    
    // Create progress bar
    let progress_bar = create_progress_bar("Updating County Codes");
    
    // First, find records with mismatched county codes and generate update queries
    let (checked_count, mismatch_count) = db::query::update_county_by_zip(
        &connection, config, results_dir, &progress_bar
    )?;
    
    if mismatch_count > 0 {
        println!("Found {} records with mismatched county codes", mismatch_count);
        log::info!("Found {} records with mismatched county codes", mismatch_count);
        
        // Execute the update queries
        let (success_count, error_count) = execute_queries(&connection, results_dir, &progress_bar)?;
        
        progress_bar.finish_with_message(
            format!("Updated county codes: {} successful, {} failed", success_count, error_count)
        );
        
        println!("Updated county codes: {} successful, {} failed", success_count, error_count);
        log::info!("Updated county codes: {} successful, {} failed", success_count, error_count);
    } else {
        progress_bar.finish_with_message("No county code updates needed");
        println!("No county code updates needed. All records have correct county codes.");
        log::info!("No county code updates needed. All records have correct county codes.");
    }
    
    Ok(())
}

fn update_county_code_from_countyfp(config: &AppConfig, results_dir: &str) -> Result<(), Box<dyn Error>> {
    println!("Starting County Code Update from FIPS Phase");
    log::info!("Starting County Code Update from FIPS Phase");
    
    // Create database connection
    let connection = create_connection(config)?;
    
    // Create progress bar
    let progress_bar = create_progress_bar("Updating County Codes from FIPS");
    
    // Generate update queries for two-digit county codes
    let (checked_count, updated_count) = db::query::update_county_code_from_countyfp(
        &connection, config, results_dir, &progress_bar
    )?;
    
    if updated_count > 0 {
        println!("Generated {} county code update queries from {} records", updated_count, checked_count);
        log::info!("Generated {} county code update queries from {} records", updated_count, checked_count);
        
        // Ask user if they want to execute the queries
        let response = prompt_user("Do you want to execute the update queries now?");
        if response.to_uppercase().starts_with('Y') {
            // Execute the update queries
            let (success_count, error_count) = execute_queries(&connection, results_dir, &progress_bar)?;
            
            progress_bar.finish_with_message(
                format!("Updated county codes: {} successful, {} failed", success_count, error_count)
            );
            
            println!("Updated county codes: {} successful, {} failed", success_count, error_count);
            log::info!("Updated county codes: {} successful, {} failed", success_count, error_count);
        } else {
            progress_bar.finish_with_message("Update queries generated but not executed");
            println!("Update queries have been generated but not executed. You can run 'execute' command later to apply them.");
            log::info!("Update queries generated but not executed");
        }
    } else {
        progress_bar.finish_with_message("No county code updates needed");
        println!("No county code updates generated. Check your selection query and data.");
        log::info!("No county code updates generated");
    }
    
    Ok(())
}