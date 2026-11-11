//! WRITER-MVP-E batch `mvpe-ent-1`: entity set-up and per-frame entity work on the loading / walking path
//! (`CEntity::CreateEffects`, `CObject::ProcessControl`). Ported from the 1.0 US bytes (SecuROM fragments inlined; the
//! 11-case jump table of `CreateEffects` at 0x533BB8 decoded by hand; gta-reversed a reading aid only).
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::common::{f32_of, g, rd, wr, x};
use super::entity::get_heading;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CScriptsForBrains` instance `ScriptsForBrains` (0xA90CF0)
const SCRIPTS_FOR_BRAINS: u32 = 0xA90CF0;
/// `g_fx` (`Fx_c`, 0xA9AE00)
const G_FX: u32 = 0xA9AE00;
/// `CEntryExitManager::mp_poolEntryExits`
const ENTRY_EXIT_POOL: usize = 0x96A7D8;
/// `RADTODEG(1)` (0x859878, f32 57.29578)
const K_RAD_TO_DEG: usize = 0x859878;

orig! {
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70, `ret 4`)
    fn get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4c4c70 => unported;
    /// `CScriptsForBrains::RequestAttractorScriptBrainWithThisName(const char*)` (0x46AA80, `ret 4`)
    fn request_attractor_brain(b: u32, name: GPtr<u8>) = thiscall 0x46aa80 => unported;
    /// `Fx_c::CreateEntityFx(CEntity*, char* name, RwV3d* pos, RwMatrix*)` (0x4A11E0, `ret 0x10`)
    fn fx_create_entity_fx(fx: u32, e: GPtr<u8>, name: GPtr<u8>, pos: GPtr<u8>, m: GPtr<u8>) = thiscall 0x4a11e0 => unported;
    /// `C2dEffect::Roadsign_GetPaletteIDFromFlags(CRoadsignAttrFlags)` (0x6FA6A0, cdecl)
    fn roadsign_palette(flags: u32) -> u32 = cdecl 0x6fa6a0 => unported;
    /// `C2dEffect::Roadsign_GetNumLettersFromFlags(CRoadsignAttrFlags)` (0x6FA670, cdecl, AX)
    fn roadsign_letters(flags: u32) -> u32 = cdecl 0x6fa670 => unported;
    /// `C2dEffect::Roadsign_GetNumLinesFromFlags(CRoadsignAttrFlags)` (0x6FA640, cdecl, AX)
    fn roadsign_lines(flags: u32) -> u32 = cdecl 0x6fa640 => unported;
    /// `CCustomRoadsignMgr::CreateRoadsignAtomic(float sx, float sy, int32 lines, char* l1, char* l2,
    /// char* l3, char* l4, int32 letters, uint8 palette)` (0x6FF2D0, cdecl)
    #[allow(clippy::too_many_arguments)]
    fn create_roadsign_atomic(sx: u32, sy: u32, lines: u32, l1: GPtr<u8>, l2: GPtr<u8>, l3: GPtr<u8>, l4: GPtr<u8>,
        letters: u32, pal: u32) -> GPtr<u8> = cdecl 0x6ff2d0 => unported;
    /// `RwFrameSetIdentity(RwFrame*)` (0x7F10B0)
    fn rw_frame_set_identity(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7f10b0 => unported;
    /// `RwFrameRotate(RwFrame*, const RwV3d* axis, RwReal angle, RwOpCombineType)` (0x7F1010)
    fn rw_frame_rotate(f: GPtr<u8>, axis: GPtr<u8>, angle: u32, op: i32) -> GPtr<u8> = cdecl 0x7f1010 => unported;
    /// `RwFrameTranslate(RwFrame*, const RwV3d*, RwOpCombineType)` (0x7F0E30)
    fn rw_frame_translate(f: GPtr<u8>, v: GPtr<u8>, op: i32) -> GPtr<u8> = cdecl 0x7f0e30 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7f0910 => unported;
    /// `CEntity::TransformFromObjectSpace(CVector& out, const CVector& offset)` (0x5334F0, `ret 8`)
    fn transform_ref(e: GPtr<u8>, out: GPtr<u8>, off: GPtr<u8>) -> GPtr<u8> = thiscall 0x5334f0 => super::entity::transform_from_object_space_ref;
    /// `CEntity::TransformFromObjectSpace(const CVector&)` (0x533560, hidden result pointer, `ret 8`)
    fn transform_ptr(e: GPtr<u8>, out: GPtr<u8>, off: GPtr<u8>) -> GPtr<u8> = thiscall 0x533560 => super::entity::transform_from_object_space_ptr;
    /// CRT `srand(uint32)` (0x821B11)
    fn srand(seed: u32) = cdecl 0x821b11 => sa_core::crt::srand_owned;
    /// `CEntryExitManager::AddOne(float x, y, z, float angle, float sx, float sy, float sz(0), float exitX,
    /// exitY, exitZ, float exitAngle, int32 area, int32 flags, int32 skyColor, int32 timeOn, int32 timeOff,
    /// int32 numPeds, const char* name)` (0x43FA00, cdecl; returns the pool index or -1)
    #[allow(clippy::too_many_arguments)]
    fn entry_exit_add_one(x: u32, y: u32, z: u32, ang: u32, sx: u32, sy: u32, sz: u32, ex: u32, ey: u32, ez: u32,
        eang: u32, area: i32, flags: u32, sky: u32, on: u32, off: u32, peds: u32, name: GPtr<u8>) -> i32 = cdecl 0x43fa00 => unported;
    /// `CEscalators::AddOne(const CVector& bottom, const CVector& top, const CVector& end, const CVector&
    /// start, bool moveDown, CEntity*)` (0x717C90, cdecl)
    fn escalators_add_one(a: u32, b: u32, c: u32, d: u32, down: u32, e: u32) = cdecl 0x717c90 => super::escalators::add_one;
}

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// A CVector copied with `fld dword` / `fstp dword` (bit-exact except that a signalling NaN is quietened).
fn fcopy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<f32>(dst, o, f32_of(x(rd::<f32>(src, o))));
    }
}

