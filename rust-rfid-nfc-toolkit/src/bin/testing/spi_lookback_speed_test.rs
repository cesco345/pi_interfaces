use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let test_data = [0, 1, 2, 4, 8, 16, 32, 64, 128, 255];
    let mut freq = 250_000;
    let mut ok = true;
    
    println!("SPI speed test starting");
    
    while ok {
        // Recreate the SPI device with the new frequency
        let mut spi = match Spi::new(Bus::Spi0, SlaveSelect::Ss0, freq, Mode::Mode0) {
            Ok(spi) => spi,
            Err(e) => {
                println!("Failed to create SPI at {} Hz: {}", freq, e);
                break;
            }
        };
        
        println!("\nspi.clock_speed: {} Hz", freq);
        println!("TX: {:?}", test_data);
        
        // Create a receive buffer
        let mut recv_data = [0u8; 10];
        
        // Perform the transfer
        match spi.transfer(&mut recv_data, &test_data) {
            Ok(_) => {
                println!("RX: {:?}", recv_data);
                
                // Check if received data matches sent data
                ok = recv_data == test_data;
                
                if ok {
                    println!("Success");
                } else {
                    println!("Failed - data mismatch");
                }
            },
            Err(e) => {
                println!("Transfer failed: {}", e);
                ok = false;
            }
        }
        
        // Double the frequency for next attempt
        if ok {
            freq = freq * 2;
        }
    }
    
    println!("Maximum reliable speed: {} Hz", freq / 2);
    
    Ok(())
}
