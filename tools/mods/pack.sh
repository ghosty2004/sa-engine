# Sourced by mods/build.sh and examples/mods/build.sh: builds one mod crate for wasm32-unknown-unknown and puts its
# .wasm with its manifest in its own folder.
#
#   pack PACKAGE NAME MANIFEST [cargo build arguments...]
#
# OUT/NAME/{mod.toml,NAME.wasm}, from the crate PACKAGE and the manifest file MANIFEST. Needs $root and $out.
pack() {
    pkg=$1 name=$2 manifest=$3
    shift 3
    wasm=${CARGO_TARGET_DIR:-$root/target}/wasm32-unknown-unknown/release
    cargo build -q --release --target wasm32-unknown-unknown -p "$pkg" "$@"
    mkdir -p "$out/$name"
    cp "$wasm/$(echo "$pkg" | tr - _).wasm" "$out/$name/$name.wasm"
    cp "$manifest" "$out/$name/mod.toml"
}
