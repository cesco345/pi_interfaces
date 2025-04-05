// src/bin/covid_test_writer.rs
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use chrono::{Utc, DateTime, TimeZone, Datelike, Timelike};
use serde::{Serialize, Deserialize};
use pcsc::{Card, Context, Scope, ShareMode, Protocols};
use ndef_explorer::commands::ndef_commands::send_apdu;

// Minimal COVID Test Result structure - essential fields only
#[derive(Serialize, Deserialize, Debug)]
struct CovidTestResult {
    res: String,       // result as "p" (positive), "n" (negative), "i" (invalid)
    ts: i64,           // timestamp of test as Unix epoch
    exp: i64,          // expiration timestamp of test result (calculated from ts + validity hours)
    lot: String,       // lot_number
    mfg: i64,          // manufacturing date as Unix epoch
    shf: u32,          // shelf life in months
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("COVID-19 Test Result Writer for NTAG213");
    println!("=======================================\n");
    
    // Create a minimal test result with only essential data
    let test_result = create_compact_test_result();
    
    // Serialize the test result to JSON
    let json_data = serde_json::to_string(&test_result)?;
    println!("Minimal Test Result JSON ({}bytes):", json_data.len());
    println!("{}", json_data);
    
    // Calculate actual NDEF message size
    let ndef_record = create_ndef_text_record(&json_data);
    let ndef_size = ndef_record.len();
    let ntag213_capacity = 144;
    
    println!("\nActual NDEF size: {} bytes", ndef_size);
    
    if ndef_size > ntag213_capacity {
        println!("⚠️ Warning: Data size still too large for NTAG213 ({}/{} bytes)", 
            ndef_size, ntag213_capacity);
        println!("Consider using NTAG215/216 or further reducing data.");
        return Err("Data too large for NTAG213".into());
    } else {
        println!("✅ Data will fit within NTAG213 capacity ({}/{} bytes)", 
            ndef_size, ntag213_capacity);
        
        // Human-readable interpretation
        let timestamp = test_result.ts;
        let expiration = test_result.exp;
        let manufacturing_date = test_result.mfg;
        let now = Utc::now().timestamp();
        
        // Calculate physical test expiration date (manufacturing date + shelf life)
        let mfg_date = DateTime::<Utc>::from_timestamp(manufacturing_date, 0).unwrap();
        
        // Fixed type conversion issues
        let mut shelf_exp_year = mfg_date.year();
        let mut shelf_exp_month = mfg_date.month() + test_result.shf as u32;
        
        // Adjust year if the month exceeds 12
        if shelf_exp_month > 12 {
            shelf_exp_year += ((shelf_exp_month - 1) / 12) as i32;
            shelf_exp_month = ((shelf_exp_month - 1) % 12) + 1;
        }
        
        let shelf_exp_date = Utc.with_ymd_and_hms(
            shelf_exp_year, 
            shelf_exp_month, 
            mfg_date.day(), 
            mfg_date.hour(), 
            mfg_date.minute(), 
            mfg_date.second()
        ).unwrap();
        
        println!("\nTest Details:");
        println!("- Result: {}", if test_result.res == "p" { "Positive" } 
                               else if test_result.res == "n" { "Negative" } 
                               else { "Invalid" });
        println!("- Test Date: {}", DateTime::<Utc>::from_timestamp(timestamp, 0)
                                    .unwrap()
                                    .format("%Y-%m-%d %H:%M:%S UTC"));
        println!("- Test Result Valid Until: {}", DateTime::<Utc>::from_timestamp(expiration, 0)
                                     .unwrap()
                                     .format("%Y-%m-%d %H:%M:%S UTC"));
        println!("- Manufacturing Date: {}", mfg_date.format("%Y-%m-%d"));
        println!("- Lot Number: {}", test_result.lot);
        println!("- Shelf Life: {} months", test_result.shf);
        println!("- Test Kit Expires: {}", shelf_exp_date.format("%Y-%m-%d"));
        println!("- Test Result Status: {}", if now < expiration { "VALID" } else { "EXPIRED" });
        println!("- Test Kit Status: {}", if now < shelf_exp_date.timestamp() { "NOT EXPIRED" } else { "EXPIRED" });
    }
    
