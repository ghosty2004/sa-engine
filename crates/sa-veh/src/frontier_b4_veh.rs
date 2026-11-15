//! FRONTIER-2 turn 4, lane B (batch `frontier2-b13`): monster truck, plane and boat virtuals. Ported from the 1.0 US
//! bytes; gta-reversed as a reading aid only (the bytes win). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::automobile_render::set_transmission_rotation;
use crate::bindings::set_component_rotation;
use crate::common::{f32_of, g, rd, wr, xi};
use sa_math::fpu::le_or_nan;

/// `CMonsterTruck::PreRender()` (0x6C7DE0, vtable slot). The four suspension positions +0x838.. are capped at their
/// maxima +0x878.. (copied when greater; a NaN keeps its value); `CAutomobile::PreRender`; the transmissions: frame
/// +0x694 from +0x838 / +0x840 and wheel 0's position (`SetTransmissionRotation(..., true)`), frame +0x698 from
/// +0x83C / +0x844 and wheel 1's (`false`); the dumper (model 406) turns its bed frame +0x6A0 about x by
/// `+0x86C (u16) * 0x8D33A8`. A default-constructed stack `CMatrix` is destroyed at the end.
#[hook(addr = 0x6C7DE0, conv = thiscall, name = "CMonsterTruck::PreRender", this = "CMonsterTruck", expect = "64 A1 00 00 00 00 6A FF 68 08 85 84 00",
       unverified = "frontier2-b13", route = "L2", hazards = "heap-writes",
       note = "writes the frames' RwMatrix; the SEH frame is not reproduced")]
pub fn monster_truck_pre_render(this: GPtr<u8>) {
    use sa_stream::bindings::get_wheel_posn;
    use sa_world::bindings::matrix_dtor;
    for k in 0..4isize {
        let cur = crate::common::x(rd::<f32>(this, 0x838 + 4 * k));
        let max = crate::common::x(rd::<f32>(this, 0x878 + 4 * k));
        if !le_or_nan(cur, max) {
            wr::<u32>(this, 0x838 + 4 * k, rd::<u32>(this, 0x878 + 4 * k));
        }
    }
    crate::trailer_ai::automobile_pre_render(this);
    let mi = crate::common::model_info(i32::from(rd::<i16>(this, 0x22)));
    let mut mat = [0u32; 0x12];
    let m = sa_core::field::ptr_of(&mut mat);
    let mut w = [0f32; 3];
    let wp = GPtr::from_raw(w.as_mut_ptr()).cast::<u8>();
    get_wheel_posn(mi, 0, wp, 0);
    set_transmission_rotation(this, rd::<GPtr<u8>>(this, 0x694), rd::<f32>(this, 0x838), rd::<f32>(this, 0x840), w[0], w[1], w[2], 1);
    get_wheel_posn(mi, 1, wp, 0);
    set_transmission_rotation(this, rd::<GPtr<u8>>(this, 0x698), rd::<f32>(this, 0x83C), rd::<f32>(this, 0x844), w[0], w[1], w[2], 0);
    if rd::<i16>(this, 0x22) == 0x196 {
        let bed = rd::<GPtr<u8>>(this, 0x6A0);
        if !bed.is_null() {
            let angle = f32_of(xi(i32::from(rd::<u16>(this, 0x86C))).fmul_m(g::<f32>(0x8D_33A8).read()));
            set_component_rotation(this, bed, 0, angle, 1);
        }
    }
    matrix_dtor(m);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_MONSTER_TRUCK_PRE_RENDER];
