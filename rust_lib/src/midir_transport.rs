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
    fn clear(&mut self) {
        self.frames.clear();
        self.pending.clear();
    }

    fn ingest(&mut self, message: &[u8]) -> bool {
        let mut completed = false;
        for &byte in message {
            if byte == 0xF0 {
                self.pending.clear();
                self.pending.push(byte);
                continue;
            }
            if self.pending.is_empty() {
                continue;
            }
            self.pending.push(byte);
            if byte == 0xF7 {
                self.frames.push_back(std::mem::take(&mut self.pending));
                completed = true;
            }
        }
        completed
    }
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
    fn transfer(&self, message: &[u8], timeout: f64) -> Result<Vec<u8>> {
        let _transaction = self.transaction.lock();
        let (state, ready) = &*self.replies;
        state.lock().clear();
        self.output
            .lock()
            .send(message)
            .map_err(|error| Error::Midi(format!("failed to send MIDI message: {error}")))?;

        let timeout = Duration::from_secs_f64(timeout.max(0.0));
        let deadline = Instant::now() + timeout;
        let mut state = state.lock();
        loop {
            if let Some(frame) = state.frames.pop_front() {
                return Ok(frame);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(Vec::new());
            }
            ready.wait_for(&mut state, remaining);
        }
    }

    fn send_raw(&self, message: &[u8]) -> Result<()> {
        self.output
            .lock()
            .send(message)
            .map_err(|error| Error::Midi(format!("failed to send MIDI message: {error}")))
    }
}

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

    #[test]
    fn reassembles_split_sysex_frames_and_ignores_stray_bytes() {
        let mut state = ReplyState::default();
        assert!(!state.ingest(&[0xFE, 0xF0, 0x41, 0x10]));
        assert!(state.ingest(&[0x12, 0x34, 0xF7, 0xF8]));
        assert_eq!(
            state.frames.pop_front(),
            Some(vec![0xF0, 0x41, 0x10, 0x12, 0x34, 0xF7])
        );
        assert!(state.pending.is_empty());
    }

    #[test]
    fn a_new_sysex_start_discards_an_incomplete_frame() {
        let mut state = ReplyState::default();
        state.ingest(&[0xF0, 0x01, 0x02]);
        assert!(state.ingest(&[0xF0, 0x41, 0xF7]));
        assert_eq!(state.frames.pop_front(), Some(vec![0xF0, 0x41, 0xF7]));
    }
}
