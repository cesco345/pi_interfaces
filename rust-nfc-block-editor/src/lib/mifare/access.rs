use std::fmt;

// Access bit configurations
pub struct AccessBits {
    pub c1: [bool; 4],  // Access conditions for C1 (least significant bit)
    pub c2: [bool; 4],  // Access conditions for C2
    pub c3: [bool; 4],  // Access conditions for C3 (most significant bit)
}

impl AccessBits {
    // Create access bits from raw bytes
    pub fn from_bytes(access_bytes: &[u8; 4]) -> Self {
        let mut c1 = [false; 4];
        let mut c2 = [false; 4];
        let mut c3 = [false; 4];
        
        // The access bits are in a weird order in the sector trailer
        // Byte 6 - b7 = C3b, b6 = C3a, b5 = C2b, b4 = C2a, b3 = C1b, b2 = C1a, b1 = C0b, b0 = C0a
        // Byte 7 - b7 = C1c, b6 = C1f, b5 = C1e, b4 = C1d, b3 = C0c, b2 = C0f, b1 = C0e, b0 = C0d
        // Byte 8 - b7 = C3c, b6 = C3f, b5 = C3e, b4 = C3d, b3 = C2c, b2 = C2f, b1 = C2e, b0 = C2d
        
        // Extract C1 bits
        c1[0] = (access_bytes[0] & 0b00000100) != 0; // C1a from byte 6 bit 2
        c1[1] = (access_bytes[0] & 0b00001000) != 0; // C1b from byte 6 bit 3
        c1[2] = (access_bytes[1] & 0b10000000) != 0; // C1c from byte 7 bit 7
        c1[3] = (access_bytes[1] & 0b01000000) != 0; // C1f from byte 7 bit 6
        
        // Extract C2 bits
        c2[0] = (access_bytes[0] & 0b00010000) != 0; // C2a from byte 6 bit 4
        c2[1] = (access_bytes[0] & 0b00100000) != 0; // C2b from byte 6 bit 5
        c2[2] = (access_bytes[2] & 0b00001000) != 0; // C2c from byte 8 bit 3
        c2[3] = (access_bytes[2] & 0b00000100) != 0; // C2f from byte 8 bit 2
        
        // Extract C3 bits
        c3[0] = (access_bytes[0] & 0b01000000) != 0; // C3a from byte 6 bit 6
        c3[1] = (access_bytes[0] & 0b10000000) != 0; // C3b from byte 6 bit 7
        c3[2] = (access_bytes[2] & 0b10000000) != 0; // C3c from byte 8 bit 7
        c3[3] = (access_bytes[2] & 0b01000000) != 0; // C3f from byte 8 bit 6
        
        Self { c1, c2, c3 }
    }
    
    // Convert access bits to raw bytes for writing to card
    pub fn to_bytes(&self) -> [u8; 4] {
        let mut access_bytes = [0u8; 4];
        
        // Byte 6
        if self.c1[0] { access_bytes[0] |= 0b00000100; } // C1a -> bit 2
        if self.c1[1] { access_bytes[0] |= 0b00001000; } // C1b -> bit 3
        if self.c2[0] { access_bytes[0] |= 0b00010000; } // C2a -> bit 4
        if self.c2[1] { access_bytes[0] |= 0b00100000; } // C2b -> bit 5
        if self.c3[0] { access_bytes[0] |= 0b01000000; } // C3a -> bit 6
        if self.c3[1] { access_bytes[0] |= 0b10000000; } // C3b -> bit 7
        
        // Byte 7
        if self.c1[2] { access_bytes[1] |= 0b10000000; } // C1c -> bit 7
        if self.c1[3] { access_bytes[1] |= 0b01000000; } // C1f -> bit 6
        
        // Byte 8
        if self.c2[2] { access_bytes[2] |= 0b00001000; } // C2c -> bit 3
        if self.c2[3] { access_bytes[2] |= 0b00000100; } // C2f -> bit 2
        if self.c3[2] { access_bytes[2] |= 0b10000000; } // C3c -> bit 7
        if self.c3[3] { access_bytes[2] |= 0b01000000; } // C3f -> bit 6
        
        // Byte 9 (usually 0x69 or some combination for user data byte)
        access_bytes[3] = 0x69;
        
        access_bytes
    }
    
