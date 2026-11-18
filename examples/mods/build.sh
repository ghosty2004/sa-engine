#!/bin/sh
# Builds the example mods for wasm32-unknown-unknown and puts each with its manifest in its own folder:
#
#   examples/mods/build.sh [OUT]      (default OUT: target/mods)
#
#   OUT/hello/{mod.toml,hello.wasm}               the hello overlay
#   OUT/resolution/{mod.toml,resolution.wasm}     the game at the desktop / a chosen resolution, with its
#                                                 config.toml (copied only when missing: the mod rewrites it)
#   OUT/trainer-hud/{mod.toml,trainer-hud.wasm}  a read-only trainer overlay: live state, mini-map, events, hooks
#   OUT/courier/{mod.toml,courier.wasm}          a gameplay mod: spawned cars, a timed run, a damage shield
#   OUT/broken-trap/{mod.toml,broken-trap.wasm}   traps after 30 frames (test of the host's error handling)
#   OUT/broken-spin/{mod.toml,broken-spin.wasm}   loops forever after 30 frames (test of the fuel budget)
#
# Run natively with `sa-game ... --mods OUT` (sa-game built with --features mods), in the browser with
# `tools/web/serve.sh --mods OUT` and the page's `?mods=hello`. docs/mods/getting-started.md.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-$root/target/mods}
cd "$root"
wasm=${CARGO_TARGET_DIR:-$root/target}/wasm32-unknown-unknown/release
cargo build -q --release --target wasm32-unknown-unknown -p sa-mod-hello
mkdir -p "$out/hello"
cp "$wasm/sa_mod_hello.wasm" "$out/hello/hello.wasm"
cp examples/mods/hello/mod.toml "$out/hello/mod.toml"
cargo build -q --release --target wasm32-unknown-unknown -p sa-mod-resolution
mkdir -p "$out/resolution"
cp "$wasm/sa_mod_resolution.wasm" "$out/resolution/resolution.wasm"
cp examples/mods/resolution/mod.toml "$out/resolution/mod.toml"
[ -e "$out/resolution/config.toml" ] || cp examples/mods/resolution/config.toml "$out/resolution/config.toml"
cargo build -q --release --target wasm32-unknown-unknown -p sa-mod-trainer-hud
mkdir -p "$out/trainer-hud"
cp "$wasm/sa_mod_trainer_hud.wasm" "$out/trainer-hud/trainer-hud.wasm"
cp examples/mods/trainer-hud/mod.toml "$out/trainer-hud/mod.toml"
cargo build -q --release --target wasm32-unknown-unknown -p sa-mod-courier
mkdir -p "$out/courier"
cp "$wasm/sa_mod_courier.wasm" "$out/courier/courier.wasm"
cp examples/mods/courier/mod.toml "$out/courier/mod.toml"
for v in trap spin; do
    feat=""
    [ "$v" = spin ] && feat="--features spin"
    # shellcheck disable=SC2086
    cargo build -q --release --target wasm32-unknown-unknown -p sa-mod-broken $feat
    mkdir -p "$out/broken-$v"
    cp "$wasm/sa_mod_broken.wasm" "$out/broken-$v/broken-$v.wasm"
    cp "examples/mods/broken/mod-$v.toml" "$out/broken-$v/mod.toml"
done
ls -l "$out"/*/*.wasm
