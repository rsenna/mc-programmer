# mc-programmer (Rust)

Rust implementation of the MC-101, MC-707, and MV-1 SysEx programming API.
It mirrors the Python and Swift libraries and works offline with an in-memory
`LoopbackTransport`.
The crate declares Rust 1.71 as its minimum supported version; see `Cargo.toml`.

## Build and verify

```bash
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo run --example four_on_the_floor
```

The default build has no hardware MIDI dependency. Enable the optional
cross-platform `midir` backend with `--features midir` when connecting to a
physical device.

## Quick start

```rust
use mc_programmer::{LoopbackTransport, MCClient, ModelId, Pitch};

let client = MCClient::new(
    LoopbackTransport::new(ModelId::Mc707, 0xFF),
    ModelId::Mc707.value(),
);
client.set_pattern(
    1,
    1,
    &[(Pitch::from("C1"), vec![1, 5, 9, 13])],
    100,
    true,
)?;
# Ok::<(), mc_programmer::Error>(())
```

For hardware, construct `MidirTransport::connect("MC-707")?` and pass it to
`MCClient::new` instead. The optional feature provides port listing and
case-insensitive name matching; connecting requires a MIDI device and backend
permissions.

## API layers

- Musician-facing helpers use 1-based track, clip, and step numbers; note
  inputs accept names (Roland octave convention: C3 = MIDI 60) or MIDI values.
- Low-level client methods use 0-based track/clip/step indexes and expose
  RQ1/DT1, chunked reads/writes, note-step records, and ZEN-Core parameters.
- `TonePreset` and `Project` support JSON serialization and validated restore.
- `AddressMap`, the sequencer codec, and protocol builders are also public.

Examples in `examples/` run against `LoopbackTransport`. See the repository
root README for the shared architecture and `reference/README.md` for the
ZEN-Core table source and regeneration instructions.
