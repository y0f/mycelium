use std::thread;

use crossbeam::channel::Sender;
use gilrs::{Event, EventType, Gilrs};
use tracing::{info, trace, warn};

use core::event::{BusEvent, ControlEvent, InputSource};

/// gamepad input handler via gilrs.
pub struct GamepadHandler {
    _handle: Option<thread::JoinHandle<()>>,
}

impl GamepadHandler {
    /// polls gamepads on a background thread.
    pub fn start(bus_sender: Sender<BusEvent>) -> Self {
        let handle = thread::Builder::new()
            .name("mycelium-gamepad".into())
            .spawn(move || {
                let mut gilrs = match Gilrs::new() {
                    Ok(g) => g,
                    Err(e) => {
                        warn!("Gamepad init failed: {e}");
                        return;
                    }
                };

                for (_id, gamepad) in gilrs.gamepads() {
                    info!("Gamepad found: {}", gamepad.name());
                }

                loop {
                    while let Some(Event { event, .. }) = gilrs.next_event() {
                        match event {
                            EventType::AxisChanged(axis, value, _) => {
                                let channel = axis as u8;
                                if let Err(e) = bus_sender.try_send(BusEvent::Control(
                                    ControlEvent::Continuous {
                                        source: InputSource::Gamepad,
                                        channel,
                                        value: (value + 1.0) * 0.5, // -1..1 -> 0..1
                                    },
                                )) {
                                    trace!("Gamepad event dropped (bus full): {e}");
                                }
                            }
                            EventType::ButtonPressed(button, _) => {
                                let channel = button as u8;
                                if let Err(e) = bus_sender.try_send(BusEvent::Control(
                                    ControlEvent::Trigger {
                                        source: InputSource::Gamepad,
                                        channel,
                                    },
                                )) {
                                    trace!("Gamepad event dropped (bus full): {e}");
                                }
                            }
                            _ => {}
                        }
                    }
                    thread::sleep(std::time::Duration::from_millis(4));
                }
            })
            .ok();

        Self { _handle: handle }
    }
}
