// Functions for reading data from different card types

use std::error::Error;
use std::thread;
use std::time::Duration;

use pcsc::Card;
use crate::commands::ndef_commands::send_apdu;
use crate::util::ndef_util::hex_string;

use super::types::{BlockData, SectorAccessMap};
use super::authentication::authenticate_sector;
use super::interpreter::interpret_ndef_data;

/// Read and display data from MIFARE Classic card
pub fn read_mifare_classic_data(card: &Card, sector_access_map: &mut SectorAccessMap) -> Result<Vec<BlockData>, Box<dyn Error>> {
    println!("\nReading data from MIFARE Classic card...");
    
    let mut all_blocks: Vec<BlockData> = Vec::new();
    
    println!("\nSector\tBlock\tData\t\t\t\tText");
    println!("----------------------------------------------------------------------");
    
    // Try to read block 0 (manufacturer data)
    let read_block0 = [0xFF, 0xB0, 0x00, 0x00, 0x10]; // Read 16 bytes from block 0
    if let Some(data) = send_apdu(card, &read_block0, "Read Block 0 (Manufacturer)") {
        let hex_data = hex_string(&data);
        let text = data.iter()
            .map(|&c| if c >= 32 && c <= 126 { c as char } else { '.' })
            .collect::<String>();
            
        println!("  0     0    {}\t{}", hex_data, text);
        
        all_blocks.push(BlockData {
            sector: 0,
            block: 0,
            data: data.clone(),
            text,
            accessible: true,
        });
    } else {
        println!("  0     0    (Read failed - manufacturer protected)");
        
        all_blocks.push(BlockData {
            sector: 0,
            block: 0,
            data: Vec::new(),
            text: "(Manufacturer protected)".to_string(),
            accessible: false,
        });
    }
    
    // Try to read all sectors (1-15 for MIFARE Classic 1K)
    for sector in 0..16 {
        let first_block = sector * 4;
        let last_block = first_block + 3;
        
        // Try to authenticate with Key A
        let authenticated = authenticate_sector(card, sector, 0x60);
        let mut sector_accessible = authenticated;
        
        if !authenticated {
            // If Key A fails, try Key B
            let auth_keyb = authenticate_sector(card, sector, 0x61);
            sector_accessible = auth_keyb;
        }
        
        // Store sector access status
        sector_access_map.insert(sector, sector_accessible);
        
        if !sector_accessible {
            println!("Sector {}: Authentication failed with all keys", sector);
            
            // Add inaccessible blocks to the data structure
            for block in first_block..=last_block {
                if block == 0 {
                    continue; // Already handled above
                }
                
                all_blocks.push(BlockData {
                    sector,
                    block,
                    data: Vec::new(),
                    text: "(Inaccessible)".to_string(),
                    accessible: false,
                });
            }
            
            continue;
        }
        
        // Read all blocks in this sector
        for block in first_block..=last_block {
            // Skip block 0 which we already tried to read
            if block == 0 {
                continue;
            }
            
            // Small delay to avoid reader issues
            thread::sleep(Duration::from_millis(50));
            
            // Read block
            let read_cmd = [0xFF, 0xB0, 0x00, block, 0x10]; // Read 16 bytes
            
            if let Some(data) = send_apdu(card, &read_cmd, &format!("Read Block {}", block)) {
                let hex_data = hex_string(&data);
                
                // Try to interpret as text if possible
                let text = data.iter()
                    .map(|&c| if c >= 32 && c <= 126 { c as char } else { '.' })
                    .collect::<String>();
                
                println!("  {}     {}    {}\t{}", sector, block, hex_data, text);
                
                all_blocks.push(BlockData {
                    sector,
                    block,
                    data: data.clone(),
                    text,
                    accessible: true,
                });
                
                // Check if this looks like NDEF data
                if data.len() >= 2 && data[0] == 0x03 {
                    println!("  ⮕ Potential NDEF data detected in block {}!", block);
                    interpret_ndef_data(&data);
                }
            } else {
                println!("  {}     {}    (Read failed)", sector, block);
                
                all_blocks.push(BlockData {
                    sector,
                    block,
                    data: Vec::new(),
                    text: "(Read failed)".to_string(),
                    accessible: false,
                });
            }
        }
    }
    
    Ok(all_blocks)
}

