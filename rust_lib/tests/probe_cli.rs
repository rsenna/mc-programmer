#![cfg(feature = "midir")]

use mc_programmer::probe::{self, ProbeOptions};
use mc_programmer::{LoopbackTransport, MCClient, ModelId};
use std::process::Command;

#[test]
fn diff_csv_keeps_stdout_valid_and_warns_about_uncertain_snapshots() {
    let transport = LoopbackTransport::new(ModelId::Mc101, 0xff).with_hardware_reply_addresses();
    let client = MCClient::new(transport.clone(), ModelId::Mc101.value());
    transport.preload(0x20000000, &[0x10, 0x11]);
    let before = probe::walk(&client, 0x20000000, 0x20000002, &ProbeOptions::gapless(2)).unwrap();

    let mut after = before.clone();
    after.bytes.insert(0x20000000, 0x12);
    after.unconfirmed_placements = 1;
    after.unprovable_placements = 2;
    after.unplaceable_replies = 3;
    after.chunks_failed = 4;

    let stem = format!("mc-probe-cli-{}", std::process::id());
    let before_path = std::env::temp_dir().join(format!("{stem}-before.json"));
    let after_path = std::env::temp_dir().join(format!("{stem}-after.json"));
    before.save(before_path.to_str().unwrap()).unwrap();
    after.save(after_path.to_str().unwrap()).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mc-probe"))
        .arg("diff")
        .arg("--csv")
        .arg(&before_path)
        .arg(&after_path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stderr}");
    assert_eq!(stdout.lines().next(), Some("address,before,after"));
    assert_eq!(stdout.lines().count(), 2, "{stdout}");
    assert!(stderr.contains("not confirmed"), "{stderr}");
    assert!(stderr.contains("could not be resolved"), "{stderr}");
    assert!(stderr.contains("unplaceable"), "{stderr}");
    assert!(stderr.contains("chunk(s) failed"), "{stderr}");

    std::fs::remove_file(before_path).unwrap();
    std::fs::remove_file(after_path).unwrap();
}