/// `CEntity::CreateEffects()` (0x533790; epilogue `add esp, 0xC0` in a protection fragment): clears entity
/// flag 0x1000000 (+0x1C), then for each 2D effect `i` < (model info byte +0xD) of the entity's model (+0x22,
/// i16 index into `ms_modelInfoPtrs`), by the effect type (byte +0xC, jump table 0x533BB8):
/// - 0 light: flag 0x200000 on.
/// - 1 particle: `g_fx.CreateEntityFx(this, eff +0x10 (name), eff (pos), frame LTM)` where the matrix is
///   `rwObject->parent + 0x10` (null without an RW object +0x18).
/// - 3 ped attractor: type byte +0x34 == 5 → `ScriptsForBrains.RequestAttractorScriptBrainWithThisName(eff +0x38)`.
/// - 6 enter/exit: entry = `TransformFromObjectSpace(eff pos)`, exit = `TransformFromObjectSpace(eff pos +
///   eff +0x1C..+0x24)` (sums f32 through the x87; the result copied out of the returned vector); with flag 4
///   in byte +0x3A `srand(clockHours (0xB70154) - (timeOn +0x38 > timeOff +0x39 && minutes byte 0xB70153 <
///   timeOff) + this)`; `AddOne(entry, f32(heading * RADTODEG + eff +0x10), eff +0x14, eff +0x18, 0, exit,
///   f32(heading * RADTODEG + eff +0x28), i16 +0x2C, (byte +0x3A << 8) + byte +0x2E, byte +0x2F, byte +0x38,
///   byte +0x39, 0, eff +0x30)` (the heading is the raw FPATAN value, evaluated twice); on success, when the
///   new enex's link (+0x38) exists and its byte +0x31 has no 0x40, bit 0x40 of the new enex's +0x31 is
///   cleared (a free pool slot yields a null enex and the original reads +0x38 of null, as here).
/// - 7 roadsign: flag 0x1000000 on; `atomic = CreateRoadsignAtomic(size x +0x10, size y +0x14, lines, text
///   +0x28 lines (4 x 16 chars), letters, palette)` (from the u16 flags +0x24); its frame: identity, rotate
///   about Z by +0x20 (replace), X by +0x18 and Y by +0x1C (postconcat), translate by the effect position
///   (postconcat), `RwFrameUpdateObjects`; eff +0x2C = atomic.
/// - 10 escalator: the four points eff +0 / +0x10 / +0x1C / +0x28 (the last three copied through the x87
///   first) transformed from object space, `CEscalators::AddOne(p0, p1, p2, p3, byte +0x34 == 0, this)`.
/// - other types: nothing.
#[hook(addr = 0x533790, conv = thiscall, name = "CEntity::CreateEffects", expect = "81 EC C0 00 00 00",
       unverified = "mvpe-ent-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "FPATAN heading for enex angles; srand for timed enexes; creates FX, roadsign atomics, enexes, escalators; the escalator moveDown bool is pushed with ECX's upper bytes left by the last call (0 here)")]