    // Confirm with user
    print!("\nContinue with writing to NTAG213? (y/n): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return Ok(());
    }
    
    // Simple approach: wait for user to place the card
    println!("\nPlace your NTAG213 card on the reader and press Enter to continue...");
    io::stdout().flush()?;
    let mut wait_input = String::new();
    io::stdin().read_line(&mut wait_input)?;
    
    // Small delay after user confirms card placement
    thread::sleep(Duration::from_millis(300));
    
    // Connect to the card
    match connect_to_card() {
        Ok((ctx, card)) => {
            // Write data to the NTAG213 card
            match write_to_ntag213(&card, &json_data) {
                Ok(_) => {
                    println!("\n✅ Successfully wrote COVID-19 test result to NTAG213 tag");
                    println!("Please remove the card from the reader when finished.");
                    Ok(())
                },
                Err(e) => {
                    println!("\n❌ Error writing to card: {}", e);
                    Err("Failed to write data to card".into())
                }
            }
        },
        Err(e) => {
            println!("\n❌ Could not connect to card: {}", e);
            println!("Make sure the card is properly placed on the reader.");
            Err("Card connection failed".into())
        }
    }
}

fn create_compact_test_result() -> CovidTestResult {
    // Get current timestamp for test
    let now = Utc::now();
    let timestamp = now.timestamp();
    
    // Test result validity (72 hours)
    let valid_hours = 72;
    let expiration = timestamp + (valid_hours * 3600);
    
    // Set manufacturing date (example: 3 months ago)
    let manufacturing_date = Utc.with_ymd_and_hms(2025, 1, 15, 0, 0, 0).unwrap();
    let manufacturing_timestamp = manufacturing_date.timestamp();
    
    // Shelf life in months (e.g., 6 months from manufacturing date)
    let shelf_life_months = 6;
    
    CovidTestResult {
        res: "p".to_string(),                // "p" for positive, "n" for negative, "i" for invalid
        ts: timestamp,                       // Unix timestamp of test
        exp: expiration,                     // Expiration timestamp of test result
        lot: "25-04-123".to_string(),        // Lot number
        mfg: manufacturing_timestamp,        // Manufacturing date timestamp
        shf: shelf_life_months,              // Shelf life in months
    }
}

fn connect_to_card() -> Result<(Context, Card), Box<dyn Error>> {
    let ctx = Context::establish(Scope::User)?;
    
    let mut readers_buf = [0; 2048];
    let readers = ctx.list_readers(&mut readers_buf)?;
    
    let mut reader_found = false;
    let mut selected_reader = None;
    
    for reader in readers {
        reader_found = true;
        selected_reader = Some(reader);
        println!("Found reader: {}", reader.to_string_lossy());
        break;
    }
    
    if !reader_found {
        return Err("No smart card readers found".into());
    }
    
    let reader = selected_reader.ok_or("Failed to get reader")?;
    println!("Using reader: {}", reader.to_string_lossy());
    
    // Try to connect to the card
    let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    println!("Successfully connected to card");
    
    Ok((ctx, card))
}

