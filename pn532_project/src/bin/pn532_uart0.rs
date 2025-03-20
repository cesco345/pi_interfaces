use rppal::gpio::Gpio;
use rppal::uart::{Uart, Parity};
use std::{thread, time::Duration, io::{self, Write}};

pub struct PN532 {
    uart: Uart,
    debug: bool,
}

impl PN532 {
    /// Initialize a new PN532 instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut uart = Uart::new(115200, Parity::None, 8, 1)?;
        
        // Clear any pending data
        let mut buffer = [0u8; 100];
        let _ = uart.read(&mut buffer);
        
        Ok(Self {
            uart,
            debug: true,
        })
    }
    
    /// Send wake-up sequence to the PN532
    pub fn wake_up(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let wake_up = [
            0x55, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x03, 0xFD, 0xD4,
            0x14, 0x01, 0x17, 0x00
        ];
        
        if self.debug {
            println!("Sending wake-up sequence");
        }
        
        self.uart.write(&wake_up)?;
        
        // Wait for response
        thread::sleep(Duration::from_millis(100));
        let mut response = [0u8; 30];
        let n = self.uart.read(&mut response)?;
        
        if self.debug {
            println!("Wake-up response: {:02X?}", &response[..n]);
        }
        
        Ok(n > 0)
    }
    
    /// Get firmware version from the PN532
    pub fn get_firmware_version(&mut self) -> Result<Option<(u8, u8)>, Box<dyn std::error::Error>> {
        let firmware_cmd = [
            0x00, 0x00, 0xFF, 0x02, 0xFE, 0xD4, 0x02, 0x2A, 0x00
        ];
        
        if self.debug {
            println!("Sending firmware version command");
        }
        
        self.uart.write(&firmware_cmd)?;
        
        // Read response
        thread::sleep(Duration::from_millis(100));
        let mut response = [0u8; 30];
        let n = self.uart.read(&mut response)?;
        
        if self.debug {
            println!("Firmware response: {:02X?}", &response[..n]);
        }
        
        // Parse firmware version
        if n >= 14 {
            // Classic format: `00 00 FF 00 FF 00 00 00 FF 06 FA D5 03 32 01 06 07 E8 00`
            //                                                         ^^ ^^ version bytes
            return Ok(Some((response[14], response[15])));
        }
        
        Ok(None)
    }
    
    /// Attempt to configure the Secure Access Module (may not work)
    pub fn configure_sam(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let sam_cmd = [
            0x00, 0x00, 0xFF, 0x05, 0xFB, 0xD4, 0x14, 0x01, 0x00, 0x00, 0xE9, 0x00
        ];
        
        if self.debug {
            println!("Sending SAM configuration command");
        }
        
        self.uart.write(&sam_cmd)?;
        
        // Read response
        thread::sleep(Duration::from_millis(100));
        let mut response = [0u8; 30];
        let n = self.uart.read(&mut response)?;
        
        if self.debug {
            println!("SAM configuration response: {:02X?}", &response[..n]);
        }
        
        Ok(n > 0)
    }
    
    /// Attempt to scan for a card (may not work)
    pub fn scan_for_card(&mut self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        let scan_cmd = [
            0x00, 0x00, 0xFF, 0x04, 0xFC, 0xD4, 0x4A, 0x01, 0x00, 0xE1, 0x00
        ];
        
        if self.debug {
            println!("Scanning for card");
        }
        
        self.uart.write(&scan_cmd)?;
        
        // Read response
        thread::sleep(Duration::from_millis(150));
        let mut response = [0u8; 30];
        let n = self.uart.read(&mut response)?;
        
        if self.debug {
            if n > 0 {
                println!("Card scan response: {:02X?}", &response[..n]);
            } else {
                println!("No response to card scan");
            }
        }
        
        // Parse for UID if present
        if n > 12 {
            // Look for response pattern
            for i in 0..n-12 {
                if i+9 < n && response[i+6] == 0xD5 && response[i+7] == 0x4B && response[i+8] > 0 {
                    if i+12 < n {
                        let uid_len = response[i+12] as usize;
                        if i+13+uid_len <= n {
                            return Ok(Some(response[i+13..i+13+uid_len].to_vec()));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Close the UART connection
    pub fn close(self) {
        // UART is automatically closed when dropped
    }
    
    /// Set debug mode
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PN532 NFC Reader");
    println!("================\n");
    
    // Initialize PN532
    let mut pn532 = PN532::new()?;
    
    // Basic test sequence
    if pn532.wake_up()? {
        println!("✓ PN532 woke up successfully");
        
        // Try to get firmware version
        if let Ok(Some((version, revision))) = pn532.get_firmware_version() {
            println!("✓ PN532 firmware version: {}.{}", version, revision);
        } else {
            println!("✗ Failed to get firmware version");
        }
        
        // Optionally try SAM configuration (may not work)
        if pn532.configure_sam()? {
            println!("✓ SAM configuration successful");
        } else {
            println!("✗ SAM configuration failed");
        }
        
        // Card scanning mode (likely won't work with your hardware)
        println!("\nStarting card scan (press Ctrl+C to exit)");
        println!("Note: Card scanning may not work with this PN532 HAT");
        
        println!("\nWaiting for an NFC card...");
        
        loop {
            match pn532.scan_for_card() {
                Ok(Some(uid)) => {
                    println!("\nCard detected! UID: {:02X?}", uid);
                    thread::sleep(Duration::from_secs(1));
                },
                Ok(None) => {
                    print!(".");
                    io::stdout().flush()?;
                    thread::sleep(Duration::from_millis(500));
                },
                Err(e) => {
                    println!("\nError scanning for card: {}", e);
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }
    } else {
        println!("✗ Failed to wake up PN532");
    }
    
    Ok(())
}