pub fn entity_create_effects(this: GPtr<u8>) {
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xFEFF_FFFF);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    let count = i32::from(rd::<u8>(mi, 0xD));
    let mut i = 0i32;
    while i < count {
        let eff = get_2d_effect(mi, i);
        match rd::<u8>(eff, 0xC) {
            0 => wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x20_0000),
            1 => {
                let rw = rd::<GPtr<u8>>(this, 0x18);
                let m = if rw.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(rw, 4).byte_add(0x10) };
                fx_create_entity_fx(G_FX, this, eff.byte_add(0x10), eff, m);
            }
            3 => {
                if rd::<u8>(eff, 0x34) == 5 {
                    request_attractor_brain(SCRIPTS_FOR_BRAINS, eff.byte_add(0x38));
                }
            }
            6 => create_enex(this, eff),
            7 => create_roadsign(this, eff),
            10 => {
                let mut p0 = [0u32; 3];
                let mut p1 = [0u32; 3];
                let mut p2 = [0u32; 3];
                let mut p3 = [0u32; 3];
                let mut s1 = [0u32; 3];
                let mut s2 = [0u32; 3];
                let mut s3 = [0u32; 3];
                transform_ref(this, local(&mut p0), eff);
                fcopy3(local(&mut s1), eff.byte_add(0x10));
                transform_ref(this, local(&mut p1), local(&mut s1));
                fcopy3(local(&mut s2), eff.byte_add(0x1C));
                transform_ref(this, local(&mut p2), local(&mut s2));
                fcopy3(local(&mut s3), eff.byte_add(0x28));
                transform_ref(this, local(&mut p3), local(&mut s3));
                let down = u32::from(rd::<u8>(eff, 0x34) == 0);
                escalators_add_one(
                    local(&mut p0).addr() as u32,
                    local(&mut p1).addr() as u32,
                    local(&mut p2).addr() as u32,
                    local(&mut p3).addr() as u32,
                    down,
                    this.addr() as u32,
                );
            }
            _ => {}
        }
        i += 1;
    }
}

/// Case 6 of [`entity_create_effects`] (enter/exit marker).
fn create_enex(this: GPtr<u8>, eff: GPtr<u8>) {
    let mut exit = [0u32; 3];
    let ep = local(&mut exit);
    // fld +0x1C; fld +0x20; fld +0x24 (stored f32); then x, y, z sums stored f32
    let cz = f32_of(x(rd::<f32>(eff, 0x24)));
    wr::<f32>(ep, 0, f32_of(x(rd::<f32>(eff, 0x1C)).fadd_m(rd::<f32>(eff, 0))));
    wr::<f32>(ep, 4, f32_of(x(rd::<f32>(eff, 0x20)).fadd_m(rd::<f32>(eff, 4))));
    wr::<f32>(ep, 8, f32_of(x(cz).fadd_m(rd::<f32>(eff, 8))));
    let mut entry = [0u32; 3];
    let np = local(&mut entry);
    transform_ref(this, np, eff);
    let mut tmp = [0u32; 3];
    let r = transform_ptr(this, local(&mut tmp), ep);
    let (rx, ry, rz) = (rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8));
    wr::<u32>(ep, 0, rx);
    wr::<u32>(ep, 4, ry);
    wr::<u32>(ep, 8, rz);
    if rd::<u8>(eff, 0x3A) & 4 != 0 {
        let off = rd::<u8>(eff, 0x39);
        let mut h = u32::from(g::<u8>(0xB70154).read());
        if rd::<u8>(eff, 0x38) > off && g::<u8>(0xB70153).read() < off {
            h = h.wrapping_sub(1);
        }
        srand(h.wrapping_add(this.addr() as u32));
    }
    let h1 = get_heading(this);
    let h2 = get_heading(this);
    let k = g::<f32>(K_RAD_TO_DEG).read();
    let exit_ang = f32_of(h1.fmul_m(k).fadd_m(rd::<f32>(eff, 0x28)));
    let ang = f32_of(h2.fmul_m(k).fadd_m(rd::<f32>(eff, 0x10)));
    let flags = (u32::from(rd::<u8>(eff, 0x3A)) << 8).wrapping_add(u32::from(rd::<u8>(eff, 0x2E)));
    let idx = entry_exit_add_one(
        entry[0],
        entry[1],
        entry[2],
        ang.to_bits(),
        rd::<u32>(eff, 0x14),
        rd::<u32>(eff, 0x18),
        0,
        exit[0],
        exit[1],
        exit[2],
        exit_ang.to_bits(),
        i32::from(rd::<i16>(eff, 0x2C)),
        flags,
        u32::from(rd::<u8>(eff, 0x2F)),
        u32::from(rd::<u8>(eff, 0x38)),
        u32::from(rd::<u8>(eff, 0x39)),
        0,
        eff.byte_add(0x30),
    );
    if idx == -1 {
        return;
    }
    let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
    let ee = if rd::<u8>(rd::<GPtr<u8>>(pool, 4), idx as isize) & 0x80 != 0 {
        GPtr::<u8>::NULL
    } else {
        rd::<GPtr<u8>>(pool, 0).byte_add(0x3C * idx as isize)
    };
    let link = rd::<GPtr<u8>>(ee, 0x38);
    if link.is_null() || rd::<u8>(link, 0x31) & 0x40 != 0 {
        return;
    }
    wr::<u8>(ee, 0x31, rd::<u8>(ee, 0x31) & 0xBF);
}

