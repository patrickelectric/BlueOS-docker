//! Payloads from the network are untrusted: a hostile one must fail fast, never panic or allocate from a length it
//! claims.

use blueos_idl::{
    blueos_msgs::{CommandAck, JobList, ServiceInfo},
    cdr::{self, CdrError},
};

const HEADER: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

fn payload(body: &[u8]) -> Vec<u8> {
    [HEADER.as_slice(), body].concat()
}

#[test]
fn a_huge_sequence_count_without_elements_fails() {
    let error = cdr::decode::<JobList>(&payload(&u32::MAX.to_le_bytes())).unwrap_err();
    assert!(matches!(error, CdrError::Codec(_)), "{error}");
}

#[test]
fn a_string_longer_than_the_payload_fails() {
    let mut body = vec![1, 0, 0, 0];
    body.extend(u32::MAX.to_le_bytes());
    assert!(cdr::decode::<CommandAck>(&payload(&body)).is_err());
}

#[test]
fn a_payload_without_the_cdr_header_fails() {
    let error = cdr::decode::<CommandAck>(&[0x00, 0x00, 0x00, 0x00, 0x01]).unwrap_err();
    assert!(matches!(error, CdrError::Encapsulation));
    assert!(matches!(
        cdr::decode::<CommandAck>(&[0x00]),
        Err(CdrError::Encapsulation)
    ));
}

#[test]
fn a_string_without_its_terminator_loses_its_last_byte() {
    // cdr-encoding is lenient here, as other CDR readers are: it warns and drops the byte it expected to be NUL.
    let mut body = vec![1, 0, 0, 0];
    body.extend(3u32.to_le_bytes());
    body.extend(b"abc");
    body.extend([0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let ack: CommandAck = cdr::decode(&payload(&body)).unwrap();
    assert_eq!(ack.job_id, "ab");
}

#[test]
fn every_truncation_of_a_valid_payload_fails_without_panicking() {
    let info = ServiceInfo {
        name: "example".into(),
        version: "1.0.0".into(),
        build: "dev".into(),
        capabilities: vec!["pump".into()],
        endpoints: vec![Default::default()],
    };
    let bytes = cdr::encode(&info).unwrap();
    for length in 0..bytes.len() {
        assert!(
            cdr::decode::<ServiceInfo>(&bytes[..length]).is_err(),
            "{length} bytes decoded"
        );
    }
    assert_eq!(cdr::decode::<ServiceInfo>(&bytes).unwrap(), info);
}
