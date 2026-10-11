//! Roland System Exclusive protocol primitives for the MC-101 / MC-707 / MV-1.
//!
//! The MC family uses the same RQ1 (data request, `0x11`) and DT1 (data set,
//! `0x12`) message format documented for many other Roland instruments. Only
//! the model ID byte (index 6 of the message) is device-specific — `0x5D` for
//! the MC-707, `0x5E` for the MC-101, `0x5F` for the MV-1.
//!
//! Address and length fields are 32 bits laid out big-endian across four
//! 7-bit MIDI bytes. The checksum is the standard Roland complement-to-128
//! over the address + payload bytes.

use crate::error::{Error, Result};

pub const SYSEX_START: u8 = 0xf0;
pub const SYSEX_END: u8 = 0xf7;
pub const ROLAND_ID: u8 = 0x41;
pub const DEVICE_ID: u8 = 0x10; // default Roland device ID

pub const CMD_RQ1: u8 = 0x11; // data request 1
pub const CMD_DT1: u8 = 0x12; // data set 1
const MAX_7BIT_FIELD: u32 = 0x0fff_ffff;

/// Known model identification bytes for the MC / MV family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ModelId {
    Mc707 = 0x5d,
    Mc101 = 0x5e,
    Mv1 = 0x5f,
}

impl ModelId {
    /// The raw model identification byte used in Sys-Ex messages.
    pub const fn value(self) -> u8 {
        self as u8
    }
}

impl From<ModelId> for u8 {
    /// Convert a known model ID to its wire byte.
    fn from(id: ModelId) -> u8 {
        id as u8
    }
}

/// Human-readable model name for a raw model ID byte, e.g. `"MC-707"`.
/// Unknown bytes are rendered as `"0x5F"`-style hex.
pub fn model_name(model_id: u8) -> String {
    match model_id {
        0x5d => "MC-707".to_string(),
        0x5e => "MC-101".to_string(),
        0x5f => "MV-1".to_string(),
        other => format!("0x{other:02X}"),
    }
}

// Shared checksum implementation over two borrowed slices.
fn roland_checksum_parts(first: &[u8], second: &[u8]) -> u8 {
    let sum = first
        .iter()
        .chain(second)
        .fold(0u16, |sum, byte| (sum + u16::from(*byte & 0x7f)) & 0x7f);
    ((128 - u32::from(sum)) & 0x7f) as u8
}

/// Standard Roland complement-to-128 checksum over the supplied bytes.
///
/// For RQ1/DT1 frames this is applied to the four address bytes followed by
/// the length or data bytes, excluding the checksum byte itself.
pub fn roland_checksum(payload: &[u8]) -> u8 {
    roland_checksum_parts(&[], payload)
}

/// Reject packed addresses with a high bit set in any byte.
fn validate_address(address: u32) -> Result<()> {
    if address & 0x8080_8080 != 0 {
        return Err(Error::Value(format!(
            "SysEx address 0x{address:08X} contains a byte with its high bit set"
        )));
    }
    Ok(())
}

/// Reject model IDs that cannot be represented by a 7-bit SysEx byte.
fn validate_model_id(model_id: u8) -> Result<()> {
    if model_id > 0x7f {
        return Err(Error::Value(format!(
            "model ID 0x{model_id:02X} is not 7-bit"
        )));
    }
    Ok(())
}

/// Reject the first payload byte with its high bit set, reporting context and offset.
pub(crate) fn validate_7bit_data(data: &[u8], context: &str) -> Result<()> {
    if let Some((offset, byte)) = data
        .iter()
        .copied()
        .enumerate()
        .find(|(_, byte)| byte & 0x80 != 0)
    {
        return Err(Error::Value(format!(
            "{context} byte at offset {offset} is 0x{byte:02X}; SysEx data must be 7-bit"
        )));
    }
    Ok(())
}

/// Split a packed Roland address into four bytes, masking each to seven bits.
fn address_bytes(address: u32) -> [u8; 4] {
    [
        ((address >> 24) & 0x7f) as u8,
        ((address >> 16) & 0x7f) as u8,
        ((address >> 8) & 0x7f) as u8,
        (address & 0x7f) as u8,
    ]
}

