//! Nudge the track-2 coarse tune five times, reading back the value each step.
//!
//! Runs end-to-end against [`LoopbackTransport`]. Mirror of the original
//! `zen_core_tools.py` demo.
//!
//! Run: `cargo run --example zen_core_coarse_tune`

use mc_programmer::{LoopbackTransport, MCClient, ModelId};

/// Center track 2 coarse tune, then increment and print it five times using loopback.
fn main() {
    let client = MCClient::new(
        LoopbackTransport::new(ModelId::Mc707, 0xFF),
        ModelId::Mc707.value(),
    );

    // Initialise the coarse-tune parameter to center (64).
    client.write_coarse_tune(1, None, 64).unwrap();
    println!(
        "starting coarse tune = {}",
        client.read_coarse_tune(1, None).unwrap()
    );

    for i in 0..5 {
        let v = client.nudge_coarse_tune(1, None, 1).unwrap();
        println!("step {}: coarse tune now = {}", i + 1, v);
    }
}
