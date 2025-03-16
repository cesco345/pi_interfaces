use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time::Duration};
use std::error::Error;

// MFRC522 Register Addresses
const COMMAND_REG: u8 = 0x01;
const COM_IRQ_REG: u8 = 0x04;
const ERROR_REG: u8 = 0x06;
const STATUS1_REG: u8 = 0x07;
const FIFO_DATA_REG: u8 = 0x09;
const FIFO_LEVEL_REG: u8 = 0x0A;
const CONTROL_REG: u8 = 0x0C;
const BIT_FRAMING_REG: u8 = 0x0D;
const MODE_REG: u8 = 0x11;
const TX_MODE_REG: u8 = 0x12;
const RX_MODE_REG: u8 = 0x13;
const TX_CONTROL_REG: u8 = 0x14;
const TX_ASK_REG: u8 = 0x15;
const VERSION_REG: u8 = 0x37;

// MFRC522 Command Set
const CMD_IDLE: u8 = 0x00;
const CMD_TRANSCEIVE: u8 = 0x0C;

// MIFARE Commands
const PICC_WUPA: u8 = 0x52;
const PICC_ANTICOLL: u8 = 0x93;

// Status flags
const IRQ_RX: u8 = 0x20;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== MFRC522 Card Reader (Optimized for Your Hardware) ===");
    println!("Hold a card near the RFID module to read its ID");
    println!("Press Ctrl+C to exit\n");
    
    // Open SPI with the configuration we found works best
    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 100_000, Mode::Mode0)?;
    
    // Show version information
    let version = read_register(&mut spi, VERSION_REG)?;
    println!("MFRC522 Version: 0x{:02X}", version);
    
    // Initialize the MFRC522
    initialize_mfrc522(&mut spi)?;
    
    println!("MFRC522 initialized successfully");
    println!("Waiting for cards...\n");
    
    // Main loop - look for cards continuously
    loop {
        if let Some(uid) = read_card_uid(&mut spi)? {
            println!("Card detected!");
            println!("UID: {}", uid_to_string(&uid));
            println!("Waiting for card to be removed...");
            
            // Wait for card to be removed
            thread::sleep(Duration::from_secs(1));
            while read_card_uid(&mut spi)?.is_some() {
                thread::sleep(Duration::from_millis(200));
            }
            
            println!("Card removed. Ready for next card.\n");
        }
        
        thread::sleep(Duration::from_millis(200));
    }
}

// Initialize the MFRC522 with settings optimized for your hardware
fn initialize_mfrc522(spi: &mut Spi) -> Result<(), Box<dyn Error>> {
    // Soft reset
    write_register(spi, COMMAND_REG, 0x0F)?;
    thread::sleep(Duration::from_millis(50));
    
    // Initialize configuration
    write_register(spi, TX_MODE_REG, 0x00)?;    // Turn off CRC and use 106kbps
    write_register(spi, RX_MODE_REG, 0x00)?;    // Turn off CRC and use 106kbps
    write_register(spi, MODE_REG, 0x3D)?;       // Set defaults for CRC, polarity, etc.
    write_register(spi, CONTROL_REG, 0x10)?;    // Stop timer
    write_register(spi, TX_CONTROL_REG, 0x83)?; // Set Tx1RFEn and Tx2RFEn (antenna on)
    
    // Key setting for your specific hardware - 100% ASK modulation
    write_register(spi, TX_ASK_REG, 0x40)?;
    
    Ok(())
}

// Read a card's UID using the method that works with your hardware
fn read_card_uid(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Step 1: Send WUPA command to detect a card in the field with 100% ASK
    if !request_card(spi)? {
        return Ok(None);
    }
    
    // Step 2: Perform anticollision to get card UID
    match get_card_uid(spi)? {
        Some(uid) => Ok(Some(uid)),
        None => Ok(None),
    }
}