/// Try to read blocks from a Type 2 Tag (Ultralight/NTAG)
pub fn read_type2_tag_data(card: &Card) -> Result<Vec<BlockData>, Box<dyn Error>> {
    println!("\nAttempting to read as Type 2 Tag (MIFARE Ultralight/NTAG)...");
    
    let mut all_blocks: Vec<BlockData> = Vec::new();
    
    println!("\nPage\tData\t\t\tText");
    println!("----------------------------------------------");
    
    // Read the first 16 pages (more for NTAG215/216)
    for page in 0..16 {
        let read_cmd = [0xFF, 0xB0, 0x00, page, 0x04]; // Read 4 bytes (page size for Type 2)
        
        if let Some(data) = send_apdu(card, &read_cmd, &format!("Read Page {}", page)) {
            let hex_data = hex_string(&data);
            
            // Try to interpret as text if possible
            let text = data.iter()
                .map(|&c| if c >= 32 && c <= 126 { c as char } else { '.' })
                .collect::<String>();
            
            println!("  {}    {}\t{}", page, hex_data, text);
            
            all_blocks.push(BlockData {
                sector: 0, // Type 2 tags don't have sectors
                block: page,
                data: data.clone(),
                text,
                accessible: true,
            });
            
            // Check if this looks like NDEF TLV data
            if data.len() >= 1 && data[0] == 0x03 {
                println!("  ⮕ NDEF TLV detected on page {}!", page);
            }
        } else {
            println!("  {}    (Read failed)", page);
            
            all_blocks.push(BlockData {
                sector: 0,
                block: page,
                data: Vec::new(),
                text: "(Read failed)".to_string(),
                accessible: false,
            });
        }
    }
    
    Ok(all_blocks)
}

/// Try to read basic info from a DESFire card
pub fn read_desfire_basic_info(card: &Card) -> Result<(), Box<dyn Error>> {
    println!("\nAttempting to read basic DESFire information...");
    
    // Get DESFire version info
    let get_version = [0x90, 0x60, 0x00, 0x00, 0x00];
    
    if let Some(version_data) = send_apdu(card, &get_version, "Get DESFire Version") {
        println!("DESFire Version Data: {}", hex_string(&version_data));
    } else {
        println!("Could not get DESFire version info.");
    }
    
    // Try to get application IDs
    let get_apps = [0x90, 0x6A, 0x00, 0x00, 0x00];
    
    if let Some(app_data) = send_apdu(card, &get_apps, "Get Applications") {
        println!("Application List: {}", hex_string(&app_data));
    } else {
        println!("Could not get application list.");
    }
    
    println!("\nFor more detailed DESFire operations, please use 'focused_ndef_reader'.");
    
    Ok(())
}

/// Generic read attempt for unknown card types
pub fn attempt_generic_read(card: &Card) -> Result<Vec<BlockData>, Box<dyn Error>> {
    println!("\nAttempting generic read operations...");
    
    let mut all_blocks: Vec<BlockData> = Vec::new();
    
    // Try to read first few blocks as MIFARE Classic
    for block in 0..8 {
        let read_cmd = [0xFF, 0xB0, 0x00, block, 0x10]; // Read 16 bytes
        
        if let Some(data) = send_apdu(card, &read_cmd, &format!("Read Block {}", block)) {
            let hex_data = hex_string(&data);
            
            // Try to interpret as text if possible
            let text = data.iter()
                .map(|&c| if c >= 32 && c <= 126 { c as char } else { '.' })
                .collect::<String>();
            
            println!("Block {}: {}\t{}", block, hex_data, text);
            
            all_blocks.push(BlockData {
                sector: block / 4,
                block,
                data: data.clone(),
                text,
                accessible: true,
            });
        } else {
            println!("Block {}: (Read failed)", block);
            
            all_blocks.push(BlockData {
                sector: block / 4,
                block,
                data: Vec::new(),
                text: "(Read failed)".to_string(),
                accessible: false,
            });
        }
    }
    
    // Try to read first few pages as Type 2 Tag
    for page in 0..8 {
        let read_cmd = [0xFF, 0xB0, 0x00, page, 0x04]; // Read 4 bytes
        
        if let Some(data) = send_apdu(card, &read_cmd, &format!("Read Page {}", page)) {
            let hex_data = hex_string(&data);
            println!("Page {}: {}", page, hex_data);
        }
    }
    
    Ok(all_blocks)
}
