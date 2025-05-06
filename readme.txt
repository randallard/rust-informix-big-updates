# Informix Batch Processor
# Installation and Configuration Guide

## Overview

This utility connects to an Informix database via ODBC to perform batch updates on records. It can identify records that need updates, generate SQL queries, test them for syntax errors, and execute them with transaction support.

## Requirements

Before using this tool, ensure your system has:

1. Informix ODBC Driver installed
2. An ODBC Data Source Name (DSN) properly configured 
3. Database credentials with appropriate permissions

## Installation

1. Extract all files from the .zip archive to a directory on your machine
2. No additional installation is required - the executable is ready to use

## Configuration

The application can be configured using one of the following methods (in order of priority):

### 1. Environment Variables

Set the following environment variables with the IBP_ prefix:

```
IBP_ODBC_DSN=Your_DSN_Name
IBP_DB_USERNAME=your_username
IBP_DB_PASSWORD=your_password
IBP_KEY_FIELD_NAME=key_field
IBP_ZIP_FIELD_NAME=zip_code
IBP_COUNTY_FIELD_NAME=county
IBP_SELECTION_QUERY=SELECT key_field, field1, field2 FROM table_name WHERE condition = 't'
IBP_UPDATE_QUERY_TEMPLATE=UPDATE table_name SET field1 = 'new_value' WHERE key_field = '{{key}}'
IBP_BATCH_SIZE=100
IBP_TIMEOUT_SECONDS=30
IBP_DATA_PATH=processed_records.json
IBP_CHECK_AGAIN_AFTER=1800
```

### 2. Config File (config.toml)

Create a config.toml file in the same directory as the executable with the following content:

```
odbc_dsn = "Your_DSN_Name"
db_username = "your_username"
db_password = "your_password"
key_field_name = "key_field"
zip_field_name = "zip_code"
county_field_name = "county"
selection_query = "SELECT key_field, field1, field2 FROM table_name WHERE condition = 't'"
update_query_template = "UPDATE table_name SET field1 = 'new_value' WHERE key_field = '{{key}}'"
batch_size = 100
timeout_seconds = 30
data_path = "processed_records.json"
check_again_after = 1800
```

### 3. Batch File (for Windows)

For convenience, you can use the included run-ibp.bat file, which sets environment variables and runs the application:

```
@echo off
REM Set environment variables for Informix Batch Processor
set IBP_ODBC_DSN=Your_DSN_Name
set IBP_DB_USERNAME=your_username
set IBP_DB_PASSWORD=your_password
set IBP_DATA_PATH=processed_records.json
set IBP_CHECK_AGAIN_AFTER=1800
set IBP_KEY_FIELD_NAME=key_field
set IBP_ZIP_FIELD_NAME=zip_code
set IBP_COUNTY_FIELD_NAME=county

REM Run the application (will default to test mode)
informix-batch-processor.exe
```

## Configuration Parameters Explained

- `odbc_dsn`: The ODBC Data Source Name configured in your ODBC Data Source Administrator
- `db_username`: Database username
- `db_password`: Database password
- `key_field_name`: The primary key field name in your table
- `zip_field_name`: The field containing ZIP codes
- `county_field_name`: The field containing county codes
- `selection_query`: SQL query to select records for processing
- `update_query_template`: Template for update queries with {{key}} placeholder
- `batch_size`: Number of records to process in each batch
- `timeout_seconds`: Database operation timeout in seconds
- `data_path`: Path to store processed records information
- `check_again_after`: Time in seconds to wait before checking again in continuous mode

## ODBC Configuration

To set up your ODBC Data Source:

1. Open ODBC Data Source Administrator (32-bit or 64-bit, depending on the driver)
2. Go to "System DSN" tab and click "Add"
3. Select the Informix ODBC driver and click "Finish"
4. Configure the connection parameters:
   - Data Source Name: Enter the same name used in your configuration
   - Description: Optional description
   - Server: Your Informix server name
   - Service: Informix service port (typically 9088)
   - Protocol: TCP/IP
   - Database: Your database name
   - Client Locale: Your preferred locale (e.g., en_US.utf8)
5. Click "Test Connection" to verify settings
6. Click "OK" to save the DSN

## Running the Application

The executable supports several operation modes:

### Default Mode (Test Mode)

Simply run the executable without arguments:
```
informix-batch-processor.exe
```
This will generate queries and test them for syntax errors without executing them.

### Available Commands

Run the following commands by adding them after the executable name:

```
informix-batch-processor.exe run
```
- `generate`: Generate SQL queries based on selection criteria
- `execute`: Execute previously generated queries
- `test`: Test queries for syntax errors (default)
- `run`: Run both generation and execution phases
- `setup-test --count 1000`: Generate 1000 test records
- `clean-test`: Clean test data
- `update-county-codes`: Update county codes based on ZIP codes (3-digit FIPS)
- `update-county-code-from-countyfp`: Update county codes (2-digit county codes)

### Additional Options

- `--clean`: Clean existing query files before starting
  Example: `informix-batch-processor.exe --clean test`

## Output Files

The application creates a timestamped directory (`results_[unix_epoch]`) for each run containing:

1. Individual JSON files for each record/query
2. Consolidated error log (`errors.json`)
3. Log file (`batch_process.log`)
4. Processed records log (`processed_records.json`)

## Continuous Mode

Run in continuous mode to process records at regular intervals:
```
informix-batch-processor.exe run
```

In continuous mode:
- The application will sleep for the duration specified in `check_again_after`
- Press 'R' key to trigger an immediate check instead of waiting
- The next check time will be displayed in the console

## Troubleshooting

If you encounter issues:

1. Check the log file in the results directory (`batch_process.log`)
2. Verify ODBC connection settings
3. Ensure the database user has appropriate permissions
4. Check that your selection and update queries are valid SQL
5. Examine the errors.json file for specific query failures

## Common Errors

- "Failed to connect to database": Check DSN name, username, and password
- "Query syntax error": Verify your SQL queries in the configuration
- "Permission denied": Ensure the user has write access to the directory
- "ODBC driver not found": Install the Informix ODBC driver
- "Table not found": Verify table names in your queries

## Support

For issues or questions, please contact your system administrator or the developer.