use rppal::gpio::{Gpio, Level, Trigger};
use std::error::Error;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    // BCM pin 21 = Physical pin 40 (Button)
    // BCM pin 20 = Physical pin 38 (LED)
    let button_pin_bcm = 21;
    let led_pin_bcm = 20;
    
    // Initialize GPIO
    let gpio = Gpio::new()?;
    
    // Set up the button pin with internal pull-up resistor
    let mut button = gpio.get(button_pin_bcm)?.into_input_pullup();
    
    // Set up the LED pin as output
    let mut led = gpio.get(led_pin_bcm)?.into_output();
    led.set_low(); // Start with LED off
    
    // Set up interrupt handling
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Handle Ctrl+C
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\nReceived Ctrl+C, exiting...");
    })?;
    
    // Display initial state
    println!("Push button monitoring started. Press Ctrl+C to exit.");
    println!("Current state: {} ({})",
             if button.is_high() { "HIGH" } else { "LOW" },
             if button.is_high() { "Button NOT pressed" } else { "Button pressed" });
    
    // Initial LED state based on button state
    if button.is_high() { led.set_low(); } else { led.set_high(); }
    
    // Set up interrupt for both rising and falling edge
    button.set_interrupt(Trigger::Both)?;
    
    while running.load(Ordering::SeqCst) {
        match button.poll_interrupt(true, Some(Duration::from_millis(100)))? {
            Some(level) => {
                match level {
                    Level::Low => {  // Button pressed
                        println!("Button pressed! Turning LED ON");
                        led.set_high();
                    },
                    Level::High => {  // Button released
                        println!("Button released! Turning LED OFF");
                        led.set_low();
                    },
                }
            },
            None => {} // Timeout, do nothing
        }
    }
    
    // Turn off LED before exiting
    led.set_low();
    println!("Program completed");
    
    Ok(())
}
