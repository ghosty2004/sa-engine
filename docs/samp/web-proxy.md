# SAMP-WEB-NET: browser networking through `sa-udp-proxy`

Status: proxy, client transports and tests done (2026-11-12). User decision of 2026-11-12 (ROADMAP decisions,
item 3): the browser (wasm32) build of sa-game + samp-rs reaches **unmodified** SA:MP 0.3.7-R4 / open.mp
servers through a separate Rust proxy that the user runs, WebSocket (WebTransport later) ↔ UDP.

```
browser: sa-game + samp-rs (wasm32)                      user's machine / host          any SA:MP server
  RakNet ── sa_net::WebSocketTransport ──ws://──▶  sa-udp-proxy  ──UDP (one socket per WS)──▶  open.mp
```

## 1. Pieces

| piece | where | what |
|---|---|---|
| proxy | `crates/sa-udp-proxy` (bin `sa-udp-proxy`, lib for tests) | tokio + tokio-tungstenite, `#![forbid(unsafe_code)]`; one UDP socket per WebSocket connection |
| shim | `sa-udp-proxy shim` (`src/shim.rs`) | the reverse half for native programs: a local UDP port carried over WebSocket to the proxy (tests, debugging, even samp.dll under Wine) |
| client transports | `crates/sa-net` | `DatagramTransport` + `Connector` traits; `UdpTransport` (native), `WebSocketTransport` (wasm32, web-sys), `NativeWsTransport` (host, feature `native-ws`) |
| tests | `crates/sa-udp-proxy/tests/relay.rs`, `crates/sa-net` unit tests | §5 |
| e2e | `tools/e2e/samp/proxy-handshake.sh` | a real join of our open.mp copy through shim + proxy (§5.2) |

## 2. Wire protocol

1. The client opens a WebSocket to the proxy (any path).
2. Its **first message** is the text frame `{"connect":"host:port"}`.
3. The proxy resolves the host, checks the allowlist (§3) and answers with one text frame:
   * `{"connected":"ip:port"}`: from now on datagrams flow;
   * `{"error":"..."}`, then a close with code 1008 (policy).
4. After `connected`, **every binary frame is exactly one UDP payload**, in both directions, with no header.
   Text frames after `connected` close the connection.
