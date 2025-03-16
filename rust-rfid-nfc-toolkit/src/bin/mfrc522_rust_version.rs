use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;
use std::io::{self, Write};

// MFRC522 Commands
const PCD_IDLE: u8 = 0x00;
const PCD_AUTHENT: u8 = 0x0E;
const PCD_RECEIVE: u8 = 0x08;
const PCD_TRANSMIT: u8 = 0x04;
const PCD_TRANSCEIVE: u8 = 0x0C;
const PCD_RESETPHASE: u8 = 0x0F;
const PCD_CALCCRC: u8 = 0x03;

// MIFARE Commands
const PICC_REQIDL: u8 = 0x26;
const PICC_REQALL: u8 = 0x52;
const PICC_ANTICOLL: u8 = 0x93;
const PICC_SELECTTAG: u8 = 0x93;
const PICC_AUTHENT1A: u8 = 0x60;
const PICC_AUTHENT1B: u8 = 0x61;
const PICC_READ: u8 = 0x30;
const PICC_WRITE: u8 = 0xA0;
const PICC_DECREMENT: u8 = 0xC0;
const PICC_INCREMENT: u8 = 0xC1;
const PICC_RESTORE: u8 = 0xC2;
const PICC_TRANSFER: u8 = 0xB0;
const PICC_HALT: u8 = 0x50;

// Status codes
const MI_OK: u8 = 0;
const MI_NOTAGERR: u8 = 1;
const MI_ERR: u8 = 2;

// MFRC522 Registers
const COMMAND_REG: u8 = 0x01;
const COM_IEN_REG: u8 = 0x02;
const DIV_IEN_REG: u8 = 0x03;
const COM_IRQ_REG: u8 = 0x04;
const DIV_IRQ_REG: u8 = 0x05;
const ERROR_REG: u8 = 0x06;
const STATUS1_REG: u8 = 0x07;
const STATUS2_REG: u8 = 0x08;
const FIFO_DATA_REG: u8 = 0x09;
const FIFO_LEVEL_REG: u8 = 0x0A;
const WATER_LEVEL_REG: u8 = 0x0B;
const CONTROL_REG: u8 = 0x0C;
const BIT_FRAMING_REG: u8 = 0x0D;
const COLL_REG: u8 = 0x0E;

const MODE_REG: u8 = 0x11;
const TX_MODE_REG: u8 = 0x12;
const RX_MODE_REG: u8 = 0x13;
const TX_CONTROL_REG: u8 = 0x14;
const TX_AUTO_REG: u8 = 0x15;
const TX_SEL_REG: u8 = 0x16;
const RX_SEL_REG: u8 = 0x17;
const RX_THRESHOLD_REG: u8 = 0x18;
const DEMOD_REG: u8 = 0x19;
const MIFARE_REG: u8 = 0x1C;
const SERIAL_SPEED_REG: u8 = 0x1F;

const CRC_RESULT_REG_M: u8 = 0x21;
const CRC_RESULT_REG_L: u8 = 0x22;
const MOD_WIDTH_REG: u8 = 0x24;
const RF_CFG_REG: u8 = 0x26;
const GS_N_REG: u8 = 0x27;
const CW_GS_P_REG: u8 = 0x28;
const MOD_GS_P_REG: u8 = 0x29;
const T_MODE_REG: u8 = 0x2A;
const T_PRESCALER_REG: u8 = 0x2B;
const T_RELOAD_REG_H: u8 = 0x2C;
const T_RELOAD_REG_L: u8 = 0x2D;

const VERSION_REG: u8 = 0x37;

