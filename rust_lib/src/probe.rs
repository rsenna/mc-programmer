//! Address-space probe — find undocumented MC blocks by dumping and diffing.
//!
//! The MC's address map is only partly documented. Blocks like the scene block
//! (which clip each track plays, scene name, scene tempo) and the scene chain
//! have no published address, so the only way to locate them is empirical:
//!
//! 1. [`walk`] a candidate address range and record every byte the device
//!    actually answers for, giving a [`Snapshot`].
//! 2. Change exactly one thing on the device's front panel.
//! 3. [`walk`] the same range again.
//! 4. [`diff`] the two snapshots. The changed bytes are where that thing lives.
//!
//! This is the method the original `hack_tools.py` exploration scripts used
//! (see this repository's git history) rebuilt as a supported, tested part of
//! the library.
//!
//! # Address arithmetic
//!
//! Roland addresses carry 28 useful bits in four 7-bit bytes, so the packed
//! address space has holes: `0x00000080` is not a valid address, `0x00000100`
//! is the byte after `0x0000007F`. The walker therefore steps through *linear*
//! addresses and converts, rather than incrementing the packed value — see
//! [`crate::addresses::roland_address_to_linear`]. Every address a
//! [`Snapshot`] records is a valid packed address.
//!
//! # Example
//!
//! ```
//! use mc_programmer::{probe, LoopbackTransport, MCClient, ModelId};
//!
//! // The fake must report reply addresses like hardware, since resolving
//! // those is exactly what the probe does.
//! let transport = LoopbackTransport::new(ModelId::Mc101, 0xFF).with_hardware_reply_addresses();
//! let client = MCClient::new(transport.clone(), ModelId::Mc101.value());
//! let opts = probe::ProbeOptions::default();
//!
//! transport.preload(0x20000000, &[0x01, 0x02, 0x03, 0x04]);
//! let before = probe::walk(&client, 0x20000000, 0x20000010, &opts).unwrap();
//!
//! transport.preload(0x20000002, &[0x33]);
//! let after = probe::walk(&client, 0x20000000, 0x20000010, &opts).unwrap();
//!
//! let changes = probe::diff(&before, &after);
//! assert_eq!(changes.len(), 1);
//! assert_eq!(changes[0].address, 0x20000002);
//! assert_eq!((changes[0].before, changes[0].after), (Some(0x03), Some(0x33)));
//! ```

use std::collections::BTreeMap;
use std::fs;

use serde_json::{json, Value};

use crate::addresses::{
    resolve_reply_location, resolve_reply_location_from, roland_address_from_linear, roland_address_to_linear,
    ReplyLocation,
};
use crate::client::MCClient;
use crate::error::{Error, Result};
use crate::preset::{bytes_to_hex, hex_to_bytes};
use crate::protocol::model_name;
use crate::transport::Transport;

/// File-format tag written into saved snapshots.
pub const SNAPSHOT_FORMAT: &str = "mc_programmer.probe.snapshot.v1";

/// Bytes per request. 16 keeps a miss cheap and lines up with hex-dump rows.
pub const DEFAULT_CHUNK: u32 = 0x10;

/// Largest payload one RQ1 may ask for, matching [`MCClient::CHUNK_SIZE`].
pub const MAX_CHUNK: u32 = 0x80;

/// How many individual failure addresses a snapshot keeps. The count is always
/// exact; only the per-address detail is capped, so a link that is failing
/// everywhere cannot balloon the file.
pub const MAX_RECORDED_FAILURES: usize = 64;

