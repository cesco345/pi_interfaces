use rppal::gpio::{Gpio, OutputPin};
use std::thread::sleep;
use std::time::Duration;
use std::error::Error;
use std::io::{self, Write};

// Define the 5 LED pins using BOARD pins 37, 35, 33, 31, 29 converted to BCM
// Note: rppal uses BCM pin numbering, not BOARD (physical) numbers
// BOARD pin 37 = BCM 26, BOARD pin 35 = BCM 19, BOARD pin 33 = BCM 13, 
// BOARD pin 31 = BCM 6, BOARD pin 29 = BCM 5
const LED_PINS: [u8; 5] = [26, 19, 13, 6, 5];  // BCM numbering for your pins

struct BinaryCounter {
    leds: Vec<OutputPin>,
}

impl BinaryCounter {
    fn new() -> Result<Self, Box<dyn Error>> {
        let gpio = Gpio::new()?;
        let mut leds = Vec::new();
        
        // Initialize all LED pins
        for &pin in &LED_PINS {
            let mut output_pin = gpio.get(pin)?.into_output();
            output_pin.set_low();  // Start with all LEDs off
            leds.push(output_pin);
        }
        
        Ok(BinaryCounter { leds })
    }
    
    fn display_binary(&mut self, number: u8) -> Result<(), Box<dyn Error>> {
        if number > 31 {
            println!("Number must be between 0 and 31");
            return Ok(());
        }
        
        // Convert to binary string
        let binary = format!("{:05b}", number);
        println!("Decimal: {} | Binary: {}", number, binary);
        
        // First turn off all LEDs
        for led in &mut self.leds {
            led.set_low();
        }
        
        // Turn on LEDs based on binary representation
        // Note: binary string is left-to-right (MSB to LSB)
        // but we want to map it to LEDs right-to-left (LSB to MSB)
        let binary_chars: Vec<char> = binary.chars().collect();
        for i in 0..binary_chars.len() {
            if binary_chars[binary_chars.len() - 1 - i] == '1' {
                self.leds[i].set_high();
            }
        }
        
        // Debug output
        let mut led_states = Vec::new();
        for i in 0..self.leds.len() {
            if binary_chars[binary_chars.len() - 1 - i] == '1' {
                led_states.push("ON");
            } else {
                led_states.push("OFF");
            }
        }
        println!("LED states: {:?}", led_states);
        
        Ok(())
    }
    
    fn reset(&mut self) {
        for led in &mut self.leds {
            led.set_low();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut counter = BinaryCounter::new()?;
    
    println!("Binary Counter (0-31)");
    
    loop {
        println!("\nOptions:");
        println!("1. Count automatically from 0 to 31");
        println!("2. Enter a specific number");
        println!("3. Quit");
        
        print!("Enter your choice (1-3): ");
        io::stdout().flush()?;
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        
        match choice.trim() {
            "1" => {
                println!("Counting from 0 to 31...");
                for i in 0..32 {
                    counter.display_binary(i)?;
                    sleep(Duration::from_secs(1));
                }
            },
            "2" => {
                print!("Enter a number (0-31): ");
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                match input.trim().parse::<u8>() {
                    Ok(num) if num <= 31 => {
                        counter.display_binary(num)?;
                    },
                    Ok(_) => println!("Number must be between 0 and 31"),
                    Err(_) => println!("Please enter a valid number"),
                }
            },
            "3" => {
                println!("Exiting...");
                break;
            },
            _ => println!("Invalid choice, please try again"),
        }
    }
    
    // Clean up before exiting
    counter.reset();
    println!("Program completed");
    
    Ok(())
}
