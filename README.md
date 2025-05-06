# Rust CLI Informix ODBC Batch Processor

## Table of Contents
1. [Project Overview](#project-overview)
2. [Key Features](#key-features)
3. [Technical Requirements](#technical-requirements)
4. [Installation](#installation)
5. [Deployment Instructions](#deployment-instructions)
6. [Development Environment](#development-environment)
7. [Configuration](#configuration)
8. [Usage](#usage)
9. [Project Structure](#project-structure)
10. [Output Files](#output-files)
11. [Working with County and Zip Code Data](#working-with-county-and-zip-code-data)
12. [Interactive Features](#interactive-features)
13. [Error Handling](#error-handling)
14. [Code Organization](#code-organization)
15. [AI Chat File Collection](#ai-chat-file-collection)

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
- **Test data generation with county and zip code mappings for TDD**
- **County code correction based on ZIP code mapping (both 2-digit and 3-digit FIPS formats)**
- **Improved error handling** - queries that execute successfully but affect no rows are not treated as errors

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

## Deployment Instructions

To deploy the application to another machine:

1. **Compile the executable** using one of these methods:
   ```bash
   # For standard 64-bit build
   cargo build --release
   
   # For 32-bit build (if using 32-bit ODBC drivers)
   cargo build --release --target i686-pc-windows-msvc
   ```

2. **Create a deployment package** containing:
   - The executable (`target/release/informix-batch-processor.exe`)
   - A sample `config.toml` file (or use the batch file method)
   - The `run-ibp.bat` batch file (for Windows)
   - This README file

3. **Set up the target machine**:
   - Ensure the Informix ODBC driver is installed
   - Configure ODBC Data Source in Windows ODBC Data Source Administrator:
     1. Open ODBC Data Source Administrator (32-bit or 64-bit, depending on your driver)
     2. Go to "System DSN" tab and click "Add"
     3. Select the Informix ODBC driver and click "Finish"
     4. Configure the connection parameters:
        - Data Source Name: Enter the same name you'll use in your config
        - Server: Your Informix server name
        - Service: Informix service port (typically 9088)
        - Protocol: TCP/IP
        - Database: Your database name
     5. Test the connection before saving

4. **Configure the application** using one of these methods:
   - Create a `config.toml` file in the same directory as the executable
   - Set environment variables with the `IBP_` prefix
   - Edit the `run-ibp.bat` batch file with your specific settings

5. **Run the application**:
   - Double-click the `run-ibp.bat` file, or
   - Run from command line: `informix-batch-processor.exe [command] [options]`

For automated deployment, use the included PowerShell script:
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
    condition CHAR(1) DEFAULT 'f',
    county VARCHAR(3),
    zip_code VARCHAR(10)
);
-- Insert sample data one row at a time to avoid comma issues
INSERT INTO table_name VALUES ('key1', 'old_value1', 'data1', 't', NULL, NULL);
INSERT INTO table_name VALUES ('key2', 'old_value2', 'data2', 't', NULL, NULL);
INSERT INTO table_name VALUES ('key3', 'old_value3', 'data3', 't', NULL, NULL);
INSERT INTO table_name VALUES ('key4', 'old_value4', 'data4', 'f', NULL, NULL);
INSERT INTO table_name VALUES ('key5', 'old_value5', 'data5', 't', NULL, NULL);
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
odbc_dsn = "local informix"
db_username = "informix"
db_password = "in4mix"

# Field name mappings for your schema
key_field_name = "key_field"
zip_field_name = "zip_code"
county_field_name = "county"

# Query parameters
selection_query = "SELECT key_field, field1, field2 FROM table_name WHERE condition = 't'"
update_query_template = "UPDATE table_name SET field1 = 'new_value' WHERE key_field = '{{key}}'"
batch_size = 100
timeout_seconds = 30

# File paths and settings
data_path = "processed_records.json"
check_again_after = 1800  # 30 minutes in seconds
```

Alternatively, you can use environment variables with the `IBP_` prefix (e.g., `IBP_ODBC_DSN`, `IBP_KEY_FIELD_NAME`).

The configuration structure supports:

- **Database connection**: ODBC DSN, username, and password
- **Field mapping**: Customize field names for your schema (key_field_name, zip_field_name, county_field_name)
- **Query parameters**: Define selection and update query templates with placeholders
- **Batch processing**: Configure batch size and timeout
- **File management**: Data path for processed records
- **Operation interval**: Delay between checks in continuous mode

Configuration values are loaded with the following priority:
1. Environment variables (with IBP_ prefix)
2. Config file values (config.toml)
3. Default values defined in the code

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

# Generate test data with county and zip code mappings (1000 records by default)
informix-batch-processor.exe setup-test --count 1000

# Clean all test data
informix-batch-processor.exe clean-test

# Update county codes based on zip codes (using 3-digit FIPS codes)
informix-batch-processor.exe update-county-codes

# Update county codes based on zip codes (using 2-digit county codes)
informix-batch-processor.exe update-county-code-from-countyfp
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
set IBP_KEY_FIELD_NAME=key_field
set IBP_ZIP_FIELD_NAME=zip_code
set IBP_COUNTY_FIELD_NAME=county

REM Run the application (will default to test mode)
informix-batch-processor.exe
```

## Project Structure

```
informix-batch-processor/
├── src/
│   ├── main.rs
│   ├── db/
│   │   ├── mod.rs                  # Module definition
│   │   ├── connection.rs           # Database connection handling
│   │   ├── query.rs                # Facade for all query functionality
│   │   ├── query_types.rs          # Core query data structures
│   │   ├── query_generation.rs     # Query generation logic
│   │   ├── query_execution.rs      # Query execution logic
│   │   ├── query_testing.rs        # Query testing and validation
│   │   ├── county_operations.rs    # County/ZIP code operations
│   │   └── sql_helpers.rs          # SQL parsing and manipulation helpers
│   ├── files/
│   │   ├── json_handler.rs
│   │   ├── file_manager.rs
│   │   └── processed.rs
│   ├── ui/
│   │   └── progress.rs
│   ├── utils/
│   │   ├── mod.rs
│   │   └── test_data.rs
│   ├── tests/
│   │   └── mod.rs
│   ├── config.rs
│   └── zip_county_map.rs
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
     "result": "success - operation completed|success - no rows affected|error: message",
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

## Working with County and Zip Code Data

### Washington State ZIP Code to County Code Mapping

This application includes a comprehensive dataset mapping Washington State ZIP codes to:

1. Two-digit county codes (01-39)
2. Three-digit FIPS county codes (001-077)
3. County names (e.g., "King County", "Spokane County")

This mapping is used for:
- Generating test data with correct county codes
- Updating existing records to ensure county codes match ZIP codes
- Validating data integrity during processing

### County Code Formats

The application supports two different county code formats:

1. **Two-digit County Codes (01-39)**: These are traditional Washington State county codes used in many legacy systems. Use the `update-county-code-from-countyfp` command to update records with these codes.

2. **Three-digit FIPS County Codes (001-077)**: These are federal FIPS (Federal Information Processing Standard) county codes that are widely used for interoperability with federal systems. Use the `update-county-codes` command to update records with these codes.

### Test Data Generation

The test data generator:
1. Creates records with random values for most fields
2. Sets each record's zip_code to a valid zip code from Washington State
3. Sets the county field to the corresponding FIPS county code
4. Formats zip codes as 10-character strings with 5-digit zip plus random 4-digit extension

Example test record:
```
key_field: 'testkey_123'
field1: 'value_4567'
field2: 'data_789'
condition: 't'
county: '033'  # Stevens County FIPS code
zip_code: '99148-1234'  # ZIP code for Stevens County with random extension
```

This data is ideal for testing the application's ability to correctly handle county code and zip code mappings, which is a common requirement in government data processing applications.

### Setting Up Test Data

Before using the test data generation feature, you need to modify your test database schema:

1. Connect to your Informix container:
   ```bash
   docker exec -it ifx bash
   ```

2. Create a SQL script to modify your table (if you didn't include these fields initially):
   ```sql
   -- Create file add_county_fields.sql
   DATABASE stores;
   ALTER TABLE table_name ADD county VARCHAR(3);
   ALTER TABLE table_name ADD zip_code VARCHAR(10);
   ```

3. Run the SQL script:
   ```bash
   dbaccess sysmaster add_county_fields.sql
   ```

### Updating County Codes Based on ZIP Codes

The application provides two commands for updating county codes based on ZIP codes:

#### 1. Update with Three-digit FIPS County Codes

```bash
# Update county codes based on zip codes (using 3-digit FIPS county codes)
informix-batch-processor.exe update-county-codes
```

This command:
- Scans records with ZIP codes but potentially incorrect county codes
- Looks up the correct 3-digit FIPS county code for each ZIP code
- Identifies records where the county code doesn't match the expected FIPS code
- Generates and executes UPDATE statements to correct these mismatches

#### 2. Update with Two-digit County Codes

```bash
# Use the selection query from config to update records with 2-digit county codes
informix-batch-processor.exe update-county-code-from-countyfp
```

This command:
- Uses the selection query from your config file to find relevant records
- For each record with a ZIP code, looks up the corresponding 2-digit county code
- Generates SQL UPDATE statements to set the county field to the 2-digit code
- Prompts you to execute the updates immediately or save them for later

This command is particularly useful for:
- Legacy systems that use 2-digit county codes instead of FIPS codes
- Preparing data for integration with other Washington State systems
- Standardizing county codes across your database

### County Code Conversion Table

Here's a sample of the county code mapping used in the application:

| County Name | 2-Digit Code | FIPS Code |
|-------------|--------------|-----------|
| Adams       | 01           | 001       |
| Asotin      | 02           | 003       |
| Benton      | 03           | 005       |
| Chelan      | 04           | 007       |
| Clallam     | 05           | 009       |
| King        | 17           | 033       |
| Pierce      | 27           | 053       |
| Spokane     | 32           | 063       |
| Yakima      | 39           | 077       |

### Development with TDD

To use this feature in a Test-Driven Development workflow:

1. Set up your test database with the required fields
2. Generate test data using the `setup-test` command
3. Write tests that verify the application correctly maps ZIP codes to county FIPS codes
4. Run tests to verify your implementation works correctly
5. Clean test data using the `clean-test` command when finished

The test data generation leverages the zip_county_map.rs module, which provides a mapping between Washington State ZIP codes and county FIPS codes.

### Example Use Case: Fixing County-Zip Mismatches

A common real-world scenario is where records have incorrect county codes that don't match their zip codes. The test data generator intentionally creates records with the correct mappings, but you can modify it to introduce errors for testing your correction logic.

To test a solution for fixing county-zip mismatches:

1. Generate test data with correct mappings
2. Run a SQL update to corrupt some of the county values:
   ```sql
   -- Corrupt 20% of the county values for testing
   UPDATE table_name SET county = '001' 
   WHERE MOD(DBINFO('SQLCA.SQLERRD1'), 5) = 0 AND key_field LIKE 'testkey_%';
   ```
3. Develop your solution to detect and fix the mismatches
4. Test your solution against the corrupted data
5. Verify that the county codes now match their respective zip codes

### Testing County-ZIP Code Correction

To test the county code correction functionality:

1. Generate test data with FIPS county codes
   ```bash
   informix-batch-processor.exe setup-test --count 1000
   ```

2. Use the 2-digit county code update command to convert to 2-digit format
   ```bash
   informix-batch-processor.exe update-county-code-from-countyfp
   ```

3. Verify that county codes have been updated to 2-digit format
   ```sql
   SELECT COUNT(*) FROM table_name 
   WHERE key_field LIKE 'testkey_%' AND LENGTH(county) = 2;
   -- Should match the number of test records
   ```

This workflow allows you to easily convert between different county code formats based on ZIP codes.

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
- **Improved handling of zero-row-affected cases**: When a query executes successfully but affects zero rows, this is now treated as a success rather than an error, with a distinct success message

## Code Organization

The codebase has been restructured to follow a modular design pattern with clear separation of concerns:

- **Facade Pattern**: `db/query.rs` re-exports functionality from specialized modules, maintaining backward compatibility
- **Single Responsibility**: Each module has a specific, focused purpose
- **Organized Functionality**: Related code is grouped together for better maintainability
- **Improved Error Handling**: Clear distinction between actual errors and zero-affected-rows cases
- **Reduced File Sizes**: Breaking up large files makes the code easier to understand and maintain

## AI Chat File Collection

### Overview
This project includes a utility script to gather all source files into a single directory for easier sharing with AI assistants. This simplifies the process of providing context about your codebase when working with AI tools.

### Usage

1. Run the collection script from the project root:
   ```bash
   ./collect_files.sh
   ```

2. The script will:
   - Create an `ai-chat-files` directory in your project root
   - Copy all files from the `src` directory and its subdirectories
   - Rename files to preserve directory structure information (e.g., `src/db/connection.rs` becomes `db_connection.rs`)

3. You can now easily upload all files from the `ai-chat-files` directory to an AI assistant for better context when discussing your codebase.