/// How a walk reads a range.
#[derive(Debug, Clone, Copy)]
pub struct ProbeOptions {
    /// Bytes per RQ1. Must be 1..=[`MAX_CHUNK`] and divide `0x80`, which keeps
    /// it a power of two so that halving during subdivision stays exact.
    ///
    /// It does **not** stop a chunk straddling a 7-bit byte boundary — nothing
    /// constrains the start address to be `0x80`-aligned, and straddling is
    /// harmless anyway because the walker works in linear addresses.
    pub chunk: u32,
    /// Linear distance between chunk starts. Equal to `chunk` for a gapless
    /// sweep; larger to sample sparsely when first bracketing a region.
    pub step: u32,
    /// Reply timeout in seconds. Every unmapped address costs this much.
    pub timeout: f64,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        ProbeOptions {
            chunk: DEFAULT_CHUNK,
            step: DEFAULT_CHUNK,
            timeout: 0.1,
        }
    }
}

impl ProbeOptions {
    /// Gapless sweep with the given chunk size.
    pub fn gapless(chunk: u32) -> Self {
        ProbeOptions {
            chunk,
            step: chunk,
            timeout: 0.1,
        }
    }

    fn validate(&self) -> Result<()> {
        let max = MAX_CHUNK;
        if self.chunk == 0 || self.chunk > max {
            return Err(Error::Value(format!(
                "probe chunk must be 1..={max} bytes, got {}",
                self.chunk
            )));
        }
        if max % self.chunk != 0 {
            return Err(Error::Value(format!(
                "probe chunk {} must divide 0x{max:X}, so that halving it during subdivision stays exact",
                self.chunk
            )));
        }
        if self.step < self.chunk {
            return Err(Error::Value(format!(
                "probe step 0x{:X} is smaller than chunk 0x{:X}, which would re-read bytes",
                self.step, self.chunk
            )));
        }
        if self.timeout <= 0.0 || self.timeout.is_nan() {
            return Err(Error::Value("probe timeout must be positive".to_string()));
        }
        Ok(())
    }
}

/// What one walk saw: every byte the device answered for, by packed address.
///
/// Addresses the device did not answer for are *absent*, not zero — the
/// difference matters, because a region going from unmapped to mapped is itself
/// a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Human-readable model name, e.g. `"MC-101"`.
    pub model: String,
    /// Free-text note, e.g. `"scene 1 selected"`.
    pub label: Option<String>,
    /// Packed address → byte value.
    pub bytes: BTreeMap<u32, u8>,
    /// Chunks requested during the walk.
    pub chunks_probed: usize,
    /// Chunks that produced a reply.
    pub chunks_answered: usize,
    /// Chunks whose reply could not be parsed. A sweep keeps going when this
    /// happens — a single garbled reply must not discard minutes of work —
    /// and the addresses land in [`failures`](Snapshot::failures).
    pub chunks_failed: usize,
    /// Address and error text for each reply the sweep could not use —
    /// unparseable frames and replies whose address could not be resolved — in
    /// sweep order, capped at [`MAX_RECORDED_FAILURES`].
    pub failures: Vec<(u32, String)>,
    /// Replies the device moved forward from the requested address, which it
    /// does to skip unmapped bytes. Normal near a region edge; the bytes are
    /// placed where the reply says, not where they were asked for.
    pub relocated_replies: usize,
    /// Replies whose masked address could not be resolved against the
    /// request. Their payload is **discarded** rather than placed at a guessed
    /// address, so a non-zero count means the sweep has holes — refine the
    /// range or chunk size and re-run.
    pub unplaceable_replies: usize,
    /// Ambiguous resolutions settled by follow-up requests. This is the
    /// normal state, not an anomaly: a masked reply almost always has a
    /// higher candidate, so nearly every answered chunk is confirmed. What it
    /// buys is a placement that is a fact rather than a guess.
    pub confirmations: usize,
    /// Placements kept on the strength of silence from every follow-up:
    /// consistent with a one-byte run at that address, but not proven, since
    /// silence is also what a dropped frame or a request the device declines
    /// looks like. Expected at the top edge of each mapped region. Treat these
    /// addresses as probable rather than established.
    pub unconfirmed_placements: usize,
    /// Follow-up exchanges those confirmations cost. Roughly equal to
    /// `confirmations` on a healthy sweep, so expect a range that answers to
    /// cost about twice the requests `chunks_probed` reports.
    pub confirmation_exchanges: usize,
    /// Replies whose address a follow-up could not settle, and whose payload
    /// was therefore **discarded** rather than placed at a guessed address. A
    /// non-zero count means the sweep has holes. See
    /// [`crate::addresses::resolve_reply_location`].
    pub unprovable_placements: usize,
    /// Payloads cut short because they ran past the end of the swept range or
    /// the top of the 28-bit address space. The device answers with the next
    /// mapped run at or after the request, so it can volunteer bytes beyond
    /// what was asked for; those are dropped rather than silently widening the
    /// snapshot.
    pub truncated_placements: usize,
    /// Requests the sweep had to split because the reply could not be shown to
    /// cover its request. Expected around the edges of mapped regions; the cost
    /// of placing every byte at an address it was actually confirmed at.
    pub subdivisions: usize,
}

