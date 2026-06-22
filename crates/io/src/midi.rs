use std::sync::Arc;

use crossbeam::channel::Sender;
use midir::{MidiInput, MidiInputConnection};
use parking_lot::Mutex;
use tracing::{info, trace, warn};

use core::event::{BusEvent, ControlEvent, InputSource};

/// maps a MIDI CC number to a bus control channel.
#[derive(Debug, Clone)]
pub struct MidiMapping {
    pub cc: u8,
    pub channel: u8,
}

/// listens for MIDI messages and sends ControlEvents to the bus.
pub struct MidiHandler {
    _connection: Option<MidiInputConnection<()>>,
    /// in learn mode, the next CC received is stored here.
    pub last_cc: Arc<Mutex<Option<(u8, u8)>>>,
}

impl MidiHandler {
    /// listens on the first available MIDI input port.
    pub fn start(bus_sender: Sender<BusEvent>) -> Self {
        let last_cc: Arc<Mutex<Option<(u8, u8)>>> = Arc::new(Mutex::new(None));
        let last_cc_clone = Arc::clone(&last_cc);

        let connection = match Self::open_first_port(bus_sender, last_cc_clone) {
            Ok(conn) => Some(conn),
            Err(e) => {
                warn!("No MIDI input available: {e}");
                None
            }
        };

        Self {
            _connection: connection,
            last_cc,
        }
    }

    fn open_first_port(
        bus_sender: Sender<BusEvent>,
        last_cc: Arc<Mutex<Option<(u8, u8)>>>,
    ) -> Result<MidiInputConnection<()>, String> {
        let midi_in = MidiInput::new("mycelium-midi")
            .map_err(|e| format!("Failed to create MIDI input: {e}"))?;

        let ports = midi_in.ports();
        if ports.is_empty() {
            return Err("No MIDI input ports found".into());
        }

        let port = &ports[0];
        let port_name = midi_in
            .port_name(port)
            .unwrap_or_else(|_| "unknown".into());
        info!("Opening MIDI port: {}", port_name);

        let connection = midi_in
            .connect(
                port,
                "mycelium-midi-in",
                move |_timestamp, message, _| {
                    if message.len() < 3 {
                        return;
                    }

                    let status = message[0] & 0xF0;
                    let channel = message[0] & 0x0F;

                    match status {
                        // control change
                        0xB0 => {
                            let cc = message[1];
                            let value = message[2] as f32 / 127.0;

                            // store for learn mode
                            *last_cc.lock() = Some((channel, cc));

                            if let Err(e) = bus_sender.try_send(BusEvent::Control(
                                ControlEvent::Continuous {
                                    source: InputSource::Midi,
                                    channel: cc,
                                    value,
                                },
                            )) {
                                trace!("MIDI event dropped (bus full): {e}");
                            }
                        }
                        // note on
                        0x90 => {
                            let _note = message[1];
                            let velocity = message[2];
                            if velocity > 0 {
                                if let Err(e) = bus_sender.try_send(BusEvent::Control(
                                    ControlEvent::Trigger {
                                        source: InputSource::Midi,
                                        channel: message[1],
                                    },
                                )) {
                                    trace!("MIDI event dropped (bus full): {e}");
                                }
                            }
                        }
                        _ => {}
                    }
                },
                (),
            )
            .map_err(|e| format!("Failed to connect MIDI: {e}"))?;

        Ok(connection)
    }
}
