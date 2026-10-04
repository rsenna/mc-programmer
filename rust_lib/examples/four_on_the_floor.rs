//! Write a 4-on-the-floor kick to clip 1 of track 1, then read it back.
//!
//! Runs end-to-end against [`LoopbackTransport`] — an in-memory fake MC — so no
//! MIDI hardware is required. Swap in a real transport (e.g. the optional
//! `midir` backend) to drive hardware.
//!
//! Run: `cargo run --example four_on_the_floor`

use mc_programmer::{LoopbackTransport, MCClient, ModelId, NoteComponent, NoteStep};

/// Write a kick pattern to loopback memory and print the steps containing notes.
fn main() {
    let client = MCClient::new(
        LoopbackTransport::new(ModelId::Mc707, 0xff),
        ModelId::Mc707.value(),
    );

    // 1) Wipe the first 16 steps of clip 1 / track 1.
    for s in 0..16 {
        client
            .write_note_step(0, 0, s, &NoteStep::default())
            .unwrap();
    }

    // 2) Place a kick (C1 = MIDI note 36) on every downbeat.
    for s in [0, 4, 8, 12] {
        let mut step = NoteStep::default();
        step.components[0] = NoteComponent {
            note: 36,
            velocity: 120,
            ..NoteComponent::empty()
        };
        client.write_note_step(0, 0, s, &step).unwrap();
    }

    // 3) Read the steps back to verify.
    let mut hits = Vec::new();
    for s in 0..16 {
        let step = client.read_note_step(0, 0, s).unwrap();
        if !step.active_components().is_empty() {
            hits.push(s);
        }
    }
    println!("Steps with notes: {hits:?}");
    let step4 = client.read_note_step(0, 0, 4).unwrap();
    println!("Step 4 lane 0 note: {}", step4.components[0].note);
}
