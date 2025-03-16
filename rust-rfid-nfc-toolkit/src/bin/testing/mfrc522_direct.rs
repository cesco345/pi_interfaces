use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{env, thread, time::Duration};
use std::error::Error;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn Error>> {
    println!("MFRC522 Direct Command Tool");
    println!("---------------------------");
    
    // Try to find a working SPI configuration
    let configs = [
        (Bus::Spi0, SlaveSelect::Ss0),
        (Bus::Spi0, SlaveSelect::Ss1),
        (Bus::Spi1, SlaveSelect::Ss0),
        (Bus::Spi1, SlaveSelect::Ss1),
        (Bus::Spi1, SlaveSelect::Ss2),
    ];
    
    let speed = 1_000_000;  // 1 MHz
    let mode = Mode::Mode0;
    
    let mut spi_device = None;
    let mut bus_num = 0;
    let mut cs_num = 0;
    
    println!("Scanning for available SPI interfaces...");
    
    for (bus, cs) in &configs {
        let bus_id = if *bus == Bus::Spi0 { 0 } else { 1 };
        let cs_id = match cs {
            SlaveSelect::Ss0 => 0,
            SlaveSelect::Ss1 => 1,
            SlaveSelect::Ss2 => 2,
            _ => 0, // Default to 0 for other CS pins
        };
        
        print!("Trying SPI{} CS{} ... ", bus_id, cs_id);
        io::stdout().flush()?;
        
        match Spi::new(*bus, *cs, speed, mode) {
            Ok(device) => {
                println!("SUCCESS");
                spi_device = Some(device);
                bus_num = bus_id;
                cs_num = cs_id;
                break;
            },
            Err(e) => {
                println!("FAILED ({})", e);
            }
        }
    }
    
    let mut spi = match spi_device {
        Some(device) => device,
        None => {
            println!("\nError: Could not find any available SPI interfaces.");
            println!("Please make sure SPI is enabled on your Raspberry Pi.");
            println!("Run 'sudo raspi-config' and enable SPI under 'Interface Options'.");
            println!("Also check your wiring and verify the MFRC522 module is properly connected.");
            return Ok(());
        }
    };
    
    println!("\nFound working SPI interface!");
    println!("Using SPI{} CS{} at {} Hz in {:?} mode", bus_num, cs_num, speed, mode);
    
    println!("\nRunning tests with MFRC522 module...");
    
    // Test commands
    run_command(&mut spi, "VERSION", &[0xB7, 0x00])?;
    thread::sleep(Duration::from_millis(500));
    
    run_command(&mut spi, "STATUS", &[0x8F, 0x00])?;
    thread::sleep(Duration::from_millis(500));
    
    run_command(&mut spi, "ERROR", &[0x8D, 0x00])?;
    thread::sleep(Duration::from_millis(500));
    
    // Reset the module
    run_command(&mut spi, "RESET CMD", &[0x02, 0x0F])?;
    thread::sleep(Duration::from_millis(1000));  // Give it time to reset
    
    // Try to activate the antenna
    run_command(&mut spi, "ANTENNA ON", &[0x28, 0x03])?;
    thread::sleep(Duration::from_millis(500));
    
    // Check status again
    run_command(&mut spi, "STATUS AFTER ANTENNA", &[0x8F, 0x00])?;
    
    // FIFO level
    run_command(&mut spi, "FIFO LEVEL", &[0x95, 0x00])?;
    
    // Try another register
    run_command(&mut spi, "TX CONTROL", &[0x29, 0x00])?;
    
    // Try a firmware version command
    run_command(&mut spi, "VERSION AGAIN", &[0xB7, 0x00])?;
    
    println!("\nAll commands completed!");
    
    Ok(())
}

fn run_command(spi: &mut Spi, name: &str, cmd: &[u8]) -> Result<(), Box<dyn Error>> {
    println!("\n[{}]", name);
    
    let mut rx_buf = vec![0u8; cmd.len()];
    
    println!("Sending: [0x{:02X}, 0x{:02X}]", cmd[0], cmd[1]);
    
    match spi.transfer(&mut rx_buf, cmd) {
        Ok(_) => {
            println!("Received: [0x{:02X}, 0x{:02X}]", rx_buf[0], rx_buf[1]);
            
            // The second byte is what we're typically interested in
            if cmd[0] & 0x80 != 0 {  // Read operation
                println!("Register value: 0x{:02X} ({})", rx_buf[1], rx_buf[1]);
            }
        },
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    
    Ok(())
}
