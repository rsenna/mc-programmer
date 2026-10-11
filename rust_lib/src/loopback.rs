//! [`LoopbackTransport`] — an in-memory fake MC device.
//!
//! Implements [`Transport`] by maintaining a sparse in-memory address space:
//! every DT1 (data set) message it receives gets written into the store, and
//! every RQ1 (data request) it receives is answered with a DT1 reply built
//! from the store.
//!
//! This lets examples and tests run end-to-end without any MIDI hardware using
//! exactly the same [`crate::client::MCClient`] calls they would use against a
//! real device. No reply staging, no message inspection — just write code that
//! looks like the production path.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::addresses::{add_roland_offset, roland_address_to_linear};
use crate::error::{Error, Result};
use crate::protocol::{build_dt1, parse_dt1_payload, roland_checksum, validate_7bit_data, ModelId};
use crate::transport::Transport;

#[derive(Default)]
struct Inner {
    /// Sparse address-space store: address → byte value.
    memory: HashMap<u32, u8>,
    model_id: u8,
    unread_byte: u8,
    /// Every non-Sys-Ex message that's gone through `send_raw`.
    sent_raw: Vec<Vec<u8>>,
    /// Most-recently-selected scene number (1-based), if any.
    active_scene: Option<u8>,
    /// Per-MIDI-channel record of the most-recently-selected clip (1-based).
    active_clip_per_channel: HashMap<u8, u8>,
    /// Report reply addresses the way real hardware does (masked linear)
    /// rather than echoing the packed request address.
    hardware_reply_addresses: bool,
}

/// In-memory fake MC device. Cheap to clone — clones share the same address
/// space, so a test can hand one clone to a client and inspect another.
#[derive(Clone)]
pub struct LoopbackTransport {
    inner: Arc<Mutex<Inner>>,
}

impl LoopbackTransport {
    /// Create an in-memory fake MC.
    ///
    /// `unread_byte` is the backing-store sentinel for never-written addresses.
    /// `snapshot` and RQ1 reads both expose only 7-bit SysEx values.
    pub fn new(model_id: ModelId, unread_byte: u8) -> Self {
        LoopbackTransport {
            inner: Arc::new(Mutex::new(Inner {
                memory: HashMap::new(),
                model_id: model_id.value(),
                unread_byte,
                ..Default::default()
            })),
        }
    }

    /// Report reply addresses the way a real MC does: the data's **linear**
    /// address with every byte masked to 7 bits, instead of echoing back the
    /// packed address that was requested.
    ///
    /// Off by default, because most of the library is written against the
    /// packed echo. Turn it on for anything that maps replies back to
    /// addresses — see [`crate::probe`] — so the fake can actually exercise
    /// that mapping.
    ///
    /// Clones share one address space, so this also affects every existing
    /// clone of the transport, not just the returned value.
    pub fn with_hardware_reply_addresses(self) -> Self {
        self.inner.lock().hardware_reply_addresses = true;
        self
    }

    /// Pre-load a region of the address space (useful in tests).
    pub fn preload(&self, address: u32, bytes: &[u8]) {
        let mut inner = self.inner.lock();
        for (i, &b) in bytes.iter().enumerate() {
            let Ok(offset) = u32::try_from(i) else {
                break;
            };
            let Ok(address) = add_roland_offset(address, offset) else {
                break;
            };
            inner.memory.insert(address, b);
        }
    }

    /// Read the 7-bit values visible to RQ1 reads (for assertions).
    pub fn snapshot(&self, address: u32, count: usize) -> Vec<u8> {
        let inner = self.inner.lock();
        (0..count)
            .map(|i| {
                let address = u32::try_from(i)
                    .ok()
                    .and_then(|offset| add_roland_offset(address, offset).ok());
                address
                    .and_then(|address| inner.memory.get(&address).copied())
                    .unwrap_or(inner.unread_byte)
                    & 0x7f
            })
            .collect()
    }

    /// Every non-Sys-Ex message that's gone through `send_raw`.
    pub fn sent_raw(&self) -> Vec<Vec<u8>> {
        self.inner.lock().sent_raw.clone()
    }

    /// Most-recently-selected scene number (1-based), if any.
    pub fn active_scene(&self) -> Option<u8> {
        self.inner.lock().active_scene
    }

    /// Most-recently-selected clip (1-based) on the given MIDI channel.
    pub fn active_clip(&self, channel: u8) -> Option<u8> {
        self.inner
            .lock()
            .active_clip_per_channel
            .get(&channel)
            .copied()
    }
}

