//! `mc-probe` — command-line address-space explorer for MC hardware.
//!
//! Locates undocumented blocks (scene block, scene chain, scene part
//! parameters, drum-kit tone parameters) by dumping a range, changing one
//! thing on the device, dumping again, and diffing.
//!
//! ```text
//! mc-probe ports
//! mc-probe backup   device-backup.json
//! mc-probe dump     --start 01000000 --end 01010000 --label "scene 1" -o before.json
//! mc-probe dump     --start 01000000 --end 01010000 --label "scene 2" -o after.json
//! mc-probe diff     before.json after.json
//! mc-probe restore  device-backup.json
//! ```
//!
//! Back up before probing. Probing only ever *reads*, but a session on real
//! hardware involves changing things by hand, and `restore` puts them back.

use std::process::ExitCode;
use std::time::Instant;

use mc_programmer::probe::{self, ProbeOptions, Snapshot};
use mc_programmer::{MCClient, MidirTransport, Project};

const DEFAULT_DEVICE: &str = "MC-101";

/// Flags that take no value. Everything else beginning with `-` consumes the
/// next argument, so a value that looks like a flag (`--timeout -1`) is
/// reported rather than silently ignored.
const BOOLEAN_FLAGS: &[&str] = &["--csv", "--force", "-h", "--help"];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("mc-probe: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let Some(command) = args.first() else {
        print_usage();
        return Ok(());
    };
    let rest = &args[1..];

    match command.as_str() {
        "ports" => cmd_ports(),
        "dump" => cmd_dump(rest),
        "diff" => cmd_diff(rest),
        "scene" => cmd_scene(rest),
        "clip" => cmd_clip(rest),
        "backup" => cmd_backup(rest),
        "restore" => cmd_restore(rest),
        "help" | "-h" | "--help" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown command {other:?}. Try `mc-probe help`.")),
    }
}

fn print_usage() {
    println!(
        "\
mc-probe — MC address-space explorer

  ports                                 list MIDI ports
  dump   --start HEX --end HEX -o FILE  dump a packed address range
         [--chunk N] [--step N] [--timeout SECONDS] [--label TEXT]
         [--device NAME] [--csv-out FILE]
  diff   BEFORE.json AFTER.json         report every changed byte
         [--csv]
  scene N [--channel C]                 recall scene N via Program Change
  clip   TRACK N [--channel C]          recall clip N on TRACK via Program Change
  backup FILE.json [--device NAME]      dump the whole device project
  restore FILE.json [--device NAME]     write a project back [--force]

Addresses are packed Roland hex, with or without 0x. Every byte is 7-bit,
so 0x...0080 is not a valid address and 0x...0100 is the byte after
0x...007F; ranges are interpreted in that linear order."
    );
}

// --- commands ----------------------------------------------------------------

fn cmd_ports() -> Result<(), String> {
    let ports = MidirTransport::list_ports().map_err(|e| e.to_string())?;
    println!("inputs:");
    print_port_list(&ports.inputs);
    println!("outputs:");
    print_port_list(&ports.outputs);
    Ok(())
}

fn print_port_list(names: &[String]) {
    if names.is_empty() {
        println!("  (none)");
    }
    for name in names {
        println!("  {name}");
    }
}

