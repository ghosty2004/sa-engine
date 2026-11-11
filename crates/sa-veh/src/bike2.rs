//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `CBike` (0x814 bytes), `CBmx` (0x838 bytes) and
//! `CRideAnimData` (0x1C bytes). Ported from the 1.0 US bytes; gta-reversed `Bike.cpp` / `Bmx.cpp` as
//! reading aids. `bool` results set only AL (callers test AL).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, g, jne_mask, rd, wr, x};

/// `CBike*` / `CBmx*` (thiscall `this`).
type Bike = GPtr<u8>;

orig! {
    /// `CVehicle::operator delete(void*)` (0x6E2D90, catalogue `delete`)
    pub fn vehicle_delete(p: GPtr<u8>) = cdecl 0x6E2D90 => unported;
    /// `CBike::Destructor()` (0x6B57A0)
    pub fn bike_destructor_call(bk: GPtr<u8>) = thiscall 0x6B57A0 => destructor;
    /// `CBmx::~CBmx()` (0x6BF9D0)
    pub fn bmx_destructor_call(bk: GPtr<u8>) = thiscall 0x6BF9D0 => bmx_destructor;
    /// `CAEVehicleAudioEntity::Terminate()` (0x4FB8C0)
    pub fn audio_terminate(a: GPtr<u8>) = thiscall 0x4FB8C0 => unported;
    /// `CRideAnimData::~CRideAnimData()` (0x6D0B30, a `ret`)
    pub fn ride_anim_data_dtor_call(r: GPtr<u8>) = thiscall 0x6D0B30 => ride_anim_data_destructor;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CVehicle::Destructor()` (0x6E2B40)
    pub fn vehicle_destructor(v: GPtr<u8>) = thiscall 0x6E2B40 => unported;
    /// `CBike::ProcessRiderAnims(CPed*, CVehicle*, CRideAnimData*, tBikeHandlingData*, int16)` (0x6B7280)
    pub fn process_rider_anims(ped: GPtr<u8>, v: GPtr<u8>, data: GPtr<u8>, handling: GPtr<u8>, a5: u32) = cdecl 0x6B7280 => unported;
    /// `RwRenderStateGet` (RwEngineInstance +0x24)
    pub fn rw_render_state_get(state: u32, out: GPtr<u32>) -> i32 = cdecl table 0xC97B24 0x24 => unported;
    /// `RwRenderStateSet` (RwEngineInstance +0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CVehicle::Render()` (0x6D0E60)
    pub fn vehicle_render(v: GPtr<u8>) = thiscall 0x6D0E60 => unported;
    /// `CBike::CalculateLeanMatrix()` (0x6B7150)
    pub fn calculate_lean_matrix_call(bk: GPtr<u8>) = thiscall 0x6B7150 => calculate_lean_matrix;
    /// `CVehicle::DoHeadLightBeam(int32, CMatrix&, bool)` (0x6E0E20)
    pub fn do_head_light_beam(v: GPtr<u8>, kind: u32, m: GPtr<u8>, right: u32) = thiscall 0x6E0E20 => unported;
    /// `CMatrix::SetRotateX(float)` (0x59B060)
    pub fn matrix_set_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B060 => unported;
    /// `CMatrix::RotateY(float)` (0x59B2C0)
    pub fn matrix_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59B2C0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    pub fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80)
    pub fn matrix_mul_assign(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x411A80 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0)
    pub fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BCF0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CBike::ProcessControl()` (0x6B9250)
    pub fn bike_process_control(bk: GPtr<u8>) = thiscall 0x6B9250 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    pub fn clump_get_association(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CPhysical::SkipPhysics()` (0x5433B0)
    pub fn skip_physics(p: GPtr<u8>) = thiscall 0x5433B0 => unported;
    /// `CPhysical::ApplyMoveSpeed()` (0x542DD0)
    pub fn apply_move_speed(p: GPtr<u8>) = thiscall 0x542DD0 => unported;
    /// `CPhysical::ApplyTurnSpeed()` (0x542E20)
    pub fn apply_turn_speed(p: GPtr<u8>) = thiscall 0x542E20 => unported;
    /// `CPhysical::CheckCollision()` (0x54D920)
    pub fn check_collision(p: GPtr<u8>) -> u8 = thiscall 0x54D920 => unported;
    /// `CBike::Constructor(int32 modelIndex, eVehicleCreatedBy)` (0x6BF430)
    pub fn bike_constructor_call(bk: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6BF430 => unported;
    /// virtual `CBaseModelInfo` slot +0x38 (the vehicle model's anim file / group index)
    pub fn mi_vslot_38(mi: GPtr<u8>) -> i32 = thiscall vslot 0x38 => unported;
    /// `RwMatrixTransform(RwMatrix*, const RwMatrix*, RwOpCombineType)` (0x7F25A0)
    pub fn rw_matrix_transform(m: GPtr<u8>, t: GPtr<u8>, op: u32) -> GPtr<u8> = cdecl 0x7F25A0 => unported;
    /// `CVector::CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730)
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    pub fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    pub fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    pub fn blend_animation(clump: GPtr<u8>, group: u32, id: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80)
    pub fn assoc_set_current_time(a: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBE0)
    pub fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: GPtr<u8>) = thiscall 0x4CEBE0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
}

