use rppal::gpio::Gpio;
use std::thread::sleep;
use std::time::Duration;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Get a handle to the GPIO peripheral
    let gpio = Gpio::new()?;
    
    // Get a handle to GPIO pin 11 (physical pin 11, BCM pin 17)
    // Note: RPPAL uses BCM pin numbering, not physical pin numbers
    let pin_bcm = 17; // This corresponds to physical pin 11 in BOARD mode
    let mut pin = gpio.get(pin_bcm)?.into_output();
    
    // Turn off initially
    pin.set_low();
    println!("LED is OFF");
    sleep(Duration::from_secs(3));
    
    // Toggle on and off a few times
    for _ in 0..5 {
        // Turn on
        pin.set_high();
        println!("LED is ON");
        sleep(Duration::from_secs(1));
        
        // Turn off
        pin.set_low();
        println!("LED is OFF");
        sleep(Duration::from_secs(1));
    }
    
    // Explicit cleanup
    println!("Cleaning up GPIO");
    // Make sure the pin is set to low before releasing it
    pin.set_low();
    // Return the pin to its default state (input)
    drop(pin);
    
    println!("Program completed");
    
    Ok(())
}