fn cmd_dump(args: &[String]) -> Result<(), String> {
    let opts = Args::parse(args)?;
    let start = opts.required_address("--start")?;
    let end = opts.required_address("--end")?;
    let out = opts.required("-o")?;

    let probe_opts = ProbeOptions {
        chunk: opts.number("--chunk")?.unwrap_or(probe::DEFAULT_CHUNK),
        step: opts.number("--step")?.unwrap_or_else(|| {
            opts.number("--chunk")
                .ok()
                .flatten()
                .unwrap_or(probe::DEFAULT_CHUNK)
        }),
        timeout: opts.float("--timeout")?.unwrap_or(0.1),
    };

    let client = connect(&opts)?;
    eprintln!(
        "probing {start:08X}..{end:08X}, chunk 0x{:X}, step 0x{:X}, timeout {}s",
        probe_opts.chunk, probe_opts.step, probe_opts.timeout
    );

    let began = Instant::now();
    let mut answered = 0usize;
    let mut probed = 0usize;
    let snapshot = probe::walk_with_progress(&client, start, end, &probe_opts, &mut |address, len| {
        probed += 1;
        if len > 0 {
            answered += 1;
        }
        // Progress on one rewritten line — a wide sweep is thousands of chunks.
        eprint!("\r  {address:08X}  {answered} answered / {probed} probed");
    })
    .map_err(|e| e.to_string())?;
    eprintln!();

    let snapshot = match opts.value("--label") {
        Some(label) => snapshot.labelled(label),
        None => snapshot,
    };

    snapshot.save(&out).map_err(|e| e.to_string())?;
    eprintln!(
        "{} byte(s) from {}/{} chunk(s) in {:.1}s -> {out}",
        snapshot.len(),
        snapshot.chunks_answered,
        snapshot.chunks_probed,
        began.elapsed().as_secs_f64()
    );
    if snapshot.subdivisions > 0 || snapshot.truncated_placements > 0 {
        eprintln!(
            "  {} request(s) split because a reply could not be shown to cover it{}",
            snapshot.subdivisions,
            if snapshot.truncated_placements > 0 {
                format!(
                    ", {} payload(s) truncated at the top of the address space",
                    snapshot.truncated_placements
                )
            } else {
                String::new()
            }
        );
    }
    if snapshot.assumed_contiguous_placements > 0 || snapshot.unprovable_placements > 0 {
        eprintln!(
            "note: {} run(s) assumed contiguous and {} address(es) unproven — the protocol \
             cannot confirm either. Re-sweep with a different --chunk to cross-check.",
            snapshot.assumed_contiguous_placements, snapshot.unprovable_placements
        );
    }
    if snapshot.unplaceable_replies > 0 {
        eprintln!(
            "warning: {} reply/replies could not be placed and were DISCARDED — this sweep has holes. \
             Narrow --start/--end or lower --chunk and re-run.",
            snapshot.unplaceable_replies
        );
    }
    if snapshot.chunks_failed > 0 {
        eprintln!(
            "warning: {} chunk(s) replied with something unparseable and were skipped",
            snapshot.chunks_failed
        );
        for (address, error) in snapshot.failures.iter().take(5) {
            eprintln!("  {address:08X}  {error}");
        }
        if snapshot.failures.len() > 5 {
            eprintln!("  ... and {} more", snapshot.failures.len() - 5);
        }
    }

    if let Some(csv_path) = opts.value("--csv-out") {
        std::fs::write(&csv_path, snapshot.to_csv()).map_err(|e| e.to_string())?;
        eprintln!("csv -> {csv_path}");
    }
    if !snapshot.is_empty() {
        print!("{}", snapshot.hex_dump());
    }
    Ok(())
}

fn cmd_diff(args: &[String]) -> Result<(), String> {
    let opts = Args::parse(args)?;
    let paths = opts.positionals();
    let [before_path, after_path] = paths.as_slice() else {
        return Err("diff needs exactly two snapshot files".to_string());
    };

    let before = Snapshot::load(before_path).map_err(|e| e.to_string())?;
    let after = Snapshot::load(after_path).map_err(|e| e.to_string())?;

    describe(before_path, &before);
    describe(after_path, &after);
    println!();

    let diffs = probe::diff(&before, &after);
    if opts.flag("--csv") {
        println!("address,before,after");
        for d in &diffs {
            println!(
                "{:08X},{},{}",
                d.address,
                hex_or_dash(d.before),
                hex_or_dash(d.after)
            );
        }
        return Ok(());
    }
    print!("{}", probe::format_diff(&probe::diff_runs(&diffs)));
    Ok(())
}

fn describe(path: &str, snapshot: &Snapshot) {
    if snapshot.unplaceable_replies > 0 {
        println!(
            "{path}: warning — {} reply/replies were discarded as unplaceable, so this \
             snapshot has holes unrelated to the device's state",
            snapshot.unplaceable_replies
        );
    }
    if snapshot.chunks_failed > 0 {
        println!(
            "{path}: warning — {} chunk(s) failed during this sweep, so some \
             addresses are missing for reasons unrelated to the device's state",
            snapshot.chunks_failed
        );
    }
    println!(
        "{path}: {} byte(s), {}{}",
        snapshot.len(),
        snapshot.model,
        snapshot
            .label
            .as_ref()
            .map(|l| format!(", {l:?}"))
            .unwrap_or_default()
    );
}

fn hex_or_dash(byte: Option<u8>) -> String {
    byte.map(|b| format!("{b:02X}"))
        .unwrap_or_else(|| "--".to_string())
}

/// Recall a scene by Program Change on the control channel.
///
/// The MC-101 MIDI implementation chart documents scene change as PC 0–127 on
/// the control channel, and the Ver.1.7x system settings expose
/// `Control Channel` plus `Control Rx`. If this appears to do nothing, check
/// that `Control Rx` is ON and that `--channel` matches `Control Channel`.
fn cmd_scene(args: &[String]) -> Result<(), String> {
    let opts = Args::parse(args)?;
    let scene = opts
        .positionals()
        .first()
        .ok_or("scene needs a scene number (1-128)")?
        .parse::<u32>()
        .map_err(|_| "scene number must be 1-128".to_string())?;
    let channel = opts.number("--channel")?.unwrap_or(16);

    let client = connect(&opts)?;
    client
        .select_scene(scene, channel)
        .map_err(|e| e.to_string())?;
    eprintln!("sent scene {scene} recall as Program Change on channel {channel}");
    Ok(())
}