/// Case 7 of [`entity_create_effects`] (custom roadsign).
fn create_roadsign(this: GPtr<u8>, eff: GPtr<u8>) {
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x100_0000);
    let text = rd::<GPtr<u8>>(eff, 0x28);
    let fl = u32::from(rd::<u16>(eff, 0x24));
    let pal = roadsign_palette(fl);
    let letters = roadsign_letters(fl) & 0xFFFF;
    let lines = roadsign_lines(fl) & 0xFFFF;
    let atomic = create_roadsign_atomic(
        rd::<u32>(eff, 0x10),
        rd::<u32>(eff, 0x14),
        lines,
        text,
        text.byte_add(0x10),
        text.byte_add(0x20),
        text.byte_add(0x30),
        letters,
        pal,
    );
    let frame = rd::<GPtr<u8>>(atomic, 4);
    rw_frame_set_identity(frame);
    let mut ax = [0x3F80_0000u32, 0, 0];
    let mut ay = [0u32, 0x3F80_0000, 0];
    let mut az = [0u32, 0, 0x3F80_0000];
    rw_frame_rotate(frame, local(&mut az), rd::<u32>(eff, 0x20), 0);
    rw_frame_rotate(frame, local(&mut ax), rd::<u32>(eff, 0x18), 2);
    rw_frame_rotate(frame, local(&mut ay), rd::<u32>(eff, 0x1C), 2);
    let mut pos = [rd::<u32>(eff, 0), rd::<u32>(eff, 4), rd::<u32>(eff, 8)];
    rw_frame_translate(frame, local(&mut pos), 2);
    rw_frame_update_objects(frame);
    wr::<GPtr<u8>>(eff, 0x2C, atomic);
}

