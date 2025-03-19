// ---------- src/lib/mifare/magic/detect/mod.rs ----------
pub mod types;        // Data structures for detection results
pub mod card_tests;   // Basic card behavior tests
pub mod write_tests;  // Write capability tests
pub mod activation;   // Activation sequence tests
pub mod utils;         // Utility functions for detection
pub mod detect_impl;  // Main implementation

// Re-export the main detect_magic_card function for easier imports
pub use self::detect_impl::detect_magic_card;
