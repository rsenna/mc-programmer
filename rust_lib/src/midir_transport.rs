//! Optional cross-platform MIDI transport built on [`midir`].
//!
//! Enable it with the `midir` Cargo feature. On macOS it uses CoreMIDI, on
//! Linux ALSA, and on Windows WinMM.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort, MidiOutput, MidiOutputConnection, MidiOutputPort};
use parking_lot::{Condvar, Mutex};

use crate::error::{Error, Result};
use crate::protocol::{
    parse_dt1_view, roland_checksum, universal_device_inquiry, validate_7bit_data, CMD_RQ1, ROLAND_ID, SYSEX_END,
    SYSEX_START,
};
use crate::transport::Transport;

/// Port names visible to the MIDI backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiPortNames {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Default)]
struct ReplyState {
    frames: VecDeque<Vec<u8>>,
    pending: Vec<u8>,
}

impl ReplyState {
    /// Discard queued replies and any incomplete SysEx frame before a transaction.
    fn clear(&mut self) {
        self.frames.clear();
        self.pending.clear();
    }

    /// Collect incoming bytes into SysEx frames and report whether a frame completed.
    /// Ignore bytes outside frames and restart assembly on a new `0xF0`.
    fn ingest(&mut self, message: &[u8]) -> bool {
        let mut completed = false;
        for &byte in message {
            if byte == 0xf0 {
                self.pending.clear();
                self.pending.push(byte);
                continue;
            }
            if (0xf8..=0xff).contains(&byte) {
                continue;
            }
            if self.pending.is_empty() {
                continue;
            }
            self.pending.push(byte);
            if byte == 0xf7 {
                self.frames.push_back(std::mem::take(&mut self.pending));
                completed = true;
            }
        }
        completed
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpectedReply {
    NoReply,
    DeviceIdentity,
    Rq1 {
        model_id: u8,
        address: u32,
        count: u32,
    },
}

fn expected_reply(request: &[u8]) -> Result<ExpectedReply> {
    if request == universal_device_inquiry() {
        return Ok(ExpectedReply::DeviceIdentity);
    }
    if request.get(7) == Some(&crate::protocol::CMD_DT1) {
        parse_dt1_view(request)?;
        return Ok(ExpectedReply::NoReply);
    }
    if request.len() != 18
        || request[0] != SYSEX_START
        || request[1] != ROLAND_ID
        || request[7] != CMD_RQ1
        || request[request.len() - 1] != SYSEX_END
    {
        return Err(Error::Value(
            "Midir transfer supports only valid RQ1, DT1, and device-inquiry requests".to_string(),
        ));
    }
    validate_7bit_data(&request[1..request.len() - 1], "RQ1 request")?;
    let checksum_index = request.len() - 2;
    if request[checksum_index] != roland_checksum(&request[8..checksum_index]) {
        return Err(Error::Value("RQ1 request checksum is invalid".to_string()));
    }
    let address = u32::from_be_bytes([request[8], request[9], request[10], request[11]]);
    let count =
        ((request[12] as u32) << 21) | ((request[13] as u32) << 14) | ((request[14] as u32) << 7) | request[15] as u32;
    Ok(ExpectedReply::Rq1 {
        model_id: request[6],
        address,
        count,
    })
}

fn reply_matches(expected: &ExpectedReply, reply: &[u8]) -> bool {
    match expected {
        ExpectedReply::NoReply => false,
        ExpectedReply::DeviceIdentity => {
            reply.len() >= 7
                && reply[0] == SYSEX_START
                && reply[1] == 0x7e
                && reply[3] == 0x06
                && reply[4] == 0x02
                && reply[5] == ROLAND_ID
                && reply[reply.len() - 1] == SYSEX_END
                && validate_7bit_data(&reply[1..reply.len() - 1], "identity reply").is_ok()
        }
        ExpectedReply::Rq1 {
            model_id,
            address,
            count,
        } => {
            let Ok((reply_model, reply_address, data)) = parse_dt1_view(reply) else {
                return false;
            };
            reply_model == *model_id && reply_address == *address && data.len() == *count as usize
        }
    }
}

fn timeout_duration(timeout: f64) -> Result<Duration> {
    if !timeout.is_finite() {
        return Err(Error::Value("MIDI timeout must be finite".to_string()));
    }
    Duration::try_from_secs_f64(timeout.max(0.0))
        .map_err(|_| Error::Value("MIDI timeout exceeds the supported duration".to_string()))
}

/// Synchronous request/reply transport for real MIDI hardware.
///
/// Construct it with [`MidirTransport::connect`]. Calls to [`Transport::transfer`]
/// are serialized so a reply can never be consumed by a concurrent request.
pub struct MidirTransport {
    // midir connection handles are Send but not Sync. The mutexes satisfy the
    // Transport trait's Sync requirement and keep each connection alive.
    _input: Mutex<MidiInputConnection<()>>,
    output: Mutex<MidiOutputConnection>,
    replies: Arc<(Mutex<ReplyState>, Condvar)>,
    transaction: Mutex<()>,
}

impl MidirTransport {
    /// List all available MIDI input and output port names.
    pub fn list_ports() -> Result<MidiPortNames> {
        let input = MidiInput::new("mc_programmer-list-input")
            .map_err(|error| Error::Midi(format!("failed to initialize MIDI input: {error}")))?;
        let output = MidiOutput::new("mc_programmer-list-output")
            .map_err(|error| Error::Midi(format!("failed to initialize MIDI output: {error}")))?;
        Ok(MidiPortNames {
            inputs: input_port_names(&input)?,
            outputs: output_port_names(&output)?,
        })
    }

    /// Connect to the first input and output ports containing `device_name`.
    /// Matching is case-insensitive because backend-specific suffixes differ.
    pub fn connect(device_name: &str) -> Result<Self> {
        Self::connect_with_ports(device_name, device_name, "mc_programmer")
    }

    /// Connect when the device exposes different input and output names.
    pub fn connect_with_ports(input_name: &str, output_name: &str, client_name: &str) -> Result<Self> {
        let mut input = MidiInput::new(&format!("{client_name}-input"))
            .map_err(|error| Error::Midi(format!("failed to initialize MIDI input: {error}")))?;
        input.ignore(Ignore::None);
        let output = MidiOutput::new(&format!("{client_name}-output"))
            .map_err(|error| Error::Midi(format!("failed to initialize MIDI output: {error}")))?;

        let input_port = find_input_port(&input, input_name)?;
        let output_port = find_output_port(&output, output_name)?;
        let replies = Arc::new((Mutex::new(ReplyState::default()), Condvar::new()));
        let callback_replies = Arc::clone(&replies);
        let input_connection = input
            .connect(
                &input_port,
                &format!("{client_name}-input-connection"),
                move |_timestamp, message, _data| {
                    let (state, ready) = &*callback_replies;
                    let mut state = state.lock();
                    let completed = state.ingest(message);
                    drop(state);
                    if completed {
                        ready.notify_all();
                    }
                },
                (),
            )
            .map_err(|error| {
                Error::Midi(format!(
                    "failed to connect MIDI input {input_name:?}: {error}"
                ))
            })?;
        let output_connection = output
            .connect(&output_port, &format!("{client_name}-output-connection"))
            .map_err(|error| {
                Error::Midi(format!(
                    "failed to connect MIDI output {output_name:?}: {error}"
                ))
            })?;

        Ok(Self {
            _input: Mutex::new(input_connection),
            output: Mutex::new(output_connection),
            replies,
            transaction: Mutex::new(()),
        })
    }
}

impl Transport for MidirTransport {
    /// Serialize a MIDI request; return its matching SysEx reply when expected.
    /// DT1 writes are send-only, while RQ1 and identity requests await replies.
    fn transfer(&self, message: &[u8], timeout: f64) -> Result<Vec<u8>> {
        let expected = expected_reply(message)?;
        let timeout = timeout_duration(timeout)?;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| Error::Value("MIDI timeout exceeds the supported duration".to_string()))?;

        let _transaction = self.transaction.lock();
        let (state, ready) = &*self.replies;
        state.lock().clear();
        self.output
            .lock()
            .send(message)
            .map_err(|error| Error::Midi(format!("failed to send MIDI message: {error}")))?;
        if expected == ExpectedReply::NoReply {
            return Ok(Vec::new());
        }

        let mut state = state.lock();
        loop {
            while let Some(frame) = state.frames.pop_front() {
                if reply_matches(&expected, &frame) {
                    return Ok(frame);
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(Vec::new());
            }
            ready.wait_for(&mut state, remaining);
        }
    }

    /// Send a raw MIDI message through the output connection without waiting for a reply.
    fn send_raw(&self, message: &[u8]) -> Result<()> {
        self.output
            .lock()
            .send(message)
            .map_err(|error| Error::Midi(format!("failed to send MIDI message: {error}")))
    }
}

/// Collect backend input port names, propagating name lookup failures.
fn input_port_names(input: &MidiInput) -> Result<Vec<String>> {
    input
        .ports()
        .iter()
        .map(|port| {
            input
                .port_name(port)
                .map_err(|error| Error::Midi(format!("failed to read MIDI input port name: {error}")))
        })
        .collect()
}

/// Collect backend output port names, propagating name lookup failures.
fn output_port_names(output: &MidiOutput) -> Result<Vec<String>> {
    output
        .ports()
        .iter()
        .map(|port| {
            output
                .port_name(port)
                .map_err(|error| Error::Midi(format!("failed to read MIDI output port name: {error}")))
        })
        .collect()
}

/// Find the first input port containing `target`, ignoring case.
/// Report available input names when no port matches.
fn find_input_port(input: &MidiInput, target: &str) -> Result<MidiInputPort> {
    let target_lower = target.to_lowercase();
    for port in input.ports() {
        let name = input
            .port_name(&port)
            .map_err(|error| Error::Midi(format!("failed to read MIDI input port name: {error}")))?;
        if name.to_lowercase().contains(&target_lower) {
            return Ok(port);
        }
    }
    Err(Error::Midi(format!(
        "no MIDI input matching {target:?}; available: {:?}",
        input_port_names(input)?
    )))
}

/// Find the first output port containing `target`, ignoring case.
/// Report available output names when no port matches.
fn find_output_port(output: &MidiOutput, target: &str) -> Result<MidiOutputPort> {
    let target_lower = target.to_lowercase();
    for port in output.ports() {
        let name = output
            .port_name(&port)
            .map_err(|error| Error::Midi(format!("failed to read MIDI output port name: {error}")))?;
        if name.to_lowercase().contains(&target_lower) {
            return Ok(port);
        }
    }
    Err(Error::Midi(format!(
        "no MIDI output matching {target:?}; available: {:?}",
        output_port_names(output)?
    )))
}

#[cfg(test)]
mod tests {
    use super::ReplyState;

    /// Verify split callbacks form one reply while bytes outside the frame are ignored.
    #[test]
    fn reassembles_split_sysex_frames_and_ignores_stray_bytes() {
        let mut state = ReplyState::default();
        assert!(!state.ingest(&[0xfe, 0xf0, 0x41, 0xf8, 0x10]));
        assert!(state.ingest(&[0x12, 0xfe, 0x34, 0xf7, 0xf8]));
        assert_eq!(
            state.frames.pop_front(),
            Some(vec![0xf0, 0x41, 0x10, 0x12, 0x34, 0xf7])
        );
        assert!(state.pending.is_empty());
    }

    /// Verify a fresh SysEx start replaces an unfinished reply.
    #[test]
    fn a_new_sysex_start_discards_an_incomplete_frame() {
        let mut state = ReplyState::default();
        state.ingest(&[0xf0, 0x01, 0x02]);
        assert!(state.ingest(&[0xf0, 0x41, 0xf7]));
        assert_eq!(state.frames.pop_front(), Some(vec![0xf0, 0x41, 0xf7]));
    }
}

#[cfg(test)]
mod request_match_tests {
    use super::{expected_reply, reply_matches, timeout_duration};
    use crate::protocol::{build_dt1, build_rq1, universal_device_inquiry};
    use std::time::Duration;

    #[test]
    fn rq1_accepts_only_matching_valid_dt1_frames() {
        let request = build_rq1(0x5d, 0x2000_0000, 2).unwrap();
        let expected = expected_reply(&request).unwrap();
        let valid = build_dt1(0x5d, 0x2000_0000, &[1, 2]).unwrap();
        assert!(reply_matches(&expected, &valid));
        assert!(!reply_matches(
            &expected,
            &build_dt1(0x5d, 0x2000_0100, &[1, 2]).unwrap()
        ));
        assert!(!reply_matches(
            &expected,
            &build_dt1(0x5e, 0x2000_0000, &[1, 2]).unwrap()
        ));
        assert!(!reply_matches(
            &expected,
            &build_dt1(0x5d, 0x2000_0000, &[1]).unwrap()
        ));
        let mut corrupt = valid;
        let checksum = corrupt.len() - 2;
        corrupt[checksum] ^= 1;
        assert!(!reply_matches(&expected, &corrupt));
    }

    #[test]
    fn device_inquiry_accepts_only_identity_replies() {
        let request = universal_device_inquiry();
        let expected = expected_reply(&request).unwrap();
        assert!(reply_matches(
            &expected,
            &[0xf0, 0x7e, 0x10, 0x06, 0x02, 0x41, 0x5d, 0x00, 0xf7]
        ));
        assert!(!reply_matches(
            &expected,
            &[0xf0, 0x7e, 0x10, 0x06, 0x02, 0x42, 0x5d, 0x00, 0xf7]
        ));
        assert!(!reply_matches(
            &expected,
            &build_dt1(0x5d, 0x2000_0000, &[1]).unwrap()
        ));
    }

    #[test]
    fn timeout_conversion_rejects_unrepresentable_values() {
        assert!(timeout_duration(f64::INFINITY).is_err());
        assert!(timeout_duration(f64::MAX).is_err());
        assert_eq!(timeout_duration(-1.0).unwrap(), Duration::ZERO);
    }

    #[test]
    fn dt1_write_is_a_send_only_transfer() {
        let request = build_dt1(0x5d, 0x2000_0000, &[1]).unwrap();
        assert!(expected_reply(&request).is_ok());
    }
}
