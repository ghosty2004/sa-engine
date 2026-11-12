//! `CTheScripts::ProcessAllSearchLights` (batch `mvpk-script-2`): the script search lights' movement toward their
//! targets and the orientation of their tower and bulb objects. Ported from the 1.0 US bytes (gta-reversed
//! `TheScripts.cpp` as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
//!
//! `tScriptSearchlight` (0x7C bytes, 8 at 0xA94D68): +0 used, +3 state (bit 7 active, low bits the mode 1..4),
//! +0x14 the current point, +0x28 target A, +0x34 target B, +0x40 speed, +0x48 the followed entity, +0x50 the bulb
//! object, +0x54 the tower object.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// `CTheScripts::ScriptSearchLightArray` (0xA94D68, 8 of 0x7C)
const SEARCH_LIGHTS: usize = 0xA94D68;

orig! {
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CMatrix::SetRotateX(float)` (0x59B060, `ret 4`)
    fn matrix_set_rotate_x(m: GPtr<u8>, a: u32) = thiscall 0x59B060 => unported;
    /// `CMatrix::RotateZ(float)` (0x59B390, `ret 4`)
    fn matrix_rotate_z(m: GPtr<u8>, a: u32) = thiscall 0x59B390 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
}

fn rd<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}
fn wr<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v)
}
fn x(v: f32) -> X87 {
    X87::from_f32(v)
}
/// An entity's position (matrix +0x30, else the placement +4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// One light's movement toward `target` (read at `t`); returns true when it arrived (the distance not above the
/// speed: the current point = the target, re-read).
fn search_light_move(sl: GPtr<u8>, t: GPtr<u8>) -> bool {
    let d = [
        x(rd::<f32>(t, 0)).fsub_m(rd::<f32>(sl, 0x14)).to_f32(),
        x(rd::<f32>(t, 4)).fsub_m(rd::<f32>(sl, 0x18)).to_f32(),
        x(rd::<f32>(t, 8)).fsub_m(rd::<f32>(sl, 0x1C)).to_f32(),
    ];
    let len = x(d[2]).fmul_m(d[2]).fadd(x(d[1]).fmul_m(d[1])).fadd(x(d[0]).fmul_m(d[0])).fsqrt();
    if len.fcom(x(rd::<f32>(sl, 0x40))).test_parity(0x41) {
        let mut v = d;
        vector_normalise(GPtr::from_raw(v.as_mut_ptr()).cast());
        let speed = x(rd::<f32>(sl, 0x40));
        let s = [x(v[0]).fmul(speed).to_f32(), x(v[1]).fmul(speed).to_f32(), speed.fmul_m(v[2]).to_f32()];
        for (i, si) in s.iter().enumerate() {
            let o = 0x14 + 4 * i as isize;
            wr::<f32>(sl, o, x(*si).fadd_m(rd::<f32>(sl, o)).to_f32());
        }
        return false;
    }
    for i in 0..3 {
        wr::<u32>(sl, 0x14 + 4 * i, rd::<u32>(t, 4 * i));
    }
    true
}

/// One object's orientation: `SetRotateX(pitch)`, `RotateZ(heading)`, matrix position += `base`, then its RW matrix
/// (when it has an RW object: `CMatrix::UpdateRwMatrix` or, without a matrix, `CSimpleTransform::UpdateRwMatrix`)
/// and `UpdateRwFrame`. The matrix (+0x14) is used without a NULL check (original).
fn search_light_orient(obj: GPtr<u8>, pitch: u32, heading: u32, base: [f32; 3]) {
    matrix_set_rotate_x(rd::<GPtr<u8>>(obj, 0x14), pitch);
    matrix_rotate_z(rd::<GPtr<u8>>(obj, 0x14), heading);
    let m = rd::<GPtr<u8>>(obj, 0x14);
    for (i, b) in base.iter().enumerate() {
        let o = 0x30 + 4 * i as isize;
        wr::<f32>(m, o, x(*b).fadd_m(rd::<f32>(m, o)).to_f32());
    }
    let rw = rd::<GPtr<u8>>(obj, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let mat = rd::<GPtr<u8>>(obj, 0x14);
        if mat.is_null() {
            simple_transform_update_rw_matrix(obj.byte_add(4), rwm);
        } else {
            matrix_update_rw_matrix(mat, rwm);
        }
    }
    entity_update_rw_frame(obj);
}

/// `CTheScripts::ProcessAllSearchLights()` (0x4939F0, cdecl). x87 (FSQRT, FPATAN). For each of the 8 lights that is
/// used and active (state bit 7):
///
/// - Mode 1: move toward target A; arrived → state 0x82. Mode 2: toward target B; arrived → state 0x81. Mode 3:
///   toward the followed entity's position (no state change). Mode 4: toward target A; arrived → state 0x80 and target
///   A and the speed zeroed. Moving: `d = f32(target - current)` per component, `sqrt(dz² + dy² + dx²)` above the speed
///   (or NaN) → `Normalise(d)`, current += f32(d * speed) per component; else current = target.
/// - Then, with a tower object (+0x54): `d = Normalise(f32(current - tower position))`, `heading = f32 fpatan(-d.x,
///   d.y)`, `pitch = f32 fpatan(d.z, sqrt(d.x² + d.y²))`; the tower's matrix position is saved, then the tower and the
///   bulb (+0x50, no NULL check) get `SetRotateX(pitch)`, `RotateZ(heading)`, position += the saved position.
#[hook(addr = 0x4939f0, conv = cdecl, name = "CTheScripts::ProcessAllSearchLights", expect = "83 EC 60 53 55",
       route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "writes the light objects' matrices (heap)")]
