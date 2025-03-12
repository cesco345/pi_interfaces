import RPi.GPIO as GPIO
import time
import sys

# Check Python version
is_python2 = sys.version_info[0] == 2

# Setup
GPIO.setwarnings(False)
GPIO.setmode(GPIO.BOARD)

# Define the 5 LED pins using your specified pins
# From least significant bit (2^0) to most significant bit (2^4)
LED_PINS = [37, 35, 33, 31, 29]  # Your specified pins
# Note: Pin 39 is used for ground, not controlled by software

# Setup all pins as outputs
for pin in LED_PINS:
    GPIO.setup(pin, GPIO.OUT)
    GPIO.output(pin, False)  # Start with all LEDs off

def display_binary(number):
    """Display a number (0-31) in binary using 5 LEDs"""
    if number < 0 or number > 31:
        print("Number must be between 0 and 31")
        return
    
    binary = bin(number)[2:].zfill(5)  # Convert to binary and pad to 5 digits
    print("Decimal: {} | Binary: {}".format(number, binary))
    
    # First turn off all LEDs
    for pin in LED_PINS:
        GPIO.output(pin, False)
    
    # Turn on LEDs based on binary representation
    # Note: binary string is left-to-right (MSB to LSB)
    # but we want to map it to LEDs right-to-left (LSB to MSB)
    for i, bit in enumerate(reversed(binary)):
        if bit == '1':
            GPIO.output(LED_PINS[i], True)
    
    # Debug output
    led_states = []
    for pin in LED_PINS:
        led_states.append("ON" if GPIO.input(pin) else "OFF")
    print("LED states: {}".format(led_states))

def get_input(prompt):
    """Compatible input function for both Python 2 and Python 3"""
    if is_python2:
        return raw_input(prompt)
    else:
        return input(prompt)

try:
    # Interactive mode
    while True:
        print("\nBinary Counter (0-31)")
        print("Options:")
        print("1. Count automatically from 0 to 31")
        print("2. Enter a specific number")
        print("3. Quit")
        
        choice = get_input("Enter your choice (1-3): ").strip()
        
        if choice == '1':
            print("Counting from 0 to 31...")
            for i in range(32):
                display_binary(i)
                time.sleep(1)
        elif choice == '2':
            try:
                num = int(get_input("Enter a number (0-31): ").strip())
                if 0 <= num <= 31:
                    display_binary(num)
                else:
                    print("Number must be between 0 and 31")
            except ValueError:
                print("Please enter a valid number")
        elif choice == '3':
            print("Exiting...")
            break
        else:
            print("Invalid choice, please try again")

except KeyboardInterrupt:
    print("\nProgram stopped by user")
finally:
    # Turn off all LEDs and clean up
    for pin in LED_PINS:
        GPIO.output(pin, False)
    GPIO.cleanup()
    print("GPIO cleanup completed")
