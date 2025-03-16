#!/bin/bash

# MFRC522 Test Script
# This script runs a series of SPI commands to test the MFRC522 RFID module

# Text colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== MFRC522 RFID Module Test Script ===${NC}"
echo -e "${YELLOW}This script will run a series of SPI commands to test your MFRC522 module${NC}"
echo -e "${YELLOW}Ensure the module is properly connected to your Raspberry Pi SPI interface${NC}"
echo

# Function to run a test and display the result
run_test() {
    local name=$1
    local cmd=$2
    local description=$3
    
    echo -e "${BLUE}[$name]${NC} $description"
    echo -e "Running: $cmd"
    echo -e "${YELLOW}Output:${NC}"
    eval $cmd
    echo -e "${GREEN}Test completed${NC}"
    echo
    sleep 1
}

# Ask which SPI bus and chip select to use
read -p "Enter SPI bus number (0 or 1, default: 0): " spi_bus
spi_bus=${spi_bus:-0}

read -p "Enter chip select (0, 1, or 2, default: 0): " chip_select
chip_select=${chip_select:-0}

# Build the base command with the specified bus and chip select
base_cmd="cargo run --bin spi_explorer_tool -- -b $spi_bus -c $chip_select"

echo -e "${GREEN}Starting tests with SPI$spi_bus, CS$chip_select${NC}"
echo

# 1. VERSION CHECK - Read chip version
run_test "VERSION" "$base_cmd -t xfer -L 2 -F 0xB7 -r 1" "Reading MFRC522 version register (expected: 0x91 or 0x92)"

# 2. SOFT RESET - Reset the device
run_test "RESET" "$base_cmd -t xfer -L 2 -F 0x02 -F 0x0F -r 1" "Performing soft reset"

# 3. STATUS CHECK - Read status register
run_test "STATUS" "$base_cmd -t xfer -L 2 -F 0x8F -r 1" "Reading status register"

# 4. ERROR CHECK - Read error register
run_test "ERROR" "$base_cmd -t xfer -L 2 -F 0x8D -r 1" "Reading error register (0x00 means no errors)"

# 5. SET ANTENNA ON - Turn on the RF field
run_test "ANTENNA_ON" "$base_cmd -t xfer -L 2 -F 0x28 -F 0x03 -r 1" "Turning antenna ON"

# Ask if user wants to test card detection
read -p "Do you want to run a continuous card detection test? (y/n, default: n): " run_detect
run_detect=${run_detect:-n}

if [[ $run_detect == "y" || $run_detect == "Y" ]]; then
    echo -e "${GREEN}Running continuous card detection test. Press Ctrl+C to stop.${NC}"
    echo -e "${YELLOW}Bring an RFID card close to the reader...${NC}"
    
    # Clear FIFO buffer
    eval "$base_cmd -t xfer -L 2 -F 0x0A -F 0x80 -r 1 > /dev/null"
    
    # Set up card detection by checking FIFO status repeatedly
    while true; do
        # Read FIFO level
        fifo_level=$(eval "$base_cmd -t xfer -L 2 -F 0x95 -r 1" | grep "RX:" | head -1)
        
        # Read status register
        status=$(eval "$base_cmd -t xfer -L 2 -F 0x8F -r 1" | grep "RX:" | head -1)
        
        echo -e "FIFO Level: $fifo_level | Status: $status"
        sleep 0.5
    done
fi

# 6. SET ANTENNA OFF - Turn off the RF field (cleanup)
run_test "ANTENNA_OFF" "$base_cmd -t xfer -L 2 -F 0x28 -F 0x00 -r 1" "Turning antenna OFF"

echo -e "${GREEN}All tests completed!${NC}"
echo -e "${YELLOW}Note: For full functionality, you'll need a proper MFRC522 library${NC}"