orig! {
    /// `CBaseModelInfo::GetModelType()` (vtable +0x24)
    fn v_model_type(mi: GPtr<u8>) -> u32 = thiscall vslot 0x24 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, s: u32) = thiscall vslot 0x10 => unported;
    /// vtable slot +0x20 of the object (`CPhysical::RemoveAndAdd`)
    fn v_slot_20(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `FUN_0059f050(int16 modelIndex)` (0x59F050, cdecl, AL)
    fn model_check_59f050(model: i32) -> u8 = cdecl 0x59f050 => unported;
    /// `CRopes::IsCarriedByRope(CPhysical*)` (0x555F80, cdecl, AL)
    fn ropes_is_carried_by_rope(e: GPtr<u8>) -> u8 = cdecl 0x555f80 => unported;
    /// `CObject::ObjectDamage(float damage, CVector* fxOrigin, CVector* fxDirection, CEntity* damager,
    /// eWeaponType)` (0x5A0D90, `ret 0x14`)
    fn object_damage(o: GPtr<u8>, dmg: u32, origin: GPtr<u8>, dir: GPtr<u8>, damager: GPtr<u8>, weapon: u32) = thiscall 0x5a0d90 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60, cdecl): `a - b`
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40fe60 => super::mvpa_math::vector_sub;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, cdecl, out first)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40fe30 => super::mvpa_math::vector_add;
    /// `operator/(const CVector&, float)` (0x4119D0, cdecl, out first)
    fn vector_div(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x4119d0 => super::mvpa_math::vector_div_scalar;
    /// `CVector::SquaredMagnitude()` (0x406DA0, thiscall, ST0)
    fn vector_sq_mag(v: GPtr<u8>) -> f64 = thiscall 0x406da0 => unported;
    /// `CPhysical::GetSpeed(CVector point)` (0x542CE0, hidden result pointer, `ret 0x10`)
    fn physical_get_speed(e: GPtr<u8>, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> = thiscall 0x542ce0 => crate::physical::forces::get_speed;
    /// `CPhysical::AttachEntityToEntity(CPhysical*, CVector* offset, RtQuat* rotation)` (0x54D690, `ret 0xC`)
    fn attach_quat(e: GPtr<u8>, to: GPtr<u8>, off: GPtr<u8>, rot: GPtr<u8>) -> GPtr<u8> = thiscall 0x54d690 => crate::physical::attach::attach_entity_to_entity_quat;
    /// `CPhysical::ProcessControl()` (0x5485E0)
    fn physical_process_control(e: GPtr<u8>) = thiscall 0x5485e0 => crate::physical::attach::process_control;
    /// `cBuoyancy::ProcessBuoyancy(CPhysical*, float, CVector* turnPoint, CVector* force)` (0x6C3EF0, `ret 0x10`, AL)
    fn process_buoyancy(b: GPtr<u8>, e: GPtr<u8>, buoy: f32, turn: GPtr<CVector>, force: GPtr<CVector>) -> u8 = thiscall 0x6c3ef0 => crate::physical::buoyancy::process_buoyancy;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0, `ret 0xC`)
    fn apply_move_force(e: GPtr<u8>, f: CVector) = thiscall 0x5429f0 => crate::physical::forces::apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50, `ret 0x18`)
    fn apply_turn_force(e: GPtr<u8>, f: CVector, p: CVector) = thiscall 0x542a50 => crate::physical::forces::apply_turn_force;
    /// `CPhysical::SetVelocity(CVector)` (0x441130, `ret 0xC`)
    fn set_velocity(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x441130 => crate::physical::base::set_velocity;
    /// `CAudioEngine::ReportDoorMovement(CPhysical*)` (0x506F50, `ret 4`)
    fn report_door_movement(a: u32, e: GPtr<u8>) = thiscall 0x506f50 => unported;
    /// `CObject::LockDoor()` (0x59F5C0)
    fn object_lock_door(o: GPtr<u8>) = thiscall 0x59f5c0 => super::object::lock_door;
}

/// CRT `_CIpow` (0x822130): `st1 ^ st0`.
fn ci_pow(base: X87, exp: f32) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(0x822130, base.to_f64(), f64::from(exp)))
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;

/// The entity position: matrix +0x30 when it has one (+0x14), else the placement (+4).
fn epos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn zero3(p: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(p, o, 0);
    }
}

fn cv(p: GPtr<u8>) -> CVector {
    CVector { x: rd::<f32>(p, 0), y: rd::<f32>(p, 4), z: rd::<f32>(p, 8) }
}

/// `fcom; fnstsw; test ah, 0x41; jne` not taken: `a > b` (ordered).
fn gt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_any(0x41)
}

/// `fcom; fnstsw; test ah, 5; jnp` taken: exactly C0 of C0/C2, `a < b` (ordered).
fn lt(a: X87, b: X87) -> bool {
    let c = a.fcom(b);
    c.test_any(1) && !c.test_any(4)
}

