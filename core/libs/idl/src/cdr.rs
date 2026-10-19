//! CDR as ROS 2 writes it: a little-endian encapsulation header, then plain CDR.

use byteorder::LittleEndian;
use serde::{Serialize, de::DeserializeOwned};

use crate::Message;

/// The CDR_LE encapsulation header. Only its first two bytes name the encoding, the other two are options.
const HEADER: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

#[derive(Debug, thiserror::Error)]
pub enum CdrError {
    #[error("payload does not start with a little-endian CDR header")]
    Encapsulation,
    #[error(transparent)]
    Codec(#[from] cdr_encoding::Error),
}

pub fn encode<M: Serialize>(message: &M) -> Result<Vec<u8>, CdrError> {
    let mut bytes = HEADER.to_vec();
    cdr_encoding::to_writer::<M, LittleEndian, _>(&mut bytes, message)?;
    Ok(bytes)
}

pub fn decode<M: DeserializeOwned>(bytes: &[u8]) -> Result<M, CdrError> {
    match bytes.split_at_checked(HEADER.len()) {
        Some((header, body)) if header[..2] == HEADER[..2] => Ok(cdr_encoding::from_bytes::<M, LittleEndian>(body)?.0),
        _ => Err(CdrError::Encapsulation),
    }
}

/// The Zenoh encoding of a message: `application/cdr;<pkg>/msg/<Name>`.
pub fn encoding<M: Message>() -> String {
    format!("application/cdr;{}", M::SCHEMA_NAME)
}
