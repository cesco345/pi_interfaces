use rppal::spi::Spi;
use std::error::Error;

use super::constants::*;
use super::communication::*;
// Add at the top:
use crate::lib::mfrc522::communication::{mfrc522_to_card, calculate_crc};

// Read a block from the card
pub fn mfrc522_read(spi: &mut Spi, block_addr: u8) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    let mut recv_data: Vec<u8> = Vec::new();
    recv_data.push(PICC_READ);
    recv_data.push(block_addr);
    
    let crc = calculate_crc(spi, &recv_data)?;
    recv_data.push(crc[0]);
    recv_data.push(crc[1]);
    
    let (status, back_data, _) = mfrc522_to_card(spi, PCD_TRANSCEIVE, &recv_data)?;
    
    if status != MI_OK {
        println!("Error while reading!");
        return Ok(None);
    }
    
    if back_data.len() == 16 {
        return Ok(Some(back_data));
    } else {
        return Ok(None);
    }
}

// Write a block to the card
pub fn mfrc522_write(spi: &mut Spi, block_addr: u8, write_data: &[u8]) -> Result<u8, Box<dyn Error>> {
    let mut buf: Vec<u8> = Vec::new();
    buf.push(PICC_WRITE);
    buf.push(block_addr);
    
    let crc = calculate_crc(spi, &buf)?;
    buf.push(crc[0]);
    buf.push(crc[1]);
    
    let (status, back_data, back_len) = mfrc522_to_card(spi, PCD_TRANSCEIVE, &buf)?;
    
    if (status != MI_OK) || (back_len != 4) || ((back_data[0] & 0x0F) != 0x0A) {
        return Ok(MI_ERR);
    }
    
    // If status is OK, and we have received 4 bytes with the correct response (0x0A)
    // then proceed with writing the data
    if status == MI_OK {
        // Prepare the data with CRC
        let mut buf: Vec<u8> = Vec::new();
        
        // Data must be exactly 16 bytes
        for i in 0..16 {
            if i < write_data.len() {
                buf.push(write_data[i]);
            } else {
                buf.push(0);
            }
        }
        
        let crc = calculate_crc(spi, &buf)?;
        buf.push(crc[0]);
        buf.push(crc[1]);
        
        let (status, back_data, back_len) = mfrc522_to_card(spi, PCD_TRANSCEIVE, &buf)?;
        
        if (status != MI_OK) || (back_len != 4) || ((back_data[0] & 0x0F) != 0x0A) {
            println!("Error while writing");
            return Ok(MI_ERR);
        } else {
            println!("Data written successfully to block {}", block_addr);
            return Ok(MI_OK);
        }
    }
    
    Ok(MI_ERR)
}
