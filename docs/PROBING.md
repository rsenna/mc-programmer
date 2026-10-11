# Probing the MC address space

How to find an MC block that has no published address — the scene block, the
scene chain, scene part parameters, drum-kit tone parameters. The method is
differential: dump, change one thing on the device, dump again, diff.

The tool is `mc-probe`, built from `rust_lib`:

```bash
cd rust_lib
cargo run --features midir --bin mc-probe -- help
```

It only ever *reads* from the device. The risk in a probing session is the
manual front-panel edits you make between dumps, which is why step 1 saves
the project from the device itself.

## How the MC answers an RQ1

Three behaviours, all measured on an MC-101 and all of which will corrupt an
address map if ignored. `probe::walk` handles them; anything else reading
replies has to as well.

**1. The reply address is linear, and masked.** An RQ1 request carries the
address as four 7-bit packed bytes. The DT1 reply carries the *linear* value
— and every byte of it is masked to `0x7F` by the MIDI pipe, so up to 8
linear addresses arrive as the same four bytes. Packed `0x40000000`,
`0x40000100` and `0x40020000` have linear forms `0x08000000`, `0x08000080`
and `0x08008000`, and all three reply `08 00 00 00`. The request is what
disambiguates them. (A Roland Clan forum report of request `32 10 00 00`
answering `06 44 00 00` is the same rule: `0x06440000` is that address's
linear form. The poster called it the device encoding the address
"incorrectly".)

**2. The device answers with the next mapped run, at or after the address
asked for.** Ask about packed `0x40000020` and it may answer for
`0x40000026`, or for `0x4000002E` — past the end of the span. The count is a
hint, not a bound. So the payload does *not* necessarily begin at the
requested address.

**3. A short reply is gap-compressed.** When a span contains unmapped bytes,
the device returns only the mapped ones, concatenated, with the holes
squeezed out. Byte *i* of a short payload is therefore not byte *i* of the
span. A 128-byte request returned 60 apparently contiguous bytes across a
span where 16-byte requests showed an 8-byte hole.

Consequence: a payload may be laid down contiguously only when the reply is
exactly as long as the request **and ends within it**. Neither condition alone
is enough, and even together they are necessary rather than sufficient — see
"Two things the protocol cannot confirm" below. `probe::walk` splits any shorter reply in
half and re-probes, down to single bytes, so a byte is placed only where the
exchange supports it. The cost is extra requests near region edges
(`Snapshot::subdivisions` counts them); the benefit is that sweeps at chunk
`0x10`, `0x20` and `0x80` of the same range now agree byte for byte, where
before they disagreed by tens of bytes.

### What the probe can and cannot confirm

**The address: confirmed, at the cost of one extra exchange.** If the device
relocated forward by exactly `0x80`, the relocated address and the requested
one produce the *same* masked reply, and one exchange cannot tell them apart.
Taking the lower candidate would write a payload `0x80` bytes from where it
belongs and report it as certain. So whenever the resolution is not forced,
the sweep asks again from one byte above: that drops the requested address out
of the candidate set, and the two cases give different answers.
`Snapshot::confirmations` counts those follow-ups. When a follow-up settles
nothing, the payload is **discarded** and counted in
`unprovable_placements` — a non-zero value means the sweep has holes.

**Contiguity: not confirmable.** A gap-compressed reply can match the
requested length exactly — asked for 8 bytes over a span whose middle 4 are
unmapped, a device that keeps reading past the span returns 4 from inside and
4 from beyond. Nothing in (address, length, count) separates that from a clean
8-byte run, and per-byte confirmation is unavailable because the MC largely
ignores 1-byte requests. There is no counter for this, because it applies to
every multi-byte placement and so carries no signal. Cross-check a run that
matters by re-sweeping at a different `chunk`: requests then start at
different addresses and compress differently.

`Snapshot` reports `relocated_replies`, `subdivisions`, `confirmations`,
`unplaceable_replies`, `unprovable_placements` and `truncated_placements`.

## Address arithmetic, and why ranges look odd

A Roland address carries 28 useful bits in four 7-bit bytes. No byte may have
its MSB set, so `0x00000080` is not an address at all: the byte after
`0x0000007F` is `0x00000100`. `mc-probe` steps through *linear* addresses and
converts, so a range is always interpreted in the device's own byte order and
every address it reports is valid.

