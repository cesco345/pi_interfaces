use rppal::gpio::{Gpio, Level};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};

// PN532 SPI status bytes
const PN532_SPI_STATREAD: u8 = 0x02;
const PN532_SPI_DATAWRITE: u8 = 0x01;
const PN532_SPI_DATAREAD: u8 = 0x03;
const PN532_SPI_READY: u8 = 0xFF;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PN532 NFC HAT - Simple Test");
    
    // Initialize GPIO for reset and chip select
    let gpio = Gpio::new()?;
    let mut reset_pin = gpio.get(20)?.into_output();
    let mut cs_pin = gpio.get(4)?.into_output();
    
    // Ensure CS is high initially
    cs_pin.set_high();
    
    // Hard reset the PN532
    println!("Performing hardware reset");
    reset_pin.set_high();
    thread::sleep(Duration::from_millis(100));
    reset_pin.set_low();
    thread::sleep(Duration::from_millis(500));
    reset_pin.set_high();
    thread::sleep(Duration::from_millis(1000));
    
    // Initialize SPI
    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)?;
    
    // Try a simple wakeup sequence first
    println!("Sending wakeup sequence");
    cs_pin.set_low();
    let wakeup = [0x55, 0x55, 0x00, 0x00, 0x00];
    let mut rx_buf = vec![0u8; wakeup.len()];
    spi.transfer(&mut rx_buf, &wakeup)?;
    cs_pin.set_high();
    
    println!("Wakeup response: {:?}", rx_buf);
    thread::sleep(Duration::from_millis(100));
    
    // Try a basic status check
    println!("Checking status");
    for i in 0..10 {
        cs_pin.set_low();
        thread::sleep(Duration::from_micros(100));
        
        let status_cmd = [PN532_SPI_STATREAD, 0x00];
        let mut status_response = [0u8; 2];
        
        spi.transfer(&mut status_response, &status_cmd)?;
        
        cs_pin.set_high();
        
        println!("Status check {}: 0x{:02X}", i, status_response[1]);
        
        thread::sleep(Duration::from_millis(10));
    }
    
    // Try simple firmware version command
    println!("Sending get firmware version command");
    
    // The command as a raw bytestream
    let firmware_cmd = [
        PN532_SPI_DATAWRITE,    // SPI data write
        0x00, 0x00, 0xFF,       // Preamble
        0x02,                   // Length
        0xFE,                   // Length checksum
        0xD4, 0x02,             // TFI, Command
        0x2A,                   // Checksum
        0x00                    // Postamble
    ];
    
    cs_pin.set_low();
    let mut rx_buf = vec![0u8; firmware_cmd.len()];
    spi.transfer(&mut rx_buf, &firmware_cmd)?;
    cs_pin.set_high();
    
    println!("Command response: {:?}", rx_buf);
    
    // Let's try continuous status checks for a while to see if anything changes
    println!("Checking status continuously");
    for i in 0..30 {
        cs_pin.set_low();
        thread::sleep(Duration::from_micros(100));
        
        let status_cmd = [PN532_SPI_STATREAD, 0x00];
        let mut status_response = [0u8; 2];
        
        spi.transfer(&mut status_response, &status_cmd)?;
        
        cs_pin.set_high();
        
        println!("Status check {}: 0x{:02X}", i, status_response[1]);
        
        thread::sleep(Duration::from_millis(10));
    }
    
    // Try reading data even if status doesn't say ready
    println!("Trying to read data anyway");
    cs_pin.set_low();
    
    let read_cmd = [PN532_SPI_DATAREAD, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let mut read_response = [0u8; 15];
    
    spi.transfer(&mut read_response, &read_cmd)?;
    
    cs_pin.set_high();
    
    println!("Read response: {:?}", read_response);
    
    Ok(())
}
