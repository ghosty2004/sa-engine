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

Status (2026-11-12, WEB-2): headless Chromium on WebGL2 boots to the world and CJ walks and looks around with real
keys and mouse, on WebGL2 and on WebGPU (an AMD GPU). An unported callee ends only its main-loop iteration (a
"blocked" step, as natively; the page shows the count and the last blocked function); unlike natively, repeated
blocks do not stop the loop. Click the picture to capture the mouse, Esc releases it. The page applies the play
package's exploration stubs by default: `&stub=ADDR=RET/N,...` replaces them, `&stub=none` drops them. `?backend=gl` forces WebGL2, the path
verified headless. Details: `docs/arch/platform.md` section 22.

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

## SA:MP (WEB-SAMP)

The browser build joins SA:MP 0.3.7 / open.mp servers with samp-rs's network core (`crates/sa-game/src/samp_net.rs`,
feature `samp`). A page cannot open UDP, so the client's datagrams go over a WebSocket to `sa-udp-proxy`
(`crates/sa-udp-proxy`, docs/samp/web-proxy.md), which relays them unchanged to the server; natively the same client
uses plain UDP. RakNet runs inside the main-loop iteration (no update thread, no blocking waits).

```sh
FEATURES=samp tools/web/build.sh        # the wasm32 build with the SA:MP client
tools/web/samp-demo.sh                  # private open.mp copy (udp 127.0.0.1:7813) + sa-udp-proxy (ws://127.0.0.1:7781)
                                        # + this page on http://127.0.0.1:8644/, all loopback only; prints the URL
```

URL options: `?samp=HOST:PORT` (the server, as the proxy reaches it), `&proxy=ws://127.0.0.1:7781` (default),
`&name=WebCJ` (default). The auth table (third-party data, never committed) is served to this machine only by
`serve.sh --samp-auth FILE` as `/samp-auth.txt` and passed to the game as `SA_SAMP_AUTH`. The client serial's seed is
new per page load. By hand: `sa-udp-proxy 127.0.0.1:7813 --bind 127.0.0.1:7781`, then
`tools/web/serve.sh --port 8644 --game DIR --samp-auth FILE`.

Status (2026-11-12, WEB-SAMP): headless Chromium on WebGL2 (`--gpu native`) joins the private open.mp copy through
the proxy as WebCJ, picks class 0 and spawns in Grove Street 18 s after the page opened; the server's chat lines and
game text show through the engine-drawn SA:MP UI, and while a real W key is held the server's `[cov] where` position
follows CJ (on-foot sync, y -1686.6 to -1642.3 until he reaches a wall). Blocked iterations on the unported FX getter
`info_vget_value` (0x4A56E0) occur while walking (22 in 40 s, a few white frames); the run continues.

## Files

* `index.html`, `main.js`: the page (folder picker, input forwarding, status line, log panel)
* `worker.js`: the worker (module loader, synchronous file reads, main loop scheduling)
* `pkg/`: build output of `tools/web/build.sh` (not committed)
* `../tools/web/headless.mjs`: unattended evidence runs in headless Chromium over the DevTools protocol
