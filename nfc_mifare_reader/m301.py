#!/usr/bin/env python3
import evdev
import select
import time
import re

def key_to_char(keycode, shift):
    """Convert a key code to the correct character based on shift state"""
    # Numbers
    if keycode == 'KEY_0': return ')' if shift else '0'
    if keycode == 'KEY_1': return '!' if shift else '1'
    if keycode == 'KEY_2': return '@' if shift else '2'
    if keycode == 'KEY_3': return '#' if shift else '3'
    if keycode == 'KEY_4': return '$' if shift else '4'
    if keycode == 'KEY_5': return '%' if shift else '5'
    if keycode == 'KEY_6': return '^' if shift else '6'
    if keycode == 'KEY_7': return '&' if shift else '7'
    if keycode == 'KEY_8': return '*' if shift else '8'
    if keycode == 'KEY_9': return '(' if shift else '9'
    
    # Letters (always uppercase with shift)
    if keycode.startswith('KEY_') and len(keycode) == 5:
        letter = keycode[4]
        if letter.isalpha():
            return letter.upper() if shift else letter.lower()
    
    return None

def parse_to_hex(card_data):
    """Convert the raw card data to a clean hexadecimal format"""
    # Common substitutions in RFID readers
    replacements = {
        '!': '1',  # Shift+1
        '@': '2',  # Shift+2
        '#': '3',  # Shift+3
        '$': '4',  # Shift+4
        '%': '5',  # Shift+5
        '^': '6',  # Shift+6
        '&': '7',  # Shift+7
        '*': '8',  # Shift+8
        '(': '9',  # Shift+9
        ')': '0',  # Shift+0
    }
    
    # Replace special characters
    for special, normal in replacements.items():
        card_data = card_data.replace(special, normal)
    
    # Try to extract a valid hexadecimal string
    hex_data = ""
    for char in card_data:
        if char.isdigit() or char.lower() in 'abcdef':
            hex_data += char
    
    # Format the hex string with spaces for readability
    if len(hex_data) > 0:
        # Group in pairs (bytes)
        formatted_hex = ' '.join(hex_data[i:i+2] for i in range(0, len(hex_data), 2))
        return formatted_hex.upper()
    
    return ""  # Return empty string if no valid hex characters

# Use the correct device path we identified
RFID_DEVICE_PATH = '/dev/input/event5'

try:
    rfid_device = evdev.InputDevice(RFID_DEVICE_PATH)
    print(f"Connected to RFID reader: {rfid_device.name} at {RFID_DEVICE_PATH}")
    print("Waiting for card scan...")
    
    card_data = ""
    shift_pressed = False
    
    while True:
        r, w, x = select.select([rfid_device], [], [], 1)
        if r:
            for event in rfid_device.read():
                if event.type == evdev.ecodes.EV_KEY:
                    key_event = evdev.categorize(event)
                    
                    # Track shift key state
                    if key_event.keycode == 'KEY_LEFTSHIFT':
                        shift_pressed = event.value == 1  # 1 is pressed, 0 is released
                        continue
                    
                    # Only process key down events
                    if event.value == 1:  # Key down
                        # Handle ENTER key as end of card read
                        if key_event.keycode == 'KEY_ENTER':
                            # Parse the raw data to hexadecimal
                            hex_value = parse_to_hex(card_data)
                            decimal_value = int(hex_value.replace(" ", ""), 16) if hex_value else 0
                            
                            # Print the results
                            print(f"\nCard ID: {card_data}")
                            print(f"Hex: {hex_value}")
                            print(f"Decimal: {decimal_value}")
                            
                            card_data = ""  # Reset for next card
                        else:
                            # Map the key to the correct character
                            char = key_to_char(key_event.keycode, shift_pressed)
                            if char:
                                card_data += char
                
except Exception as e:
    print(f"Error: {e}")
finally:
    # Clean up
    try:
        rfid_device.close()
    except:
        pass
