//! CBoat (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{g, jne_mask, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `nWakeSkipCounters[4]` (file static of Boat.cpp)
    pub static WAKE_SKIP_COUNTERS @ 0xC279A4: [i16; 4] = [0; 4];
    /// `nWakeSkipCounterVertex[4]` (file static of Boat.cpp)
    pub static WAKE_SKIP_COUNTER_VERTEX @ 0xC279AC: [i16; 4] = [0; 4];
    /// `fTimeMult` (file static of Boat.cpp: `1.2 / CBoat::WAKE_LIFETIME`, set by `__init_fTimeMult`)
    pub static TIME_MULT @ 0xC279CC: f32 = 0.0;
}

/// `CBoat*`.
type Boat = GPtr<u8>;

/// `CBoat::PruneWakeTrail()` (0x6F0E20): ages the 32 wake point lifetimes at `+0x748` by
/// `CTimer::ms_fTimeStep`. At the first lifetime `<= 0` (or one `<= timeStep`, which is zeroed) the
/// wake point count `+0x644` (u16) becomes its index; when all 32 survive the count is left unchanged.
/// Ported but unhookable: the loop branch at 0x6F0E5F (`jb 0x6F0E23`) targets the 6-byte patch range
/// (`33 D2 56 0F B7 C2`), so sa-hook rejects it. Callers keep ECX across the call (0x6F179A).
pub fn prune_wake_trail(b: Boat) {
    let step = g::<f32>(0xB7CB5C);
    let mut i: u16 = 0;
    loop {
        let off = 0x748 + 4 * i as isize;
        if jne_mask(x(rd::<f32>(b, off)), x(0.0), 0x41) {
            break;
        }
        if jne_mask(x(rd::<f32>(b, off)), x(step.read()), 0x41) {
            wr::<u32>(b, off, 0);
            break;
        }
        wr::<f32>(b, off, x(rd::<f32>(b, off)).fsub_m(step.read()).to_f32());
        i += 1;
        if i >= 0x20 {
            return;
        }
    }
    wr::<u16>(b, 0x644, i);
}

/// `CBoat::IsComponentPresent(int32)` (0x6F0200): always true (`mov al, 1`).
#[hook(addr = 0x6F0200, conv = thiscall, name = "CBoat::IsComponentPresent", expect = "B0 01 C2 04 00", this = "CBoat")]
pub fn is_component_present(_b: Boat, _comp: i32) -> u8 {
    1
}

/// `GetBoatAtomicObjectCB(RwObject*, void*)` (0x6F00D0): stores the object in `*data` when its flags
/// byte (`+2`) has rpATOMICRENDER (4); returns the object.
#[hook(addr = 0x6F00D0, conv = cdecl, name = "CBoat::GetBoatAtomicObjectCB", expect = "8B 44 24 04 F6 40 02 04")]
pub fn get_boat_atomic_object_cb(obj: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> {
    if rd::<u8>(obj, 2) & 4 != 0 {
        data.write(obj);
    }
    obj
}

/// `CBoat::ModifyHandlingValue(const bool&)` (0x6F0DE0): with pad number `+0x63C == 4`, the
/// handling's `+0xA0` += 1.0 (or -1.0 when the bool is false).
#[hook(addr = 0x6F0DE0, conv = thiscall, name = "CBoat::ModifyHandlingValue", expect = "8B 44 24 04 D9 05 1C 8C 85 00", this = "CBoat")]
pub fn modify_handling_value(b: Boat, plus: GPtr<u8>) {
    let v = if plus.read() != 0 { x(1.0) } else { x(-1.0) };
    if rd::<u8>(b, 0x63C) == 4 {
        let h = rd::<GPtr<u8>>(b, 0x384);
        wr::<f32>(h, 0xA0, v.fadd_m(rd::<f32>(h, 0xA0)).to_f32());
    }
}

/// `CBoat::CheckForSkippingCalculations()` (0x6F10C0): per wake k, when the vertex counter is 1..7
/// the skip counter becomes `max(vertex, skip - 1)`, else the skip counter is decremented when
/// positive; the vertex counter is then reset to 8.
#[hook(addr = 0x6F10C0, conv = cdecl, name = "CBoat::CheckForSkippingCalculations", expect = "56 33 C0 BE 04 00 00 00")]
pub fn check_for_skipping_calculations() {
    let skip = WAKE_SKIP_COUNTERS.ptr().cast::<i16>();
    let vert = WAKE_SKIP_COUNTER_VERTEX.ptr().cast::<i16>();
    for k in 0..4 {
        let v = vert.wrapping_index(k).read();
        if v > 0 && v < 8 {
            let s = i32::from(skip.wrapping_index(k).read()) - 1;
            let n = if i32::from(v) > s { i32::from(v) } else { s };
            skip.wrapping_index(k).write(n as i16);
        } else {
            let s = skip.wrapping_index(k).read();
            if s > 0 {
                skip.wrapping_index(k).write(s - 1);
            }
        }
        vert.wrapping_index(k).write(8);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_COMPONENT_PRESENT,
    HOOK_GET_BOAT_ATOMIC_OBJECT_CB,
    HOOK_MODIFY_HANDLING_VALUE,
    HOOK_CHECK_FOR_SKIPPING_CALCULATIONS,
];