const MAX_LEN: usize = 16;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== MFRC522 RFID Reader/Writer (Full Rust Version) ===");
    println!("This implementation is a Rust Answer to SimpleMFRC522");
    println!("Press Ctrl+C to exit\n");
    
    // Open SPI with the configuration we found works best
    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 1_000_000, Mode::Mode0)?;
    
    // Show version information
    let version = read_register(&mut spi, VERSION_REG)?;
    println!("MFRC522 Version: 0x{:02X}", version);
    
    // Initialize the MFRC522
    mfrc522_init(&mut spi)?;
    
    println!("MFRC522 initialized successfully");
    
    // Menu system
    loop {
        println!("\nSelect an option:");
        println!("1. Read card UID");
        println!("2. Read card data");
        println!("3. Write data to card");
        println!("4. Dump card contents");
        println!("5. Exit");
        print!("Enter choice: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();
        
        match choice {
            "1" => {
                println!("\n=== Reading Card UID ===");
                
                match wait_for_card(&mut spi, 15, read_card_uid)? {
                    Some(uid) => {
                        println!("UID: {}", uid_to_string(&uid));
                        println!("UID as decimal: {}", uid_to_num(&uid));
                        wait_for_card_removal(&mut spi)?;
                    },
                    None => println!("No card detected during the timeout period."),
                }
            },
            "2" => {
                println!("\n=== Reading Card Data ===");
                
                match wait_for_card(&mut spi, 15, read_card_data)? {
                    Some((uid, text)) => {
                        println!("UID: {}", uid_to_string(&uid));
                        println!("Data read from card: \"{}\"", text.trim_end());
                        wait_for_card_removal(&mut spi)?;
                    },
                    None => println!("No card detected during the timeout period."),
                }
            },
            "3" => {
                println!("\n=== Writing to Card ===");
                println!("Enter the text to write to the card:");
                
                let mut text_input = String::new();
                io::stdin().read_line(&mut text_input)?;
                let text_to_write = text_input.trim();
                
                println!("\nText to write: \"{}\"", text_to_write);
                
                // Countdown before trying to read card
                countdown_for_card_placement(10)?;
                
                let result = write_card_data(&mut spi, text_to_write)?;
                
                match result {
                    Some((uid, text)) => {
                        println!("✅ Card write successful!");
                        println!("UID: {}", uid_to_string(&uid));
                        println!("Data written: \"{}\"", text);
                        wait_for_card_removal(&mut spi)?;
                    },
                    None => {
                        println!("❌ Write failed. The card may have been removed too soon or there might be permissions issues.");
                        println!("Try again with a fresh card or make sure you're using the default keys.");
                    },
                }
            },
            "4" => {
                println!("\n=== Dumping Card Contents ===");
                println!("This will try to read all sectors with default keys.");
                
                match wait_for_card(&mut spi, 15, dump_card)? {
                    Some(uid) => {
                        println!("Card dump complete for UID: {}", uid_to_string(&uid));
                        wait_for_card_removal(&mut spi)?;
                    },
                    None => println!("No card detected during the timeout period."),
                }
            },
            "5" => {
                println!("\nExiting program. Goodbye!");
                break;
            },
            _ => println!("Invalid choice, please try again")
        }
    }
    
    Ok(())
}

// Helper function for countdown timer when placing card
fn countdown_for_card_placement(seconds: u64) -> Result<(), Box<dyn Error>> {
    println!("\nPrepare your card. You have {} seconds to place it on the reader...", seconds);
    
    // Progress bar width
    let width = 30;
    
    for i in (1..=seconds).rev() {
        let filled = ((seconds - i) as f64 / seconds as f64 * width as f64) as usize;
        
        print!("\r[");
        for j in 0..width {
            if j < filled {
                print!("=");
            } else if j == filled {
                print!(">");
            } else {
                print!(" ");
            }
        }
        print!("] {:2}/{} seconds", seconds - i + 1, seconds);
        io::stdout().flush()?;
        
        thread::sleep(Duration::from_secs(1));
    }
    
    println!("\n\nReading card now...");
    Ok(())
}

// Generic function to wait for a card with nice UI
fn wait_for_card<T, F>(spi: &mut Spi, timeout_seconds: u64, read_fn: F) 
    -> Result<Option<T>, Box<dyn Error>> 
where
    F: Fn(&mut Spi) -> Result<Option<T>, Box<dyn Error>>
{
    println!("Hold a card near the reader...");
    println!("You have {} seconds to place a card", timeout_seconds);
    
    // Show a progress bar
    let width = 30;
    let update_interval = 200; // milliseconds
    let steps = (timeout_seconds * 1000) / update_interval;
    
    for i in 0..steps {
        // Try to read the card
        if let Some(result) = read_fn(spi)? {
            print!("\r");
            for _ in 0..width + 30 {
                print!(" ");
            }
            print!("\r");
            return Ok(Some(result));
        }
        
        // Update progress bar
        let filled = (i as f64 / steps as f64 * width as f64) as usize;
        let time_passed = (i * update_interval) / 1000;
        let time_remaining = timeout_seconds - time_passed;
        
        print!("\r[");
        for j in 0..width {
            if j < filled {
                print!("=");
            } else if j == filled {
                print!(">");
            } else {
                print!(" ");
            }
        }
        print!("] {:2}/{} seconds left    ", time_remaining, timeout_seconds);
        io::stdout().flush()?;
        
        thread::sleep(Duration::from_millis(update_interval));
    }
    
    print!("\r");
    for _ in 0..width + 30 {
        print!(" ");
    }
    print!("\r");
    Ok(None)
}

