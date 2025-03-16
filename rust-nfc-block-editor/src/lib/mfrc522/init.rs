use rppal::spi::Spi;
use std::error::Error;
use std::thread;
use std::time::Duration;

use super::constants::*;
use super::register::*;

// Initialize the MFRC522
pub fn mfrc522_init(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    // Soft reset
    write_register(spi, COMMAND_REG, PCD_RESETPHASE)?;
    thread::sleep(Duration::from_millis(50));
    
    // Timer configurations
    write_register(spi, T_MODE_REG, 0x8D)?;
    write_register(spi, T_PRESCALER_REG, 0x3E)?;
    write_register(spi, T_RELOAD_REG_L, 30)?;
    write_register(spi, T_RELOAD_REG_H, 0)?;
    
    // Auto configurations
    write_register(spi, TX_AUTO_REG, 0x40)?;
    write_register(spi, MODE_REG, 0x3D)?;
    
    // Turn on the antenna
    antenna_on(spi)?;
    
    Ok(())
}

// Turn antenna on
pub fn antenna_on(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    let temp = read_register(spi, TX_CONTROL_REG)?;
    if (temp & 0x03) != 0x03 {
        set_bit_mask(spi, TX_CONTROL_REG, 0x03)?;
    }
    Ok(())
}

// Turn antenna off
pub fn antenna_off(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_bit_mask(spi, TX_CONTROL_REG, 0x03)?;
    Ok(())
}
