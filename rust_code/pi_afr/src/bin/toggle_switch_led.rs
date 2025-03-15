use rppal::gpio::{Gpio, Level};
use std::error::Error;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn Error>> {
    // BCM pin 21 = Physical pin 40 (Button)
    // BCM pin 20 = Physical pin 38 (LED)
    let button_pin_bcm = 21;
    let led_pin_bcm = 20;
    
    // Initialize GPIO
    let gpio = Gpio::new()?;
    
    // Set up the button pin with internal pull-up resistor
    let button = gpio.get(button_pin_bcm)?.into_input_pullup();
    
    // Set up the LED pin as output
    let mut led = gpio.get(led_pin_bcm)?.into_output();
    led.set_low(); // Start with LED off
    
    // LED state
    let mut led_state = false;
    
    // Button state tracking for debouncing
    let mut last_button_state = Level::High;  // Button not pressed initially (pull-up)
    let mut last_toggle_time = Instant::now();
    let debounce_delay = Duration::from_millis(100);
    
    // Set up interrupt handling for Ctrl+C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\nReceived Ctrl+C, exiting...");
    })?;
    
    println!("LED toggle switch program started. Press Ctrl+C to exit.");
    println!("Press button to toggle LED on/off");
    
    while running.load(Ordering::SeqCst) {
        // Read current button state
        let current_button_state = button.read();
        
        // If button was just pressed (transition from High to Low)
        if current_button_state == Level::Low && last_button_state == Level::High {
            // Check if enough time has passed since last toggle (debounce)
            if last_toggle_time.elapsed() > debounce_delay {
                // Toggle LED state
                led_state = !led_state;
                if led_state {
                    led.set_high();
                    println!("LED turned ON");
                } else {
                    led.set_low();
                    println!("LED turned OFF");
                }
                
                // Update last toggle time
                last_toggle_time = Instant::now();
                
                // Wait until button is released to avoid multiple toggles
                while button.read() == Level::Low {
                    std::thread::sleep(Duration::from_millis(10));
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                }
            }
        }
        
        // Update last button state
        last_button_state = current_button_state;
        
        // Small delay
        std::thread::sleep(Duration::from_millis(10));
    }
    
    // Turn off LED before exiting
    led.set_low();
    println!("Program completed");
    
    Ok(())
}