/// `CObject::ProcessControl()` (0x5A2130). `virt` = the model type (vtable +0x24) is 2, model byte +0x13 bit 0
/// and an RW object (+0x18): the entity is made non-static around the physics.
/// 1. Hit (+0xD8 > 0) while flag 0x80000 of +0x140 is set, not attached (+0xFC), model not rejected by
///    0x59F050, no +0x40 & 0x60, the damager (+0xDC) a model 0x196, or a 0x212 with u16 +0x86C > 0, and not
///    carried by a rope: with byte +0x144 and +0xD8 > [0x858C80] `ObjectDamage(+0xD8, &+0xEC, &+0xE0, damager,
///    0x31)`; with +0xE8 > [0x858C24]: byte +0xB8 = 0, `rel = moveSpeed - damager.GetSpeed(pos - damager pos)`;
///    `|rel|^2 <= [0x866F50]` and `|turnSpeed|^2 <= [0x866F54]` (ordered) → `AttachEntityToEntity(damager,
///    null, null)`, +0x9C = 0.2.
/// 2. Flag 0x80000 cleared. Unless static / waiting (+0x1C bits 0x10, 4, 0x40000) or +0x40 & 0xE0: average
///    move / turn speed (+0x74, +0x80) = (avg + speed) / 2 (CRT-style helpers); `t = f32(timeStep *
///    [0x859CD8])`; both averages' squared magnitudes below `t*t` (strictly, NaN fails) → ++byte +0xB8, and
///    above 10 unless +0x40 bit 0x2000000: +0xB8 = 10, `SetIsStatic(true)` (unless `virt`), speeds +0x44 /
///    +0x50 and +0x5C / +0x68 zeroed, return. A failed test: +0xB8 = 0.
/// 3. Not static: `CPhysical::ProcessControl`; `virt` → `SetIsStatic(false)`. `ProcessBuoyancy(this, +0xA0)`:
///    floating → +0x40 |= 0x8000100, `SetIsStatic(false)`, move force, turn force at the buoyancy point, the
///    six speed floats *= `pow([0x866FD0] (double), timeStep)`; else +0x40 bit 0x8000000 cleared unless the
///    model is [0x8CD608].
/// 4. Breakable on touch (object info +0x160 byte +0x1F, +0x140 & 0x40, +0x1C bit 7) and `rand() & 0x1F ==
///    10`: +0x40 |= 0x800000, +0x1C &= ~0x81, `SetVelocity(0)`, +0x40 &= ~2, vtable +0x20.
/// 5. Model [0x8CD634]: turn force `k * moveSpeed` (k = [0x866F48] * +0x90, components f32) at
///    `-matrix.forward`; move speed *= `pow(1 - |moveSpeed|^2 * [0x866F4C], timeStep)`.
/// 6. +0x1C bit 0x100 → bit 0x20.
/// 7. Door (+0x40 & 0x20) with +0x158 > [0x859948]: the heading (raw FPATAN) wrapped by 2PI towards +0x158;
///    `d = f32(+0x158 - heading)`; |d| > [0x866F58] → +0x140 |= 0x8000; d clamped to ±[0x8D0A34]; (d > 0 and
///    +0x58 < [0x8D0A30]) or (d <= 0 and +0x58 >= -[0x8D0A30]): +0x58 += timeStep * [0x8D0A2C] * d; d != 0 (or
///    NaN) with +0x140 & 0x4000 → `ReportDoorMovement`; not static, |d| <= [0x858C58] and (+0x140 & 0x4000 or
///    |+0x58| <= [0x858C58]): `SetIsStatic(true)`, speeds zeroed, `LockDoor` with +0x140 bits 0x4000 and 0x8000.
#[hook(addr = 0x5A2130, conv = thiscall, name = "CObject::ProcessControl", expect = "83 EC 40 53 55 56 8B F1",
       unverified = "mvpe-ent-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "rand at 0x5A2608 (at_site 0x5A260D); _CIpow (0x822130) twice; FPATAN heading of doors")]
