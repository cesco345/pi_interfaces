/// Print DESFire error codes as descriptive strings
pub fn print_desfire_error(error_code: u8) -> &'static str {
    match error_code {
        0x00 => "Operation successful",
        0x0C => "No changes made",
        0x0E => "Out of EEPROM memory",
        0x1C => "Illegal command code",
        0x1E => "Integrity error",
        0x40 => "No such key",
        0x6E => "Error in authentication",
        0x7E => "More data available",
        0x9C => "Permission denied (authentication required)",
        0x9E => "Parameter error",
        0xA0 => "Application not found",
        0xAE => "Authentication error",
        0xDE => "Duplicate file/application",
        0xEE => "File not found",
        0xF0 => "File/application parameter error",
        0xCA => "Command aborted",
        _ => "Unknown error code",
    }
}

/// Check if a DESFire response indicates success
pub fn is_operation_success(status: &[u8]) -> bool {
    status.len() >= 2 && 
    status[status.len() - 2] == 0x91 && 
    status[status.len() - 1] == 0x00
}

/// Check if a DESFire response indicates more data is available
pub fn is_more_data_available(status: &[u8]) -> bool {
    status.len() >= 2 && 
    status[status.len() - 2] == 0x91 && 
    status[status.len() - 1] == 0xAF
}

/// Converts a DESFire status code to a Result
pub fn check_desfire_status<T>(data: T, status: &[u8]) -> Result<T, String> {
    if is_operation_success(status) {
        Ok(data)
    } else if status.len() >= 2 {
        let error = status[status.len() - 1];
        Err(format!("DESFire error: {:02X} ({})", 
            error, print_desfire_error(error)))
    } else {
        Err("Invalid response format".to_string())
    }
}
