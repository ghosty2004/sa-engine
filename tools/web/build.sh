#!/bin/sh
# Builds the web version of sa-game (WEB-1): crates/sa-game as wasm32-unknown-unknown + wasm-bindgen glue in web/pkg/.
#
# The module is linked with its data, heap and stack above 0x1600000 (`--global-base`, `--no-stack-first`), so the
# low linear memory is free for the legacy address arena: the original's .rdata/.data/.bss (0x858000..0xCB1000), the
# read-only code bytes the no-CD protection reads (0x401000..0x858000, .HOODLUM 0x1556000..0x1577000) and sa-game's
# COM object pages (0x30000..) live at their original addresses (docs/arch/platform.md section 20).
#
#   tools/web/build.sh            release build (PROFILE=dev for a debug build)
#
# Needs: rustup target wasm32-unknown-unknown, wasm-bindgen-cli 0.2.130 (on PATH or in target/tools/bin:
# `cargo binstall --root target/tools wasm-bindgen-cli@0.2.130`).
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
profile=${PROFILE:-release}
dir=$profile
[ "$profile" = dev ] && dir=debug

low_end=$((0x1600000))   # crates/sa-game/src/arena.rs WASM_LOW_END
stack=$((16 * 1024 * 1024))

RUSTFLAGS="-C link-arg=--no-stack-first -C link-arg=--global-base=$low_end -C link-arg=-zstack-size=$stack ${RUSTFLAGS:-}" \
CARGO_PROFILE_RELEASE_DEBUG=${CARGO_PROFILE_RELEASE_DEBUG:-0} \
    cargo build -j"${JOBS:-8}" --profile "$profile" -p sa-game --lib --target wasm32-unknown-unknown --no-default-features

bindgen=wasm-bindgen
[ -x target/tools/bin/wasm-bindgen ] && bindgen=target/tools/bin/wasm-bindgen
"$bindgen" --target web --no-typescript --out-dir web/pkg "target/wasm32-unknown-unknown/$dir/sa_game.wasm"
ls -l web/pkg/sa_game_bg.wasm