#[allow(clippy::collapsible_match)]
pub fn the_scripts_process_all_search_lights() {
    for i in 0..8isize {
        let sl = GPtr::<u8>::from_addr(SEARCH_LIGHTS).byte_add(i * 0x7C);
        if rd::<u8>(sl, 0) == 0 {
            continue;
        }
        let st = rd::<u8>(sl, 3);
        if (st as i8) >= 0 {
            continue;
        }
        match st & 0x7F {
            1 => {
                if search_light_move(sl, sl.byte_add(0x28)) {
                    wr::<u8>(sl, 3, 0x82);
                }
            }
            2 => {
                if search_light_move(sl, sl.byte_add(0x34)) {
                    wr::<u8>(sl, 3, 0x81);
                }
            }
            3 => {
                let e = rd::<GPtr<u8>>(sl, 0x48);
                search_light_move(sl, entity_pos(e));
            }
            4 => {
                if search_light_move(sl, sl.byte_add(0x28)) {
                    wr::<u8>(sl, 3, 0x80);
                    wr::<u32>(sl, 0x28, 0);
                    wr::<u32>(sl, 0x2C, 0);
                    wr::<u32>(sl, 0x30, 0);
                    wr::<u32>(sl, 0x40, 0);
                }
            }
            _ => {}
        }
        let tower = rd::<GPtr<u8>>(sl, 0x54);
        if tower.is_null() {
            continue;
        }
        let tp = entity_pos(tower);
        let mut d = [
            x(rd::<f32>(sl, 0x14)).fsub_m(rd::<f32>(tp, 0)).to_f32(),
            x(rd::<f32>(sl, 0x18)).fsub_m(rd::<f32>(tp, 4)).to_f32(),
            x(rd::<f32>(sl, 0x1C)).fsub_m(rd::<f32>(tp, 8)).to_f32(),
        ];
        vector_normalise(GPtr::from_raw(d.as_mut_ptr()).cast());
        let heading = x(d[0]).fchs().fpatan(x(d[1])).to_f32();
        let m = rd::<GPtr<u8>>(tower, 0x14);
        let base = [rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)];
        let horiz = x(d[0]).fmul_m(d[0]).fadd(x(d[1]).fmul_m(d[1])).fsqrt();
        let pitch = x(d[2]).fpatan(horiz).to_f32();
        search_light_orient(tower, pitch.to_bits(), heading.to_bits(), base);
        search_light_orient(rd::<GPtr<u8>>(sl, 0x50), pitch.to_bits(), heading.to_bits(), base);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_THE_SCRIPTS_PROCESS_ALL_SEARCH_LIGHTS];
