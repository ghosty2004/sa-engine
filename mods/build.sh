#!/bin/sh
# Builds the first-party mods for wasm32-unknown-unknown and puts each with its manifest in its own folder:
#
#   mods/build.sh [OUT]      (default OUT: target/mods)
#
#   OUT/resolution/{mod.toml,resolution.wasm}     the game at the desktop / a chosen resolution, with its
#                                                 config.toml (copied only when missing: the mod rewrites it)
#   OUT/samp/{mod.toml,samp.wasm}                 SA:MP: samp-rs's client core on the mod API (settings in its storage)
#
# Run natively with `sa-game ... --mods OUT/resolution` (sa-game built with --features mods), in the browser with
# `tools/web/serve.sh --mods OUT` and the page's `?mods=resolution`. The SDK examples: examples/mods/build.sh.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/target/mods}
cd "$root"
. tools/mods/pack.sh
pack sa-mod-resolution resolution mods/resolution/mod.toml
pack sa-mod-samp samp mods/samp/mod.toml
[ -e "$out/resolution/config.toml" ] || cp mods/resolution/config.toml "$out/resolution/config.toml"
ls -l "$out"/*/*.wasm
