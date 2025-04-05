use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{Utc, Datelike, Timelike};

fn main() -> io::Result<()> {
    // Get input file from command line args, or use default
    let args: Vec<String> = env::args().collect();
    let input_file = args.get(1).map_or("taginfo.txt".to_string(), |s| s.clone());
    let output_json = "ntag213_clone.json";

    println!("NTAG213 Clone Tool");
    println!("=================");
    println!("Extracting data from {} to create clone data...", input_file);

    // Read the input file
    let file = File::open(&input_file)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    // Extract UID
    let mut uid = String::new();
    for line in &lines {
        if line.contains("ID:") {
            let parts: Vec<&str> = line.split("ID:").collect();
            if parts.len() > 1 {
                uid = parts[1].trim().replace(":", "");
                break;
            }
        }
    }

    if uid.is_empty() {
        // Try alternative format
        for line in &lines {
            if line.contains("UID:") {
                let parts: Vec<&str> = line.split("UID:").collect();
                if parts.len() > 1 {
                    uid = parts[1].trim().replace(":", "");
                    break;
                }
            }
        }
    }

    if uid.is_empty() {
        println!("Warning: UID not found in the input file!");
        uid = "Unknown".to_string(); // Fixed: Convert &str to String
    }

    // Extract memory content (all pages)
    let mut memory_content = String::new();
    let mut in_memory_section = false;
    let mut page_data = vec![String::new(); 40]; // NTAG213 has 40 pages (0-39)

    for line in &lines {
        // Check if we're in the memory section
        if line.contains("Memory Content") {
            in_memory_section = true;
            continue;
        }

        if in_memory_section {
            // Look for page data format like "[00]" followed by hex values
            if line.contains("[") && line.contains("]") {
                let parts: Vec<&str> = line.split("]").collect();
                if parts.len() > 1 {
                    let page_part = parts[0].trim();
                    // Extract page number from format like "[00]"
                    let page_num = page_part
                        .trim_start_matches('[')
                        .parse::<usize>()
                        .unwrap_or(99); // Use 99 for invalid pages
                    
                    // Extract hex data
                    let hex_data_parts: Vec<&str> = parts[1].split('|').collect();
                    if hex_data_parts.len() > 0 {
                        let hex_data = hex_data_parts[0].trim();
                        
                        // Only collect user memory (pages 4-39)
                        if page_num < 40 {
                            // Store the page data
                            page_data[page_num] = hex_data.replace(" ", ":");
                            
                            // Add to overall memory content
                            if page_num >= 4 {  // User memory starts at page 4
                                if !memory_content.is_empty() {
                                    memory_content.push(':');
                                }
                                memory_content.push_str(hex_data.replace(" ", ":").as_str());
                            }
                        }
                    }
                }
            }
        }
    }

    // Clean up memory content format
    memory_content = memory_content.replace(" ", "");
    
    // If we couldn't extract memory content, create a placeholder
    if memory_content.is_empty() {
        println!("Warning: Couldn't extract memory content from the input file!");
        memory_content = "00:00:00:00".to_string();  // Fixed: Convert &str to String
    }

    // Get timestamp in milliseconds for ids
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
        
    // Get current time
    let now = Utc::now();

    // Create the JSON content with the EXACT format matching CardExport struct
    let json_content = format!(r#"{{
  "id": "card_{}",
  "name": "NTAG213_{}",
  "applicationId": 1,
  "fileId": 1,
  "fileData": "{}",
  "format": "ntag_213",
  "exportDate": "{}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z"
}}"#, 
        timestamp,
        uid,
        memory_content,
        // Format date as ISO string
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.timestamp_subsec_millis()
    );

    // Write to file - use a borrow to avoid moving json_content
    fs::write(output_json, &json_content)?;

    // Print summary information
    println!("\nTag Information:");
    println!("  Type: NTAG213");
    println!("  UID: {}", uid);
    println!("  User Memory Size: 144 bytes (36 pages of 4 bytes)");
    println!("\nMemory Content Summary:");
    for page_num in 4..=39 {
        if !page_data[page_num].is_empty() && page_data[page_num] != "00:00:00:00" {
            println!("  Page {:02}: {}", page_num, page_data[page_num]);
        }
    }

    println!("\nCreated JSON file for cloning: {}", output_json);
    println!();
    println!("To clone this tag, run:");
    println!("cargo run --bin card_writer {}", output_json);
    println!();
    println!("To emulate this tag with your mobile app:");
    println!("1. Transfer {} to your mobile device", output_json);
    println!("2. Import the JSON file into your NFC emulation app");
    println!();
    println!("Note: Only user memory (pages 4-39) will be cloned.");
    println!("      The UID of the tag cannot be changed on standard NTAG213 tags.");

    // Display the JSON content (abbreviated for readability)
    let json_preview = if json_content.len() > 500 {
        format!("{}... (truncated)", &json_content[0..500])
    } else {
        json_content.clone()
    };
    println!("\nJSON file contents (preview):");
    println!("{}", json_preview);

    Ok(())
}