// Initialize the MFRC522 - Let's pray it works!!!
fn mfrc522_init(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    // Soft reset
    write_register(spi, COMMAND_REG, PCD_RESETPHASE)?;
    thread::sleep(Duration::from_millis(50));
    
    // Timer configurations - 
    write_register(spi, T_MODE_REG, 0x8D)?;
    write_register(spi, T_PRESCALER_REG, 0x3E)?;
    write_register(spi, T_RELOAD_REG_L, 30)?;
    write_register(spi, T_RELOAD_REG_H, 0)?;
    
    // Auto configurations - 
    write_register(spi, TX_AUTO_REG, 0x40)?;
    write_register(spi, MODE_REG, 0x3D)?;
    
    // Turn on the antenna
    antenna_on(spi)?;
    
    Ok(())
}

// Turn antenna on
fn antenna_on(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    let temp = read_register(spi, TX_CONTROL_REG)?;
    if (temp & 0x03) != 0x03 {
        set_bit_mask(spi, TX_CONTROL_REG, 0x03)?;
    }
    Ok(())
}

// Turn antenna off
fn antenna_off(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_bit_mask(spi, TX_CONTROL_REG, 0x03)?;
    Ok(())
}

// Read card UID
fn read_card_uid(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    Ok(Some(uid))
}

// Read card data using blocks specified in the literature
fn read_card_data(spi: &mut Spi) -> Result<Option<(Vec<u8>, String)>, Box<dyn Error>> {
    // Block addresses from the literature
    let block_addrs = [8, 9, 10];
    let key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Ok(None);
    }
    
    // Authenticate
    let status = mfrc522_auth(spi, PICC_AUTHENT1A, 11, &key, &uid)?;
    if status != MI_OK {
        mfrc522_stop_crypto1(spi)?;
        return Ok(None);
    }
    
    // Read data from blocks
    let mut data = Vec::new();
    
    for &block_num in &block_addrs {
        if let Some(block_data) = mfrc522_read(spi, block_num)? {
            data.extend_from_slice(&block_data);
        }
    }
    
    mfrc522_stop_crypto1(spi)?;
    
    // Convert data to text
    let text = bytes_to_ascii(&data);
    
    Ok(Some((uid, text)))
}

// Write data to card using blocks specified in the literature
fn write_card_data(spi: &mut Spi, text: &str) -> Result<Option<(Vec<u8>, String)>, Box<dyn Error>> {
    // Block addresses from the literature
    let block_addrs = [8, 9, 10];
    let key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Ok(None);
    }
    
    // Authenticate
    let status = mfrc522_auth(spi, PICC_AUTHENT1A, 11, &key, &uid)?;
    if status != MI_OK {
        mfrc522_stop_crypto1(spi)?;
        return Ok(None);
    }
    
    // Prepare data: text + padding to fill block_addrs.len() * 16 bytes
    let mut data = Vec::from(text.as_bytes());
    let total_space = block_addrs.len() * 16;
    data.resize(total_space, 0); // Pad with zeros
    
    // Write data to blocks
    for (i, &block_num) in block_addrs.iter().enumerate() {
        let start = i * 16;
        let end = start + 16;
        let block_data = &data[start..end];
        
        if mfrc522_write(spi, block_num, block_data)? != MI_OK {
            mfrc522_stop_crypto1(spi)?;
            return Ok(None);
        }
    }
    
    mfrc522_stop_crypto1(spi)?;
    
    // Return written text (trimmed to what actually fits)
    let written_text = String::from_utf8_lossy(&data).trim_end_matches('\0').to_string();
    
    Ok(Some((uid, written_text)))
}

