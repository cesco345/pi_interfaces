// Define shared types used across the mifare operations

use std::collections::HashMap;

/// Structure to store sector and block data
#[derive(Debug, Clone)]
pub struct BlockData {
    pub sector: u8,
    pub block: u8,
    pub data: Vec<u8>,
    pub text: String,
    pub accessible: bool,
}

/// Type alias for sector access mapping
pub type SectorAccessMap = HashMap<u8, bool>;
