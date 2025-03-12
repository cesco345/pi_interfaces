import RPi.GPIO as GPIO
import time

# Setup
GPIO.setwarnings(False)
GPIO.setmode(GPIO.BOARD)

# Define pins
BUTTON_PIN = 40  # Your button is on pin 40
LED_PIN = 38  # LED positive leg connected to pin 38

# Setup button pin with internal pull-up resistor
GPIO.setup(BUTTON_PIN, GPIO.IN, pull_up_down=GPIO.PUD_UP)

# Setup LED pin as output
GPIO.setup(LED_PIN, GPIO.OUT)
GPIO.output(LED_PIN, GPIO.LOW)  # Start with LED off

def button_callback(channel):
    if GPIO.input(BUTTON_PIN) == GPIO.LOW:  # Button is pressed
        print("Button pressed! Turning LED ON")
        GPIO.output(LED_PIN, GPIO.HIGH)
    else:  # Button is released
        print("Button released! Turning LED OFF")
        GPIO.output(LED_PIN, GPIO.LOW)

# Register interrupt for both rising and falling edge
GPIO.add_event_detect(BUTTON_PIN, GPIO.BOTH, callback=button_callback, bouncetime=200)

try:
    print("Push button monitoring started. Press Ctrl+C to exit.")
    print("Current state: {} ({})".format(
        "HIGH" if GPIO.input(BUTTON_PIN) == GPIO.HIGH else "LOW",
        "Button NOT pressed" if GPIO.input(BUTTON_PIN) == GPIO.HIGH else "Button pressed"
    ))
    
    # Initial LED state based on button state
    GPIO.output(LED_PIN, GPIO.LOW if GPIO.input(BUTTON_PIN) == GPIO.HIGH else GPIO.HIGH)
    
    while True:
        time.sleep(0.1)  # Just keep the program running

except KeyboardInterrupt:
    print("\nProgram stopped by user")
finally:
    # Turn off LED and clean up
    GPIO.output(LED_PIN, GPIO.LOW)
    GPIO.cleanup()
    print("GPIO cleanup completed")
