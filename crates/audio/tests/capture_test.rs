use audio::capture::RingBuffer;

#[test]
fn test_ring_buffer_push_and_read() {
    let mut rb = RingBuffer::new(8);
    rb.push(&[1.0, 2.0, 3.0, 4.0]);
    let mut out = [0.0f32; 4];
    rb.read_last_into(&mut out);
    assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_ring_buffer_wraps() {
    let mut rb = RingBuffer::new(4);
    rb.push(&[1.0, 2.0, 3.0, 4.0]);
    rb.push(&[5.0, 6.0]);
    let mut out = [0.0f32; 4];
    rb.read_last_into(&mut out);
    assert_eq!(out, [3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_ring_buffer_read_less_than_capacity() {
    let mut rb = RingBuffer::new(8);
    rb.push(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let mut out = [0.0f32; 3];
    rb.read_last_into(&mut out);
    assert_eq!(out, [3.0, 4.0, 5.0]);
}
