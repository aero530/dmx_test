//! Tests for the Enttec DMX USB Pro message framing
//! (`nucleo/src/enttec_protocol.rs`).

use common::enttec_protocol::{
    EnttecParser, END_DELIMITER, LABEL_GET_SERIAL, LABEL_OUTPUT_DMX, MAX_PAYLOAD, START_DELIMITER,
};

/// Build one framed message the way a host would send it.
fn frame(label: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![
        START_DELIMITER,
        label,
        (payload.len() & 0xFF) as u8,
        ((payload.len() >> 8) & 0xFF) as u8,
    ];
    out.extend_from_slice(payload);
    out.push(END_DELIMITER);
    out
}

/// Feed bytes and collect (label, payload) for each completed message.
fn feed_all(parser: &mut EnttecParser, bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut messages = Vec::new();
    for &b in bytes {
        if parser.feed(b) {
            messages.push((parser.label(), parser.payload().to_vec()));
        }
    }
    messages
}

#[test]
fn parses_a_simple_message() {
    let mut parser = EnttecParser::new();
    let messages = feed_all(&mut parser, &frame(LABEL_OUTPUT_DMX, &[0x00, 1, 2, 3]));
    assert_eq!(messages, vec![(LABEL_OUTPUT_DMX, vec![0x00, 1, 2, 3])]);
}

#[test]
fn parses_a_zero_length_message() {
    let mut parser = EnttecParser::new();
    let messages = feed_all(&mut parser, &frame(LABEL_GET_SERIAL, &[]));
    assert_eq!(messages, vec![(LABEL_GET_SERIAL, vec![])]);
}

#[test]
fn parses_back_to_back_messages() {
    let mut parser = EnttecParser::new();
    let mut stream = frame(LABEL_OUTPUT_DMX, &[0; 513]);
    stream.extend_from_slice(&frame(LABEL_GET_SERIAL, &[]));

    let messages = feed_all(&mut parser, &stream);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].0, LABEL_OUTPUT_DMX);
    assert_eq!(messages[0].1.len(), 513);
    assert_eq!(messages[1].0, LABEL_GET_SERIAL);
}

#[test]
fn ignores_garbage_before_the_start_delimiter() {
    let mut parser = EnttecParser::new();
    let mut stream = vec![0x00, 0x42, END_DELIMITER];
    stream.extend_from_slice(&frame(LABEL_GET_SERIAL, &[9]));

    let messages = feed_all(&mut parser, &stream);
    assert_eq!(messages, vec![(LABEL_GET_SERIAL, vec![9])]);
}

#[test]
fn drops_oversized_messages_and_stays_in_sync() {
    let mut parser = EnttecParser::new();

    // Declared length exceeds MAX_PAYLOAD: must be consumed, not returned
    let oversized = frame(LABEL_OUTPUT_DMX, &vec![0xAA; MAX_PAYLOAD + 100]);
    let mut stream = oversized;
    stream.extend_from_slice(&frame(LABEL_GET_SERIAL, &[7]));

    let messages = feed_all(&mut parser, &stream);
    assert_eq!(messages, vec![(LABEL_GET_SERIAL, vec![7])]);
}

#[test]
fn recovers_from_a_missing_end_delimiter() {
    let mut parser = EnttecParser::new();

    let mut corrupted = frame(LABEL_OUTPUT_DMX, &[1, 2, 3]);
    *corrupted.last_mut().unwrap() = 0x00; // clobber the end delimiter
    let mut stream = corrupted;
    stream.extend_from_slice(&frame(LABEL_GET_SERIAL, &[8]));

    let messages = feed_all(&mut parser, &stream);
    assert_eq!(messages, vec![(LABEL_GET_SERIAL, vec![8])]);
}

#[test]
fn accepts_the_largest_valid_payload() {
    let mut parser = EnttecParser::new();
    let payload = vec![0x55; MAX_PAYLOAD];
    let messages = feed_all(&mut parser, &frame(LABEL_OUTPUT_DMX, &payload));
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].1, payload);
}