pub fn object_process_control(this: GPtr<u8>) {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    let virt = v_model_type(mi) == 2 && rd::<u8>(mi, 0x13) & 1 != 0 && !rd::<GPtr<u8>>(this, 0x18).is_null();
    let zero = g::<f32>(0x858B50).read();
    attach_to_damager(this, zero);
    // 2. coming to rest
    let fl = rd::<u32>(this, 0x1C);
    wr::<u32>(this, 0x140, rd::<u32>(this, 0x140) & 0xFFF7_FFFF);
    let pf = rd::<u32>(this, 0x40);
    if fl & 0x10 == 0 && fl & 4 == 0 && fl & 0x40000 == 0 && pf & 0xE0 == 0 {
        let mut tmp = [0u32; 3];
        let mut q = [0u32; 3];
        for (avg, sp) in [(0x74isize, 0x44isize), (0x80, 0x50)] {
            let r = vector_add(local(&mut tmp), this.byte_add(avg), this.byte_add(sp));
            let r = vector_div(local(&mut q), r, 2.0);
            for o in [0, 4, 8] {
                wr::<u32>(this, avg + o, rd::<u32>(r, o));
            }
        }
        let t = f32_of(x(g::<f32>(TIME_STEP).read()).fmul_m(g::<f32>(0x859CD8).read()));
        let below = |v: GPtr<u8>| {
            let m = X87::from_f64_exact(vector_sq_mag(v));
            gt(x(t).fmul_m(t), m)
        };
        if below(this.byte_add(0x74)) && below(this.byte_add(0x80)) {
            let c = rd::<u8>(this, 0xB8).wrapping_add(1);
            wr::<u8>(this, 0xB8, c);
            if c > 10 && (pf >> 0x19) & 1 == 0 {
                wr::<u8>(this, 0xB8, 10);
                if !virt {
                    v_set_is_static(this, 1);
                }
                for o in [0x44, 0x5C, 0x68, 0x50] {
                    zero3(this.byte_add(o));
                }
                return;
            }
        } else {
            wr::<u8>(this, 0xB8, 0);
        }
    }
    // 3. physics and buoyancy
    let fl = rd::<u32>(this, 0x1C);
    if fl & 4 == 0 && fl & 0x40000 == 0 {
        physical_process_control(this);
    }
    if virt {
        v_set_is_static(this, 0);
    }
    let mut turn = CVector::default();
    let mut force = CVector::default();
    let floating = process_buoyancy(
        g::<u8>(0xC1C890),
        this,
        rd::<f32>(this, 0xA0),
        GPtr::from_raw(&mut turn as *mut CVector),
        GPtr::from_raw(&mut force as *mut CVector),
    );
    if floating != 0 {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x800_0100);
        v_set_is_static(this, 0);
        apply_move_force(this, force);
        apply_turn_force(this, force, turn);
        let f = ci_pow(X87::from_f64_exact(g::<f64>(0x866FD0).read()), g::<f32>(TIME_STEP).read());
        for o in [0x44, 0x48, 0x4C, 0x50, 0x54, 0x58] {
            wr::<f32>(this, o, f32_of(f.fmul_m(rd::<f32>(this, o))));
        }
    } else if i32::from(rd::<i16>(this, 0x22)) != i32::from(g::<u16>(0x8CD608).read()) {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xF7FF_FFFF);
    }
    // 4. break on touch
    let od = rd::<GPtr<u8>>(this, 0x160);
    if rd::<u8>(od, 0x1F) != 0
        && rd::<u8>(this, 0x140) & 0x40 != 0
        && rd::<u8>(this, 0x1C) & 0x80 != 0
        && sa_core::orig::at_site(0x5A260D, sa_core::crt::rand) & 0x1F == 10
    {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x80_0000);
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xFFFF_FF7E);
        set_velocity(this, 0, 0, 0);
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xFFFF_FFFD);
        v_slot_20(this);
    }
    // 5. wind-driven model
    if i32::from(rd::<i16>(this, 0x22)) == i32::from(g::<u16>(0x8CD634).read()) {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let p = CVector {
            x: f32_of(x(rd::<f32>(m, 0x10)).fchs()),
            y: f32_of(x(rd::<f32>(m, 0x14)).fchs()),
            z: f32_of(x(rd::<f32>(m, 0x18)).fchs()),
        };
        let k = x(g::<f32>(0x866F48).read()).fmul_m(rd::<f32>(this, 0x90));
        let f = CVector {
            x: f32_of(k.fmul_m(rd::<f32>(this, 0x44))),
            y: f32_of(k.fmul_m(rd::<f32>(this, 0x48))),
            z: f32_of(k.fmul_m(rd::<f32>(this, 0x4C))),
        };
        apply_turn_force(this, f, p);
        let (vx, vy, vz) = (rd::<f32>(this, 0x44), rd::<f32>(this, 0x48), rd::<f32>(this, 0x4C));
        let s = x(vx).fmul_m(vx).fadd(x(vy).fmul_m(vy)).fadd(x(vz).fmul_m(vz));
        let b = s.fmul_m(g::<f32>(0x866F4C).read()).fsubr_m(g::<f32>(0x858624).read());
        let f2 = ci_pow(b, g::<f32>(TIME_STEP).read());
        for o in [0x44, 0x48, 0x4C] {
            wr::<f32>(this, o, f32_of(f2.fmul_m(rd::<f32>(this, o))));
        }
    }
    // 6.
    let fl = rd::<u32>(this, 0x1C);
    if fl & 0x100 != 0 {
        wr::<u32>(this, 0x1C, fl | 0x20);
    }
    // 7. swinging door
    if rd::<u8>(this, 0x40) & 0x20 != 0 && gt(x(rd::<f32>(this, 0x158)), x(g::<f32>(0x859948).read())) {
        swing_door(this, zero);
    }
}

