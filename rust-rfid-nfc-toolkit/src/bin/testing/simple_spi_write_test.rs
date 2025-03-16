use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;
use std::io::{self, Read, Write};

fn main() -> Result<(), Box<dyn Error>> {
    println!("Improved SPI Write Test");
    println!("======================");
    
    // SPI Configuration
    let bus = Bus::Spi0;
    let cs = SlaveSelect::Ss0;
    let speed = 100_000; // 100 KHz
    let mode = Mode::Mode0;
    
    println!("Configuration:");
    println!("  Bus: SPI0");
    println!("  CS: 0");
    println!("  Speed: {} Hz", speed);
    println!("  Mode: {:?}", mode);
    
    // Initialize SPI
    let mut spi = Spi::new(bus, cs, speed, mode)?;
    
    println!("\nSelect what to write:");
    println!("1. Write 0x3A repeatedly (original test)");
    println!("2. Write incrementing bytes (0x00, 0x01, 0x02...)");
    println!("3. Write custom byte (you specify the value)");
    println!("4. Write pattern (0x55, 0xAA, 0xFF, 0x00)");
    print!("Selection [1]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let selection = input.trim();
    
    let mut byte_to_write = 0x3A; // Default
    let mut custom_pattern: Vec<u8> = Vec::new();
    let mut mode = match selection {
        "2" => "increment",
        "3" => {
            print!("Enter hex value to write (e.g. 3A): 0x");
            io::stdout().flush()?;
            input.clear();
            io::stdin().read_line(&mut input)?;
            byte_to_write = u8::from_str_radix(input.trim(), 16).unwrap_or(0x3A);
            "custom"
        },
        "4" => {
            custom_pattern = vec![0x55, 0xAA, 0xFF, 0x00];
            "pattern"
        },
        _ => "default"
    };
    
    println!("\nPress Ctrl+C to stop the test.");
    println!("Starting write test - monitoring bytes sent:");
    
    let mut counter = 0;
    let mut pattern_index = 0;
    
    loop {
        match mode {
            "default" => {
                spi.write(&[byte_to_write])?;
                println!("Wrote: 0x{:02X} ({})", byte_to_write, counter);
            },
            "increment" => {
                spi.write(&[counter as u8])?;
                println!("Wrote: 0x{:02X} ({})", counter as u8, counter);
                counter = (counter + 1) % 256;
            },
            "custom" => {
                spi.write(&[byte_to_write])?;
                println!("Wrote: 0x{:02X} ({})", byte_to_write, counter);
            },
            "pattern" => {
                let current_byte = custom_pattern[pattern_index];
                spi.write(&[current_byte])?;
                println!("Wrote: 0x{:02X} (pattern index {})", current_byte, pattern_index);
                pattern_index = (pattern_index + 1) % custom_pattern.len();
            },
            _ => unreachable!()
        }
        
        counter += 1;
        thread::sleep(Duration::from_millis(100));
        
        // Optional: Check for keypress to exit or change pattern
        if io::stdin().read(&mut [0]).is_ok() {
            break;
        }
    }
    
    Ok(())
}