// Dump all card data (Classic 1K) using the default key
fn dump_card(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Key to use for authentication
    let key = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    
    // Request tag
    let (status, _) = mfrc522_request(spi, PICC_REQIDL)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Anti-collision
    let (status, uid) = mfrc522_anticoll(spi)?;
    if status != MI_OK {
        return Ok(None);
    }
    
    // Select the tag
    let size = mfrc522_select_tag(spi, &uid)?;
    if size == 0 {
        return Ok(None);
    }
    
    println!("Card selected. UID: {}  Size: {}", uid_to_string(&uid), size);
    println!("\nDumping card data...");
    
    // Classic 1K has 16 sectors with 4 blocks each
    for sector in 0..16 {
        println!("\nSector {}", sector);
        println!("------------------");
        
        for block in 0..4 {
            let block_addr = sector * 4 + block;
            
            // Authenticate sector
            let status = mfrc522_auth(spi, PICC_AUTHENT1A, block_addr, &key, &uid)?;
            
            if status == MI_OK {
                if let Some(data) = mfrc522_read(spi, block_addr)? {
                    println!("  Block {}: {}", block_addr, bytes_to_hex(&data));
                    
                    // For non-sector trailer blocks, also show ASCII
                    if block != 3 {
                        println!("          ASCII: {}", bytes_to_ascii(&data));
                    } else {
                        // Sector trailer - display keys and access bits
                        println!("          Key A: {}", bytes_to_hex(&data[0..6]));
                        println!("          Access Bits: {}", bytes_to_hex(&data[6..10]));
                        println!("          Key B: {}", bytes_to_hex(&data[10..16]));
                    }
                } else {
                    println!("  Block {}: (Read failed)", block_addr);
                }
            } else {
                println!("  Authentication failed for Block {}", block_addr);
                break; // Can't read more blocks in this sector
            }
        }
    }
    
    mfrc522_stop_crypto1(spi)?;
    
    Ok(Some(uid))
}

