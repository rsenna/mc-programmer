//! Pluggable MIDI transport for the library.
//!
//! The library deliberately does **not** depend on any specific MIDI backend —
//! it speaks raw Sys-Ex byte vectors. Provide an implementation of
//! [`Transport`] that knows how to send a Sys-Ex message and collect any
//! reply, and the rest of the library will work.
//!
//! A trivial in-memory [`NullTransport`] is included for unit tests.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::error::{Error, Result};

/// Two-way Sys-Ex transport for the MC family.
///
/// Implementations need two methods:
///
/// * [`transfer`](Transport::transfer) — send a complete Sys-Ex message
///   (including `0xF0` and `0xF7`) and return the reply, if any. `timeout` is
///   in **seconds**. An empty vector means *no reply*.
/// * [`send_raw`](Transport::send_raw) — send a non-Sys-Ex MIDI message
///   (Control Change, Program Change, …) and return immediately. The MC
///   family uses this for Scene and Clip selection via Program Change.
///
/// `send_raw` has a default explicit unsupported-operation error so custom
/// transports cannot report Program Change operations as successful no-ops.
pub trait Transport: Send + Sync {
    /// Send a complete SysEx message and collect a reply within `timeout` seconds.
    /// Return an empty vector when no reply is received.
    fn transfer(&self, message: &[u8], timeout: f64) -> Result<Vec<u8>>;

    /// Send a non-SysEx MIDI message; the default implementation does nothing.
    fn send_raw(&self, _message: &[u8]) -> Result<()> {
        Err(Error::Midi(
            "transport does not support raw MIDI messages".to_string(),
        ))
    }
}

#[derive(Default)]
struct NullState {
    sent: Vec<Vec<u8>>,
    responses: Vec<Vec<u8>>,
}

/// Records messages without sending anything. Useful for tests.
///
/// Set `responses` to script replies for [`transfer`](Transport::transfer).
/// The transport is cheap to clone; clones share the same recorded state, so
/// a test can hand one clone to a client and inspect another clone afterwards.
#[derive(Clone, Default)]
pub struct NullTransport {
    state: Arc<Mutex<NullState>>,
}

impl NullTransport {
    /// Create a recorder with no queued replies or recorded messages.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a transport with a scripted queue of reply messages.
    pub fn with_responses(responses: Vec<Vec<u8>>) -> Self {
        NullTransport {
            state: Arc::new(Mutex::new(NullState {
                sent: Vec::new(),
                responses,
            })),
        }
    }

    /// Snapshot of every message sent through `transfer` or `send_raw`.
    pub fn sent(&self) -> Vec<Vec<u8>> {
        self.state.lock().sent.clone()
    }
}

impl Transport for NullTransport {
    /// Record the request and consume the next scripted reply, or return an empty vector.
    fn transfer(&self, message: &[u8], _timeout: f64) -> Result<Vec<u8>> {
        let mut state = self.state.lock();
        state.sent.push(message.to_vec());
        Ok(if state.responses.is_empty() {
            Vec::new()
        } else {
            state.responses.remove(0)
        })
    }

    /// Record a raw MIDI message without consuming a scripted reply.
    fn send_raw(&self, message: &[u8]) -> Result<()> {
        self.state.lock().sent.push(message.to_vec());
        Ok(())
    }
}
