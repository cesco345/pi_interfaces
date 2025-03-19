// src/reader/mod.rs
mod utils;
mod communication;
mod auth;
mod card_operations;
pub mod commands;
pub mod mfrc522;

// Re-export components needed elsewhere
pub use mfrc522::MifareClassic;
pub use commands::{MI_OK, MI_ERR, PICC_REQIDL};