// Wait for a card to be removed
fn wait_for_card_removal(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    println!("Waiting for card to be removed...");
    
    while read_card_uid(spi)?.is_some() {
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("Card removed");
    Ok(())
}

// Request card presence
fn mfrc522_request(spi: &mut Spi, req_mode: u8) -> Result<(u8, u8), Box<dyn Error>> {
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
fn mfrc522_anticoll(spi: &mut Spi) -> Result<(u8, Vec<u8>), Box<dyn Error>> {
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
fn mfrc522_select_tag(spi: &mut Spi, ser_num: &[u8]) -> Result<u8, Box<dyn Error>> {
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
fn mfrc522_auth(spi: &mut Spi, auth_mode: u8, block_addr: u8, sector_key: &[u8], serial_num: &[u8]) 
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
fn mfrc522_stop_crypto1(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    clear_bit_mask(spi, STATUS2_REG, 0x08)?;
    Ok(())
}

// Read a block from the card
fn mfrc522_read(spi: &mut Spi, block_addr: u8) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
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
fn mfrc522_write(spi: &mut Spi, block_addr: u8, write_data: &[u8]) -> Result<u8, Box<dyn Error>> {
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

// Calculate CRC
fn calculate_crc(spi: &mut Spi, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    clear_bit_mask(spi, DIV_IRQ_REG, 0x04)?;
    set_bit_mask(spi, FIFO_LEVEL_REG, 0x80)?;
    
    // Write data to FIFO
    for &byte in data {
        write_register(spi, FIFO_DATA_REG, byte)?;
    }
    
    write_register(spi, COMMAND_REG, PCD_CALCCRC)?;
    
    // Wait for CRC calculation to complete
    let mut i = 0xFF;
    let mut n: u8;
    
    loop {
        n = read_register(spi, DIV_IRQ_REG)?;
        i -= 1;
        
        if (i == 0) || ((n & 0x04) != 0) {
            break;
        }
    }
    
    // Read CRC result
    let mut result = Vec::new();
    result.push(read_register(spi, CRC_RESULT_REG_L)?);
    result.push(read_register(spi, CRC_RESULT_REG_M)?);
    
    Ok(result)
}

fn mfrc522_to_card(spi: &mut Spi, command: u8, data: &[u8]) -> Result<(u8, Vec<u8>, usize), Box<dyn Error>> {
    let mut back_data: Vec<u8> = Vec::new();
    let mut back_len: usize = 0;
    let mut status = MI_ERR;
    let mut irq_en: u8 = 0x00;
    let mut wait_irq: u8 = 0x00;
    
    if command == PCD_AUTHENT {
        irq_en = 0x12;
        wait_irq = 0x10;
    } else if command == PCD_TRANSCEIVE {
        irq_en = 0x77;
        wait_irq = 0x30;
    }
    
    // Enable interrupts
    write_register(spi, COM_IEN_REG, irq_en | 0x80)?;
    // Clear interrupt request bits
    clear_bit_mask(spi, COM_IRQ_REG, 0x80)?;
    // FlushBuffer=1, FIFO initialization
    set_bit_mask(spi, FIFO_LEVEL_REG, 0x80)?;
    // No action, cancel current commands
    write_register(spi, COMMAND_REG, PCD_IDLE)?;
    
    // Write data to FIFO
    for &byte in data {
        write_register(spi, FIFO_DATA_REG, byte)?;
    }
    
    // Execute command
    write_register(spi, COMMAND_REG, command)?;
    
    // StartSend=1, transmission of data starts
    if command == PCD_TRANSCEIVE {
        set_bit_mask(spi, BIT_FRAMING_REG, 0x80)?;
    }
    
    // Wait for the command to complete
    let mut i = 2000; // Wait timeout (higher value for more reliable operation)
    let mut n: u8;
    
    loop {
        n = read_register(spi, COM_IRQ_REG)?;
        i -= 1;
        
        // RxIRq or IdleIRq or Timer is set, or timeout
        if (i == 0) || ((n & 0x01) != 0) || ((n & wait_irq) != 0) {
            break;
        }
        
        thread::sleep(Duration::from_micros(100));
    }
    
    // Clear StartSend bit
    clear_bit_mask(spi, BIT_FRAMING_REG, 0x80)?;
    
    // Check for errors and retrieve data
    if i != 0 {
        // No error in communication
        if (read_register(spi, ERROR_REG)? & 0x1B) == 0x00 {
            status = MI_OK;
            
            // Check if CardIRq bit is set (timeout)
            if (n & irq_en & 0x01) != 0 {
                status = MI_NOTAGERR;
            }
            
            // Read data from FIFO if it's a transceive command
            if command == PCD_TRANSCEIVE {
                // Number of bytes in FIFO
                let mut fifo_len = read_register(spi, FIFO_LEVEL_REG)? as usize;
                // Last bits = Number of valid bits in the last received byte
                let last_bits = (read_register(spi, CONTROL_REG)? & 0x07) as usize;
                
                if last_bits != 0 {
                    back_len = (fifo_len - 1) * 8 + last_bits;
                } else {
                    back_len = fifo_len * 8;
                }
                
                // No data in FIFO
                if fifo_len == 0 {
                    fifo_len = 1;
                }
                
                // Cap maximum read to MAX_LEN
                let read_len = if fifo_len > MAX_LEN { MAX_LEN } else { fifo_len };
                
                // Read the data from FIFO
                for _ in 0..read_len {
                    back_data.push(read_register(spi, FIFO_DATA_REG)?);
                }
            }
        } else {
            // Communication error
            status = MI_ERR;
        }
    }
    
    Ok((status, back_data, back_len))
}

// Read from MFRC522 register
fn read_register(spi: &mut Spi, reg: u8) -> Result<u8, Box<dyn Error>> {
    let tx_buf = [((reg << 1) & 0x7E) | 0x80, 0x00];
    let mut rx_buf = [0u8, 0u8];
    
    spi.transfer(&mut rx_buf, &tx_buf)?;
    
    Ok(rx_buf[1])
}

// Write to MFRC522 register
fn write_register(spi: &mut Spi, reg: u8, value: u8) -> Result<(), Box<dyn Error>> {
    let tx_buf = [(reg << 1) & 0x7E, value];
    let mut rx_buf = [0u8, 0u8];
    
    spi.transfer(&mut rx_buf, &tx_buf)?;
    
    Ok(())
}

// Set bits in register
fn set_bit_mask(spi: &mut Spi, reg: u8, mask: u8) -> Result<(), Box<dyn Error>> {
    let tmp = read_register(spi, reg)?;
    write_register(spi, reg, tmp | mask)?;
    Ok(())
}

// Clear bits in register
fn clear_bit_mask(spi: &mut Spi, reg: u8, mask: u8) -> Result<(), Box<dyn Error>> {
    let tmp = read_register(spi, reg)?;
    write_register(spi, reg, tmp & (!mask))?;
    Ok(())
}

// Format UID as a hex string
fn uid_to_string(uid: &[u8]) -> String {
    uid.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(":")
}

// Convert UID to a single decimal number
fn uid_to_num(uid: &[u8]) -> u64 {
    let mut num: u64 = 0;
    
    for &byte in uid {
        num = num * 256 + (byte as u64);
    }
    
    num
}

// Format bytes as a hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(" ")
}

// Convert bytes to ASCII string (replacing non-printable chars with dots)
fn bytes_to_ascii(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|&byte| {
            if byte >= 32 && byte <= 126 {
                byte as char
            } else {
                '.'
            }
        })
        .collect()
}
