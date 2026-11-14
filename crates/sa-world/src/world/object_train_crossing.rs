//! `CObject::ProcessTrainCrossingBehaviour` (0x5A0B50, batch `frontier-1-1`): the level-crossing barriers south
//! of Grove Street (they lower when a train comes near). Ported from the 1.0 US bytes (gta-reversed's version is the
//! same shape; the bytes win on the x87 order).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::ci_acos;

use super::common::{entity_pos as pos, g, rd, wr, x};

use crate::bindings::path_set_links_bridge_lights;
use crate::bindings::simple_transform_update_rw_matrix;
use sa_core::bindings::entity_update_rw_frame;
use sa_core::bindings::matrix_update_rw_matrix;
use sa_stream::bindings::set_matrix_for_train_crossing;
orig! {
    /// `CTrain::FindNearestTrain(CVector pos, bool enginesOnly)` (0x6F7090, cdecl, by value)
    fn find_nearest_train(px: f32, py: f32, pz: f32, engines_only: u32) -> GPtr<u8> = cdecl 0x6F7090 => unported;
}

/// `ThePaths` (0x96F050)
const THE_PATHS: usize = 0x96_F050;
/// `ModelIndices::MI_TRAINCROSSING1` (0x8CD6B8, int16)
const MI_TRAINCROSSING1: usize = 0x8C_D6B8;

/// `CObject::ProcessTrainCrossingBehaviour()` (0x5A0B50, thiscall). Every 16 of 32 frames
/// (`(uint8)(CTimer::m_FrameCounter + +0x20) & 0x10` clear): flag +0x140 bit 11 = the nearest engine
/// (`FindNearestTrain(pos, true)`) is within `[0x858BB0]` in 2D (`sqrt(dy² + dx²)`, x87); for the
/// `MI_TRAINCROSSING1` model a change of the flag sets the path links' bridge lights in a ±12 box around the dummy
/// (+0x170) to the new state. Every other model then turns its barrier: `a = acos(matrix up.z)`, `s = timestep *
/// [0x858B4C]`; flag set: `a - s`, 0 at or below `[0x858B50]`; clear: `a + s`, 1.35088 (0x3FACE9CC) at or above
/// `[0x866FC8]`; `SetMatrixForTrainCrossing`, the RW matrix (CMatrix or the placement when no matrix) and
/// `UpdateRwFrame`.
#[hook(addr = 0x5A0B50, conv = thiscall, name = "CObject::ProcessTrainCrossingBehaviour", this = "CObject",
       expect = "8B 15 4C CB B7 00",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "_CIacos modelled (FPATAN at 53 bits, ledger MATH-2); path link flags for the crossing model")]
pub fn object_process_train_crossing_behaviour(this: GPtr<u8>) {
    let fc = g::<u32>(0xB7_CB4C).read();
    if (u32::from(rd::<u8>(this, 0x20)).wrapping_add(fc) as u8) & 0x10 == 0 {
        let p = pos(this);
        let train = find_nearest_train(rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8), 1);
        let f = rd::<u32>(this, 0x140);
        let was = (f >> 11) & 1 != 0;
        let cleared = f & !0x800;
        wr::<u32>(this, 0x140, cleared);
        if !train.is_null() {
            let me = pos(this);
            let tp = pos(train);
            let dx = x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(me, 0));
            let dy = x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(me, 4));
            let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
            if !d.fcom(x(g::<f32>(0x85_8BB0).read())).test_parity(0x05) {
                wr::<u32>(this, 0x140, cleared | 0x800);
            }
        }
        if i32::from(rd::<i16>(this, 0x22)) == i32::from(g::<u16>(MI_TRAINCROSSING1).read()) {
            let on = rd::<u32>(this, 0x140) & 0x800 != 0;
            if on != was {
                let dp = pos(rd::<GPtr<u8>>(this, 0x170));
                let (px, py) = (rd::<f32>(dp, 0), rd::<f32>(dp, 4));
                let k = g::<f32>(0x85_8CCC).read();
                path_set_links_bridge_lights(
                    g::<u8>(THE_PATHS),
                    x(px).fsub_m(k).to_f32(),
                    x(px).fadd_m(k).to_f32(),
                    x(py).fsub_m(k).to_f32(),
                    x(py).fadd_m(k).to_f32(),
                    u32::from(!was),
                );
            }
        }
    }
    if i32::from(rd::<i16>(this, 0x22)) == i32::from(g::<u16>(MI_TRAINCROSSING1).read()) {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let a = ci_acos(x(rd::<f32>(m, 0x28)));
    let s = x(g::<f32>(0xB7_CB5C).read()).fmul_m(g::<f32>(0x85_8B4C).read());
    let angle = if rd::<u32>(this, 0x140) & 0x800 != 0 {
        let v = a.fsub(s);
        if v.fcom(x(g::<f32>(0x85_8B50).read())).test_any(0x41) { 0.0 } else { v.to_f32() }
    } else {
        let v = a.fadd(s);
        if v.fcom(x(g::<f32>(0x86_6FC8).read())).test_parity(0x05) { f32::from_bits(0x3FAC_E9CC) } else { v.to_f32() }
    };
    set_matrix_for_train_crossing(m, angle);
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(this, 0x14);
        if m.is_null() {
            simple_transform_update_rw_matrix(this.byte_add(4), rwm);
        } else {
            matrix_update_rw_matrix(m, rwm);
        }
    }
    entity_update_rw_frame(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_OBJECT_PROCESS_TRAIN_CROSSING_BEHAVIOUR];