/// A stack `CMatrix` (0x48 bytes) as the inlined default constructor leaves it: only the attach
/// matrix pointer (+0x40) and the owns flag (+0x44) cleared.
#[repr(C, align(4))]
pub(crate) struct StackMatrix([u8; 0x48]);

impl StackMatrix {
    pub(crate) fn new() -> StackMatrix {
        StackMatrix([0; 0x48])
    }
    pub(crate) fn ptr(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr())
    }
}

/// `CRideAnimData::CRideAnimData()` (0x6D0B10): 7 zero dwords; returns `this`.
#[hook(addr = 0x6D0B10, conv = thiscall, name = "CRideAnimData::CRideAnimData", expect = "8B C1 33 C9 89 08", this = "CRideAnimData")]
pub fn ride_anim_data_constructor(r: GPtr<u8>) -> GPtr<u8> {
    for k in 0..7 {
        wr::<u32>(r, 4 * k, 0);
    }
    r
}

/// `CRideAnimData::~CRideAnimData()` (0x6D0B30): a `ret` (1 byte: not hookable, TooShort).
pub fn ride_anim_data_destructor(_r: GPtr<u8>) {}

/// `CBike::Destructor()` (0x6B57A0, entry `push -1; jmp` into a protection stub that continues at
/// 0x6B57A7): vtable 0x871360, audio Terminate (+0x138), the ride-anim-data (+0x640) and lean-matrix
/// (+0x5CC) member destructors, `CVehicle::Destructor`. Not hookable: the protection reads its entry
/// bytes (0x401571 `sbb edi, [0x6B57A3]`).
pub fn destructor(bk: Bike) {
    wr::<u32>(bk, 0, 0x0087_1360);
    audio_terminate(bk.byte_add(0x138));
    ride_anim_data_dtor_call(bk.byte_add(0x640));
    matrix_dtor(bk.byte_add(0x5CC));
    vehicle_destructor(bk);
}

/// `CBike::scalar deleting destructor(flags)` (0x6B8950): Destructor, `CVehicle::operator delete`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x6B8950, conv = thiscall, name = "CBike::`scalar deleting destructor'", expect = "56 8B F1 E8 48 CE FF FF", this = "CBike")]
pub fn scalar_deleting_destructor(bk: Bike, flags: u32) -> GPtr<u8> {
    bike_destructor_call(bk);
    if flags & 1 != 0 {
        vehicle_delete(bk);
    }
    bk
}

/// `CBmx::~CBmx()` (0x6BF9D0): vtable 0x871528, audio Terminate (+0x138), `CBike::Destructor`.
#[hook(addr = 0x6BF9D0, conv = thiscall, name = "CBmx::~CBmx", expect = "6A FF 68 C8 83 84 00", this = "CBmx")]
pub fn bmx_destructor(bk: Bike) {
    wr::<u32>(bk, 0, 0x0087_1528);
    audio_terminate(bk.byte_add(0x138));
    bike_destructor_call(bk);
}

/// `CBmx::scalar deleting destructor(flags)` (0x6C0570).
#[hook(addr = 0x6C0570, conv = thiscall, name = "CBmx::`scalar deleting destructor'", expect = "56 8B F1 E8 58 F4 FF FF", this = "CBmx")]
pub fn bmx_scalar_deleting_destructor(bk: Bike, flags: u32) -> GPtr<u8> {
    bmx_destructor_call(bk);
    if flags & 1 != 0 {
        vehicle_delete(bk);
    }
    bk
}

