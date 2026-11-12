//! `CRope::Update` and `CRope::UpdateWeightInRope` (batch `std9-render-2`): ported from the 1.0 US bytes (neither is
//! reversed in gta-reversed). Unverified (docs/verify/HOWTO.md).
//!
//! `CRope` (0x328): `m_aSegments[32]` +0, `m_aSpeed[32]` +0x180, `m_nId` +0x300, ground z +0x304, the weight +0x308,
//! the segment length +0x30C, the holder +0x310, the hook object (`m_pAttachedEntity`) +0x314, the picked-up entity
//! (`m_pRopeAttachObject`) +0x318, the cable length factor +0x31C, the expiry time +0x320, the segment count byte
//! +0x324, the type +0x325, the pick-up cooldown +0x326, the flags +0x327.
//!
//! `CRope::Update` keeps most of its intermediate values in stack slots that several sections reuse (one path reads a
//! slot another path wrote); the port keeps the original's frame as [`Frame`] (offsets relative to ESP after the four
//! register pushes) so that every such read sees what the original sees. Slots the original reads before any write
//! are zero here (`uninit-locals`).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, rd, wr, x, xi};

orig! {
    /// `CWorld::FindGroundZFor3DCoord(x, y, z, bool*, CEntity**)` (0x5696C0, ST0)
    fn find_ground_z_for_3d_coord(px: f32, py: f32, pz: f32, found: u32, entity: u32) -> f64 = cdecl 0x5696C0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetCarGunUpDown()` (0x53FC10, AX)
    fn pad_car_gun_up_down(pad: GPtr<u8>) -> u32 = thiscall 0x53FC10 => unported;
    /// `CPad::CarGunJustDown()` (0x53FFE0, AX)
    fn pad_car_gun_just_down(pad: GPtr<u8>) -> u32 = thiscall 0x53FFE0 => unported;
    /// `CAudioEngine::ReportMissionAudioEvent(int32 event, CPhysical*, float, float)` (0x5073B0)
    fn report_mission_audio_event(engine: GPtr<u8>, event: u32, physical: u32, a3: u32, a4: u32) = thiscall 0x5073B0 => unported;
    /// `CRope::UpdateWeightInRope(CVector pos, float, CVector* out)` (0x5561B0, AL)
    fn rope_update_weight_call(rope: GPtr<u8>, px: u32, py: u32, pz: u32, a5: u32, out: GPtr<u8>) -> u8 = thiscall 0x5561B0 => rope_update_weight;
    /// `CPhysical::GetSpeed(CVector* out, CVector point)` (0x542CE0, returns `out`)
    fn physical_get_speed(e: GPtr<u8>, out: GPtr<u8>, px: u32, py: u32, pz: u32) -> u32 = thiscall 0x542CE0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0)
    fn matrix_copy_ctor(this: GPtr<u8>, src: GPtr<u8>) -> u32 = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(this: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::ForceUpVector(CVector up)` (0x59B7E0)
    fn matrix_force_up_vector(m: GPtr<u8>, ux: u32, uy: u32, uz: u32) = thiscall 0x59B7E0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> u32 = thiscall 0x59BBC0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, the hidden `out` first)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> u32 = cdecl 0x59C890 => unported;
    /// `CRope::ReleasePickedUpObject()` (0x556030)
    fn rope_release_picked_up_object(rope: GPtr<u8>) = thiscall 0x556030 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0, AL)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn placeable_set_posn(e: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4241C0 => unported;
    /// `CEntity::SetIsStatic(bool)` (virtual slot 4)
    fn entity_set_is_static(e: GPtr<u8>, v: u32) = thiscall vslot 0x10 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn physical_add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `std::max<float>` (0x420800, ST0)
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => unported;
    /// `CRope::Remove()` (0x556780)
    fn rope_remove(rope: GPtr<u8>) = thiscall 0x556780 => unported;
}

