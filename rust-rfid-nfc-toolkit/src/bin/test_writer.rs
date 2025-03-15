use anyhow::Result;
use log::{info, warn};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;
use fltk::{app, prelude::*, text::TextBuffer};
use fltk_theme::{WidgetTheme, ThemeType};
use ctrlc;

use rust_rfid_nfc_toolkit::rfid::{SPI_BUS, SPI_DEVICE, RESET_PIN, SimpleMifareRW, MFRC522Wrapper};
use rust_rfid_nfc_toolkit::ui::{WriterCommand, create_ui};
use rust_rfid_nfc_toolkit::utils::init_logging;

const PYTHON_SCRIPT_PATH: &str = "python/rfid_wrapper.py";

#[derive(Debug, Clone)]
enum UiMessage {
    SetUID(String),
    SetText(String),
}

struct WriterWorker {
    mifare_rw: SimpleMifareRW,
    ui_sender: app::Sender<UiMessage>,
    should_exit: Arc<Mutex<bool>>,
}

impl WriterWorker {
    fn new(
        mifare_rw: SimpleMifareRW, 
        ui_sender: app::Sender<UiMessage>,
        should_exit: Arc<Mutex<bool>>
    ) -> Self {
        WriterWorker {
            mifare_rw,
            ui_sender,
            should_exit,
        }
    }
    
    fn run(&mut self, receiver: Receiver<WriterCommand>) {
        loop {
            if *self.should_exit.lock().unwrap() {
                info!("Writer thread exiting due to exit flag");
                break;
            }
            
            match receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(cmd) => {
                    match cmd {
                        WriterCommand::Read => {
                            info!("Received read command");
                            self.handle_read();
                        },
                        WriterCommand::Write(text) => {
                            info!("Received write command: {}", text);
                            self.handle_write(&text);
                        },
                        WriterCommand::TestKeys => {
                            info!("Received test keys command");
                            self.handle_test_keys();
                        },
                        WriterCommand::Exit => {
                            info!("Received exit command");
                            *self.should_exit.lock().unwrap() = true;
                            break;
                        }
                    }
                },
                Err(RecvTimeoutError::Timeout) => {
                    continue;
                },
                Err(_) => {
                    break;
                }
            }
        }
        
        info!("Writer thread exited");
    }
    
    fn handle_read(&mut self) {
        let mut simple_mifare = self.mifare_rw.clone();
        
        self.ui_sender.send(UiMessage::SetText("Reading card... Please wait".to_string()));
        
        match simple_mifare.read() {
            Ok((uid, text)) => {
                info!("Successfully read data from tag: {}", text);
                
                // Format UID for display
                let uid_str = uid.iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<String>>()
                    .join(" ");
                    
                info!("Card UID: {}", uid_str);
                
                self.ui_sender.send(UiMessage::SetUID(uid_str));
                
                if text.is_empty() {
                    self.ui_sender.send(UiMessage::SetText("Card is empty (no data)".to_string()));
                } else {
                    self.ui_sender.send(UiMessage::SetText(text));
                }
            },
            Err(e) => {
                warn!("Failed to read from tag: {:?}", e);
                
                // Send error message back to UI
                self.ui_sender.send(UiMessage::SetText(format!("Error reading from tag: {:?}", e)));
            }
        }
    }
    
    fn handle_write(&mut self, text: &str) {
        let mut simple_mifare = self.mifare_rw.clone();
        
        self.ui_sender.send(UiMessage::SetText("Writing to card... Please wait".to_string()));
        
        match simple_mifare.write(text) {
            Ok(uid) => {
                // Format UID for display
                let uid_str = uid.iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<String>>()
                    .join(" ");
                    
                info!("Successfully wrote data to tag with UID: {}", uid_str);
                
                self.ui_sender.send(UiMessage::SetUID(uid_str));
                self.ui_sender.send(UiMessage::SetText(format!("Successfully wrote: {}", text)));
            },
            Err(e) => {
                warn!("Failed to write to tag: {:?}", e);
                
                // Send error message back to UI
                self.ui_sender.send(UiMessage::SetText(format!("Error writing to tag: {:?}", e)));
            }
        }
    }
    
    fn handle_test_keys(&mut self) {
        let mut simple_mifare = self.mifare_rw.clone();
        
        self.ui_sender.send(UiMessage::SetText("Testing keys... Please wait".to_string()));
        
        match simple_mifare.test_keys() {
            Ok(results) => {
                info!("Key testing complete. Found {} accessible sectors:", results.len());
                
                let mut output = String::new();
                
                if results.is_empty() {
                    output.push_str("No working keys found for any sector");
                } else {
                    output.push_str(&format!("Key testing results ({} sectors):\n", results.len()));
                    
                    for (sector, key) in &results {
                        let key_str = key.iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<String>>()
                            .join(" ");
                        output.push_str(&format!("Sector {}: Key {}\n", sector, key_str));
                    }
                }
                
                // here we send the results to UI
                self.ui_sender.send(UiMessage::SetText(output));
            },
            Err(e) => {
                warn!("Failed to test keys: {:?}", e);
                
                // and send error message back to UI
                self.ui_sender.send(UiMessage::SetText(format!("Error testing keys: {:?}", e)));
            }
        }
    }
}

