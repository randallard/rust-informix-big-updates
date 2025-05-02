# Rust CLI Informix ODBC Batch Processor

## Project Overview

This project is a command-line interface (CLI) tool built in Rust that connects to an Informix database via ODBC to perform batch updates on records. The tool follows a multi-phase approach:

1. **Query Generation Phase**: Identifies records that need updates, prompts for confirmation, and generates SQL queries, storing them in JSON files.
2. **Query Testing Phase**: Tests the generated queries for syntax errors without executing them.
3. **Query Execution Phase**: Executes the generated queries with transaction support and updates the corresponding JSON files with results.

## Key Features

- Connect to Informix database using environment variables or TOML configuration
- **Automatic batch query generation** without prompting between each record
- Transaction support with rollback on failure and retry options
- Generate and store queries in individual JSON files within timestamped result directories
- Display progress with a progress bar for all phases
- Track and report errors in a separate error log file
- Maintain a record of processed queries to avoid reprocessing
- Continuous monitoring mode with configurable check intervals
- Manual trigger option to restart processing (press 'R')
- **Default test mode that automatically generates and tests queries**

## Technical Requirements

- Rust programming language
- Informix ODBC driver
- Environment variables for database connection configuration
- TOML configuration file support

## Installation

1. Ensure you have Rust and Cargo installed (https://rustup.rs/)
2. Clone this repository
3. Build the project:

```bash
cargo build --release
# For 32-bit build (if needed for 32-bit ODBC drivers)
cargo build --release --target i686-pc-windows-msvc
```

4. Run the setup script to create deployment files:

```bash
powershell -ExecutionPolicy Bypass -File setup-deployment.ps1
```

## Development Environment

### Setting Up Informix Container for Development

To set up a local Informix database for development and testing:

1. Start an Informix development container:

```bash
docker run -td --name ifx -h ifx -p 9088:9088 -p 9089:9089 -p 27017:27017 -p 27018:27018 -p 27883:27883 -e LICENSE=accept ibmcom/informix-developer-database:latest
```

2. Execute into the container and create a setup.sql file:

```bash
docker exec -it ifx bash
```

3. Inside the container, create a setup.sql file with the following content:

```sql
-- Create the stores database
CREATE DATABASE stores;
-- Connect to the stores database
DATABASE stores;
-- Create a table that matches your config.json selection query
CREATE TABLE table_name (
    key_field VARCHAR(50) NOT NULL PRIMARY KEY,
    field1 VARCHAR(100),
    field2 VARCHAR(100),
    condition CHAR(1) DEFAULT 'f'
);
-- Insert sample data one row at a time to avoid comma issues
INSERT INTO table_name VALUES ('key1', 'old_value1', 'data1', 't');
INSERT INTO table_name VALUES ('key2', 'old_value2', 'data2', 't');
INSERT INTO table_name VALUES ('key3', 'old_value3', 'data3', 't');
INSERT INTO table_name VALUES ('key4', 'old_value4', 'data4', 'f');
INSERT INTO table_name VALUES ('key5', 'old_value5', 'data5', 't');
```

4. Run the SQL script:

```bash
dbaccess sysmaster setup.sql
```

5. Update your application's configuration to connect to this development database.

## Configuration

The application can be configured using a `config.toml` file:

```toml
# Database connection parameters
odbc_dsn = "UJMS Live"
db_username = "username"
db_password = "password"

# Query parameters
selection_query = "SELECT key_field, field1, field2 FROM table_name WHERE condition = 't'"
update_query_template = "UPDATE table_name SET field1 = '{{new_value}}' WHERE key_field = '{{key}}'"
batch_size = 100
timeout_seconds = 30

# File paths and settings
data_path = "processed_records.json"
check_again_after = 1800  # 30 minutes in seconds
```

Alternatively, you can use environment variables with the `IBP_` prefix (e.g., `IBP_ODBC_DSN`).

## Usage

The application supports several operation modes:

```bash
# Just run the application (defaults to test mode which first generates queries then tests them)
informix-batch-processor.exe

# Run both query generation and execution phases
informix-batch-processor.exe run

# Only generate queries
informix-batch-processor.exe generate

# Only execute previously generated queries
informix-batch-processor.exe execute

# Test queries for syntax errors (now also automatically generates queries first)
informix-batch-processor.exe test

# Clean previous result files and run test mode (generate + test)
informix-batch-processor.exe --clean
```

For Windows users, a batch file (`run-ibp.bat`) is provided for easy use:

```batch
@echo off
REM Set environment variables for Informix Batch Processor
set IBP_ODBC_DSN=UJMS Live
set IBP_DB_USERNAME=username
set IBP_DB_PASSWORD=password
set IBP_DATA_PATH=processed_records.json
set IBP_CHECK_AGAIN_AFTER=1800

REM Run the application (will default to test mode)
informix-batch-processor.exe
```

## Project Structure

```
informix-batch-processor/
├── src/
│   ├── main.rs
│   ├── db/
│   │   ├── connection.rs
│   │   └── query.rs
│   ├── files/
│   │   ├── json_handler.rs
│   │   ├── file_manager.rs
│   │   └── processed.rs
│   ├── ui/
│   │   └── progress.rs
│   └── config.rs
├── config.toml
├── run-ibp.bat
├── setup-deployment.ps1
├── results_[unix_epoch]/
│   ├── [record_files.json]
│   └── errors.json
├── Cargo.toml
└── README.md
```

## Output Files

The application creates a timestamped directory (`results_[unix_epoch]`) for each run:

1. Individual JSON files for each record/query:
   ```json
   {
     "key": "record_key",
     "query": "UPDATE statement",
     "status": "pending|completed|failed",
     "result": "success|error: message",
     "timestamp": "2025-04-28T14:30:00Z"
   }
   ```

2. Consolidated error log (`errors.json`):
   ```json
   [
     {
       "key": "record_key",
       "file": "record_key.json",
       "error": "Error message",
       "timestamp": "2025-04-28T14:30:00Z"
     }
   ]
   ```

3. Processed records log (`processed_records.json`):
   ```json
   {
     "processed": [
       {
         "key": "record_key",
         "timestamp": "2025-04-28T14:30:00Z",
         "action": "skipped|updated"
       }
     ]
   }
   ```

## Interactive Features

- **Batch query generation** now automatically processes all matching records without prompting
- Records are displayed in the terminal during processing for visibility
- The test phase validates query syntax without making any database changes
- During query execution, transaction failures can be retried
- In continuous mode, pressing 'R' triggers an immediate check instead of waiting for the timer

## Error Handling

- All database operations are performed within transactions
- Failed transactions are rolled back automatically
- Detailed error information is stored in both the record file and consolidated error log
- Users can retry failed operations without restarting the entire process

# AI Chat File Collection

## Overview
This project includes a utility script to gather all source files into a single directory for easier sharing with AI assistants. This simplifies the process of providing context about your codebase when working with AI tools.

## Usage

1. Run the collection script from the project root:
   ```bash
   ./collect_files.sh
   ```

2. The script will:
   - Create an `ai-chat-files` directory in your project root
   - Copy all files from the `src` directory and its subdirectories
   - Rename files to preserve directory structure information (e.g., `src/db/connection.rs` becomes `db_connection.rs`)

3. You can now easily upload all files from the `ai-chat-files` directory to an AI assistant for better context when discussing your codebase.

## Example

Original structure:
```
src/
├── main.rs
├── db/
│   ├── connection.rs
│   └── models.rs
└── utils/
    └── helpers.rs
```

After running the script, the `ai-chat-files` directory will contain:
```
ai-chat-files/
├── main.rs
├── db_connection.rs
├── db_models.rs
└── utils_helpers.rs
```

This flattened structure with descriptive filenames makes it easier to share your codebase context with AI assistants while maintaining information about the original file organization.