/// One contiguous run of bytes, for display and for grouping diffs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub address: u32,
    pub bytes: Vec<u8>,
}

/// One byte that differs between two snapshots.
///
/// `None` means the address was absent from that snapshot (no reply).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteDiff {
    pub address: u32,
    pub before: Option<u8>,
    pub after: Option<u8>,
}

/// A contiguous group of [`ByteDiff`]s, which is how a real field shows up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRun {
    pub address: u32,
    pub before: Vec<Option<u8>>,
    pub after: Vec<Option<u8>>,
}

impl DiffRun {
    pub fn len(&self) -> usize {
        self.after.len()
    }

    pub fn is_empty(&self) -> bool {
        self.after.is_empty()
    }
}

/// How many rounds a confirmation climb may take before giving up.
///
/// This is a budget, **not** a proof of sufficiency. Each round resolves a
/// *different* reply with its own candidate set — the device relocates forward
/// to the next mapped run — so the climb walks up one mapped run per round and
/// a sparse region can need more rounds than this. Exhaustion is reported as
/// such rather than silently accepted, because a different `--start` or
/// `chunk` starts the climb elsewhere and may resolve what this one could not.
const MAX_CONFIRMATION_STEPS: usize = 8;

/// What a confirmation climb established about a base address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirmation {
    /// A follow-up reply positively identified this address.
    Confirmed(u32),
    /// Every follow-up went silent. That is *consistent* with the run being
    /// the single byte at this address — had it started further on, that run
    /// is mapped above the follow-up and the device would have answered — but
    /// it is not proof: [`MCClient::read_dt1`] also reports silence for a
    /// dropped frame, interleaved traffic, a busy device, or a request the
    /// device declines, and the MC in particular largely ignores single-byte
    /// requests. The bytes are worth keeping; the certainty is not.
    Unconfirmed(u32),
    /// Nothing could be established. `exhausted` distinguishes "ran out of
    /// rounds" (a different sweep may do better) from "a reply could not be
    /// resolved at all" (it cannot).
    Unresolved { exhausted: bool },
}

/// Settle an ambiguous base by asking again from just above it.
///
/// Asking from `candidate + 1` drops `candidate` out of the candidate set,
/// which is what breaks the collapse described in
/// [`resolve_reply_location`](crate::addresses::resolve_reply_location):
///
/// * **Answers `candidate + 1`.** The byte above is mapped too, so the run
///   genuinely starts at `candidate` — [`Confirmation::Confirmed`].
/// * **Answers higher.** That is the next mapped run, which may itself be a
///   collapse, so the test repeats from there.
/// * **Silence.** Consistent with a one-byte run at `candidate`, but not
///   proof — [`Confirmation::Unconfirmed`].
fn confirm_base<T: Transport>(
    client: &MCClient<T>,
    base: u32,
    count: u32,
    timeout: f64,
) -> Result<(Confirmation, usize)> {
    let mut candidate = base;
    let mut exchanges = 0usize;

    for _ in 0..MAX_CONFIRMATION_STEPS {
        let probe_linear = candidate.saturating_add(1);
        let Ok(address) = roland_address_from_linear(probe_linear) else {
            // `candidate` is the last byte of the address space: there is
            // nothing above it to test against, and nothing above it to be
            // confused with, so this one IS forced.
            return Ok((Confirmation::Confirmed(candidate), exchanges));
        };

        exchanges += 1;
        let Some(reply) = client.read_dt1(address, count, timeout)? else {
            return Ok((Confirmation::Unconfirmed(candidate), exchanges));
        };

        match resolve_reply_location_from(reply.address, probe_linear) {
            ReplyLocation::Resolved { linear, .. } if linear == probe_linear => {
                return Ok((Confirmation::Confirmed(candidate), exchanges))
            }
            ReplyLocation::Resolved { linear, .. } => candidate = linear,
            _ => return Ok((Confirmation::Unresolved { exhausted: false }, exchanges)),
        }
    }

    Ok((Confirmation::Unresolved { exhausted: true }, exchanges))
}

