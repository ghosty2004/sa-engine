# Mods: getting started (MOD-2)

sa-engine mods are WebAssembly modules (`docs/arch/ADR-004-mods.md`). The same `.wasm` runs in the native game
(wasmtime) and in the browser build (the browser's WebAssembly), sandboxed: a mod reaches the game only through the
mod API, defined once in `crates/sa-mod-api`. Mod support is a compile-time feature, `mods`; the default build has
none of it.

API 1.0 (M0) has: the API version, `init`, the overlay phase (called once per presented frame, after the game's own
drawing), filled rectangles and text in the 2D overlay, and a log. API 1.1 (M1) adds the display and the video mode
(`render`), the mod's own files (`files = "own"`), the settings phase and the resize event (below). More comes in M1
and later (ADR-004 §4, §7).

## Call points

| entry point (SDK method) | when |
|---|---|
| `sa_init` (`init`) | once, after loading (before the boot) |
| `sa_settings` (`settings`, 1.1) | once in the boot: after the game has read its settings file (`gta_sa.set`), before it selects its video mode (`psSelectDevice`) and creates its D3D device. A video mode requested here applies from the first frame |
| `sa_overlay` (`overlay`) | every presented frame, after the game's drawing |
| `sa_resize` (`resize`, 1.1) | the native window's client area / the browser canvas (its size on the page in device pixels) changed size |

Entry points added in a minor version are optional: a mod built for 1.0 has no `sa_settings` and the host skips it.

## The resolution mod

`examples/mods/resolution` runs the game at the desktop resolution (in the browser: the canvas's size on the page in
device pixels) or at a chosen one, in a window or full screen. Its settings are its own `config.toml` (it writes the
defaults when the file is missing; natively the file is in the mod's folder, in the browser it is kept for the
session only, like the game's own settings there):

```toml
resolution = "auto"     # "auto" or "WIDTHxHEIGHT", e.g. "1920x1080"
fullscreen = false      # natively: full screen (the monitor's video mode of that size, else borderless, scaled)
follow_resize = true    # with "auto": request the window's / canvas's new size when it changes
```

```sh
examples/mods/build.sh      # -> target/mods/resolution/{mod.toml,resolution.wasm,config.toml}
target/i686-unknown-linux-gnu/release/sa-game --game wine/gta-sa --mods target/mods/resolution --window
# browser: tools/web/serve.sh --game wine/gta-sa --mods target/mods, then ?autostart=1&mods=resolution&full=1
```

How the game applies a requested mode (ADR-004 §4.1; `crates/sa-game/src/mods.rs`): the D3D9 adapter lists the real
display's modes plus the requested size instead of its fixed table, and the request becomes the original's own
video-mode state before `psSelectDevice` runs: a size the original's menu list offers (4:3, 5:4, 5:3, 16:10, enough
video memory) as the menu's video mode, exactly as `gta_sa.set` holds the player's choice; any other size (16:9 among
them) through the original's first-run path (`IsVMNotSelected` clear, `gCurrentVideoMode` = the mode), with the
menu's resolution row naming the closest listed mode (the original's menu cannot show other sizes). Natively the
window opens at the size (no stretching); in the browser the canvas's drawing buffer takes the size and the page fits
the canvas to its aspect. A request after the start (a resize) is applied as the menu's resolution change when the
adapter lists the size: the mode list is built once when the game starts, so the browser canvas does not follow a
page resize at run time (the image is scaled until the next start).

Without the mod nothing changes: the adapter keeps its fixed table and the game its first-run default, 800x600.

## A mod in Rust

A mod is a `cdylib` crate using the SDK `crates/sa-mod`, built for `wasm32-unknown-unknown`, plus a `mod.toml`.
The example is `examples/mods/hello`: it draws a text line in the bottom right corner of every frame.

```rust
use sa_mod::{Mod, Overlay, Rgba};

#[derive(Default)]
struct Hello;

impl Mod for Hello {
    fn overlay(&mut self, frame: u32, o: &mut Overlay) {
        let (w, h) = o.size();
        let _ = o.text(w as f32 - 300.0, h as f32 - 30.0, 16.0, Rgba::WHITE, &format!("frame {frame}"));
    }
}

sa_mod::entry!(Hello);
```

`mod.toml`, next to the `.wasm`:

```toml
[mod]
id = "hello"
name = "Hello overlay"
version = "0.1.0"
api_version = "1.1"
entry = "hello.wasm"     # default: "<id>.wasm"

[permissions]
render = true            # the overlay functions; without it they return "denied"

[limits]                 # optional; the host caps them (64 MiB, 200M fuel)
memory_mib = 16
fuel = 10000000          # per call, about one unit per wasm instruction
```

The host font is a fixed 8x8 cell font (ASCII; other characters draw as `?`): `size` is the cell size in pixels, so
a text's width is `characters x size`.

## Build hello

```sh
rustup target add wasm32-unknown-unknown
examples/mods/build.sh            # -> target/mods/hello/{mod.toml,hello.wasm}
```

It also builds two test mods, `broken-trap` and `broken-spin`, which draw a red countdown for 30 frames and then trap
(a panic) or loop forever. The host disables such a mod, logs why, and the game and the other mods go on.

## Run it natively

```sh
cargo build --release -p sa-game --target i686-unknown-linux-gnu --features mods
target/i686-unknown-linux-gnu/release/sa-game --game wine/gta-sa --start-in-world --frames 300 \
    --mods target/mods/hello --present-dir target/mods-native
```

`--mods DIR` takes one mod folder (with `mod.toml`) or a folder of mod folders (loaded in name order, e.g.
`--mods target/mods`). Add `--window` to play with the overlay; offscreen, `--present-dir DIR` / `--screenshot FILE`
write the frames. The log shows `mod hello loaded: ...`; at exit, one line per mod with its overlay calls and time per
call. `--perf FILE` has the host's time per frame in the `mods` phase (docs/perf/measure.md).

## Run it in the browser

```sh
FEATURES=mods tools/web/build.sh
tools/web/serve.sh --game wine/gta-sa --mods target/mods
# open http://localhost:8642/?autostart=1&mods=hello      (several: ?mods=hello,broken-trap)
```

The mods are fetched from the server's `/mods/NAME/` (`mod.toml`, then the entry file). Headless evidence run:

```sh
node tools/web/headless.mjs --url 'http://localhost:8642/?autostart=1&auto=1&mods=hello' --out target/mods-web
```

## Rules a mod lives by

- It gets values and handles, never game addresses, and cannot read game memory.
- Every host function returns an `i32`: a result `>= 0`, or `-1` (not permitted by the manifest), `-2` (a bad
  argument: a range outside the mod's memory, text that is not UTF-8, drawing outside the overlay phase) or `-3`
  (`file_read`: no such file).
- A call that traps (a panic, `unreachable`, an out-of-bounds access) or runs out of fuel disables the mod for the
  session. Memory beyond the cap cannot be allocated (`memory.grow` fails).
- A level-1 (visual) mod does not change the game: deterministic runs give the same game state with and without it.

## Other languages

Anything that produces a core wasm module works: import the functions of `sa_mod_api::for_each_import!` from module
`"sa"`, export those of `for_each_export!` and the memory as `"memory"`, and use only `i32` / `f32` values (the
source of truth is `crates/sa-mod-api/src/lib.rs`).
