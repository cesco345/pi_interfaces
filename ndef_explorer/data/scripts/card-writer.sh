#!/bin/bash

# Card Writer Script - Process data from Android and write to cards
# This script is similar to transfer-script.sh but uses the new card_writer binary

# Define colors for terminal output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}=======================================${NC}"
echo -e "${GREEN}Smart Card Writer Tool${NC}"
echo -e "${GREEN}=======================================${NC}"
echo ""

# Function to check if a command exists
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# Check if qrencode and zbarcam are installed
if ! command_exists qrencode || ! command_exists zbarcam; then
  echo -e "${YELLOW}QR code utilities not found. Installing...${NC}"
  sudo apt-get update && sudo apt-get install -y qrencode zbar-tools
fi

echo "This tool allows you to write card data from your phone to physical cards."
echo "It supports multiple card types and will attempt to automatically determine the best approach."
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
    
    # Run card_writer with the scanned data
    echo "Processing card data..."
    cargo run --bin card_writer "$temp_file"
    
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
    
    # Run card_writer with the provided file
    echo "Processing card data..."
    cargo run --bin card_writer "$file_path"
    ;;
  
  *)
    echo -e "${RED}Invalid option. Exiting.${NC}"
    exit 1
    ;;
esac

echo ""
echo -e "${GREEN}Thank you for using the Smart Card Writer Tool!${NC}"