// Send the WUPA command to detect cards
fn request_card(spi: &mut Spi) -> Result<bool, Box<dyn Error>> {
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare the WUPA command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    write_register(spi, FIFO_DATA_REG, PICC_WUPA)?; // WUPA command
    
    // Set bit framing for 7 bits (WUPA uses 7 bits)
    write_register(spi, BIT_FRAMING_REG, 0x07)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x87)?; // Start transmission (0x80) + 7 bits
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 30; // Longer timeout for better card detection
    let mut counter = 0;
    
    while (irq_value & IRQ_RX) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    let success = (irq_value & IRQ_RX) != 0 && error == 0;
    
    if success {
        // Read the response (2-byte ATQA) but we don't use it for now
        for _ in 0..fifo_level {
            let _data = read_register(spi, FIFO_DATA_REG)?;
        }
    }
    
    Ok(success)
}

// Get the card's UID using anticollision procedure
fn get_card_uid(spi: &mut Spi) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    // Clear interrupts
    write_register(spi, COM_IRQ_REG, 0x7F)?;
    
    // Prepare for anticollision command
    write_register(spi, FIFO_LEVEL_REG, 0x80)?; // Clear FIFO
    write_register(spi, FIFO_DATA_REG, PICC_ANTICOLL)?; // Anticollision command
    write_register(spi, FIFO_DATA_REG, 0x20)?; // NVB = 0x20 (no data sent yet)
    
    // Configure for full bytes
    write_register(spi, BIT_FRAMING_REG, 0x00)?;
    
    // Start transceive command
    write_register(spi, COMMAND_REG, CMD_TRANSCEIVE)?;
    write_register(spi, BIT_FRAMING_REG, 0x80)?; // Start transmission
    
    // Wait for completion or timeout
    let mut irq_value: u8 = 0;
    let timeout = 30; // Timeout counter
    let mut counter = 0;
    
    while (irq_value & IRQ_RX) == 0 && counter < timeout {
        counter += 1;
        thread::sleep(Duration::from_millis(10));
        irq_value = read_register(spi, COM_IRQ_REG)?;
    }
    
    // Stop the transceive command
    write_register(spi, COMMAND_REG, CMD_IDLE)?;
    
    let error = read_register(spi, ERROR_REG)?;
    let fifo_level = read_register(spi, FIFO_LEVEL_REG)?;
    
    if (irq_value & IRQ_RX) != 0 && error == 0 && fifo_level >= 5 {
        // Read the UID (5 bytes: 4 UID bytes + 1 BCC)
        let mut uid = Vec::with_capacity(5);
        for _ in 0..fifo_level {
            uid.push(read_register(spi, FIFO_DATA_REG)?);
        }
        
        // We need at least 5 bytes (4 UID + BCC)
        if uid.len() >= 5 {
            // Verify BCC (checksum)
            let bcc = uid[4];
            let calc_bcc = uid[0] ^ uid[1] ^ uid[2] ^ uid[3];
            
            if bcc == calc_bcc {
                // Return UID without BCC
                Ok(Some(uid[0..4].to_vec()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

// Read from MFRC522 register
fn read_register(spi: &mut Spi, reg: u8) -> Result<u8, Box<dyn Error>> {
    let command = (reg << 1) | 0x80; // Read command: bit 7 set, bit 0 clear
    let mut rx_buf = [0u8, 0u8];
    
    spi.transfer(&mut rx_buf, &[command, 0x00])?;
    
    Ok(rx_buf[1])
}

// Write to MFRC522 register
fn write_register(spi: &mut Spi, reg: u8, value: u8) -> Result<(), Box<dyn Error>> {
    let command = reg << 1; // Write command: bit 7 clear, bit 0 clear
    let mut rx_buf = [0u8, 0u8];
    
    spi.transfer(&mut rx_buf, &[command, value])?;
    
    Ok(())
}

// Format UID as a hex string
fn uid_to_string(uid: &[u8]) -> String {
    uid.iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<String>>()
        .join(":")
}
