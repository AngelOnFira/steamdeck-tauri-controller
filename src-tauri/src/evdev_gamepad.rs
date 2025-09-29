use evdev::{Device, EventType, Key, AbsoluteAxisType, InputEventKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvdevGamepadInfo {
    pub device_path: String,
    pub name: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub is_gamepad: bool,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvdevControllerEvent {
    pub device_path: String,
    pub event_type: String,
    pub code: u16,
    pub value: i32,
    pub timestamp: u64,
}

pub struct EvdevGamepadManager {
    devices: Arc<Mutex<HashMap<String, Device>>>,
    gamepad_devices: Arc<Mutex<Vec<EvdevGamepadInfo>>>,
}

impl EvdevGamepadManager {
    pub fn new() -> Result<Self, String> {
        println!("🔧 Initializing EvdevGamepadManager for Steam Deck compatibility...");
        
        Ok(Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            gamepad_devices: Arc::new(Mutex::new(Vec::new())),
        })
    }
    
    pub fn scan_for_gamepad_devices(&self) -> Result<(), String> {
        let mut devices = self.devices.lock().unwrap();
        let mut gamepad_devices = self.gamepad_devices.lock().unwrap();
        
        devices.clear();
        gamepad_devices.clear();
        
        println!("🔍 Scanning /dev/input for gamepad devices...");
        
        let input_dir = Path::new("/dev/input");
        if !input_dir.exists() {
            return Err("❌ /dev/input directory not found".to_string());
        }
        
        let entries = read_dir(input_dir)
            .map_err(|e| format!("❌ Failed to read /dev/input: {}", e))?;
            
        for entry in entries {
            let entry = entry.map_err(|e| format!("❌ Failed to read entry: {}", e))?;
            let path = entry.path();
            
            if let Some(file_name) = path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    // Only check event devices
                    if name_str.starts_with("event") {
                        match self.analyze_device(&path) {
                            Ok(Some(info)) => {
                                println!("🎮 Found potential gamepad: {}", info.name);
                                
                                // Try to open the device
                                match Device::open(&path) {
                                    Ok(device) => {
                                        devices.insert(path.to_string_lossy().to_string(), device);
                                        gamepad_devices.push(info);
                                        println!("✅ Successfully opened: {}", path.display());
                                    }
                                    Err(e) => {
                                        println!("⚠️  Could not open {}: {} (permissions?)", path.display(), e);
                                        // Still add to list but mark as inaccessible
                                        let mut info_copy = info;
                                        info_copy.name = format!("{} (No Access)", info_copy.name);
                                        gamepad_devices.push(info_copy);
                                    }
                                }
                            }
                            Ok(None) => {
                                // Not a gamepad device, ignore
                            }
                            Err(e) => {
                                println!("⚠️  Error analyzing {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }
        
        println!("🎮 Found {} potential gamepad devices", gamepad_devices.len());
        Ok(())
    }
    
    fn analyze_device(&self, path: &Path) -> Result<Option<EvdevGamepadInfo>, String> {
        let device = Device::open(path)
            .map_err(|e| format!("Failed to open device: {}", e))?;
            
        let name = device.name().unwrap_or("Unknown").to_string();
        let input_id = device.input_id();
        
        // Check if this looks like a gamepad by examining capabilities
        let supported_events = device.supported_events();
        let mut capabilities = Vec::new();
        let mut has_buttons = false;
        let mut has_axes = false;
        
        if let Some(keys) = supported_events.get(&EventType::KEY) {
            // Check for common gamepad buttons
            let gamepad_buttons = [
                Key::BTN_A, Key::BTN_B, Key::BTN_X, Key::BTN_Y,
                Key::BTN_TL, Key::BTN_TR, Key::BTN_SELECT, Key::BTN_START,
                Key::BTN_GAMEPAD, Key::BTN_DPAD_UP, Key::BTN_DPAD_DOWN,
                Key::BTN_DPAD_LEFT, Key::BTN_DPAD_RIGHT,
            ];
            
            for &button in &gamepad_buttons {
                if keys.contains(button) {
                    has_buttons = true;
                    capabilities.push(format!("{:?}", button));
                }
            }
        }
        
        if let Some(abs_axes) = supported_events.get(&EventType::ABSOLUTE) {
            // Check for common gamepad axes
            let gamepad_axes = [
                AbsoluteAxisType::ABS_X, AbsoluteAxisType::ABS_Y,
                AbsoluteAxisType::ABS_RX, AbsoluteAxisType::ABS_RY,
                AbsoluteAxisType::ABS_Z, AbsoluteAxisType::ABS_RZ,
                AbsoluteAxisType::ABS_HAT0X, AbsoluteAxisType::ABS_HAT0Y,
            ];
            
            for &axis in &gamepad_axes {
                if abs_axes.contains(axis) {
                    has_axes = true;
                    capabilities.push(format!("{:?}", axis));
                }
            }
        }
        
        // Consider it a gamepad if it has both buttons and axes, or if the name suggests it's a gamepad
        let is_gamepad = (has_buttons && has_axes) || 
                        name.to_lowercase().contains("gamepad") ||
                        name.to_lowercase().contains("controller") ||
                        name.to_lowercase().contains("xbox") ||
                        name.to_lowercase().contains("steam") ||
                        name.to_lowercase().contains("deck");
        
        if is_gamepad {
            Ok(Some(EvdevGamepadInfo {
                device_path: path.to_string_lossy().to_string(),
                name,
                vendor_id: Some(input_id.vendor()),
                product_id: Some(input_id.product()),
                is_gamepad: true,
                capabilities,
            }))
        } else {
            Ok(None)
        }
    }
    
    pub fn poll_events(&self, app: &AppHandle) -> Result<(), String> {
        let mut devices = self.devices.lock().unwrap();
        
        for (device_path, device) in devices.iter_mut() {
            // Non-blocking read of events
            loop {
                match device.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            let timestamp = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                                
                            let event_type = match event.kind() {
                                InputEventKind::Key(key) => {
                                    println!("🎮 EVDEV Button: {:?} = {}", key, event.value());
                                    "button"
                                }
                                InputEventKind::AbsAxis(axis) => {
                                    println!("🎮 EVDEV Axis: {:?} = {}", axis, event.value());
                                    "axis"
                                }
                                _ => continue,
                            };
                            
                            let controller_event = EvdevControllerEvent {
                                device_path: device_path.clone(),
                                event_type: event_type.to_string(),
                                code: event.code(),
                                value: event.value(),
                                timestamp,
                            };
                            
                            // Emit the event to the frontend
                            app.emit("evdev-gamepad-input", controller_event).ok();
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            println!("⚠️  Error reading from {}: {}", device_path, e);
                        }
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub fn get_detected_devices(&self) -> Vec<EvdevGamepadInfo> {
        self.gamepad_devices.lock().unwrap().clone()
    }
    
    pub fn get_steam_deck_info(&self) -> String {
        let mut info = Vec::new();
        
        // Check for Steam Deck specific indicators
        if Path::new("/home/deck").exists() {
            info.push("✅ Running on Steam Deck (deck user detected)".to_string());
        } else {
            info.push("❓ Not running on Steam Deck (no deck user)".to_string());
        }
        
        // Check for Steam processes
        match std::process::Command::new("pgrep").arg("steam").output() {
            Ok(output) => {
                if output.status.success() && !output.stdout.is_empty() {
                    info.push("🎮 Steam is running".to_string());
                } else {
                    info.push("❌ Steam is not running".to_string());
                }
            }
            Err(_) => {
                info.push("❓ Could not check Steam status".to_string());
            }
        }
        
        // Check for Steam Input environment variables
        for var in ["STEAM_COMPAT_DATA_PATH", "STEAM_COMPAT_CLIENT_INSTALL_PATH", "SteamAppId"] {
            match std::env::var(var) {
                Ok(value) => {
                    info.push(format!("🎮 {}: {}", var, value));
                }
                Err(_) => {
                    info.push(format!("❌ {} not set", var));
                }
            }
        }
        
        info.join("\n")
    }
}