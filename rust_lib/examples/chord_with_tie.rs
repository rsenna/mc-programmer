//! Build a polyphonic chord step and a tied note crossing two steps, encode
//! them, and pretty-print the resulting on-device step records.
//!
//! Runs offline — no MIDI hardware required.
//!
//! Run: `cargo run --example chord_with_tie`

use mc_programmer::{encode_note_step, NoteComponent, NoteStep, SubStep};

/// Print a labelled byte buffer in rows of sixteen hexadecimal bytes.
fn hexdump(label: &str, raw: &[u8]) {
    println!("{label} ({} bytes):", raw.len());
    for (i, row) in raw.chunks(16).enumerate() {
        let hex = row
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  {:03X}  {hex}", i * 16);
    }
}

/// Print encoded records for a polyphonic chord and a note tied across two steps.
fn main() {
    // --- 1) A C-major chord on lanes 0..3 of one step --------------------
    let mut chord = NoteStep::default();
    for (lane, midi) in [60u8, 64, 67, 72].iter().enumerate() {
        chord.components[lane] = NoteComponent {
            note: *midi,
            velocity: 100,
            start_cents: 0,
            end_cents: 80,
            ..NoteComponent::empty()
        };
    }
    chord.compact(); // match how the firmware packs lanes
    chord.substep = SubStep::None;

    println!("== Polyphonic chord step ==");
    for c in chord.active_components() {
        println!(
            "  lane note={} vel={} end={}c",
            c.note, c.velocity, c.end_cents
        );
    }
    hexdump("encoded chord", &encode_note_step(&chord));

    // --- 2) A tied note that spans two steps -----------------------------
    let mut step_a = NoteStep::default();
    step_a.components[0] = NoteComponent {
        note: 48,
        velocity: 100,
        start_cents: -10,
        end_cents: 100,
        ..NoteComponent::empty()
    };

    let mut step_b = NoteStep::default();
    step_b.components[0] = NoteComponent {
        note: 48,
        velocity: 100,
        start_cents: 0,
        end_cents: 82,
        ..NoteComponent::empty()
    };

    println!("\n== Tied note across step A → step B ==");
    println!("  step A: note=48 start=-10c end=100c (tie)");
    println!("  step B: note=48 start=  0c end= 82c");
    hexdump("encoded step A", &encode_note_step(&step_a));
    hexdump("encoded step B", &encode_note_step(&step_b));
}