pub fn main() -> Result<()> {
    init_logging(true)?;
    
    info!("MIFARE Card Writer Test");
    
    let should_exit = Arc::new(Mutex::new(false));
    let should_exit_clone = should_exit.clone();
    
    let app = app::App::default();
    
    let theme = WidgetTheme::new(ThemeType::Dark);
    theme.apply();
    
    let (ui_sender, ui_receiver) = app::channel::<UiMessage>();
    
    let (worker_sender, worker_receiver) = channel();
    
    let (mut window, input, mut uid_label, mut data_display, mut buffer) = create_ui(worker_sender.clone())?;
    window.show();
    
    info!("Initializing MFRC522 hardware...");
    
    if !Path::new(PYTHON_SCRIPT_PATH).exists() {
        buffer.set_text("ERROR: Python script not found. Please check your installation.");
        app.run()?;
        return Err(anyhow::anyhow!("Python script not found at: {}", PYTHON_SCRIPT_PATH));
    }
    
    let mfrc522_wrapper = MFRC522Wrapper::new(SPI_BUS, SPI_DEVICE, RESET_PIN)?;
    let mifare_rw = SimpleMifareRW::from_mfrc522(mfrc522_wrapper.clone(), PYTHON_SCRIPT_PATH);
    
    // here is the Ctrl+C handler for graceful exit
    let mfrc522_for_signal = mfrc522_wrapper.clone();
    let should_exit_signal = should_exit.clone();
    ctrlc::set_handler(move || {
        info!("Cleaning up and exiting...");
        *should_exit_signal.lock().unwrap() = true;
        let _ = mfrc522_for_signal.cleanup();
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");
    
    // a new worker is created and we start the thread
    let mut worker = WriterWorker::new(mifare_rw, ui_sender, should_exit.clone());
    
    let worker_thread = thread::spawn(move || {
        worker.run(worker_receiver);
    });
    
    // here we give instructions to the user with a welcome message
    buffer.set_text("Ready. Place a card near the reader and press a button.");
    
    // Set up UI message handler
    while app.wait() {
        // be on the lookout and process any UI updates from the worker thread
        if let Some(msg) = ui_receiver.recv() {
            match msg {
                UiMessage::SetUID(uid) => {
                    uid_label.set_label(&format!("UID: {}", uid));
                },
                UiMessage::SetText(text) => {
                    buffer.set_text(&text);
                },
            }
            app.redraw(); 
        }
        
        if *should_exit_clone.lock().unwrap() {
            break;
        }
    }
    
    *should_exit_clone.lock().unwrap() = true;
    
    match worker_thread.join() {
        Ok(_) => info!("Worker thread joined successfully"),
        Err(_) => warn!("Failed to join worker thread"),
    }
    
    // Cleanup before exit
    let _ = mfrc522_wrapper.cleanup();
    
    info!("Application exited cleanly");
    Ok(())
}
