//! The client's network core outside the game: a full join (cookie exchange, RakNet connection, auth key,
//! PlayerConnect, InitGame) against a real server, from a Linux process. This is the path native sa-game will
//! use when it hosts the client.
//!
//! Needs a running server, so it is ignored unless `SAMP_TEST_SERVER=HOST:PORT` is set. The auth table is read
//! from `SAMP_AUTH_TABLE` (default `data/auth_table.txt`, see `tools/extract_auth_table.py`).
//!
//! ```text
//! SAMP_TEST_SERVER=127.0.0.1:7812 SAMP_AUTH_TABLE=data/auth_table.txt \
//!     cargo test -p samp-client --target i686-unknown-linux-gnu --test native_handshake -- --nocapture
//! ```

use std::time::{Duration, Instant};

use samp_client::netclient::{Event, NetClient, Phase, resolve};
use samp_proto::auth::AuthTable;

#[test]
fn joins_a_server_from_linux() {
    let Ok(target) = std::env::var("SAMP_TEST_SERVER") else {
        eprintln!("SAMP_TEST_SERVER not set; skipped");
        return;
    };
    let (host, port) = target.rsplit_once(':').expect("SAMP_TEST_SERVER=HOST:PORT");
    let addr = resolve(host, port.parse().expect("port")).expect("server address");
    let table = std::env::var("SAMP_AUTH_TABLE").unwrap_or_else(|_| "data/auth_table.txt".into());
    let auth = AuthTable::parse(&std::fs::read_to_string(&table).expect("auth table")).expect("auth table format");
    let mut net = NetClient::new(addr, "NativeLinux", auth).expect("session");
    net.connect().expect("connect");

    let start = Instant::now();
    let mut phases = vec![net.phase()];
    let mut init = None;
    while start.elapsed() < Duration::from_secs(15) {
        for e in net.tick(Instant::now()) {
            match e {
                Event::InitGame(g) => init = Some(g),
                Event::Rejected(r) => panic!("connection rejected: {r}"),
                Event::Packet(p) => eprintln!("packet {p:?}"),
                _ => {}
            }
        }
        let p = net.phase();
        if phases.last() != Some(&p) {
            eprintln!("{:>5} ms  {p:?}", start.elapsed().as_millis());
            phases.push(p.clone());
        }
        if init.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let init = init.unwrap_or_else(|| panic!("no InitGame within 15 s; phases {phases:?}"));
    eprintln!("InitGame: player id {:?}, {:?}", net.player_id(), init);
    assert_eq!(net.phase(), Phase::Connected);
    assert!(net.player_id().is_some());
    for p in [Phase::Opening, Phase::Connecting, Phase::Joining] {
        assert!(phases.contains(&p), "phase {p:?} not seen: {phases:?}");
    }
    net.disconnect(Instant::now());
}
