use rppal::gpio::{Gpio, InputPin, Level, Trigger};
use std::error::Error;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    // BCM pin 21 = Physical pin 40
    let button_pin_bcm = 21; 
    
    // Initialize GPIO
    let gpio = Gpio::new()?;
    
    // Set up the button pin with internal pull-up resistor
    // This means the input will read High when the button is NOT pressed
    // and Low when the button IS pressed (connecting to ground)
    let mut button = gpio.get(button_pin_bcm)?.into_input_pullup();
    
    // Set up interrupt handling
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Handle Ctrl+C
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;
    
    // Display initial state
    println!("Push button monitoring started. Press Ctrl+C to exit.");
    println!("Current state: {} ({})",
             if button.is_high() { "HIGH" } else { "LOW" },
             if button.is_high() { "Button NOT pressed" } else { "Button pressed" });
    
    // Set up interrupt for both rising and falling edge
    button.set_interrupt(Trigger::Both)?;
    
    while running.load(Ordering::SeqCst) {
        match button.poll_interrupt(true, Some(Duration::from_millis(100)))? {
            Some(level) => {
                match level {
                    Level::Low => println!("Button pressed! (LOW)"),
                    Level::High => println!("Button released! (HIGH)"),
                }
            },
            None => {} // Timeout, do nothing
        }
    }
    
    println!("Program completed");
    
    Ok(())
}
