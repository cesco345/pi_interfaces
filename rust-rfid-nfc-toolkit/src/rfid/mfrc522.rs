use anyhow::Result;
use log::{debug, info, warn};
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::rfid::constants::*;

pub struct MFRC522 {
    spi: Spi,
    reset_pin: OutputPin,
}

impl MFRC522 {
    /// here we create a new MFRC522 instance
    pub fn new(spi_bus: u8, spi_device: u8, reset_pin: u8) -> Result<Self> {
        // bring down the speed when we iInitialize SPI for better reliability
        let spi = Spi::new(
            match spi_bus {
                0 => Bus::Spi0,
                1 => Bus::Spi1,
                _ => return Err(anyhow::anyhow!("Invalid SPI bus")),
            },
            match spi_device {
                0 => SlaveSelect::Ss0,
                1 => SlaveSelect::Ss1,
                _ => return Err(anyhow::anyhow!("Invalid SPI device")),
            },
            SPI_FREQUENCY_HZ,
            Mode::Mode0
        )?;
        
        // and voila we initialize GPIO reset pin
        let gpio = Gpio::new()?;
        let mut reset_pin = gpio.get(reset_pin)?.into_output();
        reset_pin.set_high();
        
        let mut mfrc522 = MFRC522 {
            spi,
            reset_pin,
        };
        
        // Initialize the MFRC522
        mfrc522.init()?;
        
        Ok(mfrc522)
    }
    
    /// function to initialize the MFRC522 chip
    pub fn init(&mut self) -> Result<()> {
        info!("Initializing MFRC522");
        
        // we set the hardware reset with longer delays
        self.reset_pin.set_high();
        thread::sleep(Duration::from_millis(100));
        self.reset_pin.set_low();
        thread::sleep(Duration::from_millis(100));
        self.reset_pin.set_high();
        thread::sleep(Duration::from_millis(200));
        
        // we set our software resets
        self.write_register(REG_COMMAND, COMMAND_SOFT_RESET)?;
        thread::sleep(Duration::from_millis(100));
        
        // == this may vary == configure for 100% ASK modulation and set preset value
        self.write_register(REG_TX_AUTO, 0x40)?;
        thread::sleep(Duration::from_millis(20));
        
        self.write_register(REG_MODE, 0x3D)?;
        thread::sleep(Duration::from_millis(20));
        
        // set the timer for longer timeouts
        self.write_register(0x2A, 0x80)?; // TAuto=1
        self.write_register(0x2B, 0xA9)?; // Prescaler = 0xA9 = ~169 = timer frequency = 40kHz
        self.write_register(0x2C, 0x03)?; // Reload timer high byte
        self.write_register(0x2D, 0xE8)?; // Reload timer low byte = 1000 ticks = 25ms
        
        // turn on antenna
        self.antenna_on()?;
        
        // get info from hardware and get version
        let version = self.read_register(REG_VERSION)?;
        let version_text = match version {
            0x88 => "Clone",
            0x90 => "v0.0",
            0x91 => "v1.0",
            0x92 => "v2.0",
            0xB2 => "FM17522",
            _ => "Unknown",
        };
        info!("MFRC522 Version: {} (0x{:02X})", version_text, version);
        
        Ok(())
    }
    
    /// we need to be good boys and clean up MFRC522 resources
    pub fn cleanup(&mut self) -> Result<()> {
        // here we turn off the antenna
        self.antenna_off()?;
        
        // reset command register
        self.write_register(REG_COMMAND, COMMAND_IDLE)?;
        
        // reset to a known state
        self.write_register(REG_MODE, 0x3D)?;
        
        info!("MFRC522 cleanup complete");
        Ok(())
    }
    
    /// save/write a value to a register
    pub fn write_register(&mut self, reg: u8, value: u8) -> Result<()> {
        let address = (reg << 1) & 0x7E; // Format: 0XXXXXX0 where X is address
        let buffer = [address, value];
        
        // fix for rppal::spi::Spi which requires explicit read buffer and write buffer
        let mut read_buffer = [0u8; 2];
        self.spi.transfer(&mut read_buffer, &buffer)?;
        
        // create a small delay after write for stability
        thread::sleep(Duration::from_micros(500)); // Extended delay
        
        Ok(())
    }
    
    /// here we add a value from a register
    pub fn read_register(&mut self, reg: u8) -> Result<u8> {
        let address = ((reg << 1) & 0x7E) | 0x80; // Format: 1XXXXXX0 where X is address
        let buffer = [address, 0];
        
        // we need to account and fix for rppal::spi::Spi which requires explicit read buffer and write buffer
        let mut read_buffer = [0u8; 2];
        self.spi.transfer(&mut read_buffer, &buffer)?;
        
        // create a small delay after read for stability
        thread::sleep(Duration::from_micros(500)); // Extended delay
        
        Ok(read_buffer[1])
    }
    
    /// set bits in a register
    pub fn set_bit_mask(&mut self, reg: u8, mask: u8) -> Result<()> {
        let current = self.read_register(reg)?;
        self.write_register(reg, current | mask)
    }
    
    /// clear the previous bits in a register
    pub fn clear_bit_mask(&mut self, reg: u8, mask: u8) -> Result<()> {
        let current = self.read_register(reg)?;
        self.write_register(reg, current & !mask)
    }
    
    /// turn on the antenna
    pub fn antenna_on(&mut self) -> Result<()> {
        let current = self.read_register(REG_TX_CONTROL)?;
        if (current & 0x03) != 0x03 {
            self.set_bit_mask(REG_TX_CONTROL, 0x03)?;
        }
        thread::sleep(Duration::from_millis(10));
        Ok(())
    }
    
    /// turn off the antenna
    pub fn antenna_off(&mut self) -> Result<()> {
        self.clear_bit_mask(REG_TX_CONTROL, 0x03)
    }
}

// thread-safe wrapper for MFRC522
pub struct MFRC522Wrapper {
    inner: Arc<Mutex<MFRC522>>,
}

impl MFRC522Wrapper {
    /// create a new thread-safe MFRC522 wrapper
    pub fn new(spi_bus: u8, spi_device: u8, reset_pin: u8) -> Result<Self> {
        let mfrc522 = MFRC522::new(spi_bus, spi_device, reset_pin)?;
        Ok(MFRC522Wrapper {
            inner: Arc::new(Mutex::new(mfrc522)),
        })
    }
    
    /// create a clone of the wrapper (shares the same underlying MFRC522 instance)
    pub fn clone(&self) -> Self {
        MFRC522Wrapper {
            inner: self.inner.clone(),
        }
    }
    
    /// clean up MFRC522 resources
    pub fn cleanup(&self) -> Result<()> {
        if let Ok(mut mfrc522) = self.inner.lock() {
            mfrc522.cleanup()?;
        }
        Ok(())
    }
}
