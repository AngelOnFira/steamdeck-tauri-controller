use evdev::{Device, EventType};
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
        let mut capabilities = Vec::new();
        let mut has_buttons = false;
        let mut has_axes = false;
        
        // Simple capability detection based on device name and path
        if name.to_lowercase().contains("gamepad") ||
           name.to_lowercase().contains("controller") ||
           name.to_lowercase().contains("xbox") ||
           name.to_lowercase().contains("steam") ||
           name.to_lowercase().contains("deck") ||
           name.to_lowercase().contains("joy") {
            has_buttons = true;
            has_axes = true;
            capabilities.push("INFERRED_GAMEPAD".to_string());
        } else {
            capabilities.push("UNKNOWN_DEVICE".to_string());
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
    
    pub fn poll_events(&self, _app: &AppHandle) -> Result<(), String> {
        // Simplified event polling - just indicate that evdev is available
        // In a real implementation, this would use epoll or similar for non-blocking reads
        // For now, we'll just provide device enumeration
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