    // Get a predefined access configuration
    pub fn get_predefined_config(config_type: &str) -> Self {
        match config_type {
            "transport" => {
                // Transport configuration - Everything accessible with Key A
                let c1 = [false, false, false, false];
                let c2 = [false, false, false, false];
                let c3 = [false, false, false, false];
                Self { c1, c2, c3 }
            },
            "secure" => {
                // Secure configuration - Data blocks read with Key A, write with Key B
                // Key A can only authenticate, Key B can read/write/auth
                let c1 = [false, false, true, false];
                let c2 = [false, true, false, false];
                // In the "secure" configuration:
                let c3 = [true, true, false, true];
                Self { c1, c2, c3 }
            },
            "readonly" => {
                // Read-only configuration - No writes allowed
                let c1 = [false, true, true, false];
                let c2 = [false, false, false, true];
                let c3 = [true, true, false, true];
                Self { c1, c2, c3 }
            },
            _ => {
                // Default to transport configuration
                let c1 = [false, false, false, false];
                let c2 = [false, false, false, false];
                let c3 = [false, false, false, false];
                Self { c1, c2, c3 }
            }
        }
    }
    
    // Interpret the access conditions for a specific block
    pub fn interpret_access(&self, block_type: &str, block_index: usize) -> String {
        let index = match block_type {
            "data" => {
                if block_index >= 3 { return "Invalid block index".to_string(); }
                block_index
            },
            "trailer" => 3,
            _ => return "Invalid block type".to_string()
        };
        
        let c1 = self.c1[index];
        let c2 = self.c2[index];
        let c3 = self.c3[index];
        
        match block_type {
            "data" => {
                match (c1, c2, c3) {
                    (false, false, false) => "R/W: Key A|B".to_string(),
                    (false, false, true) => "R: Key A|B, W: Never".to_string(),
                    (true, false, false) => "R: Key A|B, W: Key B".to_string(),
                    (true, false, true) => "R: Key B, W: Key B".to_string(),
                    (false, true, false) => "R: Key A|B, W: Never".to_string(),
                    (false, true, true) => "R: Key B, W: Never".to_string(),
                    (true, true, false) => "R: Key A|B, W: Key B".to_string(),
                    (true, true, true) => "R: Never, W: Never".to_string(),
                }
            },
            "trailer" => {
                let key_a_access = match (c1, c2) {
                    (false, false) => "R: Never, W: Key A",
                    (false, true) => "R: Never, W: Never",
                    (true, false) => "R: Never, W: Key B",
                    (true, true) => "R: Never, W: Never",
                };
                
                let access_bits_access = match (c1, c3) {
                    (false, false) => "R: Key A|B, W: Key A",
                    (true, false) => "R: Key A|B, W: Never",
                    (false, true) => "R: Key A|B, W: Key B",
                    (true, true) => "R: Key A|B, W: Never",
                };
                
                let key_b_access = match (c2, c3) {
                    (false, false) => "R: Key A|B, W: Key A",
                    (true, false) => "R: Key A|B, W: Key B",
                    (false, true) => "R: Never, W: Key A",
                    (true, true) => "R: Never, W: Never",
                };
                
                format!("Key A: {}\nAccess Bits: {}\nKey B: {}", key_a_access, access_bits_access, key_b_access)
            },
            _ => "Invalid block type".to_string()
        }
    }
}

impl fmt::Display for AccessBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Block 0: {}\n", self.interpret_access("data", 0))?;
        write!(f, "Block 1: {}\n", self.interpret_access("data", 1))?;
        write!(f, "Block 2: {}\n", self.interpret_access("data", 2))?;
        write!(f, "Block 3 (Trailer): \n{}", self.interpret_access("trailer", 0))
    }
}
