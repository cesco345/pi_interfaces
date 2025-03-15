use anyhow::{Context, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;
use std::thread;

// define response structs for deserializing Python JSON output
#[derive(Debug, Deserialize)]
pub struct ReadCardResponse {
    pub success: bool,
    pub uid: Option<String>,
    pub text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WriteCardResponse {
    pub success: bool,
    pub uid: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeySector {
    pub sector: u8,
    pub key: String,
    #[serde(rename = "type")]  // Rename to match Python's field name "type" instead of "key_type"
    pub key_type: String,
}

#[derive(Debug, Deserialize)]
pub struct TestKeysResponse {
    pub success: bool,
    pub uid: Option<String>,
    #[serde(rename = "sectors")]
    pub sectors: Vec<KeySector>,
    pub error: Option<String>,
}

/// convert UID string to vec of bytes
pub fn uid_string_to_bytes(uid_str: &str) -> Vec<u8> {
    uid_str
        .split_whitespace()
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect()
}

/// create a struct that encapsulates RFID operations using Python's SimpleMFRC522 library
pub struct PythonRFID {
   pub  python_script_path: String,
}

impl PythonRFID {
    /// create a new PythonRFID instance
    pub fn new(script_path: &str) -> Self {
        PythonRFID {
            python_script_path: script_path.to_owned(),
        }
    }
    
    /// read a card using Python
    pub fn read_card(&self) -> Result<(Vec<u8>, String)> {
        info!("Reading card using the bridge...");
        
        // execute Python script with "read" command
        let output = Command::new("python3")
            .arg(&self.python_script_path)
            .arg("read")
            .output()
            .context("Failed to execute Python script")?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Python script error: {}", stderr);
            return Err(anyhow::anyhow!("Python script error: {}", stderr));
        }
        
        // parse JSON response
        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("Python response: {}", stdout);
        
        let response: ReadCardResponse = serde_json::from_str(&stdout)
            .context("Failed to parse Python response")?;
            
        if !response.success {
            return Err(anyhow::anyhow!("Python read failed: {}", 
                response.error.unwrap_or_else(|| "Unknown error".to_string())));
        }
        
        // extract UID and text
        let uid_str = response.uid.ok_or_else(|| anyhow::anyhow!("No UID returned"))?;
        let text = response.text.unwrap_or_default();
        
        // convert UID string to bytes
        let uid = uid_string_to_bytes(&uid_str);
        
        info!("Successfully read card with UID: {}", uid_str);
        Ok((uid, text))
    }
    
    /// write to a card using Python
    pub fn write_card(&self, text: &str) -> Result<Vec<u8>> {
        info!("Writing card using Python bridge...");
        
        // execute Python script with "write" command
        let output = Command::new("python3")
            .arg(&self.python_script_path)
            .arg("write")
            .arg(text)
            .output()
            .context("Failed to execute Python script")?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Python script error: {}", stderr);
            return Err(anyhow::anyhow!("Python script error: {}", stderr));
        }
        
        // parse JSON response
        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("Python response: {}", stdout);
        
        let response: WriteCardResponse = serde_json::from_str(&stdout)
            .context("Failed to parse Python response")?;
            
        if !response.success {
            return Err(anyhow::anyhow!("Python write failed: {}", 
                response.error.unwrap_or_else(|| "Unknown error".to_string())));
        }
        
        // extract UID
        let uid_str = response.uid.ok_or_else(|| anyhow::anyhow!("No UID returned"))?;
        
        // convert UID string to bytes
        let uid = uid_string_to_bytes(&uid_str);
        
        info!("Successfully wrote to card with UID: {}", uid_str);
        Ok(uid)
    }
    
    /// test keys on a card using Python
    pub fn test_keys(&self) -> Result<Vec<(u8, Vec<u8>)>> {
        info!("Testing keys using the bridge...");
        
        // execute Python script with "test_keys" command
        let output = Command::new("python3")
            .arg(&self.python_script_path)
            .arg("test_keys")
            .output()
            .context("Failed to execute Python script")?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Python script error: {}", stderr);
            return Err(anyhow::anyhow!("Python script error: {}", stderr));
        }
        
        // parse JSON response
        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("Python response: {}", stdout);
        
        let response: TestKeysResponse = serde_json::from_str(&stdout)
            .context("Failed to parse Python response")?;
            
        if !response.success {
            return Err(anyhow::anyhow!("Python key test failed: {}", 
                response.error.unwrap_or_else(|| "Unknown error".to_string())));
        }
        
        // convert keys to the expected format
        let mut results = Vec::new();
        for sector in &response.sectors {
            // Convert key string to bytes
            let key_bytes = uid_string_to_bytes(&sector.key);
            results.push((sector.sector, key_bytes));
        }
        
        info!("Successfully tested keys on card");
        Ok(results)
    }
}