/// Encode a request length as four 7-bit bytes (big-endian).
///
/// The reference scripts only use the low byte, but we encode all four so
/// callers can request larger blocks if the device permits it.
fn length_bytes(count: u32) -> [u8; 4] {
    [
        ((count >> 21) & 0x7f) as u8,
        ((count >> 14) & 0x7f) as u8,
        ((count >> 7) & 0x7f) as u8,
        (count & 0x7f) as u8,
    ]
}

/// Build a complete RQ1 (data request 1) Sys-Ex message.
///
/// # Errors
/// Returns an error when the address, request length, or model ID cannot be
/// represented by 7-bit SysEx bytes.
pub fn build_rq1(model_id: u8, address: u32, count: u32) -> Result<Vec<u8>> {
    validate_address(address)?;
    validate_model_id(model_id)?;
    if count > MAX_7BIT_FIELD {
        return Err(Error::Value(format!(
            "RQ1 request length {count} exceeds the 28-bit SysEx limit"
        )));
    }

    let header: [u8; 8] = [
        SYSEX_START,
        ROLAND_ID,
        DEVICE_ID,
        0x00,
        0x00,
        0x00,
        model_id,
        CMD_RQ1,
    ];
    let addr = address_bytes(address);
    let len = length_bytes(count);
    let checksum = roland_checksum_parts(&addr, &len);
    let mut out = Vec::with_capacity(18);
    out.extend_from_slice(&header);
    out.extend_from_slice(&addr);
    out.extend_from_slice(&len);
    out.push(checksum);
    out.push(SYSEX_END);
    Ok(out)
}

/// Build a complete DT1 (data set 1) Sys-Ex message.
///
/// # Errors
/// Returns an error when the address, model ID, or payload contains a byte
/// that cannot be represented by 7-bit SysEx data.
pub fn build_dt1(model_id: u8, address: u32, data: &[u8]) -> Result<Vec<u8>> {
    validate_address(address)?;
    validate_model_id(model_id)?;
    validate_7bit_data(data, "DT1 payload")?;

    let header: [u8; 8] = [
        SYSEX_START,
        ROLAND_ID,
        DEVICE_ID,
        0x00,
        0x00,
        0x00,
        model_id,
        CMD_DT1,
    ];
    let addr = address_bytes(address);
    let checksum = roland_checksum_parts(&addr, data);
    let mut out = Vec::with_capacity(14 + data.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&addr);
    out.extend_from_slice(data);
    out.push(checksum);
    out.push(SYSEX_END);
    Ok(out)
}

/// A parsed DT1 reply: the address it was sent to and the payload bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DT1Message {
    pub address: u32,
    pub data: Vec<u8>,
}

/// Validate a complete DT1 frame and borrow its model ID, address, and data.
pub(crate) fn parse_dt1_view(message: &[u8]) -> Result<(u8, u32, &[u8])> {
    if message.len() < 14 {
        return Err(Error::MessageTooShort(message.len()));
    }
    if message[0] != SYSEX_START || message[message.len() - 1] != SYSEX_END {
        return Err(Error::MissingFraming);
    }
    if message[1] != ROLAND_ID {
        return Err(Error::NotRoland(message[1]));
    }
    if message[7] != CMD_DT1 {
        return Err(Error::WrongCommand(message[7]));
    }

    let checksum_index = message.len() - 2;
    validate_7bit_data(&message[1..message.len() - 1], "DT1 response")?;
    let expected_checksum = roland_checksum(&message[8..checksum_index]);
    if message[checksum_index] != expected_checksum {
        return Err(Error::Value(format!(
            "DT1 checksum mismatch: got 0x{:02X}, expected 0x{expected_checksum:02X}",
            message[checksum_index]
        )));
    }

    let address =
        ((message[8] as u32) << 24) | ((message[9] as u32) << 16) | ((message[10] as u32) << 8) | (message[11] as u32);
    Ok((message[6], address, &message[12..checksum_index]))
}

/// Extract the address and payload from a complete DT1 SysEx frame.
pub fn parse_dt1_payload(message: &[u8]) -> Result<DT1Message> {
    let (_, address, data) = parse_dt1_view(message)?;
    Ok(DT1Message {
        address,
        data: data.to_vec(),
    })
}