/// Record a failure, keeping at most [`MAX_RECORDED_FAILURES`] of them.
fn record_failure(snapshot: &mut Snapshot, address: u32, message: String) {
    if snapshot.failures.len() < MAX_RECORDED_FAILURES {
        snapshot.failures.push((address, message));
    }
}

/// Write a payload into the snapshot at `base`.
///
/// Bytes that would fall outside the 28-bit address space are dropped rather
/// than propagated as an error: a sweep takes minutes, and aborting it over a
/// payload that runs off the top of the space would throw away everything
/// collected so far — the same policy applied to unparseable frames.
fn place(snapshot: &mut Snapshot, base: u32, data: &[u8], sweep_end: u32) {
    for (i, &byte) in data.iter().enumerate() {
        let linear = base + i as u32;
        // The device answers with the next mapped run at or after the address
        // asked for, which can reach past the range the caller swept. Those
        // bytes are real, but they are outside what was requested, and a
        // snapshot that quietly extended beyond its own range would be
        // confusing to diff.
        if linear >= sweep_end {
            snapshot.truncated_placements += 1;
            return;
        }
        let Ok(address) = roland_address_from_linear(linear) else {
            snapshot.truncated_placements += 1;
            return;
        };
        snapshot.bytes.insert(address, byte);
    }
}

/// Walk `[start, end)` and record every byte the device answers for.
///
/// `start` and `end` are packed Roland addresses; `end` is exclusive and may
/// equal `start` (yielding an empty snapshot). Reads that get no reply are
/// recorded as misses and skipped, so sweeping a mostly-unmapped range is
/// safe — just slow, at `timeout` per miss.
pub fn walk<T: Transport>(client: &MCClient<T>, start: u32, end: u32, opts: &ProbeOptions) -> Result<Snapshot> {
    walk_with_progress(client, start, end, opts, &mut |_, _| {})
}

