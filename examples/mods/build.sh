#!/bin/sh
# Builds the SDK's example and test mods for wasm32-unknown-unknown and puts each with its manifest in its own folder:
#
#   examples/mods/build.sh [OUT]      (default OUT: target/mods)
#
#   OUT/hello/{mod.toml,hello.wasm}               the hello overlay
#   OUT/trainer-hud/{mod.toml,trainer-hud.wasm}  a read-only trainer overlay: live state, mini-map, events, hooks
#   OUT/courier/{mod.toml,courier.wasm}          a gameplay mod: spawned cars, a timed run, a damage shield
#   OUT/broken-trap/{mod.toml,broken-trap.wasm}   traps after 30 frames (test of the host's error handling)
#   OUT/broken-spin/{mod.toml,broken-spin.wasm}   loops forever after 30 frames (test of the fuel budget)
#
# Run natively with `sa-game ... --mods OUT` (sa-game built with --features mods), in the browser with
# `tools/web/serve.sh --mods OUT` and the page's `?mods=hello`. docs/mods/getting-started.md. The first-party mods
# (resolution): mods/build.sh.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-$root/target/mods}
cd "$root"
. tools/mods/pack.sh
pack sa-mod-hello hello examples/mods/hello/mod.toml
pack sa-mod-trainer-hud trainer-hud examples/mods/trainer-hud/mod.toml
pack sa-mod-courier courier examples/mods/courier/mod.toml
pack sa-mod-broken broken-trap examples/mods/broken/mod-trap.toml
pack sa-mod-broken broken-spin examples/mods/broken/mod-spin.toml --features spin
ls -l "$out"/*/*.wasm
