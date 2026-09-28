#![cfg(feature = "midir")]

use mc_programmer::{MidiPortNames, MidirTransport, Result, Transport};

fn assert_transport<T: Transport>() {}
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn midir_transport_exposes_hardware_api() {
    assert_transport::<MidirTransport>();
    assert_send_sync::<MidirTransport>();

    let _list_ports: fn() -> Result<MidiPortNames> = MidirTransport::list_ports;
    let _connect: fn(&str) -> Result<MidirTransport> = MidirTransport::connect;
}
