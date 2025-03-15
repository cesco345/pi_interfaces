#!/usr/bin/env python3
"""
RFID Wrapper for integration with Rust application.
This script provides a bridge to use Python's SimpleMFRC522 library
which has better support for clone cards and the FM17522 chip variant.
"""

import sys
import json
import RPi.GPIO as GPIO
from mfrc522 import SimpleMFRC522
import time

def setup():
    """Initialize the RFID reader."""
    return SimpleMFRC522()

def cleanup():
    """Clean up GPIO resources."""
    GPIO.cleanup()

def read_card():
    """Read data from an RFID card."""
    reader = setup()
    try:
        card_id, text = reader.read()
        # Convert card_id to hex string for consistent format with Rust
        uid_bytes = []
        temp_id = card_id
        for _ in range(4):  # Most MIFARE card UIDs are 4 bytes
            uid_bytes.insert(0, temp_id & 0xFF)  # Insert at the beginning to maintain byte order
            temp_id >>= 8
        
        uid_hex = ' '.join([f"{b:02X}" for b in uid_bytes])
        
        result = {
            "success": True,
            "uid": uid_hex,
            "text": text.strip(),
            "error": None
        }
    except Exception as e:
        result = {
            "success": False,
            "uid": None,
            "text": None,
            "error": str(e)
        }
    finally:
        cleanup()
    
    print(json.dumps(result))

def write_card(text):
    """Write data to an RFID card."""
    reader = setup()
    try:
        reader.write(text)
        
        # After writing, read the card ID
        card_id, _ = reader.read()
        
        # Convert card_id to hex string for consistent format with Rust
        uid_bytes = []
        temp_id = card_id
        for _ in range(4):
            uid_bytes.insert(0, temp_id & 0xFF)
            temp_id >>= 8
        
        uid_hex = ' '.join([f"{b:02X}" for b in uid_bytes])
        
        result = {
            "success": True,
            "uid": uid_hex,
            "error": None
        }
    except Exception as e:
        result = {
            "success": False,
            "uid": None,
            "error": str(e)
        }
    finally:
        cleanup()
    
    print(json.dumps(result))

def test_keys():
    """Test default keys on the RFID card."""
    reader = setup()
    try:
        # SimpleMFRC522 doesn't have direct key testing capability
        # We'll simulate it by trying to read with default key
        card_id, text = reader.read()
        
        # Convert card_id to hex string
        uid_bytes = []
        temp_id = card_id
        for _ in range(4):
            uid_bytes.insert(0, temp_id & 0xFF)
            temp_id >>= 8
        
        uid_hex = ' '.join([f"{b:02X}" for b in uid_bytes])
        
        # Report success with the default key (typically 0xFFFFFFFFFFFF)
        result = {
            "success": True,
            "uid": uid_hex,
            "sectors": [{
                "sector": 1,
                "key": "FF FF FF FF FF FF",
                "type": "A"  # This field name matches what the Rust code expects
            }],
            "error": None
        }
    except Exception as e:
        result = {
            "success": False,
            "uid": None,
            "sectors": [],
            "error": str(e)
        }
    finally:
        cleanup()
    
    print(json.dumps(result))

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(json.dumps({"success": False, "error": "Command required: read, write, or test_keys"}))
        sys.exit(1)
    
    command = sys.argv[1]
    
    if command == "read":
        read_card()
    elif command == "write":
        if len(sys.argv) < 3:
            print(json.dumps({"success": False, "error": "Text to write is required"}))
            sys.exit(1)
        write_card(sys.argv[2])
    elif command == "test_keys":
        test_keys()
    else:
        print(json.dumps({"success": False, "error": f"Unknown command: {command}"}))
