use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

/// Helper struct to print byte slices as hex
pub struct HexSlice<'a>(pub &'a [u8]);

impl<'a> fmt::Display for HexSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{:02X} ", byte)?;
        }
        Ok(())
    }
}

/// Prompts the user to place or remove a card and handles the timing
/// for stable card detection.
/// 
/// # Arguments
/// * `action` - "place" or "remove" to customize the prompt
/// * `delay_ms` - milliseconds to wait after user confirmation
///
/// # Returns
/// Result indicating success or error
pub fn prompt_card_action(action: &str, delay_ms: u64) -> Result<(), Box<dyn Error>> {
    let message = match action.to_lowercase().as_str() {
        "place" => "PLEASE PLACE YOUR CARD on the reader now",
        "remove" => "PLEASE REMOVE ANY CARDS from the reader now",
        _ => "Please prepare the card reader",
    };
    
    println!("\n------------------------------------------------------");
    println!("{}", message);
    print!("Press ENTER when ready to continue...");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    println!("Waiting for card to stabilize ({} ms)...", delay_ms);
    sleep(Duration::from_millis(delay_ms));
    
    Ok(())
}

/// Helper function to extract status bytes from a DESFire response
pub fn get_status_bytes(response: &[u8]) -> Option<(u8, u8)> {
    if response.len() >= 2 {
        let status_word = (response[response.len() - 2], response[response.len() - 1]);
        Some(status_word)
    } else {
        None
    }
}

/// Helper function to extract data from a DESFire response (excluding status bytes)
pub fn get_response_data(response: &[u8]) -> Vec<u8> {
    if response.len() <= 2 {
        return Vec::new();
    }
    response[0..response.len()-2].to_vec()
}
