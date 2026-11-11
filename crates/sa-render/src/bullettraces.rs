//! PORT-RENDER: `CBulletTraces` (static class; `CBulletTrace` 0x2C bytes). Ported from the 1.0 US
//! bytes; gta-reversed `BulletTraces.cpp` and `docs/re/specs/RENDER/CBulletTraces.deviations.md` as
//! reading aids.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::timer::TIME_MS;

use crate::common::{rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `CBulletTraces::aTraces[16]` (CBulletTrace, 0x2C bytes: start, end, +0x18 exists, +0x1C creation
    /// time, +0x20 lifetime, +0x24 radius, +0x28 transparency)
    pub static TRACES @ 0xC7C748: [u8; 16 * 0x2C] = [0; 16 * 0x2C];
}

/// `CBulletTraces::Update()` (0x723FB0): every existing trace whose age `TIME_MS - created` (unsigned)
/// reached its lifetime stops existing (the loop unrolled 8 times, `TIME_MS` read once).
#[hook(addr = 0x723FB0, conv = cdecl, name = "CBulletTraces::Update", expect = "8B 0D 84 CB B7 00")]
pub fn update() {
    let now = TIME_MS.get();
    let base = TRACES.ptr().cast::<u8>();
    for i in 0..16 {
        let t = base.byte_add(i * 0x2C);
        if rd::<u8>(t, 0x18) != 0 && now.wrapping_sub(rd::<u32>(t, 0x1C)) >= rd::<u32>(t, 0x20) {
            wr::<u8>(t, 0x18, 0);
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_UPDATE];
