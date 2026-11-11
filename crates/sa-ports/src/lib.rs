//! The single registry of engine-crate ports (`#[hook]` registrations), in hook-table order.
//!
//! sa-hook's table, its callers test, sa-oracle's `hook_preserve` and the L1 preserve test all read
//! [`ALL`], so a new port crate is registered by adding its dependency in Cargo.toml and one line to
//! [`CRATES`] (append at the end: the order is the table order). Nothing else changes.
#![no_std]

use sa_core::abi::HookReg;

/// Every port crate's registrations, in table order. Append new crates at the end.
pub const CRATES: &[&[HookReg]] = &[
    sa_core::hooks::ALL,
    sa_stream::hooks::ALL,
    sa_world::hooks::ALL,
    sa_pop::hooks::ALL,
    sa_script::hooks::ALL,
    sa_weapon::hooks::ALL,
    sa_ped::hooks::ALL,
    sa_input::hooks::ALL,
    sa_anim::hooks::ALL,
    sa_veh::hooks::ALL,
    sa_hud::hooks::ALL,
    sa_cam::hooks::ALL,
    sa_ai::hooks::ALL,
    sa_audio::hooks::ALL,
    sa_rwcore::hooks::ALL,
    sa_render::hooks::ALL,
];

const fn total(parts: &[&[HookReg]]) -> usize {
    let (mut n, mut i) = (0, 0);
    while i < parts.len() {
        n += parts[i].len();
        i += 1;
    }
    n
}

/// Number of port registrations over all crates.
pub const COUNT: usize = total(CRATES);

const fn concat<const N: usize>(parts: &[&[HookReg]]) -> [HookReg; N] {
    let mut out = [parts[0][0]; N];
    let (mut k, mut i) = (0, 0);
    while i < parts.len() {
        let mut j = 0;
        while j < parts[i].len() {
            out[k] = parts[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(k == N);
    out
}

#[allow(clippy::large_const_arrays)]
const ALL_ARRAY: [HookReg; COUNT] = concat(CRATES);

/// Every port crate's `#[native]` registrations (phase B entries of ports that cannot be hooked in
/// phase A, `sa_core::abi::NativeReg`), as lists of module lists. Append a crate's list when it
/// gets one.
pub const NATIVES: &[&[&[sa_core::abi::NativeReg]]] = &[
    &[sa_core::hooks::NATIVES],
    &[sa_core::general::math_natives::NATIVES],
    &[sa_stream::hooks::NATIVES],
    &[sa_input::hooks::NATIVES],
    &[sa_anim::hooks::NATIVES],
    sa_rwcore::hooks::NATIVE_MODULES,
    sa_render::hooks::NATIVE_MODULES,
    sa_hud::hooks::NATIVE_MODULES,
    sa_audio::hooks::NATIVE_MODULES,
    &[sa_ai::std6_natives::NATIVES],
    &[sa_anim::std6_natives::NATIVES],
    &[sa_audio::std6_natives::NATIVES],
    &[sa_cam::std6_natives::NATIVES],
    &[sa_core::std6_natives::NATIVES],
    &[sa_input::std6_natives::NATIVES],
    &[sa_stream::std6_natives::NATIVES],
    &[sa_world::std6_natives::NATIVES],
];

/// Every `#[native]` registration of every crate.
pub fn natives() -> impl Iterator<Item = &'static sa_core::abi::NativeReg> {
    NATIVES.iter().flat_map(|c| c.iter()).flat_map(|m| m.iter())
}

/// All port registrations, concatenated in [`CRATES`] order.
pub const ALL: &[HookReg] = &ALL_ARRAY;