/// `CBike::ProcessDrivingAnims(CPed*, bool)` (0x6BF400): unless entity flag 0x20000 (+0x1C) is set
/// with a non-player status (+0x36 & 0xF8), `ProcessRiderAnims(ped, this, &rideAnimData (+0x640),
/// bike handling (+0x63C), 0)`. The bool is unused.
#[hook(addr = 0x6BF400, conv = thiscall, name = "CBike::ProcessDrivingAnims", expect = "F7 41 1C 00 00 02 00", this = "CBike")]
pub fn process_driving_anims(bk: Bike, ped: GPtr<u8>, _blend: u32) {
    if rd::<u32>(bk, 0x1C) & 0x20000 != 0 && rd::<u8>(bk, 0x36) & 0xF8 != 0 {
        return;
    }
    process_rider_anims(ped, bk, bk.byte_add(0x640), rd::<GPtr<u8>>(bk, 0x63C), 0);
}

/// `CBike::Render()` (0x6BDE20): saves the cull mode (render state 0x1E, local preset to 0), sets it
/// to 1, `+0x4D0 = CTimer::m_snTimeInMilliseconds + 3000`, `CVehicle::Render`; with +0x584 bit 0 the
/// lean matrix and `DoHeadLightBeam(0, leanMatrix, true)`; restores the cull mode.
#[hook(addr = 0x6BDE20, conv = thiscall, name = "CBike::Render", expect = "51 56 8D 44 24 04", this = "CBike")]
pub fn render(bk: Bike) {
    let mut saved: u32 = 0;
    rw_render_state_get(0x1E, GPtr::from_raw(&raw mut saved));
    rw_render_state_set(0x1E, 1);
    wr::<u32>(bk, 0x4D0, g::<u32>(0xB7CB84).read().wrapping_add(0xBB8));
    vehicle_render(bk);
    if rd::<u8>(bk, 0x584) & 1 != 0 {
        calculate_lean_matrix_call(bk);
        do_head_light_beam(bk, 0, bk.byte_add(0x5CC), 1);
    }
    rw_render_state_set(0x1E, saved);
}

/// `CBike::CalculateLeanMatrix()` (0x6B7150, entry `nop; jmp` into a protection stub): once per frame
/// (+0x5C8): lean = +0x648; stack matrix `SetRotateX(|lean| * -0.05 (0x871290))`, `RotateY(lean)`;
/// leanMatrix (+0x5CC) = matrix * that; its position += (colModel min.z * up) * (1 - cos(lean)), with
/// the x87 order and f32 stores of the original.
#[hook(addr = 0x6B7150, conv = thiscall, name = "CBike::CalculateLeanMatrix", expect = "90 E9 A6 A8 D4 FF", this = "CBike")]
pub fn calculate_lean_matrix(bk: Bike) {
    if rd::<u8>(bk, 0x5C8) != 0 {
        return;
    }
    let mut m = StackMatrix::new();
    let mp = m.ptr();
    let lean_bits = rd::<u32>(bk, 0x648);
    let ang = x(f32::from_bits(lean_bits & 0x7FFF_FFFF)).fmul_m(g::<f32>(0x871290).read());
    matrix_set_rotate_x(mp, f32_of(ang));
    matrix_rotate_y(mp, rd::<f32>(bk, 0x648));
    let lean_m = bk.byte_add(0x5CC);
    matrix_assign(lean_m, rd::<GPtr<u8>>(bk, 0x14));
    matrix_mul_assign(lean_m, mp);
    let lean = rd::<f32>(bk, 0x648);
    let col = entity_get_col_model(bk);
    let r = x(rd::<f32>(col, 8));
    let mat = rd::<GPtr<u8>>(bk, 0x14);
    let rz = r.fmul_m(rd::<f32>(mat, 0x28));
    let ry = f32_of(r.fmul_m(rd::<f32>(mat, 0x24)));
    let rx = r.fmul_m(rd::<f32>(mat, 0x20));
    wr::<u8>(bk, 0x5C8, 1);
    let rx = f32_of(rx);
    let k = f32_of(x(lean).fcos().fsubr_m(g::<f32>(0x858624).read()));
    let vz = rz.fmul_m(k);
    let vy = x(ry).fmul_m(k);
    let vx = x(rx).fmul_m(k);
    wr::<f32>(bk, 0x5FC, f32_of(vx.fadd_m(rd::<f32>(bk, 0x5FC))));
    wr::<f32>(bk, 0x600, f32_of(vy.fadd_m(rd::<f32>(bk, 0x600))));
    wr::<f32>(bk, 0x604, f32_of(vz.fadd_m(rd::<f32>(bk, 0x604))));
    matrix_dtor(mp);
}