/// The universal device-inquiry message used to detect the model ID.
pub fn universal_device_inquiry() -> [u8; 6] {
    [0xf0, 0x7e, 0x7f, 0x06, 0x01, 0xf7]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify checksum wraparound for an empty payload and a single byte.
    #[test]
    fn checksum_is_complement_to_128() {
        // Sum of all bytes = 0 → 128 - 0 = 128 → & 0x7F = 0.
        assert_eq!(roland_checksum(&[]), 0);
        assert_eq!(roland_checksum(&[0x40]), 0x40);
    }

    /// Verify the RQ1 frame contains the expected manufacturer, model, address, and terminator.
    #[test]
    fn rq1_layout() {
        let msg = build_rq1(0x5d, 0x30000000, 0x100).unwrap();
        assert_eq!(msg[0], 0xf0);
        assert_eq!(msg[1], 0x41);
        assert_eq!(msg[6], 0x5d);
        assert_eq!(msg[7], CMD_RQ1);
        assert_eq!(msg[8], 0x30); // address high byte
        assert_eq!(msg[15], 0x00); // count low byte (0x100 → low byte 0)
        assert_eq!(*msg.last().unwrap(), 0xf7);
    }

    /// Verify building and parsing a DT1 preserves its address and payload.
    #[test]
    fn parse_dt1_round_trip() {
        let data = [0x40, 0x00, 0x01];
        let msg = build_dt1(0x5e, 0x20000000, &data).unwrap();
        let parsed = parse_dt1_payload(&msg).unwrap();
        assert_eq!(parsed.address, 0x20000000);
        assert_eq!(parsed.data, data);
    }

    /// Verify truncated messages and non-Roland manufacturer IDs return specific errors.
    #[test]
    fn parse_rejects_bad_framing() {
        assert!(matches!(
            parse_dt1_payload(&[0xf0, 0x41, 0xf7]),
            Err(Error::MessageTooShort(_))
        ));
        let mut msg = build_dt1(0x5d, 0, &[0]).unwrap();
        msg[1] = 0x42;
        assert!(matches!(
            parse_dt1_payload(&msg),
            Err(Error::NotRoland(0x42))
        ));
    }
    #[test]
    fn dt1_parser_rejects_bad_checksum_and_high_bit_fields() {
        let valid = build_dt1(0x5d, 0x2000_0000, &[0x40, 0x01]).unwrap();

        let mut bad_checksum = valid.clone();
        let checksum = bad_checksum.len() - 2;
        bad_checksum[checksum] ^= 1;
        assert!(parse_dt1_payload(&bad_checksum).is_err());

        let mut high_address = valid.clone();
        high_address[8] |= 0x80;
        assert!(parse_dt1_payload(&high_address).is_err());

        let mut high_payload = valid;
        high_payload[12] |= 0x80;
        assert!(parse_dt1_payload(&high_payload).is_err());
    }

    #[test]
    fn checksum_matches_known_roland_vectors() {
        let message = build_dt1(0x5d, 0x2000_0000, &[0x40, 0x00, 0x01]).unwrap();
        assert_eq!(message[15], 0x1f);
        assert_eq!(roland_checksum(&[0x01, 0x7f]), 0);
    }
}

#[cfg(test)]
mod builder_validation_tests {
    use super::{build_dt1, build_rq1};

    /// Verify RQ1 rejects high address bits and lengths beyond 28 bits.
    #[test]
    fn rq1_rejects_unencodable_address_and_length() {
        for address in [0x8000_0000, 0x0080_0000, 0x0000_8000, 0x0000_0080] {
            assert!(build_rq1(0x5d, address, 1).is_err());
        }
        assert!(build_rq1(0x5d, 0x3000_0000, 0x1000_0000).is_err());
    }

    /// Verify DT1 rejects high bits in either its address or data.
    #[test]
    fn dt1_rejects_unencodable_address_and_payload() {
        assert!(build_dt1(0x5d, 0x8000_0000, &[0]).is_err());
        assert!(build_dt1(0x5d, 0x2000_0000, &[0x80]).is_err());
    }
}
