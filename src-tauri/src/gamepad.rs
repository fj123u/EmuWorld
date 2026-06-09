use gilrs::{Gilrs, Button, Axis, Event, EventType};
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Clone)]
pub struct GamepadState {
    pub connected: bool,
    pub name: String,
    pub buttons: Vec<bool>,
    pub axes: Vec<f32>,
}

impl Default for GamepadState {
    fn default() -> Self {
        Self {
            connected: false,
            name: String::new(),
            buttons: vec![false; 17],
            axes: vec![0.0; 4],
        }
    }
}

pub fn start_gamepad_thread(app_handle: AppHandle) {
    thread::spawn(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("Failed to init gilrs: {}", e);
                return;
            }
        };

        let mut state = GamepadState::default();
        let mut last_emitted = String::new();

        loop {
            // Process events
            while let Some(Event { id, event, .. }) = gilrs.next_event() {
                match event {
                    EventType::Connected => {
                        if let Some(gp) = gilrs.connected_gamepad(id) {
                            state.connected = true;
                            state.name = gp.name().to_string();
                        }
                    }
                    EventType::Disconnected => {
                        state.connected = false;
                        state.name.clear();
                        state.buttons = vec![false; 17];
                        state.axes = vec![0.0; 4];
                    }
                    _ => {}
                }
            }

            // Read current state from first connected gamepad
            if let Some((_id, gamepad)) = gilrs.gamepads().find(|(_, gp)| gp.is_connected()) {
                state.connected = true;
                if state.name.is_empty() {
                    state.name = gamepad.name().to_string();
                }

                // Map buttons to standard layout (same indices as Web Gamepad API)
                state.buttons = vec![
                    gamepad.is_pressed(Button::South),      // 0 = A
                    gamepad.is_pressed(Button::East),       // 1 = B
                    gamepad.is_pressed(Button::West),       // 2 = X
                    gamepad.is_pressed(Button::North),      // 3 = Y
                    gamepad.is_pressed(Button::LeftTrigger), // 4 = LB
                    gamepad.is_pressed(Button::RightTrigger), // 5 = RB
                    gamepad.is_pressed(Button::LeftTrigger2), // 6 = LT
                    gamepad.is_pressed(Button::RightTrigger2), // 7 = RT
                    gamepad.is_pressed(Button::Select),     // 8 = Select/Back
                    gamepad.is_pressed(Button::Start),      // 9 = Start
                    gamepad.is_pressed(Button::LeftThumb),  // 10 = L3
                    gamepad.is_pressed(Button::RightThumb), // 11 = R3
                    gamepad.is_pressed(Button::DPadUp),     // 12
                    gamepad.is_pressed(Button::DPadDown),   // 13
                    gamepad.is_pressed(Button::DPadLeft),   // 14
                    gamepad.is_pressed(Button::DPadRight),  // 15
                    gamepad.is_pressed(Button::Mode),       // 16 = Home
                ];

                // Axes: left stick X/Y, right stick X/Y
                // Apply a small hardware deadzone to filter noise from virtual controllers
                // (e.g. WiiUPro via WiinUPro, DS4Windows, etc.)
                let hw_deadzone = 0.08;
                let filter = |v: f32| if v.abs() < hw_deadzone { 0.0 } else { v };
                state.axes = vec![
                    filter(gamepad.value(Axis::LeftStickX)),
                    filter(-gamepad.value(Axis::LeftStickY)),
                    filter(gamepad.value(Axis::RightStickX)),
                    filter(-gamepad.value(Axis::RightStickY)),
                ];
            } else {
                if state.connected {
                    state.connected = false;
                    state.name.clear();
                    state.buttons = vec![false; 17];
                    state.axes = vec![0.0; 4];
                }
            }

            // Emit state to frontend (throttled — only if changed)
            let serialized = serde_json::to_string(&state).unwrap_or_default();
            if serialized != last_emitted {
                last_emitted = serialized;
                let _ = app_handle.emit("gamepad-state", &state);
            }

            thread::sleep(Duration::from_millis(16)); // ~60fps
        }
    });
}