/// `blendAmount (+0x18) > 0.01 (0x858C58)` of an anim association (`fcomp; test ah, 41h; jne` not
/// taken).
fn blend_above(assoc: GPtr<u8>) -> bool {
    !jne_mask(x(rd::<f32>(assoc, 0x18)), x(g::<f32>(0x858C58).read()), 0x41)
}

/// `sin(currentTime (+0x20) / hierarchy length (+0x14 -> +0x10) * 2pi (0x858CBC) + phase) * blend`.
fn anim_wave(assoc: GPtr<u8>, phase: f32) -> sa_math::x87::X87 {
    let hier = rd::<GPtr<u8>>(assoc, 0x14);
    x(rd::<f32>(assoc, 0x20))
        .fdiv_m(rd::<f32>(hier, 0x10))
        .fmul_m(g::<f32>(0x858CBC).read())
        .fadd_m(phase)
        .fsin()
        .fmul_m(rd::<f32>(assoc, 0x18))
}

/// `CBmx::ProcessControl()` (0x6BFA30): `CBike::ProcessControl`, then for a player-status BMX
/// (entity flag bit 6 clear, +0x36 & 0xF8 == 0) with a driver: pedalling anim 0xD0 blended in ->
/// +0x81C = wave(phase 0x8D328C) * (model 510 (Mountain Bike) ? 0x8D3298 : 0x8D3290); else the
/// sprint anim 0xCF blended in adds wave(phase 0xC1C820) * (510 ? 0x8D329C : 0x8D3294) to the lean
/// (+0x648), and +0x81C *= 0.95 (0x858EF0).
#[hook(addr = 0x6BFA30, conv = thiscall, name = "CBmx::ProcessControl", expect = "51 56 8B F1 E8 17 98 FF FF", this = "CBmx")]
pub fn bmx_process_control(bk: Bike) {
    bike_process_control(bk);
    if (rd::<u32>(bk, 0x1C) >> 6) & 1 != 0 || rd::<u8>(bk, 0x36) & 0xF8 != 0 {
        return;
    }
    let driver = rd::<GPtr<u8>>(bk, 0x460);
    if driver.is_null() {
        return;
    }
    let assoc = clump_get_association(rd::<GPtr<u8>>(driver, 0x18), 0xD0);
    let mtb = rd::<u16>(bk, 0x22) == 0x1FE;
    if !assoc.is_null() && blend_above(assoc) {
        let s = g::<f32>(if mtb { 0x8D3298 } else { 0x8D3290 }).read();
        let v = anim_wave(assoc, g::<f32>(0x8D328C).read()).fmul(x(s));
        wr::<f32>(bk, 0x81C, f32_of(v));
        return;
    }
    let s = g::<f32>(if mtb { 0x8D329C } else { 0x8D3294 }).read();
    let driver = rd::<GPtr<u8>>(bk, 0x460);
    let assoc = clump_get_association(rd::<GPtr<u8>>(driver, 0x18), 0xCF);
    if !assoc.is_null() && blend_above(assoc) {
        let v = anim_wave(assoc, g::<f32>(0xC1C820).read())
            .fmul_m(s)
            .fadd_m(rd::<f32>(bk, 0x648));
        wr::<f32>(bk, 0x648, f32_of(v));
    }
    let v = x(rd::<f32>(bk, 0x81C)).fmul_m(g::<f32>(0x858EF0).read());
    wr::<f32>(bk, 0x81C, f32_of(v));
}