The practical consequence when picking ranges: each address byte spans `0x80`
linear bytes, not `0x100`. So

| packed span | linear bytes |
|---|---|
| `0x...0000` → `0x...0100` | 128 |
| `0x...0000` → `0x...8000` ✗ | invalid, `0x80` has its MSB set |
| `0x00000000` → `0x01000000` | 2 MiB (`0x200000`) |

A miss costs one `--timeout` (default 0.1 s), so a wide gapless sweep is slow.
Survey coarsely first, then dump finely.

## Procedure

### 1. Save the project from the device

**Use the MC-101's own menu to save the project to its SD card.** Probing
only ever reads; what is at risk is the front-panel edits you make between
sweeps, and the device's own save is the only reliable way back.

`mc-probe` deliberately has no `backup` subcommand. `MCClient::read_chunked`
rejects any reply shorter than the request, and short replies are routine
here — gap compression and forward relocation both produce them — so
`Project::dump_from` cannot complete against real hardware. Verified on an
MC-101:

```text
mc-probe: chunk at 0x10000800 has 0 bytes; expected 32
```

That strictness is not simply a bug to revert: before it, short chunks were
zero-filled, and `Project::load_into` wrote those zeros back to the device on
restore. Both behaviours are wrong, and fixing it properly means representing
absent bytes explicitly in `Project` rather than choosing between corrupting a
restore and failing a dump.

### 2. Survey which regions answer at all

One 16-byte poke at each top-byte boundary — 128 requests, ~13 s worst case:

```bash
cargo run --features midir --bin mc-probe -- dump \
  --start 00000000 --end 7F7F7F7F --chunk 0x10 --step 0x200000 \
  -o survey-top.json
```

Then narrow into each region that answered, one byte down (`--step 0x4000`
walks the second address byte within one top byte).

### 3. Dump the candidate range twice, around one change

Pick the smallest range that plausibly contains the field, then:

```bash
# before
cargo run --features midir --bin mc-probe -- dump \
  --start 01000000 --end 01010000 --label "scene 1 selected" -o before.json

# ... now change exactly ONE thing on the device by hand ...

cargo run --features midir --bin mc-probe -- dump \
  --start 01000000 --end 01010000 --label "scene 2 selected" -o after.json
```

**Exactly one thing.** Two changes in one round produce two diff runs and no
way to tell which is which. If the device auto-saves or touches a timestamp,
you will see an extra run every time — dump twice with *no* change first to
learn what moves on its own.

### 4. Diff

```bash
cargo run --features midir --bin mc-probe -- diff before.json after.json
```

Changed bytes are grouped into contiguous runs, which is how a real field
shows up:

```text
2 changed run(s), 3 byte(s) total

01000012  +2 byte(s)
  before  00 01
  after   00 02

01000040  +1 byte(s)
  before  3C
  after   50
```

`--csv` gives one row per byte for spreadsheet work.

### 5. Confirm before believing it

A diff tells you where a byte moved, not what the field means. Before adding
an address to `addresses.rs`:

1. Change the same thing to a third value and check the same byte tracks it.
2. Change a *neighbouring* thing and check it lands elsewhere.
3. Write the byte back with `MCClient::write` and confirm the device's own
   display agrees.
4. Round-trip it: write, read back, compare byte-exact.

Only then is the capability real. Record the evidence next to the constant.

## Library API

The CLI is a thin wrapper over `mc_programmer::probe`:

```rust
use mc_programmer::{probe, probe::ProbeOptions, MCClient, MidirTransport};

let client = MCClient::detect(MidirTransport::connect("MC-101")?)?;
let opts = ProbeOptions::gapless(0x10);

let before = probe::walk(&client, 0x01000000, 0x01010000, &opts)?.labelled("scene 1");
// ... change one thing ...
let after = probe::walk(&client, 0x01000000, 0x01010000, &opts)?.labelled("scene 2");

for run in probe::diff_runs(&probe::diff(&before, &after)) {
    println!("{:08X} +{}", run.address, run.len());
}
```

A [`Snapshot`] records only the bytes the device answered for. An address that
is absent is *unmapped*, which is not the same as zero — a region that starts
or stops answering is itself a finding, and `diff` reports it with `--` on the
missing side.
