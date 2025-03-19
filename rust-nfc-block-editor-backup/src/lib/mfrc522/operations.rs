use rppal::spi::Spi;
use std::error::Error;

use super::constants::*;
use super::register::*;
use super::communication::*;
use crate::lib::mfrc522::communication::{mfrc522_to_card, calculate_crc};
// Request card presence
pub fn mfrc522_request(spi: &mut Spi, req_mode: u8) -> Result<(u8, u8), Box<dyn Error>> {
    // Set bit framing for 7 bits
    write_register(spi, BIT_FRAMING_REG, 0x07)?;
    
    let tag_type = vec![req_mode];
    let (status, back_data, back_bits) = mfrc522_to_card(spi, PCD_TRANSCEIVE, &tag_type)?;
    
    if (status != MI_OK) || (back_bits != 0x10) {
        return Ok((MI_ERR, 0));
    }
    
    Ok((MI_OK, back_bits as u8))
}

// Anti-collision detection
pub fn mfrc522_anticoll(spi: &mut Spi) -> Result<(u8, Vec<u8>), Box<dyn Error>> {
    write_register(spi, BIT_FRAMING_REG, 0x00)?;
    
    let ser_num = vec![PICC_ANTICOLL, 0x20];
    let (status, back_data, _) = mfrc522_to_card(spi, PCD_TRANSCEIVE, &ser_num)?;
    
    if status == MI_OK {
        // Verify checksum
        if back_data.len() == 5 {
            let mut check_sum: u8 = 0;
            for i in 0..4 {
                check_sum ^= back_data[i];
            }
            if check_sum != back_data[4] {
                return Ok((MI_ERR, vec![]));
            }
        } else {
            return Ok((MI_ERR, vec![]));
        }
    }
    
    Ok((status, back_data))
}

// Select a card by UID
pub fn mfrc522_select_tag(spi: &mut Spi, ser_num: &[u8]) -> Result<u8, Box<dyn Error>> {
    let mut buf: Vec<u8> = Vec::new();
    buf.push(PICC_SELECTTAG);
    buf.push(0x70);
    
    for i in 0..5 {
        if i < ser_num.len() {
            buf.push(ser_num[i]);
        } else {
            break;
        }
    }
    
    let crc = calculate_crc(spi, &buf)?;
    buf.push(crc[0]);
    buf.push(crc[1]);
    
    let (status, back_data, back_len) = mfrc522_to_card(spi, PCD_TRANSCEIVE, &buf)?;
    
    if (status == MI_OK) && (back_len == 0x18) {
        return Ok(back_data[0]);
    } else {
        return Ok(0);
    }
}

// Authenticate with card
pub fn mfrc522_auth(spi: &mut Spi, auth_mode: u8, block_addr: u8, sector_key: &[u8], serial_num: &[u8]) 
    -> Result<u8, Box<dyn Error>> {
    
    let mut buf: Vec<u8> = Vec::new();
    
    // First byte is authMode (A or B)
    buf.push(auth_mode);
    // Second byte is the block address
    buf.push(block_addr);
    
    // Append the key (usually 6 bytes)
    for i in 0..sector_key.len() {
        buf.push(sector_key[i]);
    }
    
    // Append first 4 bytes of UID
    for i in 0..4 {
        if i < serial_num.len() {
            buf.push(serial_num[i]);
        } else {
            break;
        }
    }
    
    let (status, _, _) = mfrc522_to_card(spi, PCD_AUTHENT, &buf)?;
    
    // Check if the crypto1 state is set
    if (read_register(spi, STATUS2_REG)? & 0x08) == 0 {
        return Ok(MI_ERR);
    }
    
    Ok(status)
}

// Stop the crypto1 functionality
pub fn mfrc522_stop_crypto1(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_bit_mask(spi, STATUS2_REG, 0x08)?;
    Ok(())
}