fn write_to_ntag213(card: &Card, json_data: &str) -> Result<(), Box<dyn Error>> {
    println!("\nWriting COVID-19 test data to NTAG213 tag...");
    
    // Create NDEF Text Record with the JSON data
    let data_bytes = create_ndef_text_record(json_data);
    
    // Check final data size
    println!("Final NDEF data size: {} bytes", data_bytes.len());
    
    if data_bytes.len() > 144 {
        return Err(format!("Data too large ({} bytes) for NTAG213 capacity (144 bytes)", 
                          data_bytes.len()).into());
    }
    
    // NTAG213 specific - we can only write to pages 4-39
    // Each page is 4 bytes, but we need to first prepare the tag
    // with proper NDEF formatting starting at page 3 (CC)
    
    // Page 3 (CC) - Standard values for NTAG213
    let cc_page = [0xE1, 0x10, 0x6D, 0x00];
    let write_cc_cmd = [0xFF, 0xD6, 0x00, 0x03, 0x04, cc_page[0], cc_page[1], cc_page[2], cc_page[3]];
    
    println!("Preparing NTAG213 tag for writing...");
    if let Some(_) = send_apdu(card, &write_cc_cmd, "Initialize tag") {
        println!("Tag initialized successfully");
    } else {
        println!("Warning: Tag initialization may not be complete");
    }
    
    // Add a short delay between operations
    thread::sleep(Duration::from_millis(200));
    
    println!("Writing test result data ({} bytes)...", data_bytes.len());
    
    // Ensure we don't try to write more data than the tag can hold
    let start_page = 4;
    let end_page = 39;
    let page_size = 4;
    let max_data_len = (end_page - start_page + 1) * page_size;
    
    if data_bytes.len() > max_data_len {
        return Err(format!("Data too large ({} bytes) for NTAG213 capacity ({} bytes)", 
                           data_bytes.len(), max_data_len).into());
    }
    
    // Write data in 4-byte pages
    let mut success_count = 0;
    let mut current_page = start_page;
    
    for chunk in data_bytes.chunks(page_size) {
        println!("Writing to page {}: {:02X} {:02X} {:02X} {:02X}", 
                 current_page, 
                 chunk.get(0).unwrap_or(&0), 
                 chunk.get(1).unwrap_or(&0), 
                 chunk.get(2).unwrap_or(&0), 
                 chunk.get(3).unwrap_or(&0));
        
        // Create APDU command with u8 values
        let cmd_header: [u8; 5] = [0xFF, 0xD6, 0x00, current_page as u8, 4];
        
        // Create a new vector with all u8 values
        let mut write_cmd = Vec::with_capacity(cmd_header.len() + chunk.len());
        write_cmd.extend_from_slice(&cmd_header);
        
        // Pad the chunk if needed
        let mut padded_chunk = chunk.to_vec();
        while padded_chunk.len() < page_size {
            padded_chunk.push(0x00);
        }
        
        write_cmd.extend_from_slice(&padded_chunk);
        
        if let Some(_) = send_apdu(card, &write_cmd, &format!("Write Page {}", current_page)) {
            success_count += 1;
        } else {
            // Try alternative write method using direct commands for ACR122U
            let mut direct_cmd = vec![0xFF, 0x00, 0x00, 0x00, 0x05 + padded_chunk.len() as u8, 
                                      0xD4, 0x40, current_page as u8];
            direct_cmd.extend_from_slice(&padded_chunk);
            
            if let Some(_) = send_apdu(card, &direct_cmd, &format!("Direct Write Page {}", current_page)) {
                success_count += 1;
            } else {
                println!("  Failed to write page {}", current_page);
                return Err(format!("Failed to write to page {}", current_page).into());
            }
        }
        
        current_page += 1;
    }
    
    if success_count > 0 {
        Ok(())
    } else {
        Err("Failed to write data to NTAG213 tag".into())
    }
}

fn create_ndef_text_record(text: &str) -> Vec<u8> {
    let text_bytes = text.as_bytes();
    let text_len = text_bytes.len();
    
    // NDEF Record header (Text Record, short record)
    let mut ndef_record = Vec::with_capacity(text_len + 16);
    
    // NDEF TLV header (0x03 = NDEF Message, length follows)
    ndef_record.push(0x03); // NDEF Message TLV
    
    // Calculate the total record length
    let total_len = text_len + 7; // TNF + Type Length + Payload Length + Type + Status + Language + Text
    
    if total_len > 254 {
        // For very long records (very unlikely for NTAG213 with 144 bytes)
        ndef_record.push(0xFF);
        ndef_record.push((total_len >> 8) as u8);
        ndef_record.push((total_len & 0xFF) as u8);
    } else {
        ndef_record.push(total_len as u8);
    }
    
    // NDEF Record header
    ndef_record.push(0xD1); // TNF=0x01 (NFC Forum Well Known Type), SR=1, MB=1, ME=1, IL=0
    ndef_record.push(0x01); // Type Length = 1
    ndef_record.push((text_len + 3) as u8); // Payload Length (language code + text)
    ndef_record.push(0x54); // 'T' (Text record type)
    
    // Text record payload
    ndef_record.push(0x02); // Status byte (UTF-8, 2-byte language code)
    ndef_record.push(0x65); // 'e'
    ndef_record.push(0x6E); // 'n'
    
    // Add the actual text
    ndef_record.extend_from_slice(text_bytes);
    
    // Add TLV terminator
    ndef_record.push(0xFE);
    
    ndef_record
}