/// The `ProcessControlCollisionCheck(bool applySpeed)` body shared by `CAutomobile` and `CBike`: copy
/// the matrix; clear entity flag 0x10; `SkipPhysics`; physical flags (+0x40) &= 0xFFFF6FFF; +0xD4 = 0;
/// the 4 wheel ratios at `ratios_off` = the constant at `ratio_src`. With applySpeed: apply move/turn
/// speed and, while CheckCollision hits (5 retries at most), restore the matrix and apply again.
/// Without: CheckCollision with the uses-collision bit (flag bit 0) cleared, then restored. Finally
/// flag 0x10 cleared, 0x20 set; the copy's destructor.
pub(crate) fn control_collision_check(v: GPtr<u8>, apply_speed: u32, ratio_src: usize, ratios_off: isize) {
    let mut saved = StackMatrix::new();
    let sp = saved.ptr();
    matrix_copy_ctor(sp, rd::<GPtr<u8>>(v, 0x14));
    wr::<u32>(v, 0x1C, rd::<u32>(v, 0x1C) & 0xFFFF_FFEF);
    skip_physics(v);
    let r = g::<u32>(ratio_src).read();
    wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) & 0xFFFF_6FFF);
    wr::<u32>(v, 0xD4, 0);
    for k in 0..4 {
        wr::<u32>(v, ratios_off + 4 * k, r);
    }
    if apply_speed as u8 != 0 {
        apply_move_speed(v);
        apply_turn_speed(v);
        let mut n: u16 = 0;
        while check_collision(v) != 0 {
            let c = n;
            n = n.wrapping_add(1);
            if c >= 5 {
                break;
            }
            matrix_assign(rd::<GPtr<u8>>(v, 0x14), sp);
            apply_move_speed(v);
            apply_turn_speed(v);
        }
    } else {
        let f = rd::<u32>(v, 0x1C);
        let bit = f & 1;
        wr::<u32>(v, 0x1C, f & 0xFFFF_FFFE);
        check_collision(v);
        let f = rd::<u32>(v, 0x1C);
        wr::<u32>(v, 0x1C, f ^ ((bit ^ f) & 1));
    }
    wr::<u32>(v, 0x1C, (rd::<u32>(v, 0x1C) & 0xFFFF_FFEF) | 0x20);
    matrix_dtor(sp);
}

/// `CBike::ProcessControlCollisionCheck(bool)` (0x6B6620): wheel ratios +0x710 = 1.0 (0x871284).
#[hook(addr = 0x6B6620, conv = thiscall, name = "CBike::ProcessControlCollisionCheck", expect = "6A FF 68 38 82 84 00", this = "CBike")]
pub fn process_control_collision_check(bk: Bike, apply_speed: u32) {
    control_collision_check(bk, apply_speed, 0x871284, 0x710);
}

/// The world-space RwMatrix of `frame` as the original builds it: a copy of the frame's matrix
/// (+0x10), transformed (post-concat) by each parent (+4) until null or back at `frame`.
fn frame_world_matrix(frame: GPtr<u8>, m: GPtr<u8>) {
    for k in 0..16 {
        wr::<u32>(m, 4 * k, rd::<u32>(frame, 0x10 + 4 * k));
    }
    let mut p = rd::<GPtr<u8>>(frame, 4);
    while !p.is_null() {
        rw_matrix_transform(m, p.byte_add(0x10), 2);
        p = rd::<GPtr<u8>>(p, 4);
        if p == frame {
            break;
        }
    }
}

/// `CBmx::Constructor(int32 modelIndex, eVehicleCreatedBy)` (0x6BF820): `CBike::Constructor`, vtable
/// 0x871528, subclass +0x594 = 10; ride-anim group (+0x640) = the group (0xB5D4BC + 32 * model
/// virtual +0x38), 7 unless within 7..=9; +0x814..+0x828 and bytes +0x7B4/+0x7B5/+0x834 cleared. With
/// the world positions of nodes +0x5B0 (P) and +0x5B4 (Q): +0x82C = P.y - Q.y and +0x830 =
/// P.y / (P.y - Q.y) (x87 value, not the stored one). Returns `this`.
#[hook(addr = 0x6BF820, conv = thiscall, name = "CBmx::Constructor", expect = "6A FF 68 A8 83 84 00", this = "CBmx")]
pub fn bmx_constructor(bk: Bike, model: i32, created_by: u32) -> GPtr<u8> {
    bike_constructor_call(bk, model, created_by);
    wr::<u32>(bk, 0, 0x0087_1528);
    let mi = sa_core::pools::MODEL_INFO_PTRS
        .ptr()
        .cast::<GPtr<u8>>()
        .wrapping_index(model)
        .read();
    wr::<u32>(bk, 0x594, 0xA);
    let idx = mi_vslot_38(mi);
    let grp = g::<i32>(0xB5D4BC).byte_add((idx.wrapping_shl(5)) as isize).read();
    wr::<i32>(bk, 0x640, grp);
    if !(7..=9).contains(&grp) {
        wr::<i32>(bk, 0x640, 7);
    }
    for k in 0..6 {
        wr::<u32>(bk, 0x814 + 4 * k, 0);
    }
    wr::<u8>(bk, 0x7B4, 0);
    wr::<u8>(bk, 0x7B5, 0);
    wr::<u8>(bk, 0x834, 0);
    let mut m = [0u32; 16];
    let mp = GPtr::from_raw(m.as_mut_ptr()).cast::<u8>();
    frame_world_matrix(rd::<GPtr<u8>>(bk, 0x5B0), mp);
    let py = rd::<f32>(mp, 0x34);
    frame_world_matrix(rd::<GPtr<u8>>(bk, 0x5B4), mp);
    let d = x(py).fsub_m(rd::<f32>(mp, 0x34));
    wr::<f32>(bk, 0x82C, f32_of(d));
    wr::<f32>(bk, 0x830, f32_of(x(py).fdiv(d)));
    bk
}

