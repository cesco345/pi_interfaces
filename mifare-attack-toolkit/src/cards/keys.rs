// src/cards/keys.rs

/// Common default keys for Mifare Classic cards
pub const DEFAULT_KEYS: [[u8; 6]; 9] = [
    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Most common default
    [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5], // Common key
    [0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5], // Another common key
    [0x4D, 0x3A, 0x99, 0xC3, 0x51, 0xDD], // NXP factory default
    [0x1A, 0x98, 0x2C, 0x7E, 0x45, 0x9A], // Public transport
    [0xD3, 0xF7, 0xD3, 0xF7, 0xD3, 0xF7], // Building access
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // All zeros (your card uses this for Key A)
    [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56], // Test key
    [0x71, 0x4C, 0x5C, 0x88, 0x6E, 0x97]  // Another test key
];
