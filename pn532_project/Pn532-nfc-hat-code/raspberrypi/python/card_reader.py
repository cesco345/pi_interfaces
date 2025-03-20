import serial
import time
import binascii
import sys
import RPi.GPIO as GPIO

# Setup GPIO for reset
RESET_PIN = 20
GPIO.setmode(GPIO.BCM)
GPIO.setup(RESET_PIN, GPIO.OUT)

# Open serial port
uart = serial.Serial('/dev/ttyAMA0', 115200)

def hardware_reset():
    """Perform a hardware reset of the PN532"""
    print("Performing hardware reset...")
    GPIO.output(RESET_PIN, GPIO.HIGH)
    time.sleep(0.1)
    GPIO.output(RESET_PIN, GPIO.LOW)
    time.sleep(0.5)
    GPIO.output(RESET_PIN, GPIO.HIGH)
    time.sleep(1.0)
    
    # Clear any pending data
    if uart.in_waiting:
        uart.read(uart.in_waiting)

def send_command(cmd_str):
    """Send a command as hex string"""
    cmd = bytes.fromhex(cmd_str.replace(' ', ''))
    print(f"Sending: {' '.join([f'{b:02X}' for b in cmd])}")
    uart.write(cmd)
    time.sleep(0.1)
    
    if uart.in_waiting:
        response = uart.read(uart.in_waiting)
        print(f"Response: {' '.join([f'{b:02X}' for b in response])}")
        return response
    else:
        print("No response")
        return None

try:
    print("PN532 Basic Test Script")
    
    # Perform hardware reset
    hardware_reset()
    
    # Send wake-up sequence
    print("\nSending wake-up sequence")
    wakeup = "55 55 00 00 00 00 00 00 00 00 00 00 00 00 00 00 FF 03 FD D4 14 01 17 00"
    response = send_command(wakeup)
    
    if not response:
        print("Failed to wake up PN532. Retrying with hardware reset...")
        hardware_reset()
        response = send_command(wakeup)
    
    # Wait a moment
    time.sleep(1)
    
    # Try SAM configuration - simplest command
    print("\nSending SAM configuration")
    sam_cmd = "00 00 FF 05 FB D4 14 01 01 00 EB 00"
    response = send_command(sam_cmd)
    
    # Wait a moment
    time.sleep(1)
    
    # Basic card scan
    print("\nScanning for card (press Ctrl+C to exit)")
    scan_cmd = "00 00 FF 04 FC D4 4A 01 00 E1 00"
    
    for i in range(10):  # Try 10 times
        print(f"\nScan attempt {i+1}")
        response = send_command(scan_cmd)
        time.sleep(1)

except KeyboardInterrupt:
    print("\nExiting...")
finally:
    uart.close()
    GPIO.cleanup()