/// `CBike::GetCorrectedWorldDoorPosition(CVector& out, CVector a, CVector b)` (0x6BF230): with the
/// matrix's forward F and up U, side = F x (0,0,1), side2 = side x F, D = side . U (y, z, x order),
/// H = colModel max.z - max.x when greater, else 0: out = F * (b.y - a.y) + side * ((b.x - a.x) +
/// H * D) + side2 * (b.z - a.z) + position (the matrix's, else the placeable's +4). The x87 stack
/// order and the f32 temporaries of the original are kept. Returns `out` (EAX).
#[hook(addr = 0x6BF230, conv = thiscall, name = "CBike::GetCorrectedWorldDoorPosition", expect = "83 EC 30 56 8B F1", this = "CBike")]
#[allow(clippy::too_many_arguments)]
pub fn get_corrected_world_door_position(
    bk: Bike,
    out: GPtr<u8>,
    ax: f32,
    ay: f32,
    az: f32,
    bx: f32,
    by: f32,
    bz: f32,
) -> GPtr<u8> {
    let mut v0 = [0.0f32, 0.0, 1.0];
    let mut side = [0f32; 3];
    let mut side2 = [0f32; 3];
    let v0p = GPtr::from_raw(v0.as_mut_ptr()).cast::<u8>();
    let sp = GPtr::from_raw(side.as_mut_ptr()).cast::<u8>();
    let s2p = GPtr::from_raw(side2.as_mut_ptr()).cast::<u8>();
    cross_product(sp, rd::<GPtr<u8>>(bk, 0x14).byte_add(0x10), v0p);
    cross_product(s2p, sp, rd::<GPtr<u8>>(bk, 0x14).byte_add(0x10));
    let (side, side2) = (sp.cast::<[f32; 3]>().read(), s2p.cast::<[f32; 3]>().read());
    let mat = rd::<GPtr<u8>>(bk, 0x14);
    let d = x(side[1])
        .fmul_m(rd::<f32>(mat, 0x24))
        .fadd(x(side[2]).fmul_m(rd::<f32>(mat, 0x28)))
        .fadd(x(side[0]).fmul_m(rd::<f32>(mat, 0x20)));
    let d = f32_of(d);
    let col = entity_get_col_model(bk);
    let h = if jne_mask(x(rd::<f32>(col, 0x14)), x(rd::<f32>(col, 0xC)), 0x41) {
        x(g::<f32>(0x858B50).read())
    } else {
        x(rd::<f32>(col, 0x14)).fsub_m(rd::<f32>(col, 0xC))
    };
    let e = x(by).fsub_m(ay);
    for k in 0..3 {
        wr::<u32>(out, 4 * k, 0);
    }
    let mat = rd::<GPtr<u8>>(bk, 0x14);
    let fx = e.fmul_m(rd::<f32>(mat, 0x10));
    let l20 = f32_of(e.fmul_m(rd::<f32>(mat, 0x14)));
    let l24 = f32_of(e.fmul_m(rd::<f32>(mat, 0x18)));
    let v = fx.fadd_m(rd::<f32>(out, 0));
    let s38 = f32_of(v);
    wr::<f32>(out, 0, s38);
    let v = x(l20).fadd_m(rd::<f32>(out, 4));
    let s8 = f32_of(v);
    wr::<f32>(out, 4, s8);
    let v = x(l24).fadd_m(rd::<f32>(out, 8));
    let sc = f32_of(v);
    wr::<f32>(out, 8, sc);
    let f = x(bx).fsub_m(ax);
    let gg = f.fadd(h.fmul_m(d));
    let l10x = x(side[0]).fmul(gg);
    let l20 = f32_of(x(side[1]).fmul(gg));
    let l24 = f32_of(x(side[2]).fmul(gg));
    let xx = x(s38).fadd(l10x);
    wr::<f32>(out, 0, f32_of(xx));
    let yy = x(s8).fadd_m(l20);
    wr::<f32>(out, 4, f32_of(yy));
    let zz = x(sc).fadd_m(l24);
    let s38 = f32_of(zz);
    wr::<f32>(out, 8, s38);
    let kk = x(bz).fsub_m(az);
    let l28x = x(side2[0]).fmul(kk);
    let t40 = f32_of(x(side2[1]).fmul(kk));
    let t44 = f32_of(x(side2[2]).fmul(kk));
    let xx = xx.fadd(l28x);
    let t48 = f32_of(xx);
    wr::<f32>(out, 0, t48);
    let yy = yy.fadd_m(t40);
    wr::<f32>(out, 4, f32_of(yy));
    let zz = x(s38).fadd_m(t44);
    wr::<f32>(out, 8, f32_of(zz));
    let m = rd::<GPtr<u8>>(bk, 0x14);
    let pos = if m.is_null() {
        bk.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    wr::<f32>(out, 0, f32_of(x(t48).fadd_m(rd::<f32>(pos, 0))));
    wr::<f32>(out, 4, f32_of(yy.fadd_m(rd::<f32>(pos, 4))));
    wr::<f32>(out, 8, f32_of(zz.fadd_m(rd::<f32>(pos, 8))));
    out
}

/// `assoc.flags (+0x2E) |= 5; blendDelta (+0x1C) = -8.0`.
fn fade_out(assoc: GPtr<u8>) {
    wr::<u8>(assoc, 0x2E, rd::<u8>(assoc, 0x2E) | 5);
    wr::<u32>(assoc, 0x1C, 0xC100_0000);
}

/// `CBmx::ProcessBunnyHop()` (0x6C0590). The hop anim 0xCE of the driver (association `a`):
/// - not a player-driven player-status BMX: `a` fades out (flags |= 5, blendDelta -8);
/// - pad +8 pressed while +0x38 and +0x10E are not, with timer +0x814 == 0.0: timer += time step,
///   `a` = BlendAnimation(clump, group +0x640, 0xCE, 8.0), SetCurrentTime(0), flags bit 0 cleared;
/// - timer > 0: pad +0x10E pressed -> timer = 0 (then fade out); else without `a` timer = 0 and
///   return; with `a` (flags bit 0 clear): pad +8 held -> timer += step (clamped to 0x871514),
///   SetCurrentTime(timer / 0x871514 * 0x871518); released -> when currentTime < 0x871518,
///   SetCurrentTime((0x871518 - t) / 0x871518 * (length - 0x871518) + 0x871518), then speed
///   (+0x24) = 0x871524, flags |= 1, finish callback LaunchBunnyHopCB(this);
/// - timer <= 0: `a` fades out;
/// then, with `a` blended above 0x858B8C: +0x49C = 0, the player's playerData +0x1C = 0, and unless
/// +0x428 bit 5, +0x834 = 1 when any of the 4 wheel ratios (+0x710) is below 1.0.
#[hook(addr = 0x6C0590, conv = thiscall, name = "CBmx::ProcessBunnyHop", expect = "53 55 56 57 8B F9", this = "CBmx")]
pub fn bmx_process_bunny_hop(bk: Bike) {
    let zero = g::<f32>(0x858B50).read();
    let step = || g::<f32>(0xB7CB5C).read();
    let mut assoc = GPtr::<u8>::NULL;
    let driver = rd::<GPtr<u8>>(bk, 0x460);
    if !driver.is_null() {
        assoc = clump_get_association(rd::<GPtr<u8>>(driver, 0x18), 0xCE);
    }
    if rd::<u8>(bk, 0x36) & 0xF8 != 0
        || rd::<GPtr<u8>>(bk, 0x460).is_null()
        || ped_is_player(rd::<GPtr<u8>>(bk, 0x460)) == 0
    {
        if !assoc.is_null() {
            fade_out(assoc);
        }
        return;
    }
    let pad = get_pad_from_player(rd::<GPtr<u8>>(bk, 0x460));
    if rd::<u16>(pad, 8) != 0
        && rd::<u16>(pad, 0x38) == 0
        && rd::<u16>(pad, 0x10E) == 0
        && !x(rd::<f32>(bk, 0x814)).fcom(x(zero)).test_parity(0x44)
    {
        let grp = rd::<u32>(bk, 0x640);
        let drv = rd::<GPtr<u8>>(bk, 0x460);
        wr::<f32>(bk, 0x814, f32_of(x(step()).fadd_m(rd::<f32>(bk, 0x814))));
        assoc = blend_animation(rd::<GPtr<u8>>(drv, 0x18), grp, 0xCE, 8.0);
        assoc_set_current_time(assoc, 0.0);
        wr::<u8>(assoc, 0x2E, rd::<u8>(assoc, 0x2E) & 0xFE);
    }
    let mut fade = true;
    if !jne_mask(x(rd::<f32>(bk, 0x814)), x(zero), 0x41) {
        if rd::<u16>(pad, 0x10E) == 0 {
            if assoc.is_null() {
                wr::<u32>(bk, 0x814, 0);
                return;
            }
            fade = false;
            let flags = rd::<u8>(assoc, 0x2E);
            if flags & 1 == 0 {
                if rd::<u16>(pad, 8) != 0 {
                    let max = g::<f32>(0x871514).read();
                    let v = x(step()).fadd_m(rd::<f32>(bk, 0x814));
                    wr::<f32>(bk, 0x814, f32_of(v));
                    if !jne_mask(v, x(max), 0x41) {
                        wr::<u32>(bk, 0x814, g::<u32>(0x871514).read());
                    }
                    let t = x(rd::<f32>(bk, 0x814))
                        .fdiv_m(g::<f32>(0x871514).read())
                        .fmul_m(g::<f32>(0x871518).read());
                    assoc_set_current_time(assoc, f32_of(t));
                } else {
                    let c = g::<f32>(0x871518).read();
                    if !x(rd::<f32>(assoc, 0x20)).fcom(x(c)).test_parity(0x05) {
                        let hier = rd::<GPtr<u8>>(assoc, 0x14);
                        let t = x(c).fsub_m(rd::<f32>(assoc, 0x20)).fdiv_m(c);
                        let u = x(rd::<f32>(hier, 0x10)).fsub_m(c);
                        let v = t.fmul(u).fadd_m(c);
                        assoc_set_current_time(assoc, f32_of(v));
                    }
                    let speed = g::<u32>(0x871524).read();
                    wr::<u8>(assoc, 0x2E, rd::<u8>(assoc, 0x2E) | 1);
                    wr::<u32>(assoc, 0x24, speed);
                    assoc_set_finish_callback(assoc, 0x6C_0390, bk);
                }
            }
        } else {
            wr::<u32>(bk, 0x814, 0);
        }
    }
    if fade {
        if assoc.is_null() {
            return;
        }
        fade_out(assoc);
    }
    if jne_mask(x(rd::<f32>(assoc, 0x18)), x(g::<f32>(0x858B8C).read()), 0x41) {
        return;
    }
    wr::<u32>(bk, 0x49C, 0);
    let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    wr::<u32>(pd, 0x1C, 0);
    if rd::<u8>(bk, 0x428) & 0x20 != 0 {
        return;
    }
    let one = g::<f32>(0x858624).read();
    let below = |o: isize| !x(rd::<f32>(bk, o)).fcom(x(one)).test_parity(0x05);
    if below(0x710) || below(0x714) || below(0x718) || below(0x71C) {
        wr::<u8>(bk, 0x834, 1);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RIDE_ANIM_DATA_CONSTRUCTOR,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_BMX_DESTRUCTOR,
    HOOK_BMX_SCALAR_DELETING_DESTRUCTOR,
    HOOK_PROCESS_DRIVING_ANIMS,
    HOOK_RENDER,
    HOOK_CALCULATE_LEAN_MATRIX,
    HOOK_BMX_PROCESS_CONTROL,
    HOOK_PROCESS_CONTROL_COLLISION_CHECK,
    HOOK_BMX_CONSTRUCTOR,
    HOOK_GET_CORRECTED_WORLD_DOOR_POSITION,
    HOOK_BMX_PROCESS_BUNNY_HOP,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