/// [`walk`], reporting progress as `(packed_address, bytes_answered)` per chunk.
pub fn walk_with_progress<T: Transport>(
    client: &MCClient<T>,
    start: u32,
    end: u32,
    opts: &ProbeOptions,
    progress: &mut dyn FnMut(u32, usize),
) -> Result<Snapshot> {
    opts.validate()?;

    let first = roland_address_to_linear(start)?;
    let last = roland_address_to_linear(end)?;
    if last < first {
        return Err(Error::Value(format!(
            "probe end 0x{end:08X} is below start 0x{start:08X}"
        )));
    }

    let mut snapshot = Snapshot {
        model: model_name(client.model_id),
        label: None,
        bytes: BTreeMap::new(),
        chunks_probed: 0,
        chunks_answered: 0,
        chunks_failed: 0,
        failures: Vec::new(),
        relocated_replies: 0,
        unplaceable_replies: 0,
        subdivisions: 0,
        truncated_placements: 0,
        unprovable_placements: 0,
        confirmations: 0,
        confirmation_exchanges: 0,
        unconfirmed_placements: 0,
    };

    // Work stack of (linear start, byte count) still to resolve. A short
    // reply cannot be placed as-is, so it is pushed back as two halves.
    let mut pending: Vec<(u32, u32)> = Vec::new();
    let mut linear = first;
    while linear < last {
        let count = opts.chunk.min(last - linear);
        pending.push((linear, count));
        linear = linear.saturating_add(opts.step);
    }
    pending.reverse(); // sweep low addresses first

    while let Some((start_linear, count)) = pending.pop() {
        let address = roland_address_from_linear(start_linear)?;
        snapshot.chunks_probed += 1;

        // A device on a real MIDI link occasionally answers with a frame that
        // will not parse — interleaved traffic, a truncated Sys-Ex, a reply to
        // something else. Record it and carry on: aborting would throw away
        // every byte of a sweep that takes minutes.
        let reply = match client.read_dt1(address, count, opts.timeout) {
            Ok(reply) => reply,
            Err(error) => {
                snapshot.chunks_failed += 1;
                if snapshot.failures.len() < MAX_RECORDED_FAILURES {
                    snapshot.failures.push((address, error.to_string()));
                }
                None
            }
        };

        let Some(message) = reply else {
            progress(address, 0);
            continue;
        };
        // `read_dt1` deliberately skips the payload-length check that `read`
        // applies, so a well-formed DT1 carrying no payload reaches here. It
        // must not reach the slicing below: `&data[..1]` on an empty payload
        // panics, and one such reply would abort a sweep that every other
        // error path in this function is written to keep alive.
        if message.data.is_empty() {
            snapshot.chunks_failed += 1;
            record_failure(
                &mut snapshot,
                address,
                "DT1 reply carried an empty payload".to_string(),
            );
            progress(address, 0);
            continue;
        }
        snapshot.chunks_answered += 1;

        // Where does this payload actually live? The reply reports the data's
        // linear address with every byte masked to 7 bits, so the request has
        // to disambiguate it.
        let (base, provable) = match resolve_reply_location(message.address, start_linear) {
            ReplyLocation::Resolved {
                linear, provable, ..
            } => (linear, provable),
            ReplyLocation::Ambiguous(candidates) => {
                snapshot.unplaceable_replies += 1;
                record_failure(
                    &mut snapshot,
                    address,
                    format!(
                        "reply address 0x{:08X} is ambiguous ({} candidates)",
                        message.address,
                        candidates.len()
                    ),
                );
                progress(address, 0);
                continue;
            }
            ReplyLocation::Unexplained => {
                snapshot.unplaceable_replies += 1;
                record_failure(
                    &mut snapshot,
                    address,
                    format!(
                        "reply address 0x{:08X} resolves to nothing at or after the requested address",
                        message.address
                    ),
                );
                progress(address, 0);
                continue;
            }
        };
        // A resolution that is not forced must be settled, not assumed. The
        // lowest candidate and the one CANDIDATE_SPACING above it produce the
        // same masked reply, so placing the payload here would write it 0x80
        // bytes from where it belongs whenever the device had relocated —
        // reported as certain, because `relocated` is false and the
        // beyond-span short-circuit never sees the true base. One follow-up
        // request from `base + 1` drops `base` out of the candidate set and
        // tells the two apart.
        let base = if provable {
            base
        } else {
            let (outcome, exchanges) = confirm_base(client, base, count, opts.timeout)?;
            snapshot.confirmation_exchanges += exchanges;
            match outcome {
                Confirmation::Confirmed(confirmed) => {
                    snapshot.confirmations += 1;
                    confirmed
                }
                Confirmation::Unconfirmed(assumed) => {
                    // Keep the bytes — dropping them loses the last byte of
                    // every mapped region — but do not call it a fact. Silence
                    // is also what a dropped frame looks like.
                    snapshot.unconfirmed_placements += 1;
                    assumed
                }
                Confirmation::Unresolved { exhausted } => {
                    snapshot.unprovable_placements += 1;
                    record_failure(
                        &mut snapshot,
                        address,
                        if exhausted {
                            format!(
                                "reply address 0x{:08X}: gave up after {} confirmation rounds; a \
                                 different --start or --chunk starts the climb elsewhere and may resolve it",
                                message.address, MAX_CONFIRMATION_STEPS
                            )
                        } else {
                            format!(
                                "reply address 0x{:08X}: a follow-up reply could not be resolved at all",
                                message.address
                            )
                        },
                    );
                    progress(address, 0);
                    continue;
                }
            }
        };

        // Count relocation once, against the address finally settled on. The
        // `provable` path has no confirmation step, and the confirmation path
        // may move the base, so deciding this here is the only place both
        // agree.
        if base != start_linear {
            snapshot.relocated_replies += 1;
        }

        // The device answered beyond the requested span, which means it found
        // nothing mapped inside it. Subdividing would re-ask about a range the
        // device has already answered for — 2·chunk−1 round trips to learn
        // what one reply said, with every counter inflated by the same factor.
        // Record the one byte whose address the device reported, and move on.
        if base >= start_linear.saturating_add(count) {
            place(&mut snapshot, base, &message.data[..1], last);
            progress(address, 1);
            continue;
        }

        // Can this payload be laid down contiguously from `base`?
        //
        // Two necessary conditions: it is as long as the request, and it does
        // not end past the request. The second is not implied by the first,
        // because the MC may answer past the end of the span.
        //
        // They are NOT sufficient, and cannot be made so. The device
        // gap-compresses a partly-mapped span — returning only the mapped
        // bytes, concatenated — and such a payload can match the requested
        // length exactly: asked for 8 bytes over a span whose middle 4 are
        // unmapped, a device that keeps reading past the span returns 4 from
        // inside it and 4 from beyond, totalling 8 and ending exactly at the
        // span's end. Nothing in (address, length, count) separates that from
        // a clean 8-byte run, and the obvious cross-check — confirming each
        // byte with its own 1-byte request — is unavailable, because the MC
        // largely ignores single-byte requests, so silence from one proves
        // nothing. `docs/PROBING.md` records this as a standing limitation
        // rather than a per-sweep counter, since it applies to every
        // multi-byte placement and so carries no diagnostic signal.
        let reach = base.saturating_add(message.data.len() as u32);
        let covers_request = message.data.len() as u32 == count && reach <= start_linear.saturating_add(count);

        if covers_request {
            place(&mut snapshot, base, &message.data, last);
            progress(address, message.data.len());
        } else if count == 1 {
            // One byte was asked for and the device answered at `base`, so only
            // that first byte is confirmed there; whatever followed it in the
            // device's own layout has no established address.
            place(&mut snapshot, base, &message.data[..1], last);
            progress(address, 1);
        } else {
            let half = count / 2;
            snapshot.subdivisions += 1;
            pending.push((start_linear + half, count - half));
            pending.push((start_linear, half));
            progress(address, 0);
        }
    }

    Ok(snapshot)
}

