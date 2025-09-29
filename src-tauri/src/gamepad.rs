use gilrs::{Axis, Button, Event, EventType, Gilrs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerState {
    pub buttons: HashMap<String, bool>,
    pub axes: HashMap<String, f32>,
    pub connected: bool,
    pub controller_id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerEvent {
    pub controller_id: usize,
    pub event_type: String,
    pub button: Option<String>,
    pub axis: Option<String>,
    pub value: Option<f32>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    pub gilrs_initialized: bool,
    pub total_gamepads: usize,
    pub connected_gamepads: Vec<GamepadInfo>,
    pub input_devices: Vec<String>,
    pub permissions_check: String,
    pub last_event_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadInfo {
    pub id: usize,
    pub name: String,
    pub is_connected: bool,
    pub power_info: String,
}

pub struct GamepadManager {
    gilrs: Arc<Mutex<Gilrs>>,
    states: Arc<Mutex<HashMap<usize, ControllerState>>>,
    last_event_time: Arc<Mutex<Option<u64>>>,
}

impl GamepadManager {
    pub fn new() -> Result<Self, String> {
        println!("🎮 Initializing GamepadManager...");
        let gilrs = Gilrs::new().map_err(|e| format!("Failed to initialize gamepad: {}", e))?;
        
        // Log all available gamepads at startup
        println!("🔍 Scanning for gamepads at startup...");
        for (id, gamepad) in gilrs.gamepads() {
            println!("🎮 Found gamepad: ID={:?}, Name='{}', Connected={}", 
                     id, gamepad.name(), gamepad.is_connected());
        }
        
        Ok(Self {
            gilrs: Arc::new(Mutex::new(gilrs)),
            states: Arc::new(Mutex::new(HashMap::new())),
            last_event_time: Arc::new(Mutex::new(None)),
        })
    }
    
    pub fn poll_events(&self, app: &AppHandle) {
        let mut gilrs = self.gilrs.lock().unwrap();
        
        while let Some(Event { id, event, time: _, .. }) = gilrs.next_event() {
            let controller_id = id.into();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            
            // Update last event time
            {
                let mut last_time = self.last_event_time.lock().unwrap();
                *last_time = Some(timestamp);
            }
            
            match event {
                EventType::Connected => {
                    let gamepad_name = gilrs.gamepad(id).name().to_string();
                    println!("🔗 Gamepad CONNECTED: ID={:?}, Name='{}', Time={}", 
                             id, gamepad_name, timestamp);
                    
                    let mut states = self.states.lock().unwrap();
                    states.insert(controller_id, ControllerState {
                        buttons: HashMap::new(),
                        axes: HashMap::new(),
                        connected: true,
                        controller_id,
                    });
                    
                    app.emit("gamepad-connected", controller_id).ok();
                }
                EventType::Disconnected => {
                    println!("🔌 Gamepad DISCONNECTED: ID={:?}, Time={}", id, timestamp);
                    let mut states = self.states.lock().unwrap();
                    states.remove(&controller_id);
                    
                    app.emit("gamepad-disconnected", controller_id).ok();
                }
                EventType::ButtonPressed(button, _) => {
                    println!("🔘 Button PRESSED: ID={:?}, Button={:?}, Time={}", 
                             id, button, timestamp);
                    self.update_button_state(controller_id, button, true);
                    let event = ControllerEvent {
                        controller_id,
                        event_type: "button-pressed".to_string(),
                        button: Some(format!("{:?}", button)),
                        axis: None,
                        value: None,
                        timestamp,
                    };
                    app.emit("gamepad-input", event).ok();
                }
                EventType::ButtonReleased(button, _) => {
                    println!("⚪ Button RELEASED: ID={:?}, Button={:?}, Time={}", 
                             id, button, timestamp);
                    self.update_button_state(controller_id, button, false);
                    let event = ControllerEvent {
                        controller_id,
                        event_type: "button-released".to_string(),
                        button: Some(format!("{:?}", button)),
                        axis: None,
                        value: None,
                        timestamp,
                    };
                    app.emit("gamepad-input", event).ok();
                }
                EventType::AxisChanged(axis, value, _) => {
                    // Only log significant axis changes to avoid spam
                    if value.abs() > 0.1 {
                        println!("🎚️ Axis CHANGED: ID={:?}, Axis={:?}, Value={:.3}, Time={}", 
                                 id, axis, value, timestamp);
                    }
                    self.update_axis_state(controller_id, axis, value);
                    let event = ControllerEvent {
                        controller_id,
                        event_type: "axis-changed".to_string(),
                        button: None,
                        axis: Some(format!("{:?}", axis)),
                        value: Some(value),
                        timestamp,
                    };
                    app.emit("gamepad-input", event).ok();
                }
                _ => {
                    println!("❓ Unknown event: ID={:?}, Event={:?}, Time={}", 
                             id, event, timestamp);
                }
            }
        }
    }
    
    pub fn get_controller_states(&self) -> HashMap<usize, ControllerState> {
        self.states.lock().unwrap().clone()
    }
    
    pub fn get_controller_state(&self, id: usize) -> Option<ControllerState> {
        self.states.lock().unwrap().get(&id).cloned()
    }
    
    pub fn get_debug_info(&self) -> DebugInfo {
        let gilrs = self.gilrs.lock().unwrap();
        let last_event_time = *self.last_event_time.lock().unwrap();
        
        let mut connected_gamepads = Vec::new();
        for (id, gamepad) in gilrs.gamepads() {
            connected_gamepads.push(GamepadInfo {
                id: usize::from(id),
                name: gamepad.name().to_string(),
                is_connected: gamepad.is_connected(),
                power_info: format!("{:?}", gamepad.power_info()),
            });
        }
        
        let input_devices = self.enumerate_input_devices();
        let permissions_check = self.check_permissions();
        
        DebugInfo {
            gilrs_initialized: true,
            total_gamepads: gilrs.gamepads().count(),
            connected_gamepads,
            input_devices,
            permissions_check,
            last_event_time,
        }
    }
    
    fn enumerate_input_devices(&self) -> Vec<String> {
        let mut devices = Vec::new();
        
        // Check /dev/input/event* devices
        if let Ok(entries) = std::fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        if name_str.starts_with("event") || name_str.starts_with("js") {
                            devices.push(format!("/dev/input/{}", name_str));
                        }
                    }
                }
            }
        }
        
        devices.sort();
        devices
    }
    
    fn check_permissions(&self) -> String {
        let mut checks = Vec::new();
        
        // Check /dev/uinput permissions
        match std::fs::metadata("/dev/uinput") {
            Ok(metadata) => {
                checks.push(format!("✅ /dev/uinput exists (mode: {:o})", 
                                   metadata.permissions().mode()));
            }
            Err(e) => {
                checks.push(format!("❌ /dev/uinput: {}", e));
            }
        }
        
        // Check /dev/input/event* permissions
        let event_devices: Vec<_> = (0..32)
            .map(|i| format!("/dev/input/event{}", i))
            .filter(|path| std::path::Path::new(path).exists())
            .collect();
            
        if event_devices.is_empty() {
            checks.push("❌ No /dev/input/event* devices found".to_string());
        } else {
            checks.push(format!("✅ Found {} /dev/input/event* devices", event_devices.len()));
            
            // Check permissions on first few devices
            for (_i, device) in event_devices.iter().take(3).enumerate() {
                match std::fs::metadata(device) {
                    Ok(metadata) => {
                        let mode = metadata.permissions().mode();
                        let readable = mode & 0o444 != 0;
                        let writable = mode & 0o222 != 0;
                        checks.push(format!("  {} (mode: {:o}, r:{}, w:{})", 
                                           device, mode, readable, writable));
                    }
                    Err(e) => {
                        checks.push(format!("  {} - Error: {}", device, e));
                    }
                }
            }
        }
        
        checks.join("\n")
    }
    
    fn update_button_state(&self, controller_id: usize, button: Button, pressed: bool) {
        let mut states = self.states.lock().unwrap();
        if let Some(state) = states.get_mut(&controller_id) {
            state.buttons.insert(format!("{:?}", button), pressed);
        }
    }
    
    fn update_axis_state(&self, controller_id: usize, axis: Axis, value: f32) {
        let mut states = self.states.lock().unwrap();
        if let Some(state) = states.get_mut(&controller_id) {
            state.axes.insert(format!("{:?}", axis), value);
        }
    }
}