5. Closing the WebSocket closes the UDP socket. A new connection attempt = a new WebSocket = a new UDP source
   port at the server (exactly what RakNet's `Connect` does natively).

The proxy's UDP socket is `connect()`ed to the target, so only the target's datagrams come back. Datagrams the
client sends before `connected` are queued by the client transports (`sa_net::PENDING_LIMIT` = 64).

## 3. Running the proxy

```
cargo build --release -p sa-udp-proxy
target/release/sa-udp-proxy 127.0.0.1:7777                 # ws://127.0.0.1:7780, only that server allowed
target/release/sa-udp-proxy --bind 0.0.0.0:7780 play.example.org:7777 --allow 10.0.0.5:7777
target/release/sa-udp-proxy --allow-any --origin https://game.example   # any server, only that web page
target/release/sa-udp-proxy shim --listen 127.0.0.2:7777 --proxy ws://127.0.0.1:7780 --target 127.0.0.1:7777
```

| option | default | meaning |
|---|---|---|
| `TARGET` (positional) | — | the server relayed to; the only allowed target unless widened |
| `--bind ADDR` | `127.0.0.1:7780` | WebSocket listen address |
| `--allow HOST:PORT` | — | another allowed target (repeatable). A request matches by name or by resolved address+port |
| `--allow-any` | off | every target (an open relay: only on trusted networks). Unspecified, broadcast and multicast addresses and port 0 are refused even then |
| `--origin ORIGIN` | any | allowed `Origin` headers (repeatable); a missing or other Origin gets HTTP 403. Stops arbitrary web pages from using a user's local proxy |
| `--max-datagram N` | 1500 | larger datagrams are dropped (RakNet/SA:MP MTU is 576; open.mp ≤ 1492) |
| `--max-pps N`, `--max-bps N` | 400, 524288 | token bucket per direction per connection, one second of burst; excess datagrams are dropped (UDP semantics). A SA:MP client sends ~30–100 datagrams/s |
| `--idle-timeout SECS` | 60 | no datagram either way → close (RakNet's own timeout is 10 s; samp pings every few seconds) |
| `--max-conns N`, `--max-per-ip N` | 256, 16 | open connections in total / per client IP; more are refused at accept |
| `-q` / `-v` | normal | quiet = errors and refusals; verbose = every datagram |

The handshake plus control frame must finish within 10 s. WebSocket frames are capped at 4× max(datagram, 4096)
bytes by tungstenite. Logs go to stderr: one line at start, one per connection (`relaying to ... from udp ...`),
one per close with datagram/byte counters and drops, one per refusal.

TLS: the proxy speaks plain `ws://`. Browsers allow `ws://127.0.0.1` / `ws://localhost` from `https://` pages
(localhost is a trustworthy origin); a proxy on another host behind an `https://` page needs `wss://`, i.e. a
TLS-terminating reverse proxy (Caddy, nginx) in front. Not built in, on purpose (no certificates in this crate).

## 4. Client transport (`sa-net`)

```rust
pub trait DatagramTransport {
    fn send(&mut self, datagram: &[u8]) -> io::Result<usize>; // never blocks; queued while Connecting
    fn recv(&mut self, buf: &mut [u8]) -> io::Result<usize>;   // non-blocking; ErrorKind::WouldBlock = empty
    fn state(&self) -> LinkState;                              // Connecting | Open | Closed(reason)
}
pub trait Connector { type Transport: DatagramTransport; fn connect(&self, server: &str) -> io::Result<Self::Transport>; }
```

The semantics are those of the connected, non-blocking `std::net::UdpSocket` samp-rs uses today, so the RakNet
update loop is unchanged: `recv` until `WouldBlock`, `send` per outgoing datagram.

* `UdpTransport` / `UdpConnector` (not wasm): bind `0.0.0.0:0`, connect, non-blocking; ICMP unreachable errors
  are skipped like samp-rs does.
* `WebSocketTransport` / `WebSocketConnector { proxy_url }` (wasm32 only): `web_sys::WebSocket` with
  `binaryType = arraybuffer`; `onopen` sends the control frame, `onmessage` handles `connected`/`error` and
  queues datagrams (≤ `INBOX_LIMIT` 4096), `onclose`/`onerror` set `Closed`. Returns at once in `Connecting`;
  sends are queued until `connected` and then flushed in order. `!Send` (browser objects): it must live on the
  thread (main or worker) that created it. `buffered_amount()` is a back-pressure hint.
* `NativeWsTransport` / `NativeWsConnector` (host, feature `native-ws`): the same protocol with blocking
  tungstenite for the handshake and a non-blocking socket afterwards. For tests and native tools.

Builds: `cargo clippy -p sa-net -- -D warnings` (host, also with `--features native-ws`, and for
`i686-unknown-linux-gnu`) and `cargo clippy -p sa-net --target wasm32-unknown-unknown -- -D warnings` are clean.

## 5. Tests (2026-11-12)

### 5.1 Host integration tests (`cargo test -p sa-udp-proxy -p sa-net`)

All against a UDP echo server on 127.0.0.1 and a proxy on an ephemeral port; 8 tests, ~1 s, 5 consecutive runs
green.

| test | checks |
|---|---|
| `round_trips_datagrams` | 100 datagrams of 1/3/70/576/1500 bytes echo back byte-exact; a second WebSocket gets its own UDP source port (2 source addresses at the server) |
| `refuses_targets_off_the_allowlist` | another port → `{"error":"target not allowed..."}` + close; `localhost:PORT` matches the allowed `127.0.0.1:PORT` by resolution; bad control frames (`hello`, non-string, no port, `0.0.0.0`) and a binary first frame refused; `--allow-any` relays to other targets but still refuses `255.255.255.255`; refused connections sent nothing to the server |
| `enforces_size_rate_and_idle_limits` | 601 B with max 600 dropped, connection stays; 100 datagrams in a burst at 20/s → 15–22 pass; idle timeout closes |
| `enforces_connection_and_origin_limits` | `max_per_ip = 1` refuses the second connection; `--origin`: missing / wrong Origin refused, right one accepted |
| `sa_net_transport_round_trips_through_the_proxy` | `NativeWsTransport`: refusal surfaces as `ConnectionRefused`; `WouldBlock` when empty; 50 datagrams round-trip |
| `shim_carries_a_plain_udp_client` | a plain UDP socket → shim → proxy → echo |
| `sa-net`: `control_frames`, `round_trip_over_loopback` | frame building/parsing; `UdpTransport` round trip |

### 5.2 Real join through the proxy (`tools/e2e/samp/proxy-handshake.sh`)

samp-rs's `native_handshake` test (`samp-rs/crates/samp-client/tests/native_handshake.rs`; at the time of this
run it came from the SAMP-3 step-4 patch on a scratch copy of samp-rs), built for
`i686-unknown-linux-gnu`, joins our open.mp 1.6.7 copy (`re/samp/server`, 127.0.0.1:7812) via
`UDP → shim 127.0.0.2:7812 → ws://127.0.0.1:7781 → sa-udp-proxy → UDP`, under `target/locks/samp-native.lock`:

```
  999 ms  Connecting
packet Accepted
 1079 ms  Joining
 1129 ms  Connected
InitGame: player id Some(0), ... server_name: "open.mp server", world_time: 12, weather: 10, gravity: 0.008 ...
test joins_a_server_from_linux ... ok
--- proxy log
[       0.000] sa-udp-proxy: listening on ws://127.0.0.1:7781, allowed: 127.0.0.1:7812
[       0.549] #1 127.0.0.1:38576: relaying to 127.0.0.1:7812 from udp 127.0.0.1:51665
--- server log
[connection] incoming connection: 127.0.0.1:51665 id: 0
[join] NativeLinux has joined the server (0:127.0.0.1)
[part] NativeLinux has left the server (0:1)
```

The server sees the proxy's UDP socket (51665) as the client. Join time 1.13 s vs 1.06 s direct (SAMP-3; the
first second is samp's open-connection retry interval either way). Run:

```
# runs the test from samp-rs/ in this repository; auth table default: $SA_ROOT/re/samp/auth_table.txt
tools/e2e/samp/proxy-handshake.sh   # [SAMP_RS_DIR [AUTH_TABLE]]; env SAMP_PORT (7812), WS_PORT (7781), SA_ROOT
```

The shim listens on the **server's port number** on 127.0.0.2 because SA:MP's datagram cipher key is derived
from the server port (`CipherKey::from_port(server.port())`), see §6 item 3.

Not tested: the wasm `WebSocketTransport` in a real browser (it compiles and clippy-checks for wasm32; a browser
run needs the WEB-1 build). It is the same protocol `NativeWsTransport` passes against the proxy.

## 6. Integration notes: samp-rs's RakNet layer on wasm (for the samp-rs side; nothing changed there)

samp-rs's network core is `crates/samp-client/src/netclient.rs` (920 lines, `NetClient` + `Core`). What it
needs to run on wasm32 over `sa-net`:

1. **Socket → transport.** `Core.sock: Option<UdpSocket>` becomes `Option<Box<dyn DatagramTransport>>` (or a
   generic `C: Connector` on `NetClient`). The three uses map one to one:
   * `connect()`: `UdpSocket::bind((0.0.0.0, 0))` + `connect(server)` + `set_nonblocking(true)` →
     `connector.connect(&server_string)`; keep "a new transport per attempt" (RakNet's new source port).
   * `update()`: `sock.recv(&mut buf)` loop with `WouldBlock` → break and `ConnectionReset/Refused` → continue:
     unchanged (`DatagramTransport::recv` has the same error kinds; the UDP transport already skips ICMP errors).
     Additionally, `LinkState::Closed(reason)` while `Opening`/`Connecting` should end the attempt as
     `AttemptFailed` (the proxy refused the target or went away) instead of waiting for six 1 s retries.
   * `send_plain_dg()`: `sock.send(&wire)` → `transport.send(&wire)`; `WouldBlock` is never returned by the
     WebSocket transports (they queue).
2. **Server address.** `NetClient::new` takes a resolved `SocketAddrV4` (`resolve()` uses `ToSocketAddrs`,
   which does not exist in a browser). On wasm keep the server as `host:port` text, hand it to the connector
   (the proxy resolves it), and parse only the port locally.
3. **Cipher key.** `CipherKey::from_port(server.port())` must stay the **real server port** (from the
   `host:port` the user typed), never the proxy's WebSocket port. The proxy is payload-transparent: the server
   and client encrypt/decrypt exactly as with direct UDP.
4. **No update thread.** `rak_start` spawns `raknet-update` (2 ms loop). wasm32-unknown-unknown has no
   threads in the default build and the WebSocket callbacks only run when the JS event loop turns, so use the
   existing `threaded = false` path ("updating from the frame"): call `Core::update` from `NetClient::tick`
   each frame. At 60 fps that is ~16 ms between updates instead of 2 ms; RakNet's resend/ack timers are ≥
   100 ms, and SA:MP sends sync at 30–100 ms rates, so this only adds latency. Inside a worker with atomics
   (WEB-1), a dedicated network worker could poll more often, but the transport must then be created in that
   worker (`WebSocketTransport` is `!Send`).
5. **No blocking waits.** `rak_wait` (`thread::sleep`) in `connect()` (RAKNET_START/STOP) and the disconnect
   poll would block (or panic) on the browser main thread; make them no-ops on wasm (the waits only reproduce
   samp's timing of the attempt start) or turn them into a frame-count delay.
6. **Clock.** `std::time::Instant::now()` panics on wasm32-unknown-unknown. samp-rs uses `Instant` throughout
   netclient and samp-proto's `Connection` (timeouts, resend). Use a clock that works on both (e.g. the
   `web-time` crate's drop-in `Instant`, or `performance.now()` behind a small `Clock` type) — a mechanical
   import swap.
7. **Serial / name / auth.** Unchanged: the gpci serial comes from the host (native-plan.md Q8), the auth table
   is data. Nothing in the handshake depends on the transport.
8. **What the server sees.** Every browser player arrives from the proxy's IP (and the proxy's UDP port).
   Server-side per-IP limits, bans and `GetPlayerIp` therefore see the proxy. For a public proxy serving many
   players of one server, raise the server's per-IP connection limits for the proxy's address; for a personal
   proxy (the intended use: the user runs it locally) nothing changes, it is the user's own IP.
9. **Native builds keep UDP.** `UdpConnector` is the default on native; the WebSocket connector is chosen when
   `target_arch = "wasm32"`. The native i686 sa-game path (SAMP-3) is unaffected.

Suggested order on the samp-rs branch (pure refactors first, as the adapter steps): (a) clock type swap,
(b) `Core.sock` behind `DatagramTransport` with `UdpConnector` (native behaviour byte-identical; the
`native_handshake` test and SAMP-2's A/B join must pass unchanged), (c) wasm cfgs for items 2, 4, 5,
(d) the browser join test once WEB-1 runs a page.

WebTransport later: same framing (one datagram per WebTransport datagram, the `connect` request as the first
bidirectional-stream message), a second transport impl in `sa-net` and a second listener in the proxy; the
relay core (`Proxy::relay`) does not change.

## 7. Proposed ROADMAP row (for the main session)

| ID | Task | Status | Acceptance | Notes |
|---|---|---|---|---|
| SAMP-WEB-NET | Browser networking: `sa-udp-proxy` (WebSocket ↔ UDP) + `sa-net` client transports | in-progress | the wasm build joins an unmodified open.mp server through the proxy | 2026-11-12: `crates/sa-udp-proxy` (one UDP socket per WebSocket, `{"connect":"host:port"}` control frame, allowlist default = command-line target / `--allow` / `--allow-any`, Origin allowlist, size + pps/bps + idle + connection limits, shim mode); `crates/sa-net` (`DatagramTransport`/`Connector`: UDP, wasm `web_sys::WebSocket`, native-ws); 8 host tests; samp-rs `native_handshake` joins our open.mp 1.6.7 through shim + proxy in 1.13 s (`tools/e2e/samp/proxy-handshake.sh`); clippy -D warnings host + wasm32. Next: samp-rs netclient on `DatagramTransport` (docs/samp/web-proxy.md §6), then a browser join after WEB-1. WebTransport later |
