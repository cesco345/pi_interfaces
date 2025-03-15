use anyhow::Result;
use fltk::{
    app, 
    button::Button, 
    frame::Frame, 
    group::{Group, Flex}, 
    input::Input,
    prelude::*, 
    text::{TextBuffer, TextDisplay}, 
    window::Window,
    enums::{Color, FrameType, Align, Font}
};
use log::{info, warn};

/// Represents a command to be processed by the worker thread
#[derive(Debug, Clone)]
pub enum WriterCommand {
    Read,
    Write(String),
    TestKeys,
    Exit,
}

/// Apply a modern dark theme to the application
pub fn set_dark_theme() {
    // FLTK doesn't support setting global colors directly in older versions
    // We'll apply colors to individual widgets instead
}

/// Create a styled button with consistent appearance
fn create_styled_button(label: &str) -> Button {
    let mut btn = Button::default().with_label(label);
    btn.set_frame(FrameType::RoundedBox);
    btn.set_label_size(16);
    btn.set_color(Color::from_u32(0x494d55)); // Dark button color
    btn.set_selection_color(Color::from_u32(0x61afef)); // Blue accent
    btn
}

/// Create a modern style UI for the RFID toolkit
pub fn create_ui(sender: std::sync::mpsc::Sender<WriterCommand>) -> Result<(Window, Input, Frame, TextDisplay, TextBuffer)> {
    // Create main window
    let mut window = Window::default()
        .with_size(550, 450)
        .with_label("RFID/NFC Writer");
    window.set_color(Color::from_u32(0x282c34)); // Dark background
    
    // Create flexible layout
    let mut flex = Flex::default_fill().column();
    flex.set_margin(20);
    flex.set_spacing(15);
    
    // Header
    let mut header = Frame::default().with_label("MIFARE Card Operations");
    header.set_label_size(22);
    header.set_label_color(Color::from_u32(0x61afef)); // Blue accent
    flex.fixed(&header, 40);
    
    // Input section
    let mut input_flex = Flex::default().row();
    
    let mut input_label = Frame::default().with_label("Text to Write:");
    input_label.set_label_size(16);
    input_label.set_label_color(Color::from_u32(0xdcdfe4)); // Light text
    input_flex.fixed(&input_label, 120);
    
    let mut input = Input::default();
    input.set_color(Color::from_u32(0x363a42)); // Secondary background
    input.set_text_color(Color::from_u32(0xdcdfe4)); // Light text
    input.set_selection_color(Color::from_u32(0x61afef)); // Blue accent
    
    input_flex.end();
    flex.fixed(&input_flex, 35);
    
    // Button section
    let mut button_flex = Flex::default().row();
    button_flex.set_spacing(10);
    
    let mut read_btn = create_styled_button("Read Card");
    let mut write_btn = create_styled_button("Write Card");
    let mut test_keys_btn = create_styled_button("Test Keys");
    
    button_flex.end();
    flex.fixed(&button_flex, 45);
    
    // Info section
    let mut uid_label = Frame::default().with_label("UID:");
    uid_label.set_label_size(16);
    uid_label.set_align(Align::Left | Align::Inside);
    uid_label.set_label_color(Color::from_u32(0xdcdfe4)); // Light text
    flex.fixed(&uid_label, 30);
    
    // Data section
    let mut data_frame = Frame::default().with_label("Card Data");
    data_frame.set_label_size(16);
    data_frame.set_align(Align::TopLeft | Align::Inside);
    data_frame.set_label_color(Color::from_u32(0xdcdfe4)); // Light text
    flex.fixed(&data_frame, 30);
    
    let mut data_display = TextDisplay::default();
    data_display.set_frame(FrameType::BorderBox);
    data_display.set_color(Color::from_u32(0x363a42)); // Secondary background
    data_display.set_text_color(Color::from_u32(0xdcdfe4)); // Light text
    data_display.set_text_size(16);
    
    let buffer = TextBuffer::default();
    data_display.set_buffer(buffer.clone());
    
    // Bottom section
    let mut bottom_flex = Flex::default().row();
    bottom_flex.set_spacing(10);
    
    // Add spacer to push exit button to right
    let _spacer = Frame::default();
    
    let mut exit_btn = create_styled_button("Exit");
    exit_btn.set_color(Color::from_u32(0xe06c75)); // Red color
    exit_btn.set_label_color(Color::from_u32(0xf0f0f0)); // White text
    bottom_flex.fixed(&exit_btn, 100);
    
    bottom_flex.end();
    flex.fixed(&bottom_flex, 45);
    
    flex.end();
    window.end();
    
    // Configure button callbacks
    read_btn.set_callback({
        let sender = sender.clone();
        move |_| {
            if let Err(e) = sender.send(WriterCommand::Read) {
                warn!("Failed to send read command: {:?}", e);
            }
        }
    });
    
    write_btn.set_callback({
        let sender = sender.clone();
        let input_clone = input.clone();
        move |_| {
            let text = input_clone.value();
            if text.is_empty() {
                return;
            }
            
            if let Err(e) = sender.send(WriterCommand::Write(text)) {
                warn!("Failed to send write command: {:?}", e);
            }
        }
    });
    
    test_keys_btn.set_callback({
        let sender = sender.clone();
        move |_| {
            if let Err(e) = sender.send(WriterCommand::TestKeys) {
                warn!("Failed to send test keys command: {:?}", e);
            }
        }
    });
    
    exit_btn.set_callback({
        let sender = sender.clone();
        move |_| {
            info!("Exit button pressed");
            let _ = sender.send(WriterCommand::Exit);
            app::quit();
        }
    });
    
    Ok((window, input, uid_label, data_display, buffer))
}