/// Recognize a six-byte universal device inquiry regardless of device ID.
fn is_identity_request(msg: &[u8]) -> bool {
    // F0 7E <dev> 06 01 F7
    msg.len() == 6 && msg[0] == 0xf0 && msg[1] == 0x7e && msg[3] == 0x06 && msg[4] == 0x01 && msg[5] == 0xf7
}

impl Transport for LoopbackTransport {
    /// Emulate device inquiry, DT1 writes, and RQ1 reads against shared memory.
    /// The timeout is unused; unsupported messages receive an empty reply.
    fn transfer(&self, message: &[u8], _timeout: f64) -> Result<Vec<u8>> {
        // Universal Identity Request → synthetic Identity Reply.
        if is_identity_request(message) {
            let model_id = self.inner.lock().model_id;
            // MCClient.detect_model_id reads index 6 of the reply.
            return Ok(vec![
                0xf0, 0x7e, 0x7f, 0x06, 0x02, 0x41, model_id, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf7,
            ]);
        }

        if message.len() < 14 || message[0] != 0xf0 || message[message.len() - 1] != 0xf7 || message[1] != 0x41 {
            return Ok(Vec::new());
        }

        let cmd = message[7];
        let address = ((message[8] as u32) << 24)
            | ((message[9] as u32) << 16)
            | ((message[10] as u32) << 8)
            | (message[11] as u32);

        match cmd {
            0x12 => {
                // DT1 — validate the complete packet before mutating memory.
                let dt1 = parse_dt1_payload(message)?;
                let mut inner = self.inner.lock();
                for (i, &b) in dt1.data.iter().enumerate() {
                    let offset =
                        u32::try_from(i).map_err(|_| Error::Value("MIDI payload offset exceeds u32".to_string()))?;
                    inner
                        .memory
                        .insert(add_roland_offset(dt1.address, offset)?, b);
                }
                Ok(Vec::new())
            }
            0x11 => {
                if message.len() != 18 {
                    return Err(Error::Value(format!(
                        "RQ1 message has {} bytes; expected 18",
                        message.len()
                    )));
                }
                validate_7bit_data(&message[1..message.len() - 1], "RQ1 message")?;
                if message[message.len() - 2] != roland_checksum(&message[8..message.len() - 2]) {
                    return Err(Error::Value("RQ1 checksum mismatch".to_string()));
                }
                // RQ1 — synthesize a DT1 reply only after validating fields.
                let count = ((message[12] as u32) << 21)
                    | ((message[13] as u32) << 14)
                    | ((message[14] as u32) << 7)
                    | (message[15] as u32);
                let inner = self.inner.lock();
                let data: Vec<u8> = (0..count)
                    .map(|i| {
                        add_roland_offset(address, i)
                            .ok()
                            .and_then(|address| inner.memory.get(&address).copied())
                            .unwrap_or(inner.unread_byte)
                            & 0x7f
                    })
                    .collect();
                // How the reply's address field is filled in. A real MC reports
                // the returned data's **linear** address, with every byte
                // masked to 0x7F by the 7-bit MIDI pipe — so several linear
                // addresses arrive identically (verified on an MC-101: packed
                // 0x40000000, 0x40000100 and 0x40020000 all reply
                // `08 00 00 00`). Echoing the packed request address, as this
                // fake does by default, cannot reproduce that, which is how a
                // byte-misplacement bug in the probe survived the whole
                // offline suite and only appeared against hardware.
                //
                // The faithful behaviour is opt-in via
                // [`LoopbackTransport::with_hardware_reply_addresses`] because
                // the rest of the library is written against the packed echo.
                let reply_address = if inner.hardware_reply_addresses {
                    roland_address_to_linear(address)? & 0x7f7f_7f7f
                } else {
                    address
                };
                Ok(build_dt1(inner.model_id, reply_address, &data)?)
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Record a raw MIDI message and update scene and channel clip state for Program Change.
    fn send_raw(&self, message: &[u8]) -> Result<()> {
        let mut inner = self.inner.lock();
        inner.sent_raw.push(message.to_vec());

        if message.is_empty() {
            return Ok(());
        }
        let status = message[0] & 0xf0;
        let channel = (message[0] & 0x0f) + 1;
        if status == 0xc0 && message.len() >= 2 {
            let program = (message[1] & 0x7f) + 1;
            inner.active_scene = Some(program);
            inner.active_clip_per_channel.insert(channel, program);
        }
        Ok(())
    }
}

/// Convenience: a loopback device whose model ID is MC-707.
impl Default for LoopbackTransport {
    /// Create an MC-707 loopback device with `0xFF` in unwritten memory.
    fn default() -> Self {
        Self::new(ModelId::Mc707, 0xff)
    }
}
