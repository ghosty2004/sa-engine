# Mods: getting started

sa-engine mods are WebAssembly modules (`docs/arch/ADR-004-mods.md`). The same `.wasm` runs in the native game
(wasmtime) and in the browser build (the browser's WebAssembly), sandboxed: a mod reaches the game only through the
mod API, defined once in `crates/sa-mod-api`. Mod support is a compile-time feature, `mods`; the default build has
none of it.

API 1.0 has: the API version, `init`, the overlay phase (called once per presented frame, after the game's own
drawing), filled rectangles and text in the 2D overlay, and a log. API 1.1 adds the display and the video mode
(`render`), the mod's own files (`files = "own"`), the settings phase and the resize event (below). API 1.2 (level 2)
adds read-only game state (`state = "read"`), game events, key state (`input = "read"`) and before / after hooks on
game functions (`hooks = [...]`); the reference of every function is `docs/mods/api.md`. Game control, entities,
network and storage come later (ADR-004 §4, §7).

## Call points

| entry point (SDK method) | when |
|---|---|
| `sa_init` (`init`) | once, after loading (before the boot) |
| `sa_settings` (`settings`, 1.1) | once in the boot: after the game has read its settings file (`gta_sa.set`), before it selects its video mode (`psSelectDevice`) and creates its D3D device. A video mode requested here applies from the first frame |
| `sa_overlay` (`overlay`) | every presented frame, after the game's drawing |
| `sa_resize` (`resize`, 1.1) | the native window's client area / the browser canvas (its size on the page in device pixels) changed size |
| `sa_event` (`event`, 1.2) | a game event the mod subscribed to (frame begin / end, game state, spawn, death, vehicle enter / exit, area, keys) |
| `sa_hook` (`hook`, 1.2) | a game function the mod hooked runs (before / after) |

Entry points added in a minor version are optional: a mod built for 1.0 has no `sa_settings` and the host skips it.

## The trainer HUD (level 2)

`examples/mods/trainer-hud` is a read-only trainer overlay: the player (position, heading, health, armour, money,
wanted level), the vehicle (model, health, speed), the world (game state, clock, weather, area), the camera, a
mini-map of the peds and vehicles within 100 m, the last game events, and two hooks: the result of
`CWorld::FindPlayerHeading` (an after-hook, shown next to the heading read from the state) and the peds
`CPed::ProcessControl` runs for per game step (a before-hook). F7 hides it. Its manifest:

```toml
[mod]
id = "trainer-hud"
name = "Trainer HUD (read-only)"
version = "0.1.0"
api_version = "1.2"
entry = "trainer-hud.wasm"

[permissions]
render = true
state = "read"
input = "read"
hooks = ["CWorld::FindPlayerHeading", "CPed::ProcessControl"]
```

The level-2 parts of its code:

```rust
use sa_mod::hooks::{self, Call, HookId, Phase};
use sa_mod::{Event, Mod, Value, event_kind, state};

impl Mod for TrainerHud {
    fn init(&mut self) -> Result<(), String> {
        sa_mod::events::subscribe(&[event_kind::FRAME_BEGIN, event_kind::PLAYER_SPAWN, event_kind::VEHICLE_ENTER]);
        self.heading_hook = hooks::subscribe("CWorld::FindPlayerHeading", false, true).ok();
        Ok(())
    }

    fn event(&mut self, event: Event) {
        if let Event::VehicleEnter { vehicle, .. } = event {
            let model = state::vehicle(vehicle).map_or(-1, |v| v.model.0);
            sa_mod::info(&format!("entered vehicle {:#x} (model {model})", vehicle.0));
        }
    }

    fn hook(&mut self, hook: HookId, phase: Phase, call: &Call) {
        if Some(hook) == self.heading_hook && phase == Phase::After {
            if let Some(Value::Float(h)) = call.result() {
                self.hooked_heading = Some(h);
            }
        }
    }
}
```

```sh
examples/mods/build.sh      # -> target/mods/trainer-hud/{mod.toml,trainer-hud.wasm}
target/i686-unknown-linux-gnu/release/sa-game --game wine/gta-sa --start-in-world --frames 500 \
    --mods target/mods/trainer-hud --present-dir target/hud
# browser: tools/web/serve.sh --game wine/gta-sa --mods target/mods, then ?autostart=1&mods=trainer-hud
```

The log shows the events it records (`mod trainer-hud: trainer-hud: entered vehicle 0x101 (model 492)`) and, at exit,
its overlay calls, time per frame, events and hook calls.

## The courier (level 3)

`examples/mods/courier` is a small gameplay mod: F6 spawns a car in front of the player (the next of five models each
time), F10 deletes the cars it spawned, F8 starts a run (a marker object 150 m ahead, 45 seconds to reach it; a win
pays $500 through the game's own `ADD_SCORE` command, clears the wanted level and plays the pick-up sound), F9
toggles a damage shield: a before-hook on `CPedDamageResponseCalculator::ComputeDamageResponse` that skips the call
for the player's ped. Its manifest asks for what it uses:

```toml
[permissions]
render = true
state = "write"
input = "read"
entities = true
scripts = true
audio = true
hooks = ["CPedDamageResponseCalculator::ComputeDamageResponse"]
```

```rust
fn spawn_car(&mut self) {
    let Some(p) = state::player().filter(|p| p.handle.is_some()) else { return };
    let at = ahead(p.ped.pos, p.ped.heading, 5.0);
    if let Ok(v) = entities::create_vehicle(411, at, p.ped.heading.to_degrees() + 90.0) {
        self.cars.push(v); // deleted by the host if the mod is disabled
    }
}

fn hook(&mut self, hook: HookId, phase: Phase, call: &Call) {
    if self.shield && Some(hook) == self.damage_hook && phase == Phase::Before
        && call.arg(1) == state::player().and_then(|p| p.handle).map(Value::Ped)
    {
        let _ = call.skip(Value::Int(0)); // no damage for the player this time
    }
}
```

```sh
examples/mods/build.sh      # -> target/mods/courier/{mod.toml,courier.wasm}
SA_GAME_KEYS=f6:400:402,f8:460:462 target/i686-unknown-linux-gnu/release/sa-game --game wine/gta-sa \
    --start-in-world --fixed-step 40 --frames 700 --mods target/mods/courier --present-dir target/courier
# browser: tools/web/serve.sh --game wine/gta-sa --mods target/mods, then ?autostart=1&mods=courier
```

Changes happen between frames: a change a hook asks for waits for the next frame boundary, and creating entities or
running script commands inside a hook is refused (`docs/mods/api.md`, level 3).

## The resolution mod

`mods/resolution` (a first-party mod: `mods/` holds the mods the project uses, `examples/mods/` the SDK's examples
and test mods) runs the game at the desktop resolution (in the browser: the canvas's size on the page in device
pixels) or at a chosen one, in a window or full screen. Its settings are its own `config.toml` (it writes the
defaults when the file is missing; natively the file is in the mod's folder, in the browser it is kept for the
session only, like the game's own settings there):

```toml
resolution = "auto"     # "auto" or "WIDTHxHEIGHT", e.g. "1920x1080"
fullscreen = false      # natively: full screen (the monitor's video mode of that size, else borderless, scaled)
follow_resize = true    # with "auto": request the window's / canvas's new size when it changes
```

```sh
mods/build.sh               # -> target/mods/resolution/{mod.toml,resolution.wasm,config.toml}
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
write the frames. The log shows `mod hello loaded: ...`; at exit, one line per mod with its overlay calls, time per
frame, events and hook calls. `--perf FILE` has the host's time per frame in the `mods` phase (docs/perf/measure.md).

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
  argument: a range outside the mod's memory, text that is not UTF-8, drawing outside the overlay phase, a hook value
  outside `sa_hook`) or `-3` (not found: no such file, a handle that names no live entity, a function the hook registry
  does not have).
- A call that traps (a panic, `unreachable`, an out-of-bounds access) or runs out of fuel disables the mod for the
  session. Memory beyond the cap cannot be allocated (`memory.grow` fails).
- A level-1 (visual) or level-2 (read-only) mod does not change the game: deterministic runs give the same game state
  with and without it. Hooks observe; they cannot change a call.

## Other languages

Anything that produces a core wasm module works: import the functions of `sa_mod_api::for_each_import!` from module
`"sa"`, export those of `for_each_export!` and the memory as `"memory"`, and use only `i32` / `f32` values (the
source of truth is `crates/sa-mod-api/src/lib.rs`).
