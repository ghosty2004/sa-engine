#!/bin/sh
# Builds the example mods (MOD-2) for wasm32-unknown-unknown and puts each with its manifest in its own folder:
#
#   examples/mods/build.sh [OUT]      (default OUT: target/mods)
#
#   OUT/hello/{mod.toml,hello.wasm}               the hello overlay
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
