// File: src/interpreter/mod.rs
pub mod ndef_interpreter;
pub mod ndef_interpreter_payload;

// Re-export commonly used items for easier access
pub use self::ndef_interpreter::{
    parse_capability_container,
    interpret_ndef_message,
    try_interpret_ndef
};

pub use self::ndef_interpreter_payload::{
    interpret_ndef_payload,
    interpret_text_record,
    interpret_uri_record
};