impl Snapshot {
    /// Attach a free-text note, e.g. which scene was selected.
    pub fn labelled(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Bytes recorded.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Group the recorded bytes into runs of linearly consecutive addresses.
    pub fn runs(&self) -> Vec<Run> {
        let mut runs: Vec<Run> = Vec::new();
        let mut previous_linear: Option<u32> = None;

        for (&address, &byte) in &self.bytes {
            let linear = roland_address_to_linear(address).unwrap_or(u32::MAX);
            let contiguous = previous_linear.is_some_and(|p| linear == p + 1);
            match runs.last_mut() {
                Some(run) if contiguous => run.bytes.push(byte),
                _ => runs.push(Run {
                    address,
                    bytes: vec![byte],
                }),
            }
            previous_linear = Some(linear);
        }
        runs
    }

    /// Classic `ADDRESS  HEX…  |ascii|` hex dump, 16 bytes per line.
    pub fn hex_dump(&self) -> String {
        let mut out = String::new();
        for run in self.runs() {
            for (row, chunk) in run.bytes.chunks(16).enumerate() {
                let offset = (row * 16) as u32;
                let address = roland_address_to_linear(run.address)
                    .and_then(|l| roland_address_from_linear(l + offset))
                    .unwrap_or(run.address);
                out.push_str(&format!(
                    "{address:08X}  {:<47}  |{}|\n",
                    bytes_to_hex(chunk),
                    ascii_gutter(chunk)
                ));
            }
        }
        out
    }

    /// CSV in the same shape the original exploration scripts emitted:
    /// `address,length,hex,ascii`.
    pub fn to_csv(&self) -> String {
        let mut out = String::from("address,length,hex,ascii\n");
        for run in self.runs() {
            for (row, chunk) in run.bytes.chunks(16).enumerate() {
                let offset = (row * 16) as u32;
                let address = roland_address_to_linear(run.address)
                    .and_then(|l| roland_address_from_linear(l + offset))
                    .unwrap_or(run.address);
                out.push_str(&format!(
                    "{address:08X},{},\"{}\",\"{}\"\n",
                    chunk.len(),
                    bytes_to_hex(chunk),
                    ascii_gutter(chunk).replace('"', "'")
                ));
            }
        }
        out
    }

    pub fn to_value(&self) -> Value {
        let regions: Vec<Value> = self
            .runs()
            .into_iter()
            .map(|run| {
                json!({
                    "address": format!("{:08X}", run.address),
                    "length": run.bytes.len(),
                    "data": bytes_to_hex(&run.bytes),
                })
            })
            .collect();

        json!({
            "format": SNAPSHOT_FORMAT,
            "model": self.model,
            "label": self.label,
            "chunks_probed": self.chunks_probed,
            "chunks_answered": self.chunks_answered,
            "chunks_failed": self.chunks_failed,
            "relocated_replies": self.relocated_replies,
            "unplaceable_replies": self.unplaceable_replies,
            "subdivisions": self.subdivisions,
            "truncated_placements": self.truncated_placements,
            "unprovable_placements": self.unprovable_placements,
            "confirmations": self.confirmations,
            "confirmation_exchanges": self.confirmation_exchanges,
            "unconfirmed_placements": self.unconfirmed_placements,
            "failures": self
                .failures
                .iter()
                .map(|(address, error)| json!({ "address": format!("{address:08X}"), "error": error }))
                .collect::<Vec<_>>(),
            "regions": regions,
        })
    }

    pub fn from_value(v: &Value) -> Result<Self> {
        let format = v.get("format").and_then(Value::as_str).unwrap_or_default();
        if format != SNAPSHOT_FORMAT {
            return Err(Error::Json(format!(
                "not a probe snapshot: format {format:?}, expected {SNAPSHOT_FORMAT:?}"
            )));
        }

        let mut bytes = BTreeMap::new();
        let regions = v
            .get("regions")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Json("probe snapshot has no 'regions' array".to_string()))?;

        for region in regions {
            let address_text = region
                .get("address")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Json("probe snapshot region has no 'address'".to_string()))?;
            let address = u32::from_str_radix(address_text, 16)
                .map_err(|_| Error::Json(format!("invalid region address {address_text:?}")))?;
            let data = hex_to_bytes(
                region
                    .get("data")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )?;

            let base = roland_address_to_linear(address)?;
            for (i, byte) in data.into_iter().enumerate() {
                bytes.insert(roland_address_from_linear(base + i as u32)?, byte);
            }
        }

