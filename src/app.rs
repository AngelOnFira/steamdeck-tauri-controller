#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke_without_args(cmd: &str) -> JsValue;
    
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
}

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

pub fn App() -> Element {
    let controllers = use_signal(|| HashMap::<usize, ControllerState>::new());
    let mut server_endpoint = use_signal(|| "http://localhost:8080/light-control".to_string());
    let last_event = use_signal(|| String::new());
    let app_version = use_signal(|| "0.1.0".to_string());

    // Poll for connected controllers on mount
    let mut controllers_clone = controllers.clone();
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            let result = invoke_without_args("get_connected_controllers").await;
            if let Ok(controllers_map) = serde_wasm_bindgen::from_value::<HashMap<usize, ControllerState>>(result) {
                controllers_clone.set(controllers_map);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    // Listen for gamepad events
    let mut last_event_clone = last_event.clone();
    use_effect(move || {
        spawn(async move {
            let handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
                if let Ok(event_data) = serde_wasm_bindgen::from_value::<ControllerEvent>(event) {
                    last_event_clone.set(format!(
                        "Controller {}: {} - {:?}{:?} = {:?}",
                        event_data.controller_id,
                        event_data.event_type,
                        event_data.button.as_deref().unwrap_or(""),
                        event_data.axis.as_deref().unwrap_or(""),
                        event_data.value
                    ));
                }
            });
            
            let _ = listen("gamepad-input", &handler).await;
            handler.forget();
        });
    });

    let send_to_server = {
        let server_endpoint = server_endpoint.clone();
        move |controller_id: usize, action: String| {
            let endpoint_clone = server_endpoint.clone();
            spawn(async move {
                let endpoint = endpoint_clone.read().clone();
                let data = serde_json::json!({
                    "controller_id": controller_id,
                    "action": action,
                    "timestamp": js_sys::Date::now()
                });
                
                let args = serde_wasm_bindgen::to_value(&serde_json::json!({
                    "endpoint": endpoint,
                    "data": data
                })).unwrap();
                
                let _ = invoke("send_to_light_server", args).await;
            });
        }
    };

    let check_for_updates = move |_| {
        spawn(async move {
            // Update check will be implemented with tauri-plugin-updater
            gloo_console::log!("Checking for updates...");
        });
    };

    rsx! {
        link { rel: "stylesheet", href: "styles.css" }
        main {
            class: "container",
            h1 { "Steam Deck Controller Light Show Control" }
            
            div {
                class: "version-info",
                p { "Version: {app_version}" }
                button {
                    onclick: check_for_updates,
                    "Check for Updates"
                }
            }
            
            div {
                class: "server-config",
                h2 { "Server Configuration" }
                input {
                    value: "{server_endpoint}",
                    oninput: move |event| server_endpoint.set(event.value()),
                    placeholder: "http://localhost:8080/light-control"
                }
            }
            
            div {
                class: "controllers-section",
                h2 { "Connected Controllers" }
                
                if controllers.read().is_empty() {
                    p { "No controllers connected. Please connect a controller." }
                } else {
                    {controllers.read().iter().map(|(id, controller)| {
                        let controller_id = *id;
                        let buttons_elements = controller.buttons.iter().map(|(button, pressed)| {
                            let button_name = button.clone();
                            let is_pressed = *pressed;
                            rsx! {
                                button {
                                    key: "{button_name}",
                                    class: if is_pressed { "button pressed" } else { "button" },
                                    onclick: move |_| {
                                        send_to_server(controller_id, format!("button:{}", button_name));
                                    },
                                    "{button_name}: {is_pressed}"
                                }
                            }
                        });
                        
                        let axes_elements = controller.axes.iter().map(|(axis, value)| {
                            let axis_name = axis.clone();
                            let axis_value = *value;
                            rsx! {
                                div {
                                    key: "{axis_name}",
                                    class: "axis-display",
                                    "{axis_name}: {axis_value:.2}"
                                    div {
                                        class: "axis-bar",
                                        div {
                                            class: "axis-value",
                                            style: "width: {(axis_value + 1.0) * 50.0}%"
                                        }
                                    }
                                }
                            }
                        });
                        
                        rsx! {
                            div {
                                key: "{controller_id}",
                                class: "controller-card",
                                h3 { "Controller {controller_id}" }
                                
                                div {
                                    class: "buttons-grid",
                                    h4 { "Buttons" }
                                    {buttons_elements}
                                }
                                
                                div {
                                    class: "axes-grid",
                                    h4 { "Axes" }
                                    {axes_elements}
                                }
                            }
                        }
                    })}
                }
            }
            
            div {
                class: "last-event",
                h3 { "Last Event" }
                p { "{last_event}" }
            }
        }
    }
}