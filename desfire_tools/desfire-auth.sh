#!/bin/bash
# DESFire Authentication Response Calculator
# This script calculates the proper response to a DESFire authentication challenge

# Default parameter values
CHALLENGE=""
KEY="0000000000000000"
RNDA="0001020304050607"
VERBOSE=0

# Help function
function show_help {
  echo "Usage: $0 -c CHALLENGE [-k KEY] [-r RNDA] [-v]"
  echo
  echo "Parameters:"
  echo "  -c CHALLENGE   The challenge received from the card (hex string without spaces)"
  echo "  -k KEY         The authentication key (hex string, default: all zeros)"
  echo "  -r RNDA        The random number A to use (hex string, default: 0001020304050607)"
  echo "  -v             Verbose mode, shows intermediate values"
  echo "  -h             Show this help"
  echo
  echo "Example: $0 -c 5D62A5CF70BD4582"
}

# Parse command line arguments
while getopts "c:k:r:vh" opt; do
  case $opt in
    c) CHALLENGE="$OPTARG" ;;
    k) KEY="$OPTARG" ;;
    r) RNDA="$OPTARG" ;;
    v) VERBOSE=1 ;;
    h) show_help; exit 0 ;;
    *) show_help; exit 1 ;;
  esac
done

# Check if challenge is provided
if [ -z "$CHALLENGE" ]; then
  echo "Error: Challenge is required"
  show_help
  exit 1
fi

# Remove any spaces from the hex strings
CHALLENGE=$(echo "$CHALLENGE" | tr -d ' ')
KEY=$(echo "$KEY" | tr -d ' ')
RNDA=$(echo "$RNDA" | tr -d ' ')

# Check if the challenge has the correct length (16 hex chars = 8 bytes)
if [ ${#CHALLENGE} -ne 16 ]; then
  echo "Error: Challenge must be exactly 8 bytes (16 hex chars)"
  exit 1
fi

# Create temporary directory
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

# Convert hex strings to binary
echo -n "$CHALLENGE" | xxd -r -p > "$TEMP_DIR/challenge.bin"
echo -n "$KEY" | xxd -r -p > "$TEMP_DIR/key.bin"
echo -n "$RNDA" | xxd -r -p > "$TEMP_DIR/rndA.bin"

# Step 1: Decrypt the challenge with the key
openssl enc -des-cbc -d -nopad -K "$KEY" -iv 0000000000000000 -in "$TEMP_DIR/challenge.bin" -out "$TEMP_DIR/decrypted_rndB.bin" 2>/dev/null

# Step 2: Rotate the decrypted rndB left by one byte
# Read the decrypted rndB into a byte array
RNDB=$(xxd -p "$TEMP_DIR/decrypted_rndB.bin" | tr -d '\n')
if [ $VERBOSE -eq 1 ]; then
  echo "Decrypted rndB: $RNDB"
fi

# Rotate left (first byte moves to the end)
# Each byte is 2 hex chars, so take bytes 2-8 and add byte 1 at the end
ROTATED_RNDB="${RNDB:2}${RNDB:0:2}"
if [ $VERBOSE -eq 1 ]; then
  echo "Rotated rndB: $ROTATED_RNDB"
fi

# Convert rotated rndB back to binary
echo -n "$ROTATED_RNDB" | xxd -r -p > "$TEMP_DIR/rotated_rndB.bin"

# Step 3: Concatenate rndA and rotated rndB
cat "$TEMP_DIR/rndA.bin" "$TEMP_DIR/rotated_rndB.bin" > "$TEMP_DIR/rndA_rotatedRndB.bin"

# Step 4: Encrypt the concatenated data
# DESFire protocol uses all zeros IV for the response encryption
openssl enc -des-cbc -e -nopad -K "$KEY" -iv 0000000000000000 -in "$TEMP_DIR/rndA_rotatedRndB.bin" -out "$TEMP_DIR/encrypted_response.bin" 2>/dev/null

# Get the encrypted response bytes
ENCRYPTED_RESPONSE=$(xxd -p "$TEMP_DIR/encrypted_response.bin" | tr -d '\n')
if [ $VERBOSE -eq 1 ]; then
  echo "rndA: $RNDA"
  echo "Challenge: $CHALLENGE"
  echo "Key: $KEY"
  echo "Encrypted response: $ENCRYPTED_RESPONSE"
fi

# Format the final authentication APDU command
echo "Authentication APDU command:"
echo "90 AF 00 00 10 $(echo $ENCRYPTED_RESPONSE | sed 's/.\{2\}/& /g') 00"
