import RPi.GPIO as GPIO
import time

# Setup
GPIO.setwarnings(False)
GPIO.setmode(GPIO.BOARD)
GPIO.setup(11, GPIO.OUT)

# Turn off initially
GPIO.output(11, False)
print("LED is OFF initially")

try:
    while True:
        print("\nPress Ctrl+C at any time to exit the program")
        
        # Get user input for number of toggles
        while True:
            try:
                toggle_count = int(input("How many times do you want to toggle the LED on/off? "))
                if toggle_count <= 0:
                    print("Please enter a positive number")
                    continue
                break
            except ValueError:
                print("Please enter a valid number")
        
        print("Toggling LED {} times...".format(toggle_count))
        
        # Perform the toggles
        for i in range(toggle_count):
            # Turn on
            GPIO.output(11, True)
            print("LED is ON ({}/{})".format(i+1, toggle_count))
            time.sleep(1)
            
            # Turn off
            GPIO.output(11, False)
            print("LED is OFF ({}/{})".format(i+1, toggle_count))
            time.sleep(1)
        
        print("Toggling completed")

except KeyboardInterrupt:
    print("\nProgram stopped by user")
finally:
    # Clean up
    GPIO.cleanup()
    print("GPIO cleanup completed")
