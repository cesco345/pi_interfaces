// Core modules
pub mod card;        // Card connection and basic operations
pub mod crypto;      // Cryptographic operations
pub mod util;        // Utility functions and types
pub mod error;       // Error handling and codes

// Application-specific modules
pub mod file_operations;  // File creation, reading, writing
pub mod access_control;   // Access control application
pub mod application;      // Application management

// Legacy module for backward compatibility
pub mod desfire_common;   // Re-exports from other modules