        Ok(Snapshot {
            model: v
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            label: v.get("label").and_then(Value::as_str).map(str::to_string),
            bytes,
            chunks_probed: v.get("chunks_probed").and_then(Value::as_u64).unwrap_or(0) as usize,
            chunks_answered: v
                .get("chunks_answered")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            chunks_failed: v.get("chunks_failed").and_then(Value::as_u64).unwrap_or(0) as usize,
            relocated_replies: v
                .get("relocated_replies")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            unplaceable_replies: v
                .get("unplaceable_replies")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            subdivisions: v.get("subdivisions").and_then(Value::as_u64).unwrap_or(0) as usize,
            truncated_placements: v
                .get("truncated_placements")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            unprovable_placements: v
                .get("unprovable_placements")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            confirmations: v.get("confirmations").and_then(Value::as_u64).unwrap_or(0) as usize,
            confirmation_exchanges: v
                .get("confirmation_exchanges")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            unconfirmed_placements: v
                .get("unconfirmed_placements")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            failures: v
                .get("failures")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            let address = item.get("address").and_then(Value::as_str)?;
                            let address = u32::from_str_radix(address, 16).ok()?;
                            let error = item
                                .get("error")
                                .and_then(Value::as_str)
                                .unwrap_or_default();
                            Some((address, error.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_value()).expect("serializing a serde_json::Value cannot fail")
    }

    pub fn from_json(raw: &str) -> Result<Self> {
        let v: Value =
            serde_json::from_str(raw).map_err(|e| Error::Json(format!("invalid JSON in probe snapshot: {e}")))?;
        Self::from_value(&v)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        fs::write(path, self.to_json())?;
        Ok(())
    }

    pub fn load(path: &str) -> Result<Self> {
        Self::from_json(&fs::read_to_string(path)?)
    }
}

/// Every byte that differs between two snapshots.
///
/// Addresses present in only one snapshot are reported with `None` on the
/// other side: a region that starts or stops answering is a real change.
pub fn diff(before: &Snapshot, after: &Snapshot) -> Vec<ByteDiff> {
    let mut diffs: Vec<ByteDiff> = Vec::new();

    for (&address, &b) in &before.bytes {
        match after.bytes.get(&address) {
            Some(&a) if a == b => {}
            other => diffs.push(ByteDiff {
                address,
                before: Some(b),
                after: other.copied(),
            }),
        }
    }
    for (&address, &a) in &after.bytes {
        if !before.bytes.contains_key(&address) {
            diffs.push(ByteDiff {
                address,
                before: None,
                after: Some(a),
            });
        }
    }

    diffs.sort_by_key(|d| roland_address_to_linear(d.address).unwrap_or(u32::MAX));
    diffs
}

/// Group a sorted [`diff`] into contiguous runs — one run per changed field.
pub fn diff_runs(diffs: &[ByteDiff]) -> Vec<DiffRun> {
    let mut runs: Vec<DiffRun> = Vec::new();
    let mut previous_linear: Option<u32> = None;

    for d in diffs {
        let linear = roland_address_to_linear(d.address).unwrap_or(u32::MAX);
        let contiguous = previous_linear.is_some_and(|p| linear == p + 1);
        match runs.last_mut() {
            Some(run) if contiguous => {
                run.before.push(d.before);
                run.after.push(d.after);
            }
            _ => runs.push(DiffRun {
                address: d.address,
                before: vec![d.before],
                after: vec![d.after],
            }),
        }
        previous_linear = Some(linear);
    }
    runs
}

/// Render [`diff_runs`] as a report: one block per changed run, before over
/// after, with the bytes that actually moved marked underneath.
pub fn format_diff(runs: &[DiffRun]) -> String {
    if runs.is_empty() {
        return "no differences\n".to_string();
    }

    let mut out = format!(
        "{} changed run(s), {} byte(s) total\n",
        runs.len(),
        runs.iter().map(DiffRun::len).sum::<usize>()
    );
    for run in runs {
        out.push_str(&format!("\n{:08X}  +{} byte(s)\n", run.address, run.len()));
        out.push_str(&format!("  before  {}\n", optional_hex(&run.before)));
        out.push_str(&format!("  after   {}\n", optional_hex(&run.after)));
    }
    out
}

fn optional_hex(bytes: &[Option<u8>]) -> String {
    bytes
        .iter()
        .map(|b| match b {
            Some(v) => format!("{v:02X}"),
            None => "--".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn ascii_gutter(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}
