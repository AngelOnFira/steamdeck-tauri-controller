use gilrs::{Axis, Button, Event, EventType, Gilrs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
}

pub struct GamepadManager {
    gilrs: Arc<Mutex<Gilrs>>,
    states: Arc<Mutex<HashMap<usize, ControllerState>>>,
}

impl GamepadManager {
    pub fn new() -> Result<Self, String> {
        let gilrs = Gilrs::new().map_err(|e| format!("Failed to initialize gamepad: {}", e))?;
        
        Ok(Self {
            gilrs: Arc::new(Mutex::new(gilrs)),
            states: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    pub fn poll_events(&self, app: &AppHandle) {
        let mut gilrs = self.gilrs.lock().unwrap();
        
        while let Some(Event { id, event, .. }) = gilrs.next_event() {
            let controller_id = id.into();
            
            match event {
                EventType::Connected => {
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
                    let mut states = self.states.lock().unwrap();
                    states.remove(&controller_id);
                    
                    app.emit("gamepad-disconnected", controller_id).ok();
                }
                EventType::ButtonPressed(button, _) => {
                    self.update_button_state(controller_id, button, true);
                    let event = ControllerEvent {
                        controller_id,
                        event_type: "button-pressed".to_string(),
                        button: Some(format!("{:?}", button)),
                        axis: None,
                        value: None,
                    };
                    app.emit("gamepad-input", event).ok();
                }
                EventType::ButtonReleased(button, _) => {
                    self.update_button_state(controller_id, button, false);
                    let event = ControllerEvent {
                        controller_id,
                        event_type: "button-released".to_string(),
                        button: Some(format!("{:?}", button)),
                        axis: None,
                        value: None,
                    };
                    app.emit("gamepad-input", event).ok();
                }
                EventType::AxisChanged(axis, value, _) => {
                    self.update_axis_state(controller_id, axis, value);
                    let event = ControllerEvent {
                        controller_id,
                        event_type: "axis-changed".to_string(),
                        button: None,
                        axis: Some(format!("{:?}", axis)),
                        value: Some(value),
                    };
                    app.emit("gamepad-input", event).ok();
                }
                _ => {}
            }
        }
    }
    
    pub fn get_controller_states(&self) -> HashMap<usize, ControllerState> {
        self.states.lock().unwrap().clone()
    }
    
    pub fn get_controller_state(&self, id: usize) -> Option<ControllerState> {
        self.states.lock().unwrap().get(&id).cloned()
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