// src/card_handling/desfire/setup.rs
use std::error::Error;

// Function to generate setup steps for DESFire NDEF compatibility
pub fn suggest_ndef_setup_steps(data: &str) -> String {
    // Create a multi-line string with the setup instructions
    let mut steps = String::new();
    
    steps.push_str("DESFire NDEF Setup Steps:\n");
    steps.push_str("-----------------------\n\n");
    steps.push_str("1. First, select the card:\n");
    steps.push_str("   $ ./desfire-tool select-application 000000\n\n");
    
    steps.push_str("2. Authenticate with default key:\n");
    steps.push_str("   $ ./desfire-tool authenticate 0 00000000000000000000000000000000\n\n");
    
    steps.push_str("3. Create NDEF application (AID: D2760000850101):\n");
    steps.push_str("   $ ./desfire-tool create-application D2760000850101 0F 01\n\n");
    
    steps.push_str("4. Select the NDEF application:\n");
    steps.push_str("   $ ./desfire-tool select-application D2760000850101\n\n");
    
    steps.push_str("5. Authenticate in the NDEF application context:\n");
    steps.push_str("   $ ./desfire-tool authenticate 0 00000000000000000000000000000000\n\n");
    
    // Calculate the size needed for NDEF data
    let data_size = data.len() + 16; // Add some overhead for NDEF headers
    let size_hex = format!("{:04X}", data_size);
    
    steps.push_str(&format!("6. Create NDEF container file (CC file, ID: 03):\n"));
    steps.push_str("   $ ./desfire-tool create-std-data-file 03 00 E0 20\n\n");
    
    steps.push_str(&format!("7. Write CC (Capability Container) to file 03:\n"));
    steps.push_str("   $ ./desfire-tool write-data 03 0 0000000220000F\n\n");
    
    steps.push_str(&format!("8. Create NDEF data file (ID: 04, size: {}):\n", size_hex));
    steps.push_str(&format!("   $ ./desfire-tool create-std-data-file 04 00 E0 {}\n\n", size_hex));
    
    // If we have actual NDEF data, provide commands to write it
    if !data.is_empty() {
        // For simplicity, we'll assume the data is already in NDEF format
        // In a real application, you'd need to format it properly
        steps.push_str("9. Write NDEF data to file 04:\n");
        steps.push_str(&format!("   $ ./desfire-tool write-data 04 0 {}\n\n", data));
    } else {
        steps.push_str("9. Write your NDEF data to file 04 (replace 'YOUR_DATA_HERE' with actual data):\n");
        steps.push_str("   $ ./desfire-tool write-data 04 0 YOUR_DATA_HERE\n\n");
    }
    
    steps.push_str("After completing these steps, your DESFire card should be properly formatted\n");
    steps.push_str("for NDEF compatibility and can be used with standard NDEF tools.\n");
    
    steps
}
