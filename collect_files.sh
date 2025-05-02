#!/bin/bash

# Script to gather all files from src directory and its subdirectories
# and save them to ai-chat-files directory with modified filenames

# Create the ai-chat-files directory if it doesn't exist
mkdir -p ai-chat-files

# Find all files in the src directory and its subdirectories
find src -type f | while read -r file; do
    # Extract the directory part (excluding 'src/')
    dir_part=$(dirname "$file" | sed 's/^src\///')
    
    # Extract the filename part
    file_part=$(basename "$file")
    
    # Create the new filename
    if [ "$dir_part" = "src" ]; then
        # If the file is directly in the src directory, don't add a prefix
        new_filename="$file_part"
    else
        # Replace directory separators with underscores
        dir_prefix=$(echo "$dir_part" | tr '/' '_')
        new_filename="${dir_prefix}_${file_part}"
    fi
    
    # Copy the file to the ai-chat-files directory
    cp "$file" "ai-chat-files/$new_filename"
    
    echo "Copied $file to ai-chat-files/$new_filename"
done

cp README.md ai-chat-files
echo "Copied README.md to ai-chat-files/README.md"
cp config.toml ai-chat-files
echo "Copied config.toml to ai-chat-files/config.toml"

echo "All files have been collected in the ai-chat-files directory."