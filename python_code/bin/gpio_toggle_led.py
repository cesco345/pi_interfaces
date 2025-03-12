import RPi.GPIO as GPIO
import time

# Setup
GPIO.setwarnings(False)
GPIO.setmode(GPIO.BOARD)
GPIO.setup(11, GPIO.OUT)

# Turn off initially
GPIO.output(11, False)
print("LED is OFF")
time.sleep(3)

# Toggle on and off a few times
try:
    for i in range(5):  # Repeat 5 times
        # Turn on
        GPIO.output(11, True)
        print("LED is ON")
        time.sleep(1)
        
        # Turn off
        GPIO.output(11, False)
        print("LED is OFF")
        time.sleep(1)
        
except KeyboardInterrupt:
    print("Program stopped by user")
finally:
    # Clean up
    GPIO.cleanup()
    print("GPIO cleanup completed")
