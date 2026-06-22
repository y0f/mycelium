use std::net::UdpSocket;
use std::thread;

use crossbeam::channel::Sender;
use rosc::{OscMessage, OscPacket, OscType};
use tracing::{error, info, trace, warn};

use core::event::{BusEvent, ControlEvent, InputSource};

/// receives OSC messages on a UDP port and maps them to ControlEvents.
pub struct OscHandler {
    _handle: Option<thread::JoinHandle<()>>,
}

impl OscHandler {
    pub fn start(port: u16, bus_sender: Sender<BusEvent>) -> Self {
        let handle = thread::Builder::new()
            .name("mycelium-osc".into())
            .spawn(move || {
                let addr = format!("0.0.0.0:{}", port);
                let socket = match UdpSocket::bind(&addr) {
                    Ok(s) => {
                        info!("OSC listening on {}", addr);
                        s
                    }
                    Err(e) => {
                        warn!("Failed to bind OSC port {}: {e}", port);
                        return;
                    }
                };

                let mut buf = [0u8; 4096];
                loop {
                    match socket.recv_from(&mut buf) {
                        Ok((size, _src)) => {
                            if let Ok(packet) = rosc::decoder::decode_udp(&buf[..size]) {
                                Self::handle_packet(&packet.1, &bus_sender);
                            }
                        }
                        Err(e) => {
                            error!("OSC recv error: {e}");
                        }
                    }
                }
            })
            .ok();

        Self { _handle: handle }
    }

    fn handle_packet(packet: &OscPacket, sender: &Sender<BusEvent>) {
        match packet {
            OscPacket::Message(msg) => Self::handle_message(msg, sender),
            OscPacket::Bundle(bundle) => {
                for p in &bundle.content {
                    Self::handle_packet(p, sender);
                }
            }
        }
    }

    fn handle_message(msg: &OscMessage, sender: &Sender<BusEvent>) {
        // address patterns: /param/name value, or /cc/N value (like MIDI CC)
        let parts: Vec<&str> = msg.addr.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() || msg.args.is_empty() {
            return;
        }

        let value = match &msg.args[0] {
            OscType::Float(f) => *f,
            OscType::Int(i) => *i as f32 / 127.0,
            OscType::Double(d) => *d as f32,
            _ => return,
        };

        match parts[0] {
            "cc" | "control" => {
                let channel = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                if let Err(e) = sender.try_send(BusEvent::Control(ControlEvent::Continuous {
                    source: InputSource::Osc,
                    channel,
                    value,
                })) {
                    trace!("OSC event dropped (bus full): {e}");
                }
            }
            "trigger" | "note" => {
                let channel = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                if let Err(e) = sender.try_send(BusEvent::Control(ControlEvent::Trigger {
                    source: InputSource::Osc,
                    channel,
                })) {
                    trace!("OSC event dropped (bus full): {e}");
                }
            }
            _ => {
                // hash the address to a channel number
                let channel = (msg.addr.bytes().fold(0u32, |a, b| a.wrapping_add(b as u32)) % 128) as u8;
                if let Err(e) = sender.try_send(BusEvent::Control(ControlEvent::Continuous {
                    source: InputSource::Osc,
                    channel,
                    value,
                })) {
                    trace!("OSC event dropped (bus full): {e}");
                }
            }
        }
    }
}
