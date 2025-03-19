use std::io::{self, Write};

/// Print a section header
pub fn print_section_header(title: &str) {
    println!("\n=== {} ===", title);
}

/// Print a subsection header
pub fn print_subsection(title: &str) {
    println!("\n--- {} ---", title);
}

/// Print a progress bar
pub fn print_progress_bar(current: usize, total: usize, width: usize) {
    let percent = (current as f64 / total as f64) * 100.0;
    let filled = (current as f64 / total as f64 * width as f64) as usize;
    
    print!("\r[");
    for i in 0..width {
        if i < filled {
            print!("=");
        } else if i == filled {
            print!(">");
        } else {
            print!(" ");
        }
    }
    print!("] {:.1}% ({}/{})", percent, current, total);
    io::stdout().flush().unwrap();
}

/// Print a table row
pub fn print_table_row(columns: &[&str], widths: &[usize]) {
    for (i, &col) in columns.iter().enumerate() {
        let width = if i < widths.len() { widths[i] } else { 10 };
        print!("| {:<width$} ", col, width = width);
    }
    println!("|");
}

/// Print a table header with separator
pub fn print_table_header(columns: &[&str], widths: &[usize]) {
    print_table_row(columns, widths);
    
    // Print separator line
    print!("|");
    for &width in widths {
        for _ in 0..width+2 {
            print!("-");
        }
        print!("|");
    }
    println!();
}

/// Print a hex dump of data
pub fn print_hex_dump(data: &[u8], bytes_per_line: usize) {
    for (i, chunk) in data.chunks(bytes_per_line).enumerate() {
        // Address
        print!("{:04X}: ", i * bytes_per_line);
        
        // Hex values
        for (j, &byte) in chunk.iter().enumerate() {
            print!("{:02X} ", byte);
            if j % 4 == 3 {
                print!(" ");
            }
        }
        
        // Padding for incomplete last line
        for j in chunk.len()..bytes_per_line {
            print!("   ");
            if (chunk.len() + j - 1) % 4 == 3 {
                print!(" ");
            }
        }
        
        // ASCII representation
        print!(" |");
        for &byte in chunk {
            if byte >= 32 && byte <= 126 {
                print!("{}", byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("✅ {}", message);
}

/// Print an error message
pub fn print_error(message: &str) {
    println!("❌ {}", message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("⚠️ {}", message);
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("ℹ️ {}", message);
}