/// Recall a clip on one track by Program Change on that track's channel.
fn cmd_clip(args: &[String]) -> Result<(), String> {
    let opts = Args::parse(args)?;
    let positionals = opts.positionals();
    let [track, clip] = positionals.as_slice() else {
        return Err("clip needs a track and a clip number".to_string());
    };
    let track = track
        .parse::<u32>()
        .map_err(|_| "track must be 1-4".to_string())?;
    let clip = clip
        .parse::<u32>()
        .map_err(|_| "clip must be 1-16".to_string())?;
    let channel = opts.number("--channel")?;

    let client = connect(&opts)?;
    client
        .select_clip(track, clip, channel)
        .map_err(|e| e.to_string())?;
    eprintln!("sent clip {clip} recall for track {track}");
    Ok(())
}

fn cmd_backup(args: &[String]) -> Result<(), String> {
    let opts = Args::parse(args)?;
    let path = opts
        .positionals()
        .first()
        .cloned()
        .ok_or("backup needs an output file")?;

    let client = connect(&opts)?;
    eprintln!("dumping the whole device project — this takes a few minutes");
    let project = Project::dump_from(&client, Some("mc-probe backup".to_string()), true).map_err(|e| e.to_string())?;
    project.save(&path).map_err(|e| e.to_string())?;
    eprintln!("backup -> {path}");
    Ok(())
}

fn cmd_restore(args: &[String]) -> Result<(), String> {
    let opts = Args::parse(args)?;
    let path = opts
        .positionals()
        .first()
        .cloned()
        .ok_or("restore needs an input file")?;

    let project = Project::load(&path).map_err(|e| e.to_string())?;
    let client = connect(&opts)?;
    eprintln!("writing {path} back to the device");
    project
        .load_into(&client, opts.flag("--force"))
        .map_err(|e| e.to_string())?;
    eprintln!("restored");
    Ok(())
}

fn connect(opts: &Args) -> Result<MCClient<MidirTransport>, String> {
    let device = opts.value("--device").unwrap_or(DEFAULT_DEVICE.to_string());
    let transport = MidirTransport::connect(&device)
        .map_err(|e| format!("{e}\nRun `mc-probe ports` to see what the MIDI backend can see."))?;
    let client = MCClient::detect(transport)
        .map_err(|e| format!("{e}\nIs the device powered on and not in a menu that blocks Sys-Ex?"))?;
    eprintln!(
        "connected to {} (model id 0x{:02X}, {} tracks)",
        mc_programmer::model_name(client.model_id),
        client.model_id,
        client.num_tracks()
    );
    Ok(client)
}

// --- minimal argument parsing ------------------------------------------------

/// `--key value` pairs, `--flag`s, and bare positionals. Hand-rolled to keep
/// the crate free of a CLI dependency.
struct Args {
    pairs: Vec<(String, String)>,
    flags: Vec<String>,
    positionals: Vec<String>,
}

impl Args {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut parsed = Args {
            pairs: Vec::new(),
            flags: Vec::new(),
            positionals: Vec::new(),
        };
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if !arg.starts_with('-') {
                parsed.positionals.push(arg.clone());
                i += 1;
                continue;
            }
            // Flags that take no value must not swallow the positional that
            // follows them: `diff --csv before.json after.json` has to keep
            // both paths, and `restore --force backup.json` the one path.
            if BOOLEAN_FLAGS.contains(&arg.as_str()) {
                parsed.flags.push(arg.clone());
                i += 1;
                continue;
            }
            match args.get(i + 1) {
                Some(next) => {
                    parsed.pairs.push((arg.clone(), next.clone()));
                    i += 2;
                }
                None => return Err(format!("{arg} needs a value")),
            }
        }
        Ok(parsed)
    }

    fn value(&self, key: &str) -> Option<String> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    fn flag(&self, key: &str) -> bool {
        self.flags.iter().any(|f| f == key)
    }

    fn positionals(&self) -> Vec<String> {
        self.positionals.clone()
    }

    fn required(&self, key: &str) -> Result<String, String> {
        self.value(key).ok_or_else(|| format!("missing {key}"))
    }

    fn required_address(&self, key: &str) -> Result<u32, String> {
        let raw = self.required(key)?;
        let text = raw.trim_start_matches("0x").trim_start_matches("0X");
        let address = u32::from_str_radix(text, 16).map_err(|_| format!("{key} {raw:?} is not a hex address"))?;
        if address & 0x8080_8080 != 0 {
            return Err(format!(
                "{key} 0x{address:08X} is not a valid Roland address — every byte must be 7-bit (0x00..0x7F)"
            ));
        }
        Ok(address)
    }

    fn number(&self, key: &str) -> Result<Option<u32>, String> {
        match self.value(key) {
            None => Ok(None),
            Some(raw) => {
                let text = raw.trim_start_matches("0x").trim_start_matches("0X");
                let radix = if text == raw {
                    10
                } else {
                    16
                };
                u32::from_str_radix(text, radix)
                    .map(Some)
                    .map_err(|_| format!("{key} {raw:?} is not a number"))
            }
        }
    }

    fn float(&self, key: &str) -> Result<Option<f64>, String> {
        match self.value(key) {
            None => Ok(None),
            Some(raw) => raw
                .parse()
                .map(Some)
                .map_err(|_| format!("{key} {raw:?} is not a number")),
        }
    }
}