/// Step 1 of [`object_process_control`].
fn attach_to_damager(this: GPtr<u8>, zero: f32) {
    if !gt(x(rd::<f32>(this, 0xD8)), x(zero)) || rd::<u32>(this, 0x140) & 0x80000 == 0 {
        return;
    }
    if !rd::<GPtr<u8>>(this, 0xFC).is_null() || model_check_59f050(i32::from(rd::<i16>(this, 0x22))) != 0 {
        return;
    }
    if rd::<u8>(this, 0x40) & 0x60 != 0 {
        return;
    }
    let d = rd::<GPtr<u8>>(this, 0xDC);
    if d.is_null() {
        return;
    }
    let m = rd::<u16>(d, 0x22);
    if m != 0x196 && (m != 0x212 || rd::<u16>(d, 0x86C) == 0) {
        return;
    }
    if ropes_is_carried_by_rope(this) != 0 {
        return;
    }
    if rd::<u8>(this, 0x144) != 0 && gt(x(rd::<f32>(this, 0xD8)), x(g::<f32>(0x858C80).read())) {
        object_damage(this, rd::<u32>(this, 0xD8), this.byte_add(0xEC), this.byte_add(0xE0), rd::<GPtr<u8>>(this, 0xDC), 0x31);
    }
    if !gt(x(rd::<f32>(this, 0xE8)), x(g::<f32>(0x858C24).read())) {
        return;
    }
    let d = rd::<GPtr<u8>>(this, 0xDC);
    wr::<u8>(this, 0xB8, 0);
    let mut rel = [0u32; 3];
    let rp = local(&mut rel);
    vector_sub(rp, epos(this), epos(d));
    let mut sp = [0u32; 3];
    let s = physical_get_speed(d, local(&mut sp).cast(), cv(rp));
    vector_sub(rp, this.byte_add(0x44), s.cast());
    if X87::from_f64_exact(vector_sq_mag(rp)).fcom(x(g::<f32>(0x866F50).read())).test_parity(5) {
        return;
    }
    if X87::from_f64_exact(vector_sq_mag(this.byte_add(0x50))).fcom(x(g::<f32>(0x866F54).read())).test_parity(5) {
        return;
    }
    attach_quat(this, d, GPtr::NULL, GPtr::NULL);
    wr::<u32>(this, 0x9C, 0x3E4C_CCCD);
}

/// Step 7 of [`object_process_control`] (door swing towards +0x158).
fn swing_door(this: GPtr<u8>, zero: f32) {
    let pi = g::<f32>(0x858CB8).read();
    let two_pi = g::<f32>(0x858CBC).read();
    let tgt = rd::<f32>(this, 0x158);
    let mut h = get_heading(this);
    if gt(h, x(tgt).fadd_m(pi)) {
        h = h.fsub_m(two_pi);
    } else if !h.fcom(x(tgt).fsub_m(pi)).test_parity(5) {
        h = h.fadd_m(two_pi);
    }
    let mut d = f32_of(x(tgt).fsub(h));
    if gt(x(d).fabs(), x(g::<f32>(0x866F58).read())) {
        wr::<u32>(this, 0x140, rd::<u32>(this, 0x140) | 0x8000);
    }
    let lim = g::<f32>(0x8D0A34).read();
    if gt(x(d), x(lim)) {
        d = lim;
    } else {
        let nl = x(lim).fchs();
        if !x(d).fcom(nl).test_parity(5) {
            d = f32_of(nl);
        }
    }
    let vlim = g::<f32>(0x8D0A30).read();
    let v58 = rd::<f32>(this, 0x58);
    let apply = (gt(x(d), x(zero)) && lt(x(v58), x(vlim)))
        || (!x(d).fcom(x(zero)).test_parity(5) && !x(vlim).fchs().fcom(x(v58)).test_parity(5));
    if apply {
        let v = x(g::<f32>(TIME_STEP).read()).fmul_m(g::<f32>(0x8D0A2C).read()).fmul_m(d).fadd_m(v58);
        wr::<f32>(this, 0x58, f32_of(v));
    }
    // `test ah, 0x44; jnp` skips only when equal (C3 without C2)
    let c = x(d).fcom(x(zero));
    let equal = c.test_any(0x40) && !c.test_any(4);
    if !equal && rd::<u32>(this, 0x140) & 0x4000 != 0 {
        report_door_movement(0xB6BC90, this);
    }
    let fl = rd::<u32>(this, 0x1C);
    if fl & 0x10 != 0 || fl & 4 != 0 || fl & 0x40000 != 0 {
        return;
    }
    let small = g::<f32>(0x858C58).read();
    if x(d).fabs().fcom(x(small)).test_parity(5) {
        return;
    }
    if rd::<u32>(this, 0x140) & 0x4000 == 0 && x(rd::<f32>(this, 0x58)).fabs().fcom(x(small)).test_parity(5) {
        return;
    }
    v_set_is_static(this, 1);
    for o in [0x44, 0x5C, 0x50, 0x68] {
        zero3(this.byte_add(o));
    }
    let f = rd::<u32>(this, 0x140);
    if f & 0x4000 != 0 && f & 0x8000 != 0 {
        object_lock_door(this);
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ENTITY_CREATE_EFFECTS, HOOK_OBJECT_PROCESS_CONTROL];
