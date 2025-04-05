#!/bin/bash

# DESFire Card Data Transfer Script
# This script helps transfer card data from your phone to physical cards using the Raspberry Pi

# Define colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=======================================${NC}"
echo -e "${GREEN}DESFire Card Data Transfer Tool${NC}"
echo -e "${GREEN}=======================================${NC}"
echo ""

# Function to check if a command exists
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# Check if qrencode is installed
if ! command_exists qrencode || ! command_exists zbarcam; then
  echo -e "${YELLOW}QR code utilities not found. Installing...${NC}"
  sudo apt-get update && sudo apt-get install -y qrencode zbar-tools
fi

echo "This tool allows you to transfer card data from your phone to physical cards."
echo ""
echo "Two methods are available:"
echo "1. Scan a QR code from your phone"
echo "2. Provide a JSON file with card data"
echo ""

read -p "Choose a method (1/2): " method

case $method in
  1)
    echo -e "${YELLOW}Starting QR code scanner...${NC}"
    echo "Please display the QR code from your phone."
    
    # Create a temporary file to store the QR code data
    temp_file=$(mktemp)
    
    # Use zbarcam to scan QR code and save to temp file
    zbarcam --raw -q --prescale=320x240 > "$temp_file"
    
    echo -e "${GREEN}QR code scanned successfully!${NC}"
    
    # Run ndef_reader with the import option and the scanned data
    echo "Processing card data..."
    cargo run --bin focused_ndef_reader -- --import "$temp_file"
    
    # Clean up
    rm "$temp_file"
    ;;
  
  2)
    echo "Please provide the path to the JSON file:"
    read -p "File path: " file_path
    
    if [ ! -f "$file_path" ]; then
      echo -e "${RED}Error: File not found${NC}"
      exit 1
    fi
    
    # Run ndef_reader with the import option
    echo "Processing card data..."
    cargo run --bin focused_ndef_reader -- --import "$file_path"
    ;;
  
  *)
    echo -e "${RED}Invalid option. Exiting.${NC}"
    exit 1
    ;;
esac

echo ""
echo -e "${GREEN}Thank you for using the DESFire Card Data Transfer Tool!${NC}"
