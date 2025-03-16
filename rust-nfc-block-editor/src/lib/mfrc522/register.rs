use rppal::spi::Spi;
use std::error::Error;

// Read from MFRC522 register
pub fn read_register(spi: &mut Spi, reg: u8) -> Result<u8, Box<dyn Error>> {
    let tx_buf = [((reg << 1) & 0x7E) | 0x80, 0x00];
    let mut rx_buf = [0u8, 0u8];
    
    spi.transfer(&mut rx_buf, &tx_buf)?;
    
    Ok(rx_buf[1])
}

// Write to MFRC522 register
pub fn write_register(spi: &mut Spi, reg: u8, value: u8) -> Result<(), Box<dyn Error>> {
    let tx_buf = [(reg << 1) & 0x7E, value];
    let mut rx_buf = [0u8, 0u8];
    
    spi.transfer(&mut rx_buf, &tx_buf)?;
    
    Ok(())
}

// Set bits in register
pub fn set_bit_mask(spi: &mut Spi, reg: u8, mask: u8) -> Result<(), Box<dyn Error>> {
    let tmp = read_register(spi, reg)?;
    write_register(spi, reg, tmp | mask)?;
    Ok(())
}

// Clear bits in register
pub fn clear_bit_mask(spi: &mut Spi, reg: u8, mask: u8) -> Result<(), Box<dyn Error>> {
    let tmp = read_register(spi, reg)?;
    write_register(spi, reg, tmp & (!mask))?;
    Ok(())
}