/// `CTimer::ms_fTimeStep`, `CTimer::m_snTimeInMilliseconds`, `CTimer::m_FrameCounter`
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
const FRAME_COUNTER: usize = 0xB7CB4C;
/// `TheCamera`'s matrix pointer and placement position
const CAMERA_MATRIX: usize = 0xB6F03C;
const CAMERA_PLACEMENT_POS: usize = 0xB6F02C;
/// `CRopes::PlayerControlsCrane`
const PLAYER_CONTROLS_CRANE: usize = 0xB76898;
/// The three crane control bytes (0xA44494 release, 0xA44495 down, 0xA44496 up)
const CRANE_RELEASE: usize = 0xA44494;
const CRANE_DOWN: usize = 0xA44495;
const CRANE_UP: usize = 0xA44496;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CPools::ms_pPedPool`, `ms_pVehiclePool`, `ms_pObjectPool`
const PED_POOL: usize = 0xB74490;
const VEHICLE_POOL: usize = 0xB74494;
const OBJECT_POOL: usize = 0xB7449C;
/// `CRT _CIpow`
const CI_POW: usize = 0x822130;

/// An f32 of the exe's data.
fn c(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// The u16 model index stored at `addr` (`ModelIndices`).
fn mi(addr: usize) -> i32 {
    i32::from(g::<u16>(addr).read())
}

/// An entity's position (matrix +0x30, else the placement +4).
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The bounding box's max z of an entity's model (`ms_modelInfoPtrs[model]->m_pColModel->m_boundBox.m_vecMax.z`).
fn model_max_z(e: GPtr<u8>) -> f32 {
    let model = rd::<i16>(e, 0x22) as isize;
    let info = g::<GPtr<u8>>(MODEL_INFO_PTRS + (model as usize).wrapping_mul(4)).read();
    rd::<f32>(rd::<GPtr<u8>>(info, 0x14), 0x14)
}

/// `CRope::Update`'s stack frame (offsets from ESP after the four register pushes; 0x140 bytes of locals).
struct Frame([u32; 0x150 / 4]);

impl Frame {
    fn d(&self, o: usize) -> u32 {
        self.0[o / 4]
    }
    fn setd(&mut self, o: usize, v: u32) {
        self.0[o / 4] = v;
    }
    fn f(&self, o: usize) -> f32 {
        f32::from_bits(self.0[o / 4])
    }
    fn x(&self, o: usize) -> X87 {
        x(self.f(o))
    }
    fn setf(&mut self, o: usize, v: X87) {
        self.0[o / 4] = f32_of(v).to_bits();
    }
    fn p(&mut self, o: usize) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr()).cast::<u8>().byte_add(o as isize)
    }
    fn b(&self, o: usize) -> u8 {
        self.0[o / 4].to_le_bytes()[o % 4]
    }
    fn setb(&mut self, o: usize, v: u8) {
        let mut w = self.0[o / 4].to_le_bytes();
        w[o % 4] = v;
        self.0[o / 4] = u32::from_le_bytes(w);
    }
    /// Copies three dwords from `src` into the slots at `o`.
    fn copy_in(&mut self, o: usize, src: GPtr<u8>) {
        for i in 0..3 {
            self.setd(o + 4 * i, rd::<u32>(src, 4 * i as isize));
        }
    }
}

/// `sqrt((a*a + b*b) + c*c)` with the operand order of the `fld st(k); fmul st(k+1)` sequences.
fn len3(a: X87, b: X87, cc: X87) -> X87 {
    a.fmul(a).fadd(b.fmul(b)).fadd(cc.fmul(cc)).fsqrt()
}

/// The inlined `CRope::DoControlsApply` (0x557959 / 0x5581F2): a controlled crane (type 4..7 with the matching
/// `PlayerControlsCrane` 1, 2, 4, 3) or a type 1, 2 or 3 rope.
fn controls_apply(ty: u8) -> bool {
    let ctl = g::<u32>(PLAYER_CONTROLS_CRANE).read();
    matches!((ty, ctl), (4, 1) | (5, 2) | (7, 3) | (6, 4)) || matches!(ty, 1..=3)
}

/// The cable length factor (+0x31C) after a pick-up: with the hanging height `dz = seg31.z - pickup.z` above 0,
/// `t = factor - dz * (1 / weight) - (1 / weight) * segLen`, at least 0.01 (`fcom; fst; je`).
fn shorten_after_pickup(rope: GPtr<u8>, dz: X87) {
    if dz.fcom(x(c(0x858B50))).test_any(0x41) {
        return;
    }
    let inv = x(c(0x858624)).fdiv_m(rd::<f32>(rope, 0x308));
    let t = dz.fmul(inv).fsubr_m(rd::<f32>(rope, 0x31C)).fsub(inv.fmul_m(rd::<f32>(rope, 0x30C)));
    let v = if t.fcom(x(c(0x858C58))).test_any(0x41) { c(0x858C58) } else { f32_of(t) };
    wr::<f32>(rope, 0x31C, v);
}

/// The pick-up point of an entity, `matrix * (0, 0, maxZ)`, into the frame at `out` (the vector at `v`).
fn pickup_point(fr: &mut Frame, e: GPtr<u8>, v: usize, out: usize, vec: [u32; 3]) -> GPtr<u8> {
    for (i, w) in vec.iter().enumerate() {
        fr.setd(v + 4 * i, *w);
    }
    let pv = fr.p(v);
    let po = fr.p(out);
    matrix_mul_vec(po, rd::<GPtr<u8>>(e, 0x14), pv);
    po
}

/// The hook object: position = the picked-up entity's pick-up point (its matrix times `(0, 0, maxZ)`).
fn hook_to_pickup_point(rope: GPtr<u8>, fr: &mut Frame, e: GPtr<u8>) {
    let z = model_max_z(e);
    let p = pickup_point(fr, e, 0x14, 0x68, [0, 0, f32_of(x(z)).to_bits()]);
    let hook = rd::<GPtr<u8>>(rope, 0x314);
    let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    let m = rd::<GPtr<u8>>(hook, 0x14);
    let dst = if m.is_null() { hook.byte_add(4) } else { m.byte_add(0x30) };
    wr::<f32>(dst, 0, f32_of(x(px)));
    wr::<f32>(dst, 4, f32_of(x(py)));
    wr::<f32>(dst, 8, f32_of(x(pz)));
}

/// `CRope::Update()` (0x557530, thiscall).
///
/// Nothing beyond 200 units (2D) of the camera. The first segment falls under its own speed unless the rope is held
/// (+0x327 bit 0) or has not expired; with bit 2 the ground z (+0x304) follows the first segment every 8th frame
/// (`FindGroundZFor3DCoord`) or the hook's z - 0.5 while a vehicle or one of four container models is attached. The
/// free segments get damped random speeds (`rand() & 15` twice: x and y), fall by `0.15 * timestep`, stay above the
/// ground + 0.3, are pulled back to one segment length from the previous one and get their speeds from the move.
/// Crane controls change the cable length factor (+0x31C, `[0.01, 0.9]`, `<= 0.84` for the cranes driven by the car
/// gun keys); the hanging entity is moved by `UpdateWeightInRope`, its speed coupled with the holder's and (with a
/// picked-up entity) its matrix turned towards the rope. Release, drop of wrecked vehicles, and the pick-up search
/// (vehicles, the harness's peds, objects by type) follow; a rope below z -50 is removed. Flag bit 0 is cleared.
#[hook(addr = 0x557530, conv = thiscall, name = "CRope::Update", expect = "6A FF 68 88 CC 83 00",
       route = "L2", hazards = "heap-writes,uninit-locals,x87-transcendental", recs = "grove-walk",
       note = "the frame's reused stack slots are modelled (Frame); two direct rand calls at 0x557700 / 0x55771D; \
               _CIpow through orig::x87_binary (libm in phases B/C)")]
pub fn rope_update(rope: GPtr<u8>) {
    let mut fr = Frame([0; 0x150 / 4]);
    fr.setd(0x20, rope.addr() as u32);
    let ts = c(TIME_STEP);
    let damp = sa_core::orig::x87_binary(CI_POW, g::<f64>(0x863E40).read(), f64::from(ts));
    fr.setf(0x60, X87::from_f64_exact(damp));
    // 2D distance from the camera
    let cp = {
        let m = g::<u32>(CAMERA_MATRIX).read();
        if m != 0 { g::<u8>(m as usize + 0x30) } else { g::<u8>(CAMERA_PLACEMENT_POS) }
    };
    let a = x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(rope, 4));
    let b = x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(rope, 0));
    if b.fmul(b).fadd(a.fmul(a)).fsqrt().fcom(x(c(0x858A48))).test_parity(0x05) {
        return;
    }
    let flags = rd::<u8>(rope, 0x327);
    if flags & 1 == 0 && g::<u32>(TIME_MS).read() > rd::<u32>(rope, 0x320) {
        let sz = x(ts).fmul_m(c(0x863E38)).fsubr_m(rd::<f32>(rope, 0x188));
        wr::<f32>(rope, 0x188, f32_of(sz));
        let vz = x(ts).fmul_m(rd::<f32>(rope, 0x188));
        let vy = x(ts).fmul_m(rd::<f32>(rope, 0x184));
        let vx = x(ts).fmul_m(rd::<f32>(rope, 0x180));
        wr::<f32>(rope, 0, f32_of(vx.fadd_m(rd::<f32>(rope, 0))));
        wr::<f32>(rope, 4, f32_of(vy.fadd_m(rd::<f32>(rope, 4))));
        wr::<f32>(rope, 8, f32_of(vz.fadd_m(rd::<f32>(rope, 8))));
    }
    if flags & 4 != 0 && g::<u32>(FRAME_COUNTER).read() & 7 == 2 {
        let z = find_ground_z_for_3d_coord(rd(rope, 0), rd(rope, 4), rd(rope, 8), 0, 0);
        wr::<f32>(rope, 0x304, f32_of(X87::from_f64_exact(z)));
    }
    if rd::<u8>(rope, 0x327) & 4 != 0 {
        let att = rd::<GPtr<u8>>(rope, 0x318);
        if !att.is_null() {
            let m = i32::from(rd::<i16>(att, 0x22));
            if rd::<u8>(att, 0x36) & 7 == 2 || [0x8CD6C8, 0x8CD6CC, 0x8CD6D0, 0x8CD6D8].iter().any(|&a| mi(a) == m) {
                let hz = rd::<f32>(pos_ptr(rd::<GPtr<u8>>(rope, 0x314)), 8);
                wr::<f32>(rope, 0x304, f32_of(x(hz).fsub_m(c(0x858B8C))));
            }
        }
    }
    segments(rope, &mut fr, ts);
    // types other than 1..7 skip to the end (0x557951)
    if matches!(rd::<u8>(rope, 0x325), 1..=7) {
        controls(rope, &mut fr, ts);
        hanging(rope, &mut fr);
        pickup(rope, &mut fr);
    }
    // 0x558CA3 / 0x405FA5
    if rd::<u8>(rope, 0x327) & 1 == 0 && !x(rd::<f32>(rope, 8)).fcom(x(c(0x859B60))).test_parity(0x05) {
        rope_remove(rope);
    }
    wr::<u8>(rope, 0x327, rd::<u8>(rope, 0x327) & 0xFE);
}

/// The free segments (0x5576AD), `k = count + 1 ..= 31`.
fn segments(rope: GPtr<u8>, fr: &mut Frame, ts: f32) {
    let n = usize::from(rd::<u8>(rope, 0x324));
    if n + 1 >= 32 {
        return;
    }
    fr.setf(0x50, x(c(0x858624)).fsub(fr.x(0x60)));
    for k in n + 1..32 {
        let s = rope.byte_add(12 * k as isize);
        let v = rope.byte_add(0x180 + 12 * k as isize);
        let pv = v.byte_add(-12);
        let ps = s.byte_add(-12);
        fr.copy_in(0x24, s);
        for (i, ret) in [(0isize, 0x557705), (4, 0x557722)] {
            let r = (sa_core::orig::at_site(ret, sa_core::crt::rand) & 0xF) - 8;
            fr.setd(0x34, r as u32);
            let nv = xi(r).fmul_m(c(0x858CDC)).fadd_m(rd::<f32>(v, i));
            wr::<f32>(v, i, f32_of(nv));
        }
        let d = fr.f(0x60);
        let zd = x(d).fmul_m(rd::<f32>(v, 8));
        fr.setf(0x10, x(d).fmul_m(rd::<f32>(v, 4)));
        fr.setf(0x8C, x(d).fmul_m(rd::<f32>(v, 0)));
        let zp = fr.x(0x50).fmul_m(rd::<f32>(pv, 8));
        fr.setf(0x34, fr.x(0x50).fmul_m(rd::<f32>(pv, 4)));
        fr.setf(0x74, fr.x(0x50).fmul_m(rd::<f32>(pv, 0)));
        let nz = zd.fadd(zp);
        let ny = fr.x(0x34).fadd_m(fr.f(0x10));
        let nx = fr.x(0x74).fadd_m(fr.f(0x8C));
        fr.setf(0x54, nx);
        fr.setf(0x58, ny);
        fr.setf(0x5C, nz);
        for i in 0..3 {
            wr::<u32>(v, 4 * i as isize, fr.d(0x54 + 4 * i));
        }
        // fall
        let z = x(ts).fmul_m(c(0x858FCC)).fsubr_m(rd::<f32>(s, 8));
        fr.setf(0x10, z);
        wr::<f32>(s, 8, f32_of(z));
        if rd::<u8>(rope, 0x327) & 4 != 0 {
            let gz = x(rd::<f32>(rope, 0x304)).fadd_m(c(0x858C24));
            let v = if fr.x(0x10).fcom(gz).test_any(0x41) { gz } else { fr.x(0x10) };
            wr::<f32>(s, 8, f32_of(v));
        }
        // one segment length from the previous one
        let dz = x(rd::<f32>(s, 8)).fsub_m(rd::<f32>(ps, 8));
        fr.setf(0x10, dz);
        let dy = x(rd::<f32>(s, 4)).fsub_m(rd::<f32>(ps, 4));
        let dx = x(rd::<f32>(s, 0)).fsub_m(rd::<f32>(ps, 0));
        fr.setf(0x34, dx);
        let sum = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(fr.x(0x10).fmul_m(fr.f(0x10)));
        fr.setf(0x10, sum);
        fr.setf(0x30, dz);
        let dxs = fr.x(0x34);
        let k = fr.x(0x10).fsqrt().fdivr_m(rd::<f32>(rope, 0x30C));
        fr.setf(0x30, fr.x(0x30).fmul(k));
        fr.setf(0x34, dy.fmul(k));
        fr.setf(0x80, dxs.fmul(k));
        let z = fr.x(0x30).fadd_m(rd::<f32>(ps, 8));
        let y = fr.x(0x34).fadd_m(rd::<f32>(ps, 4));
        let xx = fr.x(0x80).fadd_m(rd::<f32>(ps, 0));
        fr.setf(0x40, xx);
        fr.setf(0x44, y);
        fr.setf(0x48, z);
        for i in 0..3 {
            wr::<u32>(s, 4 * i as isize, fr.d(0x40 + 4 * i));
        }
        // speed from the move
        let mz = x(rd::<f32>(s, 8)).fsub_m(fr.f(0x2C));
        fr.setf(0x30, x(rd::<f32>(s, 4)).fsub_m(fr.f(0x28)));
        fr.setf(0x68, x(rd::<f32>(s, 0)).fsub_m(fr.f(0x24)));
        let inv = x(c(0x858624)).fdiv_m(ts);
        fr.setf(0x10, inv);
        let sz = mz.fmul(inv);
        let sy = fr.x(0x10).fmul_m(fr.f(0x30));
        let sx = fr.x(0x10).fmul_m(fr.f(0x68));
        fr.setf(0x14, sx);
        fr.setf(0x18, sy);
        fr.setf(0x1C, sz);
        for i in 0..3 {
            wr::<u32>(v, 4 * i as isize, fr.d(0x14 + 4 * i));
        }
    }
}

/// One car gun key's cable change for a controlled crane (0x5579AD / 0x557A58): `delta = stick * timestep * 1e-5`;
/// a positive delta that keeps the factor below 0.9 (raising) / above 0.01 (lowering) reports mission audio event
/// 0x68 for the holder; the factor changes by delta.
fn crane_key(rope: GPtr<u8>, fr: &mut Frame, ts: f32, stick_off: isize, raise: bool) {
    fr.setd(0x34, ts.to_bits());
    let pad = pad_get(0);
    fr.setd(0x30, i32::from(rd::<i16>(pad, stick_off)) as u32);
    let ty = rd::<u8>(rope, 0x325);
    fr.setf(0x10, xi(fr.d(0x30) as i32).fmul_m(fr.f(0x34)).fmul_m(c(0x858C14)));
    if matches!(ty, 4..=7) && !fr.x(0x10).fcom(x(c(0x858B50))).test_any(0x41) {
        let fire = if raise {
            !fr.x(0x10).fadd_m(rd::<f32>(rope, 0x31C)).fcom(x(c(0x858C20))).test_parity(0x05)
        } else {
            !x(rd::<f32>(rope, 0x31C)).fsub_m(fr.f(0x10)).fcom(x(c(0x858C58))).test_any(0x41)
        };
        if fire {
            report_mission_audio_event(g(AUDIO_ENGINE), 0x68, rd::<u32>(rope, 0x310), 0, 0x3F80_0000);
        }
    }
    let v = if raise {
        fr.x(0x10).fadd_m(rd::<f32>(rope, 0x31C))
    } else {
        x(rd::<f32>(rope, 0x31C)).fsub_m(fr.f(0x10))
    };
    wr::<f32>(rope, 0x31C, f32_of(v));
}

/// The cable length controls (0x55792D .. 0x557BC7) and the clamp of the factor to `[0.01, 0.9]`.
fn controls(rope: GPtr<u8>, fr: &mut Frame, ts: f32) {
    let ty = rd::<u8>(rope, 0x325);
    if controls_apply(ty) {
        if matches!(ty, 1..=3) {
            let ax = pad_car_gun_up_down(pad_get(0)) as u16 as i16;
            let apply = if ax < 0 { g::<u8>(CRANE_UP).read() != 0 } else { ax > 0 && g::<u8>(CRANE_DOWN).read() != 0 };
            if apply {
                fr.setd(0x30, i32::from(ax) as u32);
                let v = xi(i32::from(ax)).fmul_m(ts).fmul_m(c(0x858C14)).fsubr_m(rd::<f32>(rope, 0x31C));
                wr::<f32>(rope, 0x31C, f32_of(v));
            }
            let v = rd::<f32>(rope, 0x31C);
            let v = if x(c(0x863E34)).fcom(x(v)).test_parity(0x05) { v } else { c(0x863E34) };
            wr::<f32>(rope, 0x31C, f32_of(x(v)));
        } else {
            if g::<u8>(CRANE_UP).read() != 0 {
                crane_key(rope, fr, ts, 0x1C, true);
            }
            if g::<u8>(CRANE_DOWN).read() != 0 {
                crane_key(rope, fr, ts, 0x20, false);
            }
        }
    }
    let v = rd::<f32>(rope, 0x31C);
    let v = if x(c(0x858C58)).fcom(x(v)).test_any(0x41) { x(v) } else { x(c(0x858C58)) };
    wr::<f32>(rope, 0x31C, f32_of(v));
    let v = if x(c(0x858C20)).fcom(v).test_parity(0x05) { v } else { x(c(0x858C20)) };
    wr::<f32>(rope, 0x31C, f32_of(v));
}

/// The hanging entity (0x557BC1 .. 0x5581C7): the picked-up entity (its weight factor from its mass) or the hook.
fn hanging(rope: GPtr<u8>, fr: &mut Frame) {
    let att_cell = rope.byte_add(0x318);
    let mut e = rd::<GPtr<u8>>(rope, 0x318);
    if !e.is_null() {
        let mass = if rd::<u16>(e, 0x22) == 0x1AC { c(0x859A98) } else { rd::<f32>(e, 0x8C) };
        let w = x(c(0x8CD89C)).fmul_m(c(0x863E30)).fmul(x(mass)).fadd_m(c(0x8CD898));
        let v = if x(c(0x858B8C)).fcom(w).test_parity(0x05) { f32_of(w) } else { 0.5 };
        fr.setd(0x10, v.to_bits());
        wr::<u8>(e, 0xB8, 0);
    } else {
        e = rd::<GPtr<u8>>(rope, 0x314);
        wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) | 1);
        fr.setd(0x10, g::<u32>(0x8CD898).read());
    }
    if !e.is_null() {
        let p = pos_ptr(e);
        let out = fr.p(0x24);
        let taut = rope_update_weight_call(rope, rd(p, 0), rd(p, 4), rd(p, 8), 0x3DCC_CCCD, out);
        if taut != 0 {
            let dst = pos_ptr(e);
            for i in 0..3 {
                wr::<u32>(dst, 4 * i as isize, fr.d(0x24 + 4 * i));
            }
            let sp = e.byte_add(0x44);
            fr.copy_in(0x24, sp);
            let holder = rd::<GPtr<u8>>(rope, 0x310);
            let hp = pos_ptr(holder);
            let rz = f32_of(x(rd::<f32>(rope, 8)).fsub_m(rd::<f32>(hp, 8)));
            let ry = f32_of(x(rd::<f32>(rope, 4)).fsub_m(rd::<f32>(hp, 4)));
            let rx = f32_of(x(rd::<f32>(rope, 0)).fsub_m(rd::<f32>(hp, 0)));
            fr.setd(0x14, rx.to_bits());
            fr.setd(0x18, ry.to_bits());
            fr.setd(0x1C, rz.to_bits());
            let o = fr.p(0x54);
            physical_get_speed(holder, o, rx.to_bits(), ry.to_bits(), rz.to_bits());
            if matches!(rd::<u8>(rope, 0x325), 4..=7) {
                for i in 0..3 {
                    fr.setd(0x14 + 4 * i, 0);
                    fr.setd(0x54 + 4 * i, 0);
                }
            }
            for i in 0..3 {
                let o = 0x24 + 4 * i;
                fr.setf(o, fr.x(o).fsub_m(fr.f(0x54 + 4 * i)));
            }
            let p = pos_ptr(e);
            fr.setf(0x40, x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(rope, 0)));
            fr.setf(0x44, x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(rope, 4)));
            fr.setf(0x48, x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(rope, 8)));
            let dirp = fr.p(0x40);
            vector_normalise(dirp);
            let mut vx = fr.x(0x24);
            let dot = fr.x(0x48).fmul_m(fr.f(0x2C)).fadd(fr.x(0x40).fmul(vx)).fadd(fr.x(0x44).fmul_m(fr.f(0x28)));
            fr.setf(0x50, dot);
            if !dot.fcom(x(c(0x858B50))).test_any(0x41) {
                let z = fr.x(0x50).fmul_m(fr.f(0x48));
                let y = fr.x(0x50).fmul_m(fr.f(0x44));
                let xx = fr.x(0x40).fmul_m(fr.f(0x50));
                vx = vx.fsub(xx);
                fr.setf(0x28, fr.x(0x28).fsub(y));
                fr.setf(0x2C, fr.x(0x2C).fsub(z));
            }
            let sx = vx.fadd_m(fr.f(0x54));
            let sy = fr.x(0x58).fadd_m(fr.f(0x28));
            let sz = fr.x(0x5C).fadd_m(fr.f(0x2C));
            fr.setf(0x50, sz.fsub_m(rd::<f32>(sp, 8)));
            fr.setf(0x34, sy.fsub_m(rd::<f32>(sp, 4)));
            let dxs = sx.fsub_m(rd::<f32>(sp, 0));
            let ow = x(c(0x858624)).fsub_m(fr.f(0x10));
            fr.setf(0x30, fr.x(0x50).fmul(ow));
            fr.setf(0x64, fr.x(0x34).fmul(ow));
            fr.setf(0x68, dxs.fmul(ow));
            let nz = fr.x(0x30).fadd_m(rd::<f32>(sp, 8));
            let ny = fr.x(0x64).fadd_m(rd::<f32>(sp, 4));
            let nx = fr.x(0x68).fadd_m(rd::<f32>(sp, 0));
            fr.setf(0x14, nx);
            fr.setf(0x18, ny);
            fr.setf(0x1C, nz);
            for i in 0..3 {
                wr::<u32>(sp, 4 * i as isize, fr.d(0x14 + 4 * i));
            }
            fr.setf(0x64, fr.x(0x50).fmul_m(fr.f(0x10)));
            fr.setf(0x30, fr.x(0x34).fmul_m(fr.f(0x10)));
            fr.setf(0x68, dxs.fmul_m(fr.f(0x10)));
            let hs = rd::<GPtr<u8>>(rope, 0x310).byte_add(0x44);
            let hz = x(rd::<f32>(hs, 8)).fsub_m(fr.f(0x64));
            let hy = x(rd::<f32>(hs, 4)).fsub_m(fr.f(0x30));
            let hx = x(rd::<f32>(hs, 0)).fsub_m(fr.f(0x68));
            fr.setf(0x14, hx);
            fr.setf(0x18, hy);
            fr.setf(0x1C, hz);
            for i in 0..3 {
                wr::<u32>(hs, 4 * i as isize, fr.d(0x14 + 4 * i));
            }
            let up = |fr: &Frame, o: usize| f32_of(fr.x(o).fchs()).to_bits();
            if rd::<u32>(att_cell, 0) != 0 {
                let m = rd::<GPtr<u8>>(e, 0x14);
                let old = fr.p(0xEC);
                matrix_copy_ctor(old, m);
                let (ux, uy, uz) = (up(fr, 0x40), up(fr, 0x44), up(fr, 0x48));
                fr.setd(0x14, ux);
                fr.setd(0x18, uy);
                fr.setd(0x1C, uz);
                matrix_force_up_vector(m, ux, uy, uz);
                let new = fr.p(0xA4);
                matrix_copy_ctor(new, rd::<GPtr<u8>>(e, 0x14));
                for o in [0usize, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28] {
                    let v = fr.x(0xEC + o).fmul_m(c(0x858C20)).fadd(fr.x(0xA4 + o).fmul_m(c(0x858B1C)));
                    wr::<f32>(rd::<GPtr<u8>>(e, 0x14), o as isize, f32_of(v));
                }
                let new = fr.p(0xA4);
                matrix_dtor(new);
                let old = fr.p(0xEC);
                matrix_dtor(old);
            } else {
                let (ux, uy, uz) = (up(fr, 0x40), up(fr, 0x44), up(fr, 0x48));
                fr.setd(0x14, ux);
                fr.setd(0x18, uy);
                fr.setd(0x1C, uz);
                matrix_force_up_vector(rd::<GPtr<u8>>(e, 0x14), ux, uy, uz);
            }
        }
        // 0x558121
        let att = rd::<GPtr<u8>>(att_cell, 0);
        if !att.is_null() {
            let hook = rd::<GPtr<u8>>(rope, 0x314);
            matrix_assign(rd::<GPtr<u8>>(hook, 0x14), rd::<GPtr<u8>>(att, 0x14));
            for i in 0..3 {
                wr::<u32>(hook, 0x44 + 4 * i, rd::<u32>(att, 0x44 + 4 * i));
            }
            hook_to_pickup_point(rope, fr, att);
        }
    }
}

/// The release / drop checks and the pick-up search (0x5581CA .. 0x558C9F).
fn pickup(rope: GPtr<u8>, fr: &mut Frame) {
    fr.copy_in(0x40, rope.byte_add(0x174));
    let att_cell = rope.byte_add(0x318);
    fr.setd(0x60, att_cell.addr() as u32);
    if rd::<u32>(att_cell, 0) != 0 {
        if controls_apply(rd::<u8>(rope, 0x325))
            && g::<u8>(CRANE_RELEASE).read() != 0
            && pad_car_gun_just_down(pad_get(0)) as u16 != 0
        {
            rope_release_picked_up_object(rope);
        }
        let a = rd::<GPtr<u8>>(att_cell, 0);
        if !a.is_null() && rd::<u32>(a, 0x40) & 0x2000_0000 != 0 {
            wr::<u32>(a, 0x40, rd::<u32>(a, 0x40) & 0xFDFF_FFFF);
            let a = rd::<GPtr<u8>>(att_cell, 0);
            wr::<u32>(a, 0x40, rd::<u32>(a, 0x40) & 0x7FFF_FFFF);
            let hook = rd::<GPtr<u8>>(rope, 0x314);
            wr::<u32>(att_cell, 0, 0);
            wr::<u32>(hook, 0x1C, rd::<u32>(hook, 0x1C) | 1);
            wr::<u8>(rope, 0x326, 0x3C);
        }
        let a = rd::<GPtr<u8>>(att_cell, 0);
        if !a.is_null() && rd::<u8>(a, 0x36) & 7 == 2 && rd::<u32>(a, 0x594) == 9 && rd::<u32>(a, 0x460) != 0 {
            rope_release_picked_up_object(rope);
        }
        return;
    }
    let cd = rd::<u8>(rope, 0x326);
    if cd != 0 {
        wr::<u8>(rope, 0x326, cd - 1);
        return;
    }
    let ty = rd::<u8>(rope, 0x325);
    if !(1..=7).contains(&ty) {
        return;
    }
    for o in [0x4F, 0x3D, 0x4E, 0x3E, 0x3B, 0x3A, 0x3C, 0x3F] {
        fr.setb(o, 0);
    }
    match ty {
        2 => return ped_search(rope, fr),
        5 => return,
        3 => {
            for o in [0x3B, 0x3A, 0x3C, 0x3F, 0x4F] {
                fr.setb(o, 1);
            }
        }
        4 => fr.setb(0x3D, 1),
        6 => fr.setb(0x3E, 1),
        7 => {
            fr.setb(0x4E, 1);
            return object_search(rope, fr);
        }
        _ => {}
    }
    vehicle_search(rope, fr);
    if [0x3D, 0x3E, 0x3B, 0x3A, 0x3C, 0x3F].iter().any(|&o| fr.b(o) != 0) {
        object_search(rope, fr);
    }
}

/// `d = |P - seg31|` for P at frame 0x24 (seg31 at 0x40), compared with 2.5 (`jnp` = below).
fn near_seg31(fr: &Frame) -> bool {
    let dx = fr.x(0x24).fsub_m(fr.f(0x40));
    let dy = fr.x(0x28).fsub_m(fr.f(0x44));
    let dz = fr.x(0x2C).fsub_m(fr.f(0x48));
    !len3(dx, dz, dy).fcom(x(c(0x858FA0))).test_parity(0x05)
}

/// Takes `e` as the picked-up entity: the reference, flag bit 31 of +0x40.
fn take(fr: &mut Frame, e: GPtr<u8>) {
    let cell = g::<u8>(fr.d(0x60) as usize);
    wr::<u32>(cell, 0, e.addr() as u32);
    entity_register_reference(e, cell);
    let a = rd::<GPtr<u8>>(cell, 0);
    wr::<u32>(a, 0x40, rd::<u32>(a, 0x40) | 0x8000_0000);
}

/// The vehicle search (0x55835C): from the last pool slot down, the first live vehicle (an automobile, a train
/// without a driver... subtype +0x594 0 / 1 / 9 with +0x460 null, or models 0x1D9 / 0x21B) not already attached
/// (+0x40 bit 29), with +0x42D bit 4, without +0x4A8 bits 5-6 (and model 0x234 for type 3) whose pick-up point is
/// within 2.5 of the last segment.
fn vehicle_search(rope: GPtr<u8>, fr: &mut Frame) {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if v.is_null() {
            continue;
        }
        let t = rd::<u32>(v, 0x594);
        let model = rd::<u16>(v, 0x22);
        let ok = t == 0 || (t == 9 && rd::<u32>(v, 0x460) == 0) || t == 1 || model == 0x1D9 || model == 0x21B;
        if !ok
            || rd::<u32>(v, 0x40) & 0x2000_0000 != 0
            || rd::<u8>(v, 0x42D) & 0x10 == 0
            || (fr.b(0x4F) != 0 && model != 0x234)
            || rd::<u8>(v, 0x4A8) & 0x60 != 0
        {
            continue;
        }
        let z = model_max_z(v);
        let p = pickup_point(fr, v, 0x14, 0x68, [0, 0, f32_of(x(z)).to_bits()]);
        fr.copy_in(0x24, p);
        if !near_seg31(fr) {
            continue;
        }
        take(fr, v);
        let st = rd::<u8>(v, 0x36);
        if st & 0xF8 == 0x10 {
            wr::<u8>(v, 0x36, (st & 7) | 0x18);
        }
        hook_to_pickup_point(rope, fr, rd::<GPtr<u8>>(g::<u8>(fr.d(0x60) as usize), 0));
        let hook = rd::<GPtr<u8>>(rope, 0x314);
        let dz = fr.x(0x48).fsub_m(fr.f(0x2C));
        wr::<u32>(hook, 0x1C, rd::<u32>(hook, 0x1C) & 0xFFFF_FFFE);
        shorten_after_pickup(rope, dz);
        return;
    }
}

/// The object search (0x55876F): from the last pool slot down, objects with +0x140 bit 18 whose model the type's
/// flags accept; the four container models also try `(0, 0, -maxZ)`, `(maxZ, 0, 0)` and `(-maxZ, 0, 0)` for the
/// highest pick-up point (the matrix is then turned 2, 3 or 1 times). Within 2.5 of the last segment: picked up.
fn object_search(rope: GPtr<u8>, fr: &mut Frame) {
    if rd::<u32>(g::<u8>(fr.d(0x60) as usize), 0) != 0 {
        return;
    }
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    fr.setd(0x30, pool.addr() as u32);
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        fr.setd(0x64, i.wrapping_mul(0x19C));
        if rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() & 0x80 != 0 {
            continue;
        }
        let o = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x19C);
        if o.is_null() || rd::<u32>(o, 0x140) & 0x4_0000 == 0 {
            continue;
        }
        let m = i32::from(rd::<i16>(o, 0x22));
        let is = |addrs: &[usize]| addrs.iter().any(|&a| mi(a) == m);
        let containers = [0x8CD6C8, 0x8CD6CC, 0x8CD6D0, 0x8CD6D8];
        let matched = (fr.b(0x3D) != 0 && is(&[0x8CD6C8, 0x8CD6CC, 0x8CD6D0, 0x8CD6D4, 0x8CD6D8]))
            || (fr.b(0x4E) != 0 && is(&[0x8CD6DC]))
            || (fr.b(0x3E) != 0 && is(&[0x8CD704, 0x8CD708, 0x8CD70C, 0x8CD714]) && rd::<u32>(o, 0xFC) == 0)
            || (fr.b(0x3B) != 0 && is(&[0x8CD724]))
            || (fr.b(0x3A) != 0 && is(&[0x8CD734]))
            || (fr.b(0x3C) != 0 && is(&[0x8CD738]))
            || (fr.b(0x3F) != 0 && is(&[0x8CD73C]));
        if !matched {
            continue;
        }
        let zb = f32_of(x(model_max_z(o))).to_bits();
        let p = pickup_point(fr, o, 0x14, 0x68, [0, 0, zb]);
        fr.copy_in(0x24, p);
        let mut turns = 0;
        if is(&containers) {
            let neg = f32_of(x(model_max_z(o)).fchs()).to_bits();
            pickup_point(fr, o, 0x80, 0x54, [0, 0, neg]);
            if !fr.x(0x5C).fcom(x(fr.f(0x2C))).test_any(0x41) {
                for k in 0..3 {
                    fr.setd(0x24 + 4 * k, fr.d(0x54 + 4 * k));
                }
                turns = 2;
            }
            let p = pickup_point(fr, o, 0x8C, 0x134, [zb, 0, 0]);
            fr.copy_in(0x54, p);
            if !fr.x(0x5C).fcom(x(fr.f(0x2C))).test_any(0x41) {
                fr.copy_in(0x24, p);
                turns = 3;
            }
            let neg = f32_of(x(model_max_z(o)).fchs()).to_bits();
            let p = pickup_point(fr, o, 0x74, 0x98, [neg, 0, 0]);
            fr.copy_in(0x54, p);
            if !fr.x(0x5C).fcom(x(fr.f(0x2C))).test_any(0x41) {
                fr.copy_in(0x24, p);
                turns = 1;
            }
        }
        if !near_seg31(fr) {
            continue;
        }
        take(fr, o);
        let hook = rd::<GPtr<u8>>(rope, 0x314);
        let z = model_max_z(o);
        let pp = pickup_point(fr, o, 0x74, 0x98, [0, 0, f32_of(x(z)).to_bits()]);
        placeable_set_posn(hook, pp);
        wr::<u32>(hook, 0x1C, rd::<u32>(hook, 0x1C) & 0xFFFF_FFFE);
        let a = rd::<GPtr<u8>>(g::<u8>(fr.d(0x60) as usize), 0);
        let fl = rd::<u32>(a, 0x1C);
        if fl & 4 != 0 || fl & 0x4_0000 != 0 {
            entity_set_is_static(a, 0);
            physical_add_to_moving_list(rd::<GPtr<u8>>(g::<u8>(fr.d(0x60) as usize), 0));
        }
        let dz = fr.x(0x48).fsub_m(fr.f(0x2C));
        let a = rd::<GPtr<u8>>(g::<u8>(fr.d(0x60) as usize), 0);
        wr::<u32>(a, 0x40, rd::<u32>(a, 0x40) | 0x200_0000);
        shorten_after_pickup(rope, dz);
        while turns > 0 {
            let m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(g::<u8>(fr.d(0x60) as usize), 0), 0x14);
            fr.copy_in(0x14, m);
            fr.setf(0x54, fr.x(0x14).fchs());
            fr.setf(0x58, fr.x(0x18).fchs());
            for k in 0..3 {
                wr::<u32>(m, 4 * k, rd::<u32>(m, 0x20 + 4 * k));
            }
            fr.setf(0x5C, fr.x(0x1C).fchs());
            for k in 0..3 {
                wr::<u32>(m, 0x20 + 4 * k, fr.d(0x54 + 4 * k as usize));
            }
            turns -= 1;
        }
        return;
    }
}

/// The ped search of the harness (0x5585A4): from the last pool slot down, the first ped that is not dead (+0x530
/// 0x37), not a player, not in a vehicle, within 2.5 of the last segment. The shortening reads the frame's stale
/// slot 0x2C (the ped's position is at 0x14).
fn ped_search(rope: GPtr<u8>, fr: &mut Frame) {
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let p = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if p.is_null() || rd::<u32>(p, 0x530) == 0x37 || ped_is_player(p) != 0 || rd::<u32>(p, 0x46C) & 0x100 != 0 {
            continue;
        }
        fr.copy_in(0x14, pos_ptr(p));
        let dx = fr.x(0x14).fsub_m(fr.f(0x40));
        let dy = fr.x(0x18).fsub_m(fr.f(0x44));
        let dz = fr.x(0x1C).fsub_m(fr.f(0x48));
        if len3(dz, dy, dx).fcom(x(c(0x858FA0))).test_parity(0x05) {
            continue;
        }
        take(fr, p);
        let z = model_max_z(p);
        let pp = pickup_point(fr, p, 0x14, 0x68, [0, 0, f32_of(x(z)).to_bits()]);
        placeable_set_posn(rd::<GPtr<u8>>(rope, 0x314), pp);
        let dzz = fr.x(0x48).fsub_m(fr.f(0x2C));
        let hook = rd::<GPtr<u8>>(rope, 0x314);
        wr::<u32>(hook, 0x1C, rd::<u32>(hook, 0x1C) & 0xFFFF_FFFE);
        if dzz.fcom(x(c(0x858B50))).test_any(0x41) {
            return;
        }
        let inv = x(c(0x858624)).fdiv_m(rd::<f32>(rope, 0x308));
        let t = dzz.fmul(inv).fsubr_m(rd::<f32>(rope, 0x31C)).fsub(inv.fmul_m(rd::<f32>(rope, 0x30C)));
        let v = max_f(f32_of(t), c(0x858C58));
        wr::<f32>(rope, 0x31C, f32_of(X87::from_f64_exact(v)));
        return;
    }
}

/// `CRope::UpdateWeightInRope(CVector pos, float, CVector* out)` (0x5561B0, thiscall, `ret 0x14`, AL). The last
/// segment (31) = pos. With the free length `(31 - count) * segLen` not above the distance from the last fixed
/// segment (`count`) to pos (or NaN): `out` = that segment + the offset scaled to the free length, the free segments
/// spaced along it (the direction re-normalised and scaled by the segment length for each), TRUE. Otherwise six
/// relaxation passes (backwards from 31, pulling each segment to one length from the next and recording its speed,
/// stopping at the first segment already within length; then forwards from `count + 1` to 30), then a backward pass
/// that stops at the first segment within length; FALSE.
#[hook(addr = 0x5561B0, conv = thiscall, name = "CRope::UpdateWeightInRope", expect = "81 EC 80 00 00 00 53 56",
       unverified = "std9-render-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the float argument (0.1 from CRope::Update) is unused")]
pub fn rope_update_weight(rope: GPtr<u8>, px: u32, py: u32, pz: u32, _a5: u32, out: GPtr<u8>) -> u8 {
    let n = usize::from(rd::<u8>(rope, 0x324));
    let seg = |k: usize| rope.byte_add(12 * k as isize);
    let len = || rd::<f32>(rope, 0x30C);
    let last = [rd::<f32>(seg(n), 0), rd::<f32>(seg(n), 4), rd::<f32>(seg(n), 8)];
    wr::<u32>(rope, 0x174, px);
    wr::<u32>(rope, 0x178, py);
    wr::<u32>(rope, 0x17C, pz);
    let free = xi(31 - n as i32).fmul_m(len());
    let mut d = [
        f32_of(x(f32::from_bits(px)).fsub_m(last[0])),
        f32_of(x(f32::from_bits(py)).fsub_m(last[1])),
        f32_of(x(f32::from_bits(pz)).fsub_m(last[2])),
    ];
    let dist = x(d[0]).fmul_m(d[0]).fadd(x(d[2]).fmul_m(d[2])).fadd(x(d[1]).fmul_m(d[1])).fsqrt();
    if dist.fcom(free).test_parity(0x05) {
        let k = free.fdiv(dist);
        let place = |dv: &[f32; 3], m: X87, dst: GPtr<u8>| {
            let ox = x(dv[0]).fmul(m).fadd_m(last[0]);
            let oy = x(dv[1]).fmul(m);
            let oz = x(dv[2]).fmul(m);
            wr::<f32>(dst, 0, f32_of(ox));
            wr::<f32>(dst, 4, f32_of(x(f32_of(oy)).fadd_m(last[1])));
            wr::<f32>(dst, 8, f32_of(x(f32_of(oz)).fadd_m(last[2])));
        };
        place(&d, k, out);
        for i in n + 1..32 {
            let dp = GPtr::from_raw(d.as_mut_ptr()).cast::<u8>();
            vector_normalise(dp);
            let l = len();
            d = [f32_of(x(d[0]).fmul_m(l)), f32_of(x(d[1]).fmul_m(l)), f32_of(x(d[2]).fmul_m(l))];
            place(&d, xi((i - n) as i32), seg(i));
        }
        return 1;
    }
    // pull seg `a` to one length from seg `b` (along a - b, the length recomputed), returns a's old position
    let pull = |a: GPtr<u8>, b: GPtr<u8>| -> [u32; 3] {
        let old = [rd::<u32>(a, 0), rd::<u32>(a, 4), rd::<u32>(a, 8)];
        let dx = x(rd::<f32>(b, 0)).fsub_m(rd::<f32>(a, 0));
        let dy = x(rd::<f32>(b, 4)).fsub_m(rd::<f32>(a, 4));
        let dz = x(rd::<f32>(b, 8)).fsub_m(rd::<f32>(a, 8));
        let nd = [
            f32_of(x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0))),
            f32_of(x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4))),
            f32_of(x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8))),
        ];
        let k = len3(dx, dz, dy).fdivr_m(len());
        let ny = f32_of(x(nd[1]).fmul(k));
        let nz = f32_of(x(nd[2]).fmul(k));
        let nx = x(nd[0]).fmul(k).fadd_m(rd::<f32>(b, 0));
        let (by, bz) = (rd::<f32>(b, 4), rd::<f32>(b, 8));
        wr::<f32>(a, 0, f32_of(nx));
        wr::<f32>(a, 4, f32_of(x(ny).fadd_m(by)));
        wr::<f32>(a, 8, f32_of(x(nz).fadd_m(bz)));
        old
    };
    let seg_len = |a: GPtr<u8>, b: GPtr<u8>| -> X87 {
        let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
        let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
        let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
        len3(dx, dz, dy)
    };
    for _ in 0..6 {
        // backwards: e = 31 down to count + 2, segment e - 1 pulled to e
        let mut e = 31;
        while e > n + 1 {
            if seg_len(seg(e), seg(e - 1)).fcom(x(len())).test_any(0x41) {
                break;
            }
            let old = pull(seg(e - 1), seg(e));
            let a = seg(e - 1);
            let mx = x(rd::<f32>(a, 0)).fsub_m(f32::from_bits(old[0]));
            let my = x(rd::<f32>(a, 4)).fsub_m(f32::from_bits(old[1]));
            let mz = f32_of(x(rd::<f32>(a, 8)).fsub_m(f32::from_bits(old[2])));
            let inv = f32_of(x(c(0x858624)).fdiv_m(c(TIME_STEP)));
            let sp = rope.byte_add(0x180 + 12 * (e as isize - 1));
            wr::<f32>(sp, 0, f32_of(x(inv).fmul(mx)));
            wr::<f32>(sp, 4, f32_of(my.fmul_m(inv)));
            wr::<f32>(sp, 8, f32_of(x(mz).fmul_m(inv)));
            e -= 1;
        }
        // forwards: k = count + 1 .. 30, segment k pulled to k - 1
        for k in n + 1..31 {
            if seg_len(seg(k), seg(k - 1)).fcom(x(len())).test_any(0x41) {
                continue;
            }
            pull_forward(seg(k), seg(k - 1), len());
        }
    }
    let mut j = 31;
    while j > n + 1 {
        if seg_len(seg(j), seg(j - 1)).fcom(x(len())).test_any(0x41) {
            return 0;
        }
        pull(seg(j - 1), seg(j));
        j -= 1;
    }
    0
}

/// The forward pass's pull (0x556464): `d = a - b` stored as f32, `a = b + d * (len / |d|)`.
fn pull_forward(a: GPtr<u8>, b: GPtr<u8>, len: f32) {
    let dx = f32_of(x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0)));
    let dy = f32_of(x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4)));
    let dz = f32_of(x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8)));
    let k = len3(x(dx), x(dz), x(dy)).fdivr_m(len);
    let ny = f32_of(x(dy).fmul(k));
    let nz = f32_of(x(dz).fmul(k));
    let nx = x(dx).fmul(k).fadd_m(rd::<f32>(b, 0));
    let (by, bz) = (rd::<f32>(b, 4), rd::<f32>(b, 8));
    wr::<f32>(a, 0, f32_of(nx));
    wr::<f32>(a, 4, f32_of(x(ny).fadd_m(by)));
    wr::<f32>(a, 8, f32_of(x(nz).fadd_m(bz)));
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ROPE_UPDATE, HOOK_ROPE_UPDATE_WEIGHT];
