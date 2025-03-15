use anyhow::Result;
use log::{debug, info, warn};
use std::time::{Duration, Instant};

use crate::rfid::constants::*;
use crate::rfid::mfrc522::MFRC522Wrapper;
use crate::rfid::python_bridge::PythonRFID;

/// we are cheating -- creating a simplified MIFARE reader/writer implementation that delegates to Python
/// for operations that have compatibility issues with FM17522 chip variants
pub struct SimpleMifareRW {
    mfrc522: MFRC522Wrapper,
    python_rfid: PythonRFID,
    use_python: bool,
}

impl SimpleMifareRW {
    /// create a new SimpleMifareRW instance that uses both native code and Python
    pub fn new(spi_bus: u8, spi_device: u8, reset_pin: u8, python_script_path: &str) -> Result<Self> {
        let mfrc522 = MFRC522Wrapper::new(spi_bus, spi_device, reset_pin)?;
        let python_rfid = PythonRFID::new(python_script_path);
        
        Ok(SimpleMifareRW { 
            mfrc522, 
            python_rfid,
            use_python: true,  // we will default to using Python for better compatibility
        })
    }
    
    /// we need to create a SimpleMifareRW from an existing MFRC522Wrapper 
    pub fn from_mfrc522(mfrc522: MFRC522Wrapper, python_script_path: &str) -> Self {
        let python_rfid = PythonRFID::new(python_script_path);
        
        SimpleMifareRW { 
            mfrc522, 
            python_rfid,
            use_python: true, 
        }
    }
    
    /// create a clone that shares the same MFRC522 instance
pub fn clone(&self) -> Self {
    SimpleMifareRW {
        mfrc522: self.mfrc522.clone(),
        python_rfid: PythonRFID::new(&self.python_rfid.python_script_path),
        use_python: self.use_python,
    }
}    
    /// back and forth toggle whether to use Python for RFID operations
    pub fn set_use_python(&mut self, use_python: bool) {
        self.use_python = use_python;
    }
    
    /// read data from a MIFARE card
    pub fn read(&mut self) -> Result<(Vec<u8>, String)> {
        info!("Waiting for a tag to read...");
        
        if self.use_python {
            // use Python for reading (more compatible with clone cards)
            return self.python_rfid.read_card();
        }
        
        // native Rust implementation (less compatible but kept for reference)
        // this implementation is not recommended for FM17522 clone cards
        Err(anyhow::anyhow!("Native Rust read not implemented - use Python mode"))
    }
    
    /// write data to a MIFARE card
    pub fn write(&mut self, text: &str) -> Result<Vec<u8>> {
        info!("Waiting for a tag to write data to...");
        
        if self.use_python {
            // Use Python for writing (more compatible with clone cards)
            return self.python_rfid.write_card(text);
        }
        
        // Native Rust implementation (less compatible but kept for reference)
        // This implementation is not recommended for FM17522 clone cards
        Err(anyhow::anyhow!("Native Rust write not implemented - use Python mode"))
    }
    
    /// test keys on a MIFARE card
    pub fn test_keys(&mut self) -> Result<Vec<(u8, Vec<u8>)>> {
        info!("Waiting for a card to test keys...");
        
        if self.use_python {
            // Use Python for key testing (more compatible with clone cards)
            return self.python_rfid.test_keys();
        }
        
        // native Rust implementation (less compatible but kept for reference)
        // this implementation is not recommended for FM17522 clone cards
        Err(anyhow::anyhow!("Native Rust key testing not implemented - use Python mode"))
    }
    
    /// cleanup resources
    pub fn cleanup(&self) -> Result<()> {
        self.mfrc522.cleanup()?;
	Ok(())
}
}

