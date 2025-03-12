use rppal::gpio::Gpio;
use std::thread::sleep;
use std::time::Duration;
use std::error::Error;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process;

fn main() -> Result<(), Box<dyn Error>> {
    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\nReceived Ctrl+C, exiting...");
        // Force exit after cleanup
        sleep(Duration::from_millis(500)); // Give time for cleanup
        process::exit(0);
    })?;
    
    // Get a handle to the GPIO peripheral
    let gpio = Gpio::new()?;
    
    // Get a handle to GPIO pin 11 (physical pin 11, BCM pin 17)
    let pin_bcm = 17; // This corresponds to physical pin 11 in BOARD mode
    let mut pin = gpio.get(pin_bcm)?.into_output();
    
    // Turn off initially
    pin.set_low();
    println!("LED is OFF initially");
    
    while running.load(Ordering::SeqCst) {
        println!("\nPress Ctrl+C at any time to exit the program");
        
        // Get user input for number of toggles
        let mut toggle_count = 0;
        while toggle_count <= 0 && running.load(Ordering::SeqCst) {
            print!("How many times do you want to toggle the LED on/off? ");
            io::stdout().flush()?;
            
            // Use a separate thread for input to allow interruption
            let input_running = running.clone();
            let input_thread = std::thread::spawn(move || {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(_) if input_running.load(Ordering::SeqCst) => Some(input),
                    _ => None,
                }
            });
            
            // Wait for input or interruption
            let mut input_result = None;
            for _ in 0..100 { // 10 seconds timeout
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                
                if input_thread.is_finished() {
                    match input_thread.join() {
                        Ok(Some(input)) => {
                            input_result = Some(input);
                            break;
                        },
                        _ => break,
                    }
                }
                
                sleep(Duration::from_millis(100));
            }
            
            if !running.load(Ordering::SeqCst) {
                break;
            }
            
            if let Some(input) = input_result {
                match input.trim().parse::<u32>() {
                    Ok(count) if count > 0 => toggle_count = count,
                    Ok(_) => println!("Please enter a positive number"),
                    Err(_) => println!("Please enter a valid number"),
                }
            } else {
                println!("Input timed out or was interrupted");
                break;
            }
        }
        
        if !running.load(Ordering::SeqCst) {
            break;
        }
        
        println!("Toggling LED {} times...", toggle_count);
        
        // Perform the toggles
        for i in 0..toggle_count {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            
            // Turn on
            pin.set_high();
            println!("LED is ON ({}/{})", i + 1, toggle_count);
            
            // Sleep but check for interrupt
            for _ in 0..10 {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                sleep(Duration::from_millis(100));
            }
            
            if !running.load(Ordering::SeqCst) {
                break;
            }
            
            // Turn off
            pin.set_low();
            println!("LED is OFF ({}/{})", i + 1, toggle_count);
            
            // Sleep but check for interrupt
            for _ in 0..10 {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                sleep(Duration::from_millis(100));
            }
        }
        
        if running.load(Ordering::SeqCst) {
            println!("Toggling completed");
        }
    }
    
    // Explicit cleanup
    println!("Cleaning up GPIO");
    pin.set_low();
    
    println!("Program completed");
    
    Ok(())
}
