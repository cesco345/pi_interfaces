// src/lib/mifare/magic/mod.rs
pub mod clone;
pub mod detect;
pub mod keygen;
pub mod utils;
pub mod write;

// Re-export commonly used items for easier imports
pub use self::utils::*;  // Common utilities
pub use self::detect::detect_impl::detect_magic_card;  // Updated path to main detection function
pub use self::write::write_custom_uid;  // Main write function
pub use self::clone::clone_card;  // Main clone function
