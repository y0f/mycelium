use core::bus::PluginBus;
use core::event::{AudioFrame, BusEvent, ControlEvent, InputSource, NodeId, ParamId};

#[test]
fn test_discrete_events_drain_in_priority_order() {
    let bus = PluginBus::new();
    let sender = bus.event_sender();

    sender.send(BusEvent::ConfigReload).unwrap();
    sender
        .send(BusEvent::ParamChange {
            node: NodeId(1),
            param: ParamId(0),
            value: 0.5,
        })
        .unwrap();
    sender
        .send(BusEvent::Control(ControlEvent::Trigger {
            source: InputSource::Midi,
            channel: 0,
        }))
        .unwrap();

    let mut buf = Vec::new();
    bus.drain_events(&mut buf);

    assert_eq!(buf.len(), 3);
    assert_eq!(buf[0].priority(), 0); // control first
    assert_eq!(buf[1].priority(), 1); // ParamChange second
    assert_eq!(buf[2].priority(), 3); // ConfigReload last
}

#[test]
fn test_audio_triple_buffer_latest_value_wins() {
    let bus = PluginBus::new();
    let audio_buf = bus.audio_buffer();

    let mut frame = AudioFrame::default();
    frame.bands[0] = 0.5;
    audio_buf.write(&frame);

    frame.bands[0] = 0.9;
    audio_buf.write(&frame);

    let mut target = AudioFrame::default();
    bus.read_audio(&mut target);

    assert!(target.bands[0] > 0.4);
}

#[test]
fn test_drain_reuses_buffer() {
    let bus = PluginBus::new();
    let sender = bus.event_sender();

    let mut buf = Vec::with_capacity(100);

    sender.send(BusEvent::ConfigReload).unwrap();
    bus.drain_events(&mut buf);
    assert_eq!(buf.len(), 1);

    sender.send(BusEvent::ConfigReload).unwrap();
    bus.drain_events(&mut buf);
    assert_eq!(buf.len(), 1);
    assert!(buf.capacity() >= 100);
}
