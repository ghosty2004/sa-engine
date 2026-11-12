# sa-engine in the browser (WEB-1)

The phase B game (`crates/sa-game`) built for `wasm32-unknown-unknown` and run in a Web Worker: the same boot,
main loop and ports as the native `sa-game --window`, drawing through wgpu (WebGPU, else WebGL2) into an
`OffscreenCanvas`. Nothing of the game is shipped: it reads **your own** GTA San Andreas 1.0 US folder (its
`gta_sa.exe` is checked by SHA-256, as natively).

## Run it

```sh
tools/web/build.sh                      # once per code change (release build, ~2 min from scratch)
tools/web/serve.sh                      # http://localhost:8642/ (loopback only)
```

Open <http://localhost:8642/>, press **Choose your GTA San Andreas folder** and pick the folder that contains
`gta_sa.exe`. The browser reads the files locally (they are never uploaded). Then press Enter through the logos
and the menu (Start Game / New Game); "start in the world" skips the intro mission. Click the picture to capture
the mouse; W A S D walk, as in the game's default controls.

Development shortcut: `tools/web/serve.sh --game ~/sa-engine/wine/gta-sa` also serves that folder to this machine
only (ranged reads), and <http://localhost:8642/?autostart=1> starts with it without the picker.

URL options: `?inworld=0` plays the intro mission; `?auto=1` adds the native offscreen mode's synthetic input
(Enter through the menus, `SA_GAME_WALK`); any `SA_*` development variable (`SA_GAME_STUB=...`,
`SA_GAME_WALK=40:200`, `SA_GAME_SCRIPT_SKIP=1`, ...) is passed to the game as natively.

Needs a browser with WebGPU (Chromium 113+, Firefox 141+ on supported systems) or WebGL2 and module workers.

## How it maps the native build

| native (i686 Linux) | web (wasm32) |
|---|---|
| legacy arena mapped with `mmap(MAP_FIXED)` at 0x401000..0xCB1000, `.HOODLUM`, COM pages at 0x30000 | the same addresses in linear memory: the module is linked with `--global-base=0x1600000 --no-stack-first`, so everything below 0x1600000 is free for the arena (`tools/web/check_layout.mjs` checks it) |
| registry entries with the original calling convention | type-erased entries (`sa_core::wabi`): arguments as the original's stack words, results as EAX:EDX or ST0 |
| game folder through `std::fs` | `webfs.rs`: the page's `File`s (`FileReaderSync`) or the localhost server (ranged synchronous requests), 256 KiB block cache |
| `CdStream` streaming thread, audio threads | no threads: the streaming thread's loop body runs at each release of its semaphore; the audio threads do not run |
| frame limiter `sleep`, `Sleep()` | no sleeping; the worker runs main-loop iterations for up to 12 ms per task and yields after a `Present` |
| winit keyboard / mouse | DOM `KeyboardEvent.code` / pointer-lock motion, posted to the worker, the same `InputState` (window messages + DirectInput state) |
| CPUID | a fixed virtual SSE2 CPU (the paths of the native runs) |
| audio (cpal) | silent (the DirectSound buffers advance in real time) |

## Network seam for SA:MP (later)

SA:MP needs UDP, which a page cannot open. The plan: a separate WebSocket-to-UDP proxy (another lane writes it)
on localhost; the worker opens `ws://localhost:PORT/udp?host=SERVER&port=7777` and the samp-rs client's socket
trait gets a web implementation that sends each datagram as one binary WebSocket message (and receives the same
way). Nothing in the game loop blocks on the network: datagrams queue on the JS side and the client drains the
queue once per main-loop iteration. See `docs/arch/platform.md` section 21.

## Files

* `index.html`, `main.js`: the page (folder picker, input forwarding, status line, log panel)
* `worker.js`: the worker (module loader, synchronous file reads, main loop scheduling)
* `pkg/`: build output of `tools/web/build.sh` (not committed)
* `../tools/web/headless.mjs`: unattended evidence runs in headless Chromium over the DevTools protocol
