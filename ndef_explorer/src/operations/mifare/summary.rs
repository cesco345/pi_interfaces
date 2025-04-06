// Functions for generating and displaying summary information

use std::fmt::Write as FmtWrite;

use super::types::{BlockData, SectorAccessMap};
use super::interpreter::interpret_ndef_data;

/// Display a summary of card data
pub fn display_summary(blocks: &[BlockData], sector_access_map: &SectorAccessMap, card_type: &str) {
    println!("\n\n===============================================");
    println!("             CARD SUMMARY");
    println!("===============================================");
    println!("Card Type: {}", card_type);
    
    // Display overview of accessible vs inaccessible sectors
    display_sector_access_status(sector_access_map);
    
    // Find all text content
    display_text_content(blocks);
    
    // Display a colorful memory map of the card
    display_memory_map(blocks, card_type);
    
    // Display potential NDEF messages
    display_ndef_messages(blocks);
    
    println!("\n===============================================");
}

/// Display sector access status
fn display_sector_access_status(sector_access_map: &SectorAccessMap) {
    println!("\nSector Access Status:");
    println!("-------------------");
    let mut accessible_sectors = 0;
    let mut total_sectors = 0;
    
    for (&sector, &accessible) in sector_access_map {
        println!("Sector {:2}: {}", sector, if accessible { "✅ Accessible" } else { "❌ Inaccessible" });
        if accessible {
            accessible_sectors += 1;
        }
        total_sectors += 1;
    }
    
    println!("\nAccessible Sectors: {}/{} ({}%)", 
            accessible_sectors, 
            total_sectors, 
            (accessible_sectors as f32 / total_sectors as f32 * 100.0) as u8);
}

/// Display text content found on the card
fn display_text_content(blocks: &[BlockData]) {
    println!("\nText Content Found:");
    println!("------------------");
    let mut found_text = false;
    
    for block in blocks {
        if block.accessible && !block.text.is_empty() {
            // Only show blocks with visible text characters
            let visible_chars = block.text.chars()
                .filter(|&c| c != '.' && c != ' ' && c != '\0')
                .count();
            
            if visible_chars > 0 {
                found_text = true;
                println!("Sector {:2}, Block {:2}: \"{}\"", block.sector, block.block, block.text);
            }
        }
    }
    
    if !found_text {
        println!("No readable text found on this card.");
    }
}

/// Display a memory map of the card
fn display_memory_map(blocks: &[BlockData], card_type: &str) {
    println!("\nMemory Map:");
    println!("-----------");
    
    // For MIFARE Classic, display a 4x16 grid (16 sectors, 4 blocks each)
    if card_type.contains("Classic") {
        let mut memory_map = String::new();
        for sector in 0..16 {
            writeln!(&mut memory_map, "Sector {:2}: ", sector).unwrap();
            
            for block in 0..4 {
                let block_num = sector * 4 + block;
                let block_data = blocks.iter()
                    .find(|b| b.sector == sector && b.block == block_num);
                
                if let Some(block_info) = block_data {
                    let access_marker = if block_info.accessible { "✓" } else { "✗" };
                    
                    // Special formatting for different block types
                    let block_desc = match (sector, block) {
                        (0, 0) => format!("  {}{} [Manufacturer Block]", access_marker, block_num),
                        (_, 3) => format!("  {}{} [Sector Trailer]", access_marker, block_num),
                        _ => format!("  {}{} [Data Block]", access_marker, block_num),
                    };
                    
                    write!(&mut memory_map, "{}", block_desc).unwrap();
                    
                    // Add preview of data (truncated)
                    if block_info.accessible && !block_info.data.is_empty() {
                        let data_preview = if block_info.text.contains(|c| c != '.' && c != ' ') {
                            format!(" - \"{}\"", 
                                block_info.text.chars()
                                    .filter(|&c| c != '.' && c != ' ' && c != '\0')
                                    .take(15)
                                    .collect::<String>())
                        } else {
                            "".to_string()
                        };
                        
                        writeln!(&mut memory_map, "{}", data_preview).unwrap();
                    } else {
                        writeln!(&mut memory_map, "").unwrap();
                    }
                } else {
                    writeln!(&mut memory_map, "  ✗{} [Unknown]", block_num).unwrap();
                }
            }
            writeln!(&mut memory_map, "").unwrap();
        }
        
        println!("{}", memory_map);
    }
}

/// Display potential NDEF messages found on the card
fn display_ndef_messages(blocks: &[BlockData]) {
    println!("\nNDEF Messages:");
    println!("-------------");
    let mut found_ndef = false;
    
    for block in blocks {
        if block.accessible && block.data.len() >= 2 && block.data[0] == 0x03 {
            found_ndef = true;
            println!("NDEF message found at Sector {}, Block {}", block.sector, block.block);
            
            // Try to decode NDEF message
            interpret_ndef_data(&block.data);
        }
    }
    
    if !found_ndef {
        println!("No NDEF messages found on this card.");
    }
}
