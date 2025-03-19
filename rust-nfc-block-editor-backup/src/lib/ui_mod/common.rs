use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

/// Helper function for countdown timer when placing card
pub fn countdown_for_card_placement(seconds: u64) -> Result<(), Box<dyn Error>> {
    println!("");
    println!("Prepare your card. You have {} seconds to place it on the reader...", seconds);
    
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
    
    println!("");
    println!("Reading card now...");
    Ok(())
}

/// Wait for user input
pub fn wait_for_input(prompt: &str) -> Result<String, Box<dyn Error>> {
    print!("{}", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    // Trim newline characters
    Ok(input.trim().to_string())
}

/// Clear the terminal screen
pub fn clear_screen() {
    print!("{}[2J", 27 as char);
    print!("{}[1;1H", 27 as char);
}
