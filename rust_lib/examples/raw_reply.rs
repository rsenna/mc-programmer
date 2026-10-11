//! Print the raw Sys-Ex reply to one RQ1, byte for byte.
//!
//! Diagnostic for reply framing: `parse_dt1_payload` assumes the device's DT1
//! reply has the same 8-byte header as the RQ1 we build, so the address sits
//! at bytes 8..11. If that assumption is wrong the parsed address is garbage
//! in a way that still looks like a plausible address.
//!
//! cargo run --features midir --example raw_reply -- 40000000 10

use mc_programmer::{build_rq1, MidirTransport, Transport};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let address = u32::from_str_radix(args.first().map(String::as_str).unwrap_or("40000000"), 16).expect("hex address");
    let count: u32 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(0x10);

    let transport = MidirTransport::connect("MC-101").expect("connect to MC-101");
    let request = build_rq1(0x5e, address, count).expect("build RQ1");

    println!("request  ({} bytes):", request.len());
    print_frame(&request);

    let reply = transport.transfer(&request, 0.3).expect("transfer");
    if reply.is_empty() {
        println!("\nno reply");
        return;
    }
    println!("\nreply    ({} bytes):", reply.len());
    print_frame(&reply);

    println!("\nindex-by-index, so the address field can be located by eye:");
    for (i, b) in reply.iter().enumerate().take(16) {
        println!("  [{i:2}] 0x{b:02X}");
    }
}

fn print_frame(frame: &[u8]) {
    for chunk in frame.chunks(16) {
        println!(
            "  {}",
            chunk
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
}
