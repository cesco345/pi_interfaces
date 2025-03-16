use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;

const SPI_BUS: Bus = Bus::Spi0;  // spidev0
const SPI_SS: SlaveSelect = SlaveSelect::Ss0;  // spidev0.0
const SPI_CLOCK: u32 = 1_000_000;  // 1 MHz

fn main() -> Result<(), Box<dyn Error>> {
    // Setup SPI
    let mut spi = Spi::new(SPI_BUS, SPI_SS, SPI_CLOCK, Mode::Mode0)?;
    
    // Transfer 2 bytes at a time, Ctrl+C to exit
    let mut v: u8 = 0;
    
    println!("SPI Loopback test running. Press Ctrl+C to exit.");
    
    loop {
        let send = [v, v + 1];
        let mut receive = [0u8; 2];
        
        println!("\nTX: {:?}", send);
        
        // Perform the SPI transfer
        spi.transfer(&mut receive, &send)?;
        
        println!("RX: {:?}", receive);
        
        thread::sleep(Duration::from_millis(500));
        
        if v >= 254 {
            v = 0;
        } else {
            v = v + 2;
        }
    }
}
