use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{Utc, Datelike, Timelike};

fn main() -> io::Result<()> {
    // Get input file from command line args, or use default
    let args: Vec<String> = env::args().collect();
    let input_file = args.get(1).map_or("paste.txt".to_string(), |s| s.clone());
    let output_json = "mifare_clone.json";

    println!("Extracting data from {} to create clone data...", input_file);

    // Read the input file
    let file = File::open(&input_file)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    // Extract UID
    let mut uid = String::new();
    for line in &lines {
        if line.contains("UID:") {
            let parts: Vec<&str> = line.split("UID:").collect();
            if parts.len() > 1 {
                uid = parts[1].trim().replace(":", "");
                break;
            }
        }
    }

    // Extract data from blocks 8 and 9 (Sector 2)
    let mut block8_text = String::new();
    let mut block9_text = String::new();
    
    // Find block 8 data
    for (i, line) in lines.iter().enumerate() {
        if line.contains("[08] rwi") && i + 1 < lines.len() {
            let next_line = &lines[i + 1];
            if next_line.contains("|") {
                let parts: Vec<&str> = next_line.split("|").collect();
                if parts.len() > 1 {
                    block8_text = parts[1].trim().to_string();
                }
            }
        }
    }
    
    // Find block 9 data
    for (i, line) in lines.iter().enumerate() {
        if line.contains("[09] rwi") && i + 1 < lines.len() {
            let next_line = &lines[i + 1];
            if next_line.contains("|") {
                let parts: Vec<&str> = next_line.split("|").collect();
                if parts.len() > 1 {
                    block9_text = parts[1].trim().to_string();
                }
            }
        }
    }

    // Combine data from blocks 8 and 9
    let text_data = format!("{} {}", block8_text, block9_text).trim().to_string();

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
  "name": "Card{}",
  "applicationId": 1,
  "fileId": 1,
  "fileData": "{}",
  "format": "mifare_classic",
  "exportDate": "{}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z"
}}"#, 
        timestamp,
        uid,
        text_data,
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

    println!("Created JSON file for cloning: {}", output_json);
    println!("Card data: {}", text_data);
    println!();
    println!("To clone this card, run:");
    println!("cargo run --bin mifare_writer {}", output_json);
    println!();
    println!("After writing, verify with:");
    println!("cargo run --bin mifare_reader");

    // Display the JSON content
    println!("\nJSON file contents:");
    println!("{}", json_content);

    Ok(())
}
