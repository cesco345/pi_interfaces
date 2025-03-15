import RPi.GPIO as GPIO
import time

# Setup
GPIO.setwarnings(False)
GPIO.setmode(GPIO.BOARD)

# Define pins
BUTTON_PIN = 40  # Your button is on pin 40
LED_PIN = 38     # LED positive leg connected to pin 38

# Setup button pin with internal pull-up resistor
GPIO.setup(BUTTON_PIN, GPIO.IN, pull_up_down=GPIO.PUD_UP)

# Setup LED pin as output
GPIO.setup(LED_PIN, GPIO.OUT)
GPIO.output(LED_PIN, GPIO.LOW)  # Start with LED off

# Global variable to store LED state
led_state = False

# Debounce time in seconds
debounce_time = 0.2

def toggle_led():
    global led_state
    led_state = not led_state
    GPIO.output(LED_PIN, led_state)
    print("LED is now {}".format("ON" if led_state else "OFF"))

try:
    print("LED toggle switch program started. Press Ctrl+C to exit.")
    print("Press button to toggle LED on/off")
    
    last_press_time = 0
    
    while True:
        # Button is pressed when reading LOW (due to pull-up resistor)
        button_pressed = GPIO.input(BUTTON_PIN) == GPIO.LOW
        
        # If button is pressed and enough time has passed since last press
        current_time = time.time()
        if button_pressed and (current_time - last_press_time) > debounce_time:
            toggle_led()
            last_press_time = current_time
            
            # Wait for button release to avoid multiple toggles
            while GPIO.input(BUTTON_PIN) == GPIO.LOW:
                time.sleep(0.01)
        
        # Small delay to avoid CPU hogging
        time.sleep(0.01)

except KeyboardInterrupt:
    print("\nProgram stopped by user")
finally:
    # Turn off LED and clean up
    GPIO.output(LED_PIN, GPIO.LOW)
    GPIO.cleanup()
    print("GPIO cleanup completed")
