import RPi.GPIO as GPIO
import time

# Setup
GPIO.setwarnings(False)
GPIO.setmode(GPIO.BOARD)

# Define pins
BUTTON_PIN = 40  # Your button is on pin 40

# Setup pin with internal pull-up resistor
# This means the input will read HIGH (1) when the button is NOT pressed
# and LOW (0) when the button IS pressed (connecting to ground)
GPIO.setup(BUTTON_PIN, GPIO.IN, pull_up_down=GPIO.PUD_UP)

def button_callback(channel):
    if GPIO.input(BUTTON_PIN) == GPIO.LOW:
        print("Button pressed! (LOW)")
    else:
        print("Button released! (HIGH)")

# Register interrupt for both rising and falling edge
GPIO.add_event_detect(BUTTON_PIN, GPIO.BOTH, callback=button_callback, bouncetime=200)

try:
    print("Push button monitoring started. Press Ctrl+C to exit.")
    print("Current state: {} ({})".format(
        "HIGH" if GPIO.input(BUTTON_PIN) == GPIO.HIGH else "LOW",
        "Button NOT pressed" if GPIO.input(BUTTON_PIN) == GPIO.HIGH else "Button pressed"
    ))
    
    while True:
        time.sleep(0.1)  # Just keep the program running

except KeyboardInterrupt:
    print("\nProgram stopped by user")
finally:
    GPIO.cleanup()  # Clean up GPIO on exit
