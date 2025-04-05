// src/card_handling/desfire/commands.rs
//
// Command utilities for interacting with DESFire tools

use std::error::Error;
use std::process::{Command, Stdio};
use std::path::PathBuf;
use std::env;

/// Get the path to the DESFire tools
pub fn get_desfire_tools_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| "/home/pi".to_string());
    PathBuf::from(home).join("rust").join("pi_afr").join("desfire_tools")
}

/// Run the given DESFire tool binary
pub fn run_desfire_binary(binary_name: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let desfire_path = get_desfire_tools_path();
    
    println!("Running DESFire tool: {}", binary_name);
    
    let output = Command::new("cargo")
        .current_dir(&desfire_path)
        .arg("run")
        .arg("--bin")
        .arg(binary_name)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to run {}: {}", binary_name, error).into());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check if DESFire tools are available
pub fn check_desfire_tools() -> bool {
    let desfire_path = get_desfire_tools_path();
    desfire_path.exists() && desfire_path.join("Cargo.toml").exists()
}

/// Generate a script to run desfire tools interactively
pub fn generate_interactive_script(commands: &[&str], output_file: &str) -> Result<(), Box<dyn Error>> {
    let script_content = format!(
        "#!/bin/bash\n\n\
         cd {}\n\
         {}\n",
        get_desfire_tools_path().display(),
        commands.join("\n")
    );
    
    std::fs::write(output_file, script_content)?;
    
    // Make script executable
    Command::new("chmod")
        .args(&["+x", output_file])
        .status()?;
    
    println!("Created interactive script: {}", output_file);
    println!("Run this script to execute the DESFire operations");
    
    Ok(())
}
