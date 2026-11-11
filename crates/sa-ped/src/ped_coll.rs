//! CPed collision, attachment, bones and car-impact functions (PORT-PED-3, helper A). Ported from the 1.0 US bytes; specs in docs/re/specs/PED/, gta-reversed as a reading aid
//! (original bugs kept).
//!
//! `CPed` fields used here (besides those listed in `ped.rs`): +0x18 clump, +0x474 flags (bit 0x400 =
//! RpHAnim up to date), +0x488 bone nodes (`+0x490` = node 2, its +0x14 the bone id), +0x4F4 / +0x4F8 the
//! weapon and gun-flash objects (RwFrame), +0x504 / +0x506 left-hand gun-flash alpha / step, +0x508 /
//! +0x50A right hand, +0x530 ped state.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `aPedBoneOffsets` (CVector[], 12 bytes each; 0x8D13A8 as the bytes index it by bone tag)
const PED_BONE_OFFSETS: usize = 0x8D13A8;
/// gun-flash blend start (u16 10000, 0x8D1370)
const GUN_FLASH_BLEND_START: usize = 0x8D1370;
/// gun-flash alpha scale (int 350, 0x8D21DC)
const GUN_FLASH_ALPHA_SCALE: usize = 0x8D21DC;
/// The x axis `RwMatrixRotate` turns the gun flash around (0x8D232C)
const GUN_FLASH_AXIS: u32 = 0x8D232C;
/// `CWorld::m_aTempColPts` (CColPoint[32], 0xC102A0)
const TEMP_COL_PTS: u32 = 0xC102A0;

orig! {
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskSimpleHoldEntity::DropEntity(CPed*, bool)` (0x6930F0)
    fn hold_drop_entity(t: GPtr<u8>, ped: GPtr<u8>, add_to_world: u32) = thiscall 0x6930F0 => unported;
    /// `CEntity::DeleteRwObject` (vtable slot 0x20)
    fn entity_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// scalar deleting destructor (vtable slot 0)
    fn entity_vdelete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `RwHelper::GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn rp_hanim_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    fn rw_v3d_transform_points(out: GPtr<f32>, inp: GPtr<f32>, n: i32, m: GPtr<u8>) -> GPtr<f32> = cdecl 0x7EDD90 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    fn entity_update_rp_hanim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden return pointer)
    fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `RwMatrixRotate(RwMatrix*, const RwV3d* axis, float angle, RwOpCombineType)` (0x7F1FD0)
    fn rw_matrix_rotate(m: GPtr<u8>, axis: u32, angle: u32, combine: u32) -> GPtr<u8> = cdecl 0x7F1FD0 => unported;
    /// `RwHelper::GetFirstObject(RwFrame*)` (0x7348C0)
    fn get_first_object(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7348C0 => unported;
    /// `CVehicle::SetComponentAtomicAlpha(RpAtomic*, int32)` (0x6D2960)
    fn set_component_atomic_alpha(a: GPtr<u8>, alpha: i32) = cdecl 0x6D2960 => unported;
    /// `CWorld::FindObjectsKindaColliding(const CVector&, float, bool, int16*, int16, CEntity**, bool x5)`
    /// (0x568B80)
    fn find_objects_kinda_colliding(p: GPtr<f32>, r: u32, b2d: u32, n: GPtr<i16>, max: u32, out: GPtr<u8>, b: u32, v: u32, pe: u32, o: u32, d: u32) = cdecl 0x568B80 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector, float, const CEntity*, bool x5, bool)` (0x569E20)
    fn test_sphere_against_world(x: u32, y: u32, z: u32, r: u32, ignore: u32, b: u32, v: u32, pe: u32, o: u32, d: u32, cam: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
    /// `CCollision::ProcessColModels(const CMatrix&, CColModel&, const CMatrix&, CColModel&, CColPoint*,
    /// CColPoint*, float*, bool)` (0x4185C0)
    fn process_col_models(ma: GPtr<u8>, ca: GPtr<u8>, mb: GPtr<u8>, cb: GPtr<u8>, cps: u32, lcps: u32, maxd: u32, b: u32) -> i32 = cdecl 0x4185C0 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)`
    /// (0x5674E0, AL)
    fn process_vertical_line(pos: GPtr<f32>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, b6: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
}

/// The bone's matrix in the skin hierarchy of `ped` (`RpHAnimIDGetIndex` then the matrix array).
fn bone_matrix(hier: GPtr<u8>, bone: i32) -> GPtr<u8> {
    let idx = rp_hanim_id_get_index(hier, bone);
    rp_hanim_get_matrix_array(hier).byte_add((idx as isize).wrapping_shl(6))
}

/// Copies three dwords.
fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0isize, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

/// The world z of the head bone (node +0x490's bone id): `(0,0,0)` transformed by the bone matrix.
fn head_pos(this: GPtr<u8>) -> [f32; 3] {
    let mut v = [0.0f32; 3];
    let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(this, 0x18));
    let id = rd::<i32>(rd::<GPtr<u8>>(this, 0x490), 0x14);
    let m = bone_matrix(hier, id);
    let p = lp(&mut v).cast::<f32>();
    rw_v3d_transform_points(p, p, 1, m);
    v
}

/// `CPed::DropEntityThatThisPedIsHolding(bool bDeleteEntity)` (0x5E0360, `ret 4`): the first active task
/// of type 0x133/0x134/0x135 (hold entity / pick up / put down) drops its entity (`DropEntity(ped, true)`);
/// with `bDeleteEntity` the entity (unless an object of type 2) is `DeleteRwObject`, `CWorld::Remove`d and
/// deleted.
#[hook(addr = 0x5E0360, conv = thiscall, name = "CPed::DropEntityThatThisPedIsHolding", expect = "57 8B F9 8B 87 7C 04 00 00", this = "CPed")]
pub fn drop_entity_that_this_ped_is_holding(this: GPtr<u8>, delete: u32) {
    let tm = |this: GPtr<u8>| rd::<GPtr<u8>>(this, 0x47C).byte_add(4);
    let mut t = find_active_task_by_type(tm(this), 0x133);
    if t.is_null() {
        t = find_active_task_by_type(tm(this), 0x134);
    }
    if t.is_null() {
        t = find_active_task_by_type(tm(this), 0x135);
        if t.is_null() {
            return;
        }
    }
    let ent = rd::<GPtr<u8>>(t, 8);
    hold_drop_entity(t, this, 1);
    if ent.is_null() || delete & 0xFF == 0 {
        return;
    }
    if rd::<u8>(ent, 0x36) & 7 == 4 && rd::<u8>(ent, 0x13C) == 2 {
        return;
    }
    entity_delete_rw_object(ent);
    world_remove(ent);
    entity_vdelete(ent, 1);
}

/// `CPed::IsPedHeadAbovePos(float zOffset)` (0x5F02C0, `ret 4`, AL): `zOffset + pos.z < head.z`.
#[hook(addr = 0x5F02C0, conv = thiscall, name = "CPed::IsPedHeadAbovePos", expect = "83 EC 0C 53 56", this = "CPed")]
pub fn is_ped_head_above_pos(this: GPtr<u8>, zoff: f32) -> u8 {
    let head = head_pos(this);
    let pz = ent_pos(this).wrapping_index(2).read();
    (!x(zoff).fadd_m(pz).fcom(x(head[2])).test_parity(5)) as u8
}

/// `CPed::GetTransformedBonePosition(RwV3d& inOut, ePedBones, bool updateSkin)` (0x5E01C0, `ret 0xC`):
/// with `updateSkin` the hierarchy is updated once per frame (flag 0x400 of +0x474); with an up-to-date
/// hierarchy `inOut` is transformed by the bone matrix, else `inOut = matrix * aPedBoneOffsets[bone]` (the
/// input is ignored there, as in the original).
#[hook(addr = 0x5E01C0, conv = thiscall, name = "CPed::GetTransformedBonePosition", expect = "8A 44 24 0C 83 EC 0C", this = "CPed")]
pub fn get_transformed_bone_position(this: GPtr<u8>, v: GPtr<f32>, bone: i32, update: u32) {
    let fl = rd::<u32>(this, 0x474);
    if update & 0xFF != 0 {
        if fl & 0x400 == 0 {
            entity_update_rp_hanim(this);
            wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) | 0x400);
        }
    } else if fl & 0x400 == 0 {
        let mut out = [0.0f32; 3];
        let off = GPtr::<f32>::from_addr(PED_BONE_OFFSETS.wrapping_add((bone as usize).wrapping_mul(12)));
        let r = matrix_mul_vec(lp(&mut out).cast(), rd::<GPtr<u8>>(this, 0x14), off);
        copy3(v.cast(), r.cast());
        return;
    }
    let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(this, 0x18));
    let m = bone_matrix(hier, bone);
    rw_v3d_transform_points(v, v, 1, m);
}

/// `CPed::GetBonePosition(RwV3d& out, ePedBones, bool updateSkin)` (0x5E4280, `ret 0xC`): like
/// `GetTransformedBonePosition`, but the up-to-date path copies the bone matrix's position; without a
/// skin hierarchy it gives the entity position.
#[hook(addr = 0x5E4280, conv = thiscall, name = "CPed::GetBonePosition-Original", expect = "8A 44 24 0C 83 EC 0C", this = "CPed")]
pub fn get_bone_position(this: GPtr<u8>, out: GPtr<f32>, bone: i32, update: u32) {
    let fl = rd::<u32>(this, 0x474);
    if update & 0xFF != 0 {
        if fl & 0x400 == 0 {
            entity_update_rp_hanim(this);
            wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) | 0x400);
        }
    } else if fl & 0x400 == 0 {
        let mut tmp = [0.0f32; 3];
        let off = GPtr::<f32>::from_addr(PED_BONE_OFFSETS.wrapping_add((bone as usize).wrapping_mul(12)));
        let r = matrix_mul_vec(lp(&mut tmp).cast(), rd::<GPtr<u8>>(this, 0x14), off);
        copy3(out.cast(), r.cast());
        return;
    }
    let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(this, 0x18));
    if hier.is_null() {
        copy3(out.cast(), ent_pos(this).cast());
        return;
    }
    copy3(out.cast(), bone_matrix(hier, bone).byte_add(0x30));
}

/// `CPed::DoesLOSBulletHitPed(CColPoint&)` (0x5F01A0, `ret 4`, AL): 1 in ped state 0x2A or when the hit
/// is below the head (`cp.z < head.z`); 2 when it is below `head.z + 0.2`; else 0.
#[hook(addr = 0x5F01A0, conv = thiscall, name = "CPed::DoesLOSBulletHitPed", expect = "83 EC 0C 53 56", this = "CPed")]
pub fn does_los_bullet_hit_ped(this: GPtr<u8>, cp: GPtr<u8>) -> u8 {
    let head = head_pos(this);
    if rd::<i32>(this, 0x530) == 0x2A {
        return 1;
    }
    let cz = rd::<f32>(cp, 8);
    if !x(cz).fcom(x(head[2])).test_parity(5) {
        return 1;
    }
    if x(head[2]).fadd_m(f32::from_bits(0x3E4C_CCCD)).fcom(x(cz)).test_any(0x41) {
        0
    } else {
        2
    }
}

/// `CPed::DoGunFlash(int32 lifetime, bool bRightHand)` (0x5DF340, `ret 8`, AL): needs both the weapon
/// (+0x4F4) and gun-flash (+0x4F8) objects; alpha of the hand = `m_sGunFlashBlendStart`, its step =
/// that / lifetime (`idiv`: lifetime 0 faults as in the original), then the flash frame is rotated about
/// x by `rand() * (1/32767) * 720 - 360` degrees (`rwCOMBINEPRECONCAT`).
#[hook(addr = 0x5DF340, conv = thiscall, name = "CPed::DoGunFlash", expect = "56 8B F1 8B 86 F4 04 00 00", this = "CPed")]
pub fn do_gun_flash(this: GPtr<u8>, lifetime: i32, right: u32) -> u8 {
    if rd::<GPtr<u8>>(this, 0x4F4).is_null() || rd::<GPtr<u8>>(this, 0x4F8).is_null() {
        return 0;
    }
    let start = g::<u16>(GUN_FLASH_BLEND_START);
    let base: isize = if right & 0xFF != 0 { 0x508 } else { 0x504 };
    wr::<u16>(this, base, start.read());
    let step = i32::from(start.read()) / lifetime;
    wr::<u16>(this, base + 2, step as u16);
    let r = at_site(0x5DF3AF, rand);
    let angle = xi(r)
        .fmul_m(f32::from_bits(0x3800_0100))
        .fmul_m(f32::from_bits(0x4434_0000))
        .fsub_m(f32::from_bits(0x43B4_0000))
        .to_f32();
    let frame = rd::<GPtr<u8>>(this, 0x4F8).byte_add(0x10);
    rw_matrix_rotate(frame, GUN_FLASH_AXIS, angle.to_bits(), 1);
    1
}

/// `CPed::SetGunFlashAlpha(bool bRightHand)` (0x5DF400, `ret 4`): with a gun-flash object and either
/// hand's alpha >= 0, the flash atomic gets `min(alpha * 350 / blendStart, 255)` (`_ftol`; 0 when the
/// hand's alpha <= 0) and its render flag; a hand alpha of exactly 0 then becomes -1.
#[hook(addr = 0x5DF400, conv = thiscall, name = "CPed::SetGunFlashAlpha", expect = "53 8A 5C 24 08", this = "CPed")]
pub fn set_gun_flash_alpha(this: GPtr<u8>, right: u32) {
    let right = right & 0xFF != 0;
    let off: isize = if right { 0x508 } else { 0x504 };
    let a = rd::<i16>(this, off);
    let obj = rd::<GPtr<u8>>(this, 0x4F8);
    if obj.is_null() {
        return;
    }
    if rd::<i16>(this, 0x504) < 0 && rd::<i16>(this, 0x508) < 0 {
        return;
    }
    let atomic = get_first_object(obj);
    if !atomic.is_null() {
        let div = i32::from(g::<u16>(GUN_FLASH_BLEND_START).read());
        let v = i32::from(a).wrapping_mul(g::<i32>(GUN_FLASH_ALPHA_SCALE).read()) / div;
        let mut f = xi(v);
        let c = x(f32::from_bits(0x437F_0000));
        if !c.fcom(f).test_parity(5) {
            f = c;
        }
        let mut alpha = ftol32(f);
        if a <= 0 {
            alpha = 0;
        }
        set_component_atomic_alpha(atomic, alpha);
        wr::<u8>(atomic, 2, 4);
    }
    if rd::<i16>(this, off) == 0 {
        wr::<i16>(this, off, -1);
    }
}

/// `CPedPlacement::IsPositionClearForPed(const CVector&, float radius, int32 maxNum, CEntity** out,
/// bool vehicles, bool peds, bool objects)` (0x616860, cdecl, AL): radius -1 means 0.75, maxNum -1 means 2;
/// true when `CWorld::FindObjectsKindaColliding` (2D, no buildings/dummies) finds nothing.
#[hook(addr = 0x616860, conv = cdecl, name = "CPedPlacement::IsPositionClearForPed-OG", expect = "D9 05 1C 8C 85 00")]
pub fn is_position_clear_for_ped(pos: GPtr<f32>, radius: f32, max: i32, out: GPtr<u8>, veh: u32, peds: u32, objs: u32) -> u8 {
    let mut r = radius;
    let c = x(f32::from_bits(0xBF80_0000)).fcom(x(radius));
    if !c.test_parity(0x44) {
        r = f32::from_bits(0x3F40_0000);
    }
    let max = if max == -1 { 2 } else { max };
    let mut n: i16 = 0;
    find_objects_kinda_colliding(
        pos,
        r.to_bits(),
        1,
        lp(&mut n).cast(),
        max as u32,
        out,
        0,
        (veh & 0xFF != 0) as u32,
        (peds & 0xFF != 0) as u32,
        (objs & 0xFF != 0) as u32,
        0,
    );
    (n == 0) as u8
}

/// `CPedPlacement::IsPositionClearOfCars(const CPed*)` (0x616A40, cdecl): the vehicle
/// `CWorld::TestSphereAgainstWorld(ped pos, 0.25, vehicles only)` finds; for non-automobiles and for
/// automobiles with flag 4 of +0x429 it counts only when `CCollision::ProcessColModels(ped, vehicle)`
/// reports contacts (into `CWorld::m_aTempColPts`); else null.
#[hook(addr = 0x616A40, conv = cdecl, name = "CPedPlacement::IsPositionClearOfCars-Ped", expect = "56 57 8B 7C 24 0C")]
pub fn is_position_clear_of_cars(ped: GPtr<u8>) -> GPtr<u8> {
    let p = ent_pos(ped);
    let pos = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
    let veh = test_sphere_against_world(pos[0].to_bits(), pos[1].to_bits(), pos[2].to_bits(), 0x3E80_0000, 0, 0, 1, 0, 0, 0, 0);
    if veh.is_null() {
        return GPtr::from_addr(0);
    }
    if rd::<u32>(veh, 0x594) == 0 && rd::<u8>(veh, 0x429) & 4 == 0 {
        return veh;
    }
    let mi = |e: GPtr<u8>| g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(e, 0x22))).read();
    let vcol = rd::<GPtr<u8>>(mi(veh), 0x14);
    let pcol = rd::<GPtr<u8>>(mi(ped), 0x14);
    let n = process_col_models(rd::<GPtr<u8>>(ped, 0x14), pcol, rd::<GPtr<u8>>(veh, 0x14), vcol, TEMP_COL_PTS, 0, 0, 0);
    if n != 0 { veh } else { GPtr::from_addr(0) }
}

/// `CPedPlacement::FindZCoorForPed(CVector& pos)` (0x616920, cdecl, AL): two vertical lines (buildings,
/// vehicles, dummies) from `z + 0.5` down to `z - 100`, at (x, y) and (x + 0.1, y + 0.1); the higher hit
/// (a miss counts as -100) above -99 becomes `pos.z = hit + 1` and the result true.
#[hook(addr = 0x616920, conv = cdecl, name = "CPedPlacement::FindZCoorForPed", expect = "83 EC 4C 56 8B 74 24 54")]
pub fn find_z_coor_for_ped(pos: GPtr<f32>) -> u8 {
    let p = pos.cast::<u8>();
    let mut v = [rd::<f32>(p, 0), rd::<f32>(p, 4), x(rd::<f32>(p, 8)).fadd_m(0.5).to_f32()];
    let mut cp = [0u32; 11];
    let mut ent: u32 = 0;
    let mut z1 = f32::from_bits(0xC2C8_0000);
    let z2_miss = f32::from_bits(0xC2C8_0000);
    let end = || x(rd::<f32>(p, 8)).fsub_m(100.0).to_f32().to_bits();
    let cpp = lp(&mut cp).cast::<u8>();
    let entp = lp(&mut ent).cast::<u8>();
    if process_vertical_line(lp(&mut v).cast(), end(), cpp, entp, 1, 1, 0, 0, 1, 0, 0) != 0 {
        z1 = rd::<f32>(cpp, 8);
    }
    v[0] = x(v[0]).fadd_m(f32::from_bits(0x3DCC_CCCD)).to_f32();
    v[1] = x(v[1]).fadd_m(f32::from_bits(0x3DCC_CCCD)).to_f32();
    let z2 = if process_vertical_line(lp(&mut v).cast(), end(), cpp, entp, 1, 1, 0, 0, 1, 0, 0) != 0 {
        rd::<f32>(cpp, 8)
    } else {
        z2_miss
    };
    let mut z = x(z2);
    if !x(z1).fcom(z).test_any(0x41) {
        z = x(z1);
    }
    if z.fcom(x(f32::from_bits(0xC2C6_0000))).test_any(0x41) {
        return 0;
    }
    wr::<f32>(p, 8, z.fadd_m(1.0).to_f32());
    1
}

/// `TheCamera` (CCamera)
const THE_CAMERA: u32 = 0xB6F028;

orig! {
    /// `CPed::AttachPedToEntity(CEntity*, CVector, uint16, float, eWeaponType)` (0x5E7CB0, `ret 0x1C`)
    fn attach_ped_to_entity_call(ped: GPtr<u8>, e: GPtr<u8>, ox: u32, oy: u32, oz: u32, angle: u32, range: u32, weapon: i32) -> GPtr<u8> = thiscall 0x5E7CB0 => attach_ped_to_entity;
    /// `CPed::PositionAttachedPed()` (0x5DFDF0)
    fn position_attached_ped_call(ped: GPtr<u8>) = thiscall 0x5DFDF0 => position_attached_ped;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0, `ret 0xC`)
    fn apply_move_force(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x5429F0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn blend_animation(c: GPtr<u8>, group: i32, id: i32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(ty: i32, skill: i32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0)
    fn set_current_weapon_slot(ped: GPtr<u8>, slot: i32) = thiscall 0x5E61F0 => unported;
    /// `CPlayerPed::ClearWeaponTarget()` (0x609C80)
    fn player_clear_weapon_target(ped: GPtr<u8>) = thiscall 0x609C80 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080, `ret 0xC`)
    fn give_weapon(ped: GPtr<u8>, ty: i32, ammo: u32, b: u32) -> i32 = thiscall 0x5E6080 => unported;
    /// `CPlayerPed::MakeChangesForNewWeapon(eWeaponType)` (0x60B460)
    fn player_make_changes_for_new_weapon(ped: GPtr<u8>, ty: i32) = thiscall 0x60B460 => unported;
    /// `CCamera::SetNewPlayerWeaponMode(int16 mode, int16 maxZoom, int16 minZoom)` (0x50BFB0)
    fn camera_set_new_player_weapon_mode(cam: GPtr<u8>, mode: u32, max: u32, min: u32) = thiscall 0x50BFB0 => unported;
    /// `CPed::GetEntityThatThisPedIsHolding()` (0x5E02E0)
    fn get_entity_that_this_ped_is_holding(ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E02E0 => unported;
    /// `CPed::DropEntityThatThisPedIsHolding(bool)` (0x5E0360)
    fn drop_entity_call(ped: GPtr<u8>, delete: u32) = thiscall 0x5E0360 => drop_entity_that_this_ped_is_holding;
    /// `CObject::operator new(uint32)` (0x5A1EE0)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CObject::CObject(int32 model, bool created)` (0x5A1D70, `ret 8`)
    fn object_ctor(o: GPtr<u8>, model: i32, created: u32) -> GPtr<u8> = thiscall 0x5A1D70 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleHoldEntity::CTaskSimpleHoldEntity(CEntity*, const CVector*, uint8 boneFrameId,
    /// uint8 boneFlags, AnimationId, AssocGroupId, bool)` (0x6913A0)
    fn task_hold_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, off: GPtr<f32>, bone: u32, flags: u32, anim: i32, group: i32, b: u32) -> GPtr<u8> = thiscall 0x6913A0 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32 slot)` (0x681B60)
    fn task_manager_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, slot: i32) = thiscall 0x681B60 => unported;
    /// `CPed::SortPeds(CPed**, int32, int32)` (0x5E17E0)
    fn sort_peds_call(ped: GPtr<u8>, list: GPtr<GPtr<u8>>, min: i32, max: i32) = thiscall 0x5E17E0 => sort_peds;
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, bool x8)` (0x56BA00, AL)
    fn process_line_of_sight(a: GPtr<f32>, b: GPtr<f32>, cp: GPtr<u8>, ent: GPtr<u8>, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, b6: u32, b7: u32, b8: u32) -> u8 = cdecl 0x56BA00 => unported;
}

/// The ped's weapon slot `slot` (0x1C bytes each from +0x5A0).
fn weapon_at(ped: GPtr<u8>, slot: i8) -> GPtr<u8> {
    ped.byte_add(0x5A0 + isize::from(slot) * 0x1C)
}

/// `CPed::AttachPedToBike(CEntity*, CVector offset, uint16 angle, float range, float range2, eWeaponType)`
/// (0x5E7E60, `ret 0x20`): `AttachPedToEntity(entity, offset, angle, range, weapon)`; when that succeeds
/// +0x780 = `range2`. Returns the entity (or 0).
#[hook(addr = 0x5E7E60, conv = thiscall, name = "CPed::AttachPedToBike", expect = "8B 44 24 20 8B 54 24 14", this = "CPed")]
#[allow(clippy::too_many_arguments)]
pub fn attach_ped_to_bike(this: GPtr<u8>, e: GPtr<u8>, ox: u32, oy: u32, oz: u32, angle: u32, range: u32, range2: u32, weapon: i32) -> GPtr<u8> {
    let r = attach_ped_to_entity_call(this, e, ox, oy, oz, angle, range, weapon);
    if r.is_null() {
        return r;
    }
    wr::<u32>(this, 0x780, range2);
    e
}

/// `CPed::AttachPedToEntity(CEntity*, CVector offset, uint16 angle, float range, eWeaponType)` (0x5E7CB0,
/// `ret 0x1C`): nothing (0) without entity or while in a vehicle (+0x46C bit 0x100, read in the split chunk
/// 0x401697). Sets the attach entity (+0xFC, registered reference), offset (+0x76C), angle (word +0x778)
/// and range (+0x77C); players lose `bUsesCollision`, others ignore a vehicle's collision (+0x128). The
/// current weapon is saved once (+0x70C type / +0x784 ammo, 0x37 = none saved). Players: `GiveWeapon(w,
/// 30000)` (w != 0), player data +0x20 = w, `MakeChangesForNewWeapon(w)`, camera mode 0x2E for the
/// camera (0x2B), 0x41 (and player data +0x34 |= 8) on model 0x152 with a weapon without flag 4, else
/// 0x2D; ped state 0xC. Others: `GiveWeapon(w, 30000)` and `SetCurrentWeapon(slot of w)`. Then
/// `PositionAttachedPed`; returns the entity.
#[hook(addr = 0x5E7CB0, conv = thiscall, name = "CPed::AttachPedToEntity", expect = "55 8B 6C 24 08", this = "CPed")]
#[allow(clippy::too_many_arguments)]
pub fn attach_ped_to_entity(this: GPtr<u8>, e: GPtr<u8>, ox: u32, oy: u32, oz: u32, angle: u32, range: u32, weapon: i32) -> GPtr<u8> {
    if e.is_null() || rd::<u32>(this, 0x46C) & 0x100 != 0 {
        return GPtr::from_addr(0);
    }
    wr::<GPtr<u8>>(this, 0xFC, e);
    register_reference(e, this.byte_add(0xFC));
    wr::<u32>(this, 0x76C, ox);
    wr::<u32>(this, 0x770, oy);
    wr::<u32>(this, 0x774, oz);
    wr::<u32>(this, 0x77C, range);
    let pt = rd::<i32>(this, 0x598);
    wr::<u16>(this, 0x778, angle as u16);
    let player = pt == 0 || pt == 1;
    if player {
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & !1);
    } else if rd::<u8>(e, 0x36) & 7 == 2 {
        wr::<GPtr<u8>>(this, 0x128, e);
    }
    if rd::<i32>(this, 0x70C) == 0x37 {
        let w = weapon_at(this, rd::<i8>(this, 0x718));
        wr::<u32>(this, 0x70C, rd::<u32>(w, 0));
        wr::<u32>(this, 0x784, rd::<u32>(w, 0xC));
    }
    if !player {
        give_weapon(this, weapon, 30000, 1);
        let slot = rd::<i32>(get_weapon_info(weapon, 1), 0x14);
        set_current_weapon_slot(this, slot);
        position_attached_ped_call(this);
        return e;
    }
    if weapon != 0 {
        give_weapon(this, weapon, 30000, 1);
    }
    wr::<u8>(rd::<GPtr<u8>>(this, 0x480), 0x20, weapon as u8);
    player_make_changes_for_new_weapon(this, weapon);
    let cam = GPtr::<u8>::from_addr(THE_CAMERA as usize);
    if weapon == 0x2B {
        camera_set_new_player_weapon_mode(cam, 0x2E, 0, 0);
    } else if rd::<u16>(e, 0x22) == 0x152 && (rd::<u32>(get_weapon_info(weapon, 1), 0x18) >> 2) & 1 == 0 {
        camera_set_new_player_weapon_mode(cam, 0x41, 0, 0);
        let pd = rd::<GPtr<u8>>(this, 0x480);
        wr::<u32>(pd, 0x34, rd::<u32>(pd, 0x34) | 8);
    } else {
        camera_set_new_player_weapon_mode(cam, 0x2D, 0, 0);
    }
    wr::<i32>(this, 0x530, 0xC);
    position_attached_ped_call(this);
    e
}

/// `CPed::DettachPedFromEntity()` (0x5E7EC0): clears the attach entity (+0xFC). In ped state 0x36 (die)
/// the entity's collision is ignored (+0x128), its matrix allocated if needed, and the ped is pushed by
/// `ApplyMoveForce(-4 * entity forward + (0, 0, 4))` and loses `bIsStanding`. Not in state 0x37 (dead):
/// blend anim 3 of the ped's anim group (delta 1000), `bUsesCollision` on, a saved weapon (+0x70C != 0x37)
/// replaces the current one (ammo of the current slot zeroed, `SetCurrentWeapon(slot)`, ammo +0x784), and
/// players `ClearWeaponTarget` (a tail jump in the bytes).
#[hook(addr = 0x5E7EC0, conv = thiscall, name = "CPed::DettachPedFromEntity", expect = "83 EC 18 56 8B F1", this = "CPed")]
pub fn dettach_ped_from_entity(this: GPtr<u8>) {
    let st = rd::<i32>(this, 0x530);
    let e = rd::<GPtr<u8>>(this, 0xFC);
    wr::<u32>(this, 0xFC, 0);
    if st == 0x36 {
        wr::<GPtr<u8>>(this, 0x128, e);
        if rd::<GPtr<u8>>(e, 0x14).is_null() {
            allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let f = rd::<GPtr<u8>>(e, 0x14).byte_add(0x10);
        let m4 = f32::from_bits(0xC080_0000);
        let fx = x(rd::<f32>(f, 0)).fmul_m(m4).to_f32();
        let fy = x(rd::<f32>(f, 4)).fmul_m(m4).to_f32();
        let fz = x(rd::<f32>(f, 8)).fmul_m(m4).to_f32();
        let fz = x(fz).fadd_m(4.0).to_f32();
        apply_move_force(this, fx.to_bits(), fy.to_bits(), fz.to_bits());
        wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) & !1);
        return;
    }
    if st == 0x37 {
        return;
    }
    blend_animation(rd::<GPtr<u8>>(this, 0x18), rd::<i32>(this, 0x4D4), 3, 0x447A_0000);
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 1);
    if rd::<i32>(this, 0x70C) != 0x37 {
        wr::<u32>(weapon_at(this, rd::<i8>(this, 0x718)), 8, 0);
        wr::<u32>(weapon_at(this, rd::<i8>(this, 0x718)), 0xC, 0);
        let slot = rd::<i32>(get_weapon_info(rd::<i32>(this, 0x70C), 1), 0x14);
        set_current_weapon_slot(this, slot);
        wr::<u32>(weapon_at(this, rd::<i8>(this, 0x718)), 0xC, rd::<u32>(this, 0x784));
        wr::<u32>(this, 0x70C, 0x37);
    }
    let pt = rd::<i32>(this, 0x598);
    if pt == 0 || pt == 1 {
        player_clear_weapon_target(this);
    }
}

/// `CPed::GiveObjectToPedToHold(int32 modelIndex, bool replace)` (0x5E4390, `ret 8`): with an active hold
/// task (0x133) and no replacement (`replace` drops a held entity, `DropEntityThatThisPedIsHolding(true)`)
/// nothing (0). Else a new `CObject(model, false)` at the ped's position is added to the world and held
/// through a new `CTaskSimpleHoldEntity(object, (0,0,0), 6, 0x10, 0xBF, 0, true)` in secondary slot 4.
/// Returns the object. Not reproduced: the SEH frame of the `new` calls.
#[hook(addr = 0x5E4390, conv = thiscall, name = "CPed::GiveObjectToPedToHold", expect = "64 A1 00 00 00 00", this = "CPed")]
pub fn give_object_to_ped_to_hold(this: GPtr<u8>, model: i32, replace: u32) -> GPtr<u8> {
    let tm = rd::<GPtr<u8>>(this, 0x47C).byte_add(4);
    let t = find_active_task_by_type(tm, 0x133);
    let mut dropped = false;
    if !get_entity_that_this_ped_is_holding(this).is_null() && replace & 0xFF != 0 {
        drop_entity_call(this, 1);
        dropped = true;
    }
    if !t.is_null() && !dropped {
        return GPtr::from_addr(0);
    }
    let mem = object_new(0x17C);
    let obj = if mem.is_null() { mem } else { object_ctor(mem, model, 0) };
    let p = ent_pos(this);
    let v = [x(p.read()).to_f32(), x(p.wrapping_index(1).read()).to_f32(), x(p.wrapping_index(2).read()).to_f32()];
    let m = rd::<GPtr<u8>>(obj, 0x14);
    let dst = if m.is_null() { obj.byte_add(4) } else { m.byte_add(0x30) };
    for (i, c) in v.iter().enumerate() {
        wr::<f32>(dst, 4 * i as isize, *c);
    }
    world_add(obj);
    let mut off = [0.0f32; 3];
    let mem = task_new(0x3C);
    let task = if mem.is_null() { mem } else { task_hold_entity_ctor(mem, obj, lp(&mut off).cast(), 6, 0x10, 0xBF, 0, 1) };
    task_manager_set_task_secondary(rd::<GPtr<u8>>(this, 0x47C).byte_add(4), task, 4);
    obj
}

/// `|this pos - e pos|` as the bytes compute it: `sqrt((dz*dz + dy*dy) + dx*dx)` in x87.
fn dist_to(this: GPtr<u8>, e: GPtr<u8>) -> X87 {
    let a = ent_pos(this);
    let b = ent_pos(e);
    let dx = x(a.read()).fsub_m(b.read());
    let dy = x(a.wrapping_index(1).read()).fsub_m(b.wrapping_index(1).read());
    let dz = x(a.wrapping_index(2).read()).fsub_m(b.wrapping_index(2).read());
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// `CPed::SortPeds(CPed** list, int32 min, int32 max)` (0x5E17E0, `ret 0xC`): quicksort by distance to
/// this ped (pivot the middle element, its distance stored as f32, the scanned distances compared as x87
/// values); the left part recurses (through the hook entry, as the original's `call`), the right part
/// loops.
#[hook(addr = 0x5E17E0, conv = thiscall, name = "CPed::SortPeds", expect = "53 8B 5C 24 10", this = "CPed")]
pub fn sort_peds(this: GPtr<u8>, list: GPtr<GPtr<u8>>, min: i32, max: i32) {
    if min >= max {
        return;
    }
    let mut min = min;
    let el = |i: i32| list.wrapping_index(i);
    loop {
        let mut i = min;
        let mut j = max;
        let pivot = el(i.wrapping_add(j) / 2).read();
        let pd = x(dist_to(this, pivot).to_f32());
        loop {
            while !dist_to(this, el(i).read()).fcom(pd).test_parity(5) {
                i += 1;
            }
            while !dist_to(this, el(j).read()).fcom(pd).test_any(0x41) {
                j -= 1;
            }
            if i > j {
                break;
            }
            let a = el(i).read();
            el(i).write(el(j).read());
            el(j).write(a);
            i += 1;
            j -= 1;
            if i > j {
                break;
            }
        }
        sort_peds_call(this, list, min, j);
        if i >= max {
            return;
        }
        min = i;
    }
}

/// `CPed::OurPedCanSeeThisEntity(CEntity*, bool isSpotted)` (0x5E1660, `ret 8`, AL): unless `isSpotted`,
/// the entity must be in front (`dot(d.xy, forward.xy) >= 0`) and nearer than 40 (2D). Then a line of
/// sight from the ped (z + 1) to the entity (peds z + 1): buildings only, plus objects and the see-through
/// check when spotted; visible when nothing is hit.
#[hook(addr = 0x5E1660, conv = thiscall, name = "CPed::OurPedCanSeeThisEntity", expect = "83 EC 44 53 55", this = "CPed")]
pub fn our_ped_can_see_this_entity(this: GPtr<u8>, e: GPtr<u8>, spotted: u32) -> u8 {
    let ep = ent_pos(e);
    let pp = ent_pos(this);
    let dx = x(ep.read()).fsub_m(pp.read()).to_f32();
    let dy = x(ep.wrapping_index(1).read()).fsub_m(pp.wrapping_index(1).read()).to_f32();
    let spotted = spotted & 0xFF != 0;
    if !spotted {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let dot = x(dy).fmul_m(rd::<f32>(m, 0x14)).fadd(x(dx).fmul_m(rd::<f32>(m, 0x10)));
        if dot.fcom(X87::zero()).test_any(1) {
            return 0;
        }
        let d = x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)).fsqrt();
        if d.fcom(x(40.0)).test_parity(5) {
            return 0;
        }
    }
    let pp = ent_pos(this);
    let mut a = [pp.read(), pp.wrapping_index(1).read(), pp.wrapping_index(2).read()];
    a[2] = x(a[2]).fadd_m(1.0).to_f32();
    let ep = ent_pos(e);
    let mut b = [ep.read(), ep.wrapping_index(1).read(), ep.wrapping_index(2).read()];
    if rd::<u8>(e, 0x36) & 7 == 3 {
        b[2] = x(b[2]).fadd_m(1.0).to_f32();
    }
    let mut cp = [0u32; 11];
    let mut hit_ent: u32 = 0;
    let (ap, bp, cpp, hp) = (lp(&mut a).cast(), lp(&mut b).cast(), lp(&mut cp).cast(), lp(&mut hit_ent).cast());
    let hit = if spotted {
        process_line_of_sight(ap, bp, cpp, hp, 1, 0, 0, 1, 0, 0, 0, 1)
    } else {
        process_line_of_sight(ap, bp, cpp, hp, 1, 0, 0, 0, 0, 0, 0, 0)
    };
    (hit == 0) as u8
}

/// A `CMatrix` local of the original (0x48 bytes; attach pointer +0x40 and owns flag +0x44 are 0 as the
/// original's inlined constructor leaves them, the rest uninitialised there and 0 here).
type LocalMatrix = [u32; 18];

orig! {
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::Detach()` (0x59ACF0)
    fn matrix_detach(m: GPtr<u8>) = thiscall 0x59ACF0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(dst: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790, cdecl, returns out)
    fn multiply_3x3(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C790 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0)
    fn matrix_set_rotate_z(m: GPtr<u8>, a: u32) = thiscall 0x59B0E0 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80)
    fn matrix_mul_assign(dst: GPtr<u8>, rhs: GPtr<u8>) = thiscall 0x411A80 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610)
    fn placeable_set_matrix(e: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// `CPed::PositionAttachedPed()` (0x5DFDF0), spec `CPed.md`: the attach entity's matrix (FIRELA 0x220 with
/// a ladder frame (+0x69C) and offset z < -900: the frame's LTM, offset z + 1000) moved by
/// `Multiply3x3(M, offset)`; non-players get their heading (+0x558) limited to `+0x77C` around the entity
/// heading turned by mode +0x778 (1: +pi/2, 2: +pi, 3: -pi/2), each step through `LimitRadianAngle`;
/// `M *= RotateZ(heading - entity heading)`, `SetMatrix(M)`; vehicles/objects pass on their move and turn
/// speeds; `bIsStanding` set, no ground entity. Not reproduced: the SEH frame.
#[hook(addr = 0x5DFDF0, conv = thiscall, name = "CPed::PositionAttachedPed", expect = "64 A1 00 00 00 00", this = "CPed")]
pub fn position_attached_ped(this: GPtr<u8>) {
    let att = rd::<GPtr<u8>>(this, 0xFC);
    if att.is_null() {
        return;
    }
    let mut mm: LocalMatrix = [0; 18];
    let mut rm: LocalMatrix = [0; 18];
    let m = lp(&mut mm).cast::<u8>();
    let r = lp(&mut rm).cast::<u8>();
    let mut off: [u32; 3];
    let frame = rd::<GPtr<u8>>(att, 0x69C);
    let firela = rd::<u16>(att, 0x22) == 0x220
        && !frame.is_null()
        && !x(rd::<f32>(this, 0x774)).fcom(x(f32::from_bits(0xC461_0000))).test_parity(5);
    if firela {
        matrix_attach(m, rw_frame_get_ltm(frame), 0);
        matrix_detach(m);
        off = [rd::<u32>(this, 0x76C), rd::<u32>(this, 0x770), 0];
        off[2] = x(rd::<f32>(this, 0x774)).fadd_m(1000.0).to_f32().to_bits();
    } else {
        if rd::<GPtr<u8>>(att, 0x14).is_null() {
            allocate_matrix(att);
            simple_transform_update_matrix(att.byte_add(4), rd::<GPtr<u8>>(att, 0x14));
        }
        matrix_assign(m, rd::<GPtr<u8>>(att, 0x14));
        off = [rd::<u32>(this, 0x76C), rd::<u32>(this, 0x770), rd::<u32>(this, 0x774)];
    }
    let mut tmp = [0.0f32; 3];
    let w = multiply_3x3(lp(&mut tmp).cast(), m, lp(&mut off).cast());
    let nz = x(rd::<f32>(m, 0x38)).fadd_m(w.wrapping_index(2).read()).to_f32();
    let ny = x(rd::<f32>(m, 0x34)).fadd_m(w.wrapping_index(1).read()).to_f32();
    let nx = x(rd::<f32>(m, 0x30)).fadd_m(w.read()).to_f32();
    wr::<f32>(m, 0x30, nx);
    wr::<f32>(m, 0x34, ny);
    wr::<f32>(m, 0x38, nz);
    let h = x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14))).to_f32();
    let pt = rd::<i32>(this, 0x598);
    if pt != 0 && pt != 1 {
        const HALF_PI: f32 = f32::from_bits(0x3FC9_0FDB);
        const PI: f32 = f32::from_bits(0x4049_0FDB);
        const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
        const NEG_PI: f32 = f32::from_bits(0xC049_0FDB);
        let t = match rd::<u16>(this, 0x778) {
            1 => x(h).fadd_m(HALF_PI).to_f32(),
            2 => x(h).fadd_m(PI).to_f32(),
            3 => x(h).fsub_m(HALF_PI).to_f32(),
            _ => h,
        };
        let t = X87::from_f64_exact(limit_radian_angle(t.to_bits())).to_f32();
        let cur = X87::from_f64_exact(limit_radian_angle(rd::<u32>(this, 0x558)));
        wr::<f32>(this, 0x558, cur.to_f32());
        let mut d = cur.fsub_m(t);
        if !d.fcom(x(PI)).test_any(0x41) {
            d = d.fsub_m(TWO_PI);
        } else if !d.fcom(x(NEG_PI)).test_parity(5) {
            d = d.fadd_m(TWO_PI);
        }
        let lim = rd::<f32>(this, 0x77C);
        if !d.fcom(x(lim)).test_any(0x41) {
            wr::<f32>(this, 0x558, x(t).fadd_m(lim).to_f32());
        } else if !d.fcom(x(lim).fchs()).test_parity(5) {
            wr::<f32>(this, 0x558, x(t).fsub_m(lim).to_f32());
        }
        let v = X87::from_f64_exact(limit_radian_angle(rd::<u32>(this, 0x558))).to_f32();
        wr::<f32>(this, 0x558, v);
    }
    let a = x(rd::<f32>(this, 0x558)).fsub_m(h).to_f32();
    matrix_set_rotate_z(r, a.to_bits());
    matrix_mul_assign(m, r);
    placeable_set_matrix(this, m);
    let att = rd::<GPtr<u8>>(this, 0xFC);
    let ty = rd::<u8>(att, 0x36) & 7;
    if ty == 2 || ty == 4 {
        copy3(this.byte_add(0x44), att.byte_add(0x44));
        copy3(this.byte_add(0x50), att.byte_add(0x50));
    }
    wr::<u32>(this, 0x568, 0);
    wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) | 1);
    matrix_dtor(r);
    matrix_dtor(m);
}

/// `bHalfShoulderRotation` (static bool, 1 in the shipped .data, 0x8D21E0)
const HALF_SHOULDER_ROTATION: usize = 0x8D21E0;

orig! {
    /// `CMatrix::CMatrix(RwMatrix*, bool owns)` (0x59C050)
    fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `Invert(const CMatrix&)` (0x59BDD0, cdecl, hidden return matrix constructed by the callee)
    fn matrix_invert(out: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59BDD0 => unported;
    /// `operator*(const CMatrix&, const CMatrix&)` (0x59BE30, cdecl, hidden return matrix)
    fn matrix_mul(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59BE30 => unported;
    /// `CMatrix::ConvertToEulerAngles(float*, float*, float*, uint32)` (0x59A840)
    fn matrix_to_euler(m: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>, c: GPtr<f32>, flags: u32) = thiscall 0x59A840 => unported;
    /// `CMatrix::ConvertFromEulerAngles(float, float, float, uint32)` (0x59AA40)
    fn matrix_from_euler(m: GPtr<u8>, a: u32, b: u32, c: u32, flags: u32) = thiscall 0x59AA40 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
}

/// The RwMatrix of bone `id` in `hier` (64 bytes each).
fn node_matrix(hier: GPtr<u8>, id: i32) -> GPtr<u8> {
    bone_matrix(hier, id)
}

/// One side of `ShoulderBoneRotation` after the breast matrix `a` (attached) and the clavicle matrix `b`
/// are set up: `C = Invert(B)` (into `c` directly the first time, through `t` and `operator=` the second),
/// `A = C * A`, x Euler angle halved (when the static flag is set), `A = B * A`, `A.UpdateRW()`.
fn shoulder_side(a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, t: GPtr<u8>, first: bool) {
    if first {
        matrix_invert(c, b);
    } else {
        let r = matrix_invert(t, b);
        matrix_assign(c, r);
        matrix_dtor(t);
    }
    let r = matrix_mul(t, c, a);
    matrix_assign(a, r);
    matrix_dtor(t);
    let mut e = [0.0f32; 3];
    let ep = lp(&mut e).cast::<f32>();
    matrix_to_euler(a, ep, ep.wrapping_index(2), ep.wrapping_index(1), 0x15);
    if g::<u8>(HALF_SHOULDER_ROTATION).read() != 0 {
        e[0] = x(e[0]).fmul_m(0.5).to_f32();
    }
    matrix_from_euler(a, e[0].to_bits(), e[2].to_bits(), e[1].to_bits(), 0x15);
    let r = matrix_mul(t, b, a);
    matrix_assign(a, r);
    matrix_dtor(t);
}

/// `CPed::ShoulderBoneRotation(RpClump*)` (0x5DF560, static cdecl), deviations file: for the left then the
/// right side the breast bone (0x12E / 0x12D) matrix is replaced by the upper arm's (0x20 / 0x16), then
/// rotated relative to the clavicle (0x1F / 0x15) with half its x Euler angle (`ORDER_ZYX | SWAP_XZ` =
/// 0x15). The left side constructs `A(breast)` / `B(clavicle)` and inverts into C; the right side
/// re-attaches A and B and inverts through a temporary (as the bytes do). Not reproduced: the SEH frame.
#[hook(addr = 0x5DF560, conv = cdecl, name = "CPed::ShoulderBoneRotation", expect = "6A FF 68 75 D8 83 00")]
pub fn shoulder_bone_rotation(clump: GPtr<u8>) {
    let hier = get_anim_hierarchy_from_skin_clump(clump);
    let mut am: LocalMatrix = [0; 18];
    let mut bm: LocalMatrix = [0; 18];
    let mut cm: LocalMatrix = [0; 18];
    let mut tm: LocalMatrix = [0; 18];
    let (a, b, c, t) = (lp(&mut am).cast::<u8>(), lp(&mut bm).cast::<u8>(), lp(&mut cm).cast::<u8>(), lp(&mut tm).cast::<u8>());
    let copy16 = |dst: GPtr<u8>, src: GPtr<u8>| {
        for i in 0..16 {
            wr::<u32>(dst, 4 * i, rd::<u32>(src, 4 * i));
        }
    };

    let breast = node_matrix(hier, 0x12E);
    copy16(breast, node_matrix(hier, 0x20));
    matrix_ctor_rw(a, breast, 0);
    matrix_ctor_rw(b, node_matrix(hier, 0x1F), 0);
    shoulder_side(a, b, c, t, true);
    matrix_update_rw(a);

    let breast = node_matrix(hier, 0x12D);
    copy16(breast, node_matrix(hier, 0x16));
    matrix_attach(a, breast, 0);
    matrix_attach(b, node_matrix(hier, 0x15), 0);
    shoulder_side(a, b, c, t, false);
    matrix_update_rw(a);

    matrix_dtor(c);
    matrix_dtor(b);
    matrix_dtor(a);
}

/// `CWorld::ms_nCurrentScanCode` (u16)
const CURRENT_SCAN_CODE: usize = 0xB7CD78;
/// `CWorld::ms_aRepeatSectors` (16 x 16, 12 bytes each: vehicles, peds, objects lists)
const REPEAT_SECTORS: usize = 0xB992B8;

orig! {
    /// CRT `floor(double)` (0x8219F0, ST0)
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CPed::MakeTyresMuddySectorList(CPtrListSingleLink&)` (0x6AE0D0, `ret 4`)
    fn make_tyres_muddy_sector_list(ped: GPtr<u8>, list: GPtr<u8>) = thiscall 0x6AE0D0 => unported;
}

/// `floor` then `_ftol` of a sector coordinate, as the bytes do it: the first `floor` takes the register
/// value, a positive (resp. below-limit) result is recomputed from the value stored as f32.
fn sector_floor(v: X87) -> i32 {
    ftol32(X87::from_f64_exact(crt_floor(v.to_f64())))
}

/// `CPed::DeadPedMakesTyresBloody()` (0x6B4200): the repeat sectors overlapping the ped's position +- 2.0
/// (`GetSectorX/Y` = `v * 0.02 + 60`, floored, start clamped to >= 0 and end to <= 0x77, each kept value
/// recomputed from the f32 the bytes store), a new scan code (`ClearScanCodes` and 1 at 0xFFFF), then
/// `MakeTyresMuddySectorList` on the vehicle list of each sector `(y & 15) * 16 + (x & 15)`, rows y outer.
#[hook(addr = 0x6B4200, conv = thiscall, name = "CPed::DeadPedMakesTyresBloody", expect = "55 8B EC 83 E4 F8 83 EC 18", this = "CPed")]
pub fn dead_ped_makes_tyres_bloody(this: GPtr<u8>) {
    let two = f32::from_bits(0x4000_0000);
    let k = f32::from_bits(0x3CA3_D70A);
    let p = ent_pos(this);
    let min_x = x(p.read()).fsub_m(two);
    let max_x = x(p.read()).fadd_m(two).to_f32();
    let min_y = x(p.wrapping_index(1).read()).fsub_m(two).to_f32();
    let max_y = x(p.wrapping_index(1).read()).fadd_m(two).to_f32();
    let sector = |v: X87| {
        let s = v.fmul_m(k).fadd_m(60.0);
        (s, s.to_f32())
    };
    let (s, sf) = sector(min_x);
    let start_x = if sector_floor(s) > 0 { sector_floor(x(sf)) } else { 0 };
    let (s, sf) = sector(x(min_y));
    let start_y = if sector_floor(s) > 0 { sector_floor(x(sf)) } else { 0 };
    let (s, sf) = sector(x(max_x));
    let end_x = if sector_floor(s) < 0x77 { sector_floor(x(sf)) } else { 0x77 };
    let (s, sf) = sector(x(max_y));
    let end_y = if sector_floor(s) < 0x77 { sector_floor(x(sf)) } else { 0x77 };
    let sc = g::<u16>(CURRENT_SCAN_CODE);
    if sc.read() == 0xFFFF {
        world_clear_scan_codes();
        sc.write(1);
    } else {
        sc.write(sc.read() + 1);
    }
    let mut y = start_y;
    while y <= end_y {
        let mut xs = start_x;
        while xs <= end_x {
            let idx = ((y & 0xF) << 4).wrapping_add(xs & 0xF);
            let list = GPtr::<u8>::from_addr(REPEAT_SECTORS.wrapping_add((idx as usize).wrapping_mul(12)));
            make_tyres_muddy_sector_list(this, list);
            xs += 1;
        }
        y += 1;
    }
}

orig! {
    /// `CPedPlacement::FindZCoorForPed(CVector&)` (0x616920, AL)
    fn find_z_coor_for_ped_call(pos: GPtr<f32>) -> u8 = cdecl 0x616920 => find_z_coor_for_ped;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
}

/// Writes `v` (raw dwords) as the entity position (matrix +0x30 or placement +4).
fn set_pos_raw(e: GPtr<u8>, v: [u32; 3]) {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        for (i, c) in v.iter().enumerate() {
            wr::<u32>(e, 4 + 4 * i as isize, *c);
        }
    } else {
        for (i, c) in v.iter().enumerate() {
            wr::<u32>(m, 0x30 + 4 * i as isize, *c);
        }
    }
}

/// `CPed::PositionAnyPedOutOfCollision()` (0x5E13C0, AL): a 15 x 15 grid with step 0.5 from (x - 3.5,
/// y - 3.5) at the ped's z (x inner, the candidate's z carried over from each `FindZCoorForPed`). A
/// candidate clear of buildings/objects (sphere 0.6, ignoring this ped) is free when the vehicle-only test
/// also finds nothing, else "on a car"; per kind the nearest (`(dz^2 + dy^2) + dx^2` stored as f32,
/// compared with the best kept as an int via `_ftol`, start 999) is remembered. Free wins; else the car
/// candidate raised by the bounding-box max z of the vehicle of the LAST vehicle test (null when that
/// test found nothing: original bug, kept). Returns whether the ped was moved.
#[hook(addr = 0x5E13C0, conv = thiscall, name = "CPed::PositionAnyPedOutOfCollision", expect = "83 EC 38 53 55", this = "CPed")]
pub fn position_any_ped_out_of_collision(this: GPtr<u8>) -> u8 {
    let half = f32::from_bits(0x3F00_0000);
    let off = f32::from_bits(0x4060_0000);
    let mut best_free: i32 = 999;
    let mut best_car: i32 = 999;
    let mut found_free = false;
    let mut found_car = false;
    let mut free_pos = [0u32; 3];
    let mut car_pos = [0u32; 3];
    let mut last_veh = GPtr::<u8>::from_addr(15);
    let p = ent_pos(this);
    let mut c = [0.0f32; 3];
    c[1] = x(p.wrapping_index(1).read()).fsub_m(off).to_f32();
    c[2] = ent_pos(this).wrapping_index(2).read();
    for _ in 0..15 {
        c[0] = x(ent_pos(this).read()).fsub_m(off).to_f32();
        for _ in 0..15 {
            find_z_coor_for_ped_call(lp(&mut c).cast());
            let (cx, cy, cz) = (c[0].to_bits(), c[1].to_bits(), c[2].to_bits());
            let blocked = test_sphere_against_world(cx, cy, cz, 0x3F19_999A, this.addr() as u32, 1, 0, 0, 1, 0, 0);
            if blocked.is_null() {
                let p = ent_pos(this);
                let dx = x(c[0]).fsub_m(p.read());
                let dy = x(c[1]).fsub_m(p.wrapping_index(1).read());
                let dz = x(c[2]).fsub_m(p.wrapping_index(2).read());
                let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).to_f32();
                last_veh = test_sphere_against_world(cx, cy, cz, 0x3F19_999A, this.addr() as u32, 0, 1, 0, 0, 0, 0);
                let (best, found, pos) = if last_veh.is_null() {
                    (&mut best_free, &mut found_free, &mut free_pos)
                } else {
                    (&mut best_car, &mut found_car, &mut car_pos)
                };
                if !xi(*best).fcom(x(d2)).test_any(0x41) {
                    *best = ftol32(x(d2));
                    *pos = [cx, cy, cz];
                    *found = true;
                }
            }
            c[0] = x(c[0]).fadd_m(half).to_f32();
        }
        c[1] = x(c[1]).fadd_m(half).to_f32();
    }
    if found_free {
        set_pos_raw(this, free_pos);
        return 1;
    }
    if !found_car {
        return 0;
    }
    let col = entity_get_col_model(last_veh);
    let z = x(f32::from_bits(car_pos[2])).fadd_m(rd::<f32>(col, 0x14)).to_f32();
    set_pos_raw(this, [car_pos[0], car_pos[1], z.to_bits()]);
    1
}

/// `CWorld::pIgnoreEntity`
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `ThePaths` (CPathFind)
const THE_PATHS: u32 = 0x96F050;
/// `ThePaths.m_pPathNodes[area]` (0x96F854)
const PATH_NODES: usize = 0x96F854;

orig! {
    /// `CVehicle::IsOnItsSide()` (0x6D1DD0, AL)
    fn vehicle_is_on_its_side(v: GPtr<u8>) -> u8 = thiscall 0x6D1DD0 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn placeable_set_posn(e: GPtr<u8>, v: GPtr<u32>) = thiscall 0x4241C0 => unported;
    /// `CPhysical::CheckCollision()` (0x54D920, AL)
    fn physical_check_collision(e: GPtr<u8>) -> u8 = thiscall 0x54D920 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490, AL)
    fn get_is_line_of_sight_clear(a: GPtr<f32>, b: GPtr<u32>, bb: u32, v: u32, p: u32, o: u32, d: u32, s: u32, c: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CCarEnterExit::GetPositionToOpenCarDoor(CVector* out, CVehicle*, int32 door)` (0x64E740, cdecl)
    fn get_position_to_open_car_door(out: GPtr<u32>, veh: GPtr<u8>, door: i32) -> GPtr<u32> = cdecl 0x64E740 => unported;
    /// `CPathFind::FindNodeClosestToCoors(CNodeAddress* out, CVector, int32 type, float, bool x5)` (0x44F460,
    /// `ret 0x2C`)
    fn find_node_closest_to_coors(pf: GPtr<u8>, out: GPtr<u32>, x: u32, y: u32, z: u32, ty: i32, dist: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> GPtr<u32> = thiscall 0x44F460 => unported;
    /// `CPathNode::GetPosition(CVector* out)` (0x420A10, `ret 4`)
    fn path_node_get_position(n: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, cdecl, hidden return pointer)
    fn vector_sub(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE60 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, h: u32) = thiscall 0x59B020 => unported;
}

/// The path node of a `CNodeAddress` dword (area low word, node high word): `m_pPathNodes[area] + node * 0x1C`.
fn path_node(addr: u32) -> GPtr<u8> {
    let area = (addr & 0xFFFF) as usize;
    let node = (addr >> 16) as isize;
    g::<GPtr<u8>>(PATH_NODES + area * 4).read().byte_add(node * 0x1C)
}

/// `CPed::PositionPedOutOfCollision(int32 exitDoor, CVehicle*, bool findClosestNode)` (0x5E0820, `ret 0xC`,
/// AL), spec `CPed.md` with these corrections from the bytes: candidates c and d are
/// `V + forward * (B.min.y - 0.355)` and `V + forward * (B.max.y + 0.355)` (the spec says right * min.x / max.x);
/// the "on its side" candidate is `V + (0, 0, B.max.x + 1)` (sic). Door candidates (steps 3) go through
/// `SetPosn`, the others are written into the position directly; every candidate is tested with
/// `CheckCollision` then `GetIsLineOfSightClear(V, c, buildings, objects)`. Products are rounded as the bytes
/// store them (`k * right.y/z`, `t * forward.y/z`, `side * right.z` as f32, the x terms in the register).
/// Not found and `findClosestNode`: the closest node of type 1 (999999.88; invalid: the last candidate
/// stays), replaced by the closest node of type 0 when that is strictly nearer to V in 2D (ties/NaN keep
/// the first); `FindZCoorForPed`, `SetPosn`, the vehicle heading, the vehicle's speeds zeroed and its z
/// speed 0.02.
#[hook(addr = 0x5E0820, conv = thiscall, name = "CPed::PositionPedOutOfCollision", expect = "83 EC 64 53 55", this = "CPed")]
pub fn position_ped_out_of_collision(this: GPtr<u8>, exit_door: i32, veh: GPtr<u8>, find_closest: u32) -> u8 {
    let mut veh = veh;
    if veh.is_null() {
        veh = rd::<GPtr<u8>>(this, 0x58C);
        if veh.is_null() {
            return 0;
        }
    }
    if rd::<u32>(this, 0x474) & 1 != 0 {
        return 1;
    }
    let mut found = false;
    let b = entity_get_col_model(veh);
    let vp = ent_pos(veh).cast::<u32>();
    let mut v = [vp.read(), vp.wrapping_index(1).read(), vp.wrapping_index(2).read()];
    let vf = |v: &[u32; 3], i: usize| f32::from_bits(v[i]);
    let pp = ent_pos(this).cast::<u32>();
    let o = [pp.read(), pp.wrapping_index(1).read(), pp.wrapping_index(2).read()];
    let mut c = o;
    let saved_col = rd::<u8>(this, 0x1C) & 1;
    g::<GPtr<u8>>(IGNORE_ENTITY).write(veh);
    let fl40 = rd::<u32>(this, 0x40) | 0x10000;
    for i in 0..3 {
        wr::<u32>(this, 0x44 + 4 * i, 0);
    }
    wr::<u32>(this, 0x40, fl40);
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & !1);

    // CheckCollision then the line of sight from V to c
    let test = |this: GPtr<u8>, v: &mut [u32; 3], c: &mut [u32; 3]| -> bool {
        physical_check_collision(this) == 0
            && get_is_line_of_sight_clear(lp(v).cast(), lp(c).cast(), 1, 0, 0, 1, 0, 0, 0) != 0
    };
    let try_posn = |this: GPtr<u8>, v: &mut [u32; 3], c: &mut [u32; 3]| -> bool {
        placeable_set_posn(this, lp(c).cast());
        test(this, v, c)
    };
    let try_direct = |this: GPtr<u8>, v: &mut [u32; 3], c: &mut [u32; 3]| -> bool {
        set_pos_raw(this, *c);
        test(this, v, c)
    };

    if vehicle_is_on_its_side(veh) != 0 && rd::<i32>(veh, 0x590) != 9 {
        c = v;
        c[2] = x(vf(&v, 2)).fadd_m(rd::<f32>(b, 0xC)).fadd_m(1.0).to_f32().to_bits();
        found = try_posn(this, &mut v, &mut c);
    } else if exit_door != 0 {
        let mut tmp = [0u32; 3];
        let r = get_position_to_open_car_door(lp(&mut tmp).cast(), veh, exit_door);
        c = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
        found = try_posn(this, &mut v, &mut c);
        if !found && rd::<i32>(veh, 0x590) == 9 && (exit_door == 10 || exit_door == 11) {
            let door = if exit_door == 11 { 9 } else { 8 };
            let r = get_position_to_open_car_door(lp(&mut tmp).cast(), veh, door);
            c = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
            found = try_posn(this, &mut v, &mut c);
        }
    }

    let k355 = f32::from_bits(0x3EB5_C28F);
    let a = x(rd::<f32>(b, 0)).fsub_m(k355);
    let neg_min_z = x(rd::<f32>(b, 8)).fchs();
    let max_z = rd::<f32>(b, 0x14);
    let h = if !x(max_z).fcom(neg_min_z).test_any(0x41) { x(max_z) } else { neg_min_z };
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let mut side = x(f32::from_bits(rd::<u32>(m, 8) & 0x7FFF_FFFF)).fmul(h).fadd(a).to_f32();
    if exit_door == 8 || exit_door == 9 {
        side = x(rd::<f32>(b, 0xC)).fadd_m(k355).to_f32();
    }

    'search: {
        if found {
            break 'search;
        }
        let r = |o: isize| rd::<f32>(m, o);
        let of = |i: usize| f32::from_bits(o[i]);
        // a. along right through the ped's position
        let dx = x(of(0)).fsub_m(vf(&v, 0));
        let dy = x(of(1)).fsub_m(vf(&v, 1));
        let dz = x(of(2)).fsub_m(vf(&v, 2));
        let k = dz.fmul_m(r(8)).fadd(dy.fmul_m(r(4))).fadd(dx.fmul_m(r(0))).fsubr_m(side);
        let ky = k.fmul_m(r(4)).to_f32();
        let kz = k.fmul_m(r(8)).to_f32();
        c[0] = k.fmul_m(r(0)).fadd_m(of(0)).to_f32().to_bits();
        c[1] = x(ky).fadd_m(of(1)).to_f32().to_bits();
        c[2] = x(kz).fadd_m(of(2)).to_f32().to_bits();
        if try_posn(this, &mut v, &mut c) {
            found = true;
            break 'search;
        }
        // (V + right * s) + forward * t, the right products rounded per component
        let side_fwd = |v: &[u32; 3], s: X87, neg: bool, t: X87| -> [u32; 3] {
            let r = |o: isize| rd::<f32>(m, o);
            let tfx = t.fmul_m(r(0x10));
            let tfy = t.fmul_m(r(0x14)).to_f32();
            let tfz = t.fmul_m(r(0x18)).to_f32();
            let srx = s.fmul_m(r(0));
            let sry = s.fmul_m(r(4));
            let srz = s.fmul_m(r(8)).to_f32();
            let (tx, ty, tz) = if neg {
                (x(vf(v, 0)).fsub(srx).to_f32(), x(vf(v, 1)).fsub(sry).to_f32(), x(vf(v, 2)).fsub_m(srz).to_f32())
            } else {
                (srx.fadd_m(vf(v, 0)).to_f32(), sry.fadd_m(vf(v, 1)).to_f32(), x(srz).fadd_m(vf(v, 2)).to_f32())
            };
            [
                x(tx).fadd(tfx).to_f32().to_bits(),
                x(ty).fadd_m(tfy).to_f32().to_bits(),
                x(tz).fadd_m(tfz).to_f32().to_bits(),
            ]
        };
        // b. four points along the side
        let min_y = rd::<f32>(b, 4);
        let step = x(rd::<f32>(b, 0x10)).fsub_m(min_y).fmul_m(f32::from_bits(0x3EAA_AAAB)).to_f32();
        for i in 0..4 {
            let t = xi(i).fmul_m(step).fadd_m(min_y);
            c = side_fwd(&v, x(side), false, t);
            if try_direct(this, &mut v, &mut c) {
                found = true;
                break 'search;
            }
        }
        // c./d. ahead of / behind the vehicle along forward
        for s in [x(rd::<f32>(b, 4)).fsub_m(k355), x(rd::<f32>(b, 0x10)).fadd_m(k355)] {
            let fy = s.fmul_m(r(0x14)).to_f32();
            let fz = s.fmul_m(r(0x18)).to_f32();
            c = [
                s.fmul_m(r(0x10)).fadd_m(vf(&v, 0)).to_f32().to_bits(),
                x(fy).fadd_m(vf(&v, 1)).to_f32().to_bits(),
                x(fz).fadd_m(vf(&v, 2)).to_f32().to_bits(),
            ];
            if try_direct(this, &mut v, &mut c) {
                found = true;
                break 'search;
            }
        }
        // e./f. the other side, front and back
        for o in [4isize, 0x10] {
            c = side_fwd(&v, x(side), true, x(rd::<f32>(b, o)));
            if try_direct(this, &mut v, &mut c) {
                found = true;
                break 'search;
            }
        }
        // g. on the roof (automobiles only)
        if rd::<i32>(veh, 0x590) != 0 {
            break 'search;
        }
        let u = x(rd::<f32>(b, 0x14));
        let uy = u.fmul_m(r(0x24)).to_f32();
        let uz = u.fmul_m(r(0x28)).to_f32();
        let cz = x(uz).fadd_m(vf(&v, 2)).to_f32();
        c = [
            u.fmul_m(r(0x20)).fadd_m(vf(&v, 0)).to_f32().to_bits(),
            x(uy).fadd_m(vf(&v, 1)).to_f32().to_bits(),
            x(cz).fadd_m(1.0).to_f32().to_bits(),
        ];
        found = try_direct(this, &mut v, &mut c);
    }

    g::<u32>(IGNORE_ENTITY).write(0);
    let f1c = rd::<u32>(this, 0x1C);
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & !0x10000);
    wr::<u32>(this, 0x1C, (f1c & !1) | u32::from(saved_col));

    if found {
        wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) | 1);
        for i in 0..6 {
            wr::<u32>(this, 0x44 + 4 * i, 0);
        }
        if rd::<i32>(veh, 0x590) != 9 || find_closest & 0xFF != 0 {
            for i in 0..3 {
                wr::<u32>(veh, 0x44 + 4 * i, 0);
            }
            let z = x(rd::<f32>(veh, 0x4C)).fsub_m(f32::from_bits(0x3D4C_CCCD)).to_f32();
            for i in 0..3 {
                wr::<u32>(veh, 0x50 + 4 * i, 0);
            }
            wr::<f32>(veh, 0x4C, z);
        }
        return 1;
    }
    if find_closest & 0xFF == 0 {
        return 0;
    }
    let paths = GPtr::<u8>::from_addr(THE_PATHS as usize);
    let mut n1 = 0u32;
    let mut n2 = 0u32;
    find_node_closest_to_coors(paths, lp(&mut n1).cast(), v[0], v[1], v[2], 1, 0x4974_23FE, 0, 0, 0, 0, 0);
    find_node_closest_to_coors(paths, lp(&mut n2).cast(), v[0], v[1], v[2], 0, 0x4974_23FE, 0, 0, 0, 0, 0);
    let mut have = false;
    let mut tmp = [0.0f32; 3];
    if n1 & 0xFFFF != 0xFFFF {
        have = true;
        let r = path_node_get_position(path_node(n1), lp(&mut tmp).cast());
        c = [r.cast::<u32>().read(), r.cast::<u32>().wrapping_index(1).read(), r.cast::<u32>().wrapping_index(2).read()];
    }
    if n2 & 0xFFFF != 0xFFFF {
        let node = path_node(n2);
        let p2 = path_node_get_position(node, lp(&mut tmp).cast());
        let mut d = [0.0f32; 3];
        let dd = vector_sub(lp(&mut d).cast(), p2, lp(&mut v).cast());
        let c1x = x(f32::from_bits(c[0])).fsub_m(vf(&v, 0));
        let c1y = x(f32::from_bits(c[1])).fsub_m(vf(&v, 1));
        let d2x = x(dd.read());
        let d2y = x(dd.wrapping_index(1).read());
        let dist2 = d2x.fmul(d2x).fadd(d2y.fmul(d2y)).fsqrt();
        let dist1 = c1y.fmul(c1y).fadd(c1x.fmul(c1x)).fsqrt();
        if !dist1.fcom(dist2).test_any(0x41) {
            let r = path_node_get_position(node, lp(&mut tmp).cast());
            c = [r.cast::<u32>().read(), r.cast::<u32>().wrapping_index(1).read(), r.cast::<u32>().wrapping_index(2).read()];
        }
    } else if !have {
        return 0;
    }
    find_z_coor_for_ped_call(lp(&mut c).cast());
    placeable_set_posn(this, lp(&mut c).cast());
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let heading = if vm.is_null() {
        rd::<u32>(veh, 0x10)
    } else {
        x(rd::<f32>(vm, 0x10)).fchs().fpatan(x(rd::<f32>(vm, 0x14))).to_f32().to_bits()
    };
    let pm = rd::<GPtr<u8>>(this, 0x14);
    if pm.is_null() {
        wr::<u32>(this, 0x10, heading);
    } else {
        matrix_set_rotate_z_only(pm, heading);
    }
    wr::<u32>(this, 0x474, rd::<u32>(this, 0x474) | 1);
    for i in 0..6 {
        wr::<u32>(this, 0x44 + 4 * i, 0);
    }
    for i in 0..6 {
        wr::<u32>(veh, 0x50 - 0xC * (i / 3) + 4 * (i % 3), 0);
    }
    let z = x(rd::<f32>(veh, 0x4C)).fadd_m(f32::from_bits(0x3CA3_D70A)).to_f32();
    wr::<f32>(veh, 0x4C, z);
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DROP_ENTITY_THAT_THIS_PED_IS_HOLDING,
    HOOK_IS_PED_HEAD_ABOVE_POS,
    HOOK_GET_TRANSFORMED_BONE_POSITION,
    HOOK_GET_BONE_POSITION,
    HOOK_DOES_LOS_BULLET_HIT_PED,
    HOOK_DO_GUN_FLASH,
    HOOK_SET_GUN_FLASH_ALPHA,
    HOOK_IS_POSITION_CLEAR_FOR_PED,
    HOOK_IS_POSITION_CLEAR_OF_CARS,
    HOOK_FIND_Z_COOR_FOR_PED,
    HOOK_ATTACH_PED_TO_BIKE,
    HOOK_ATTACH_PED_TO_ENTITY,
    HOOK_DETTACH_PED_FROM_ENTITY,
    HOOK_GIVE_OBJECT_TO_PED_TO_HOLD,
    HOOK_SORT_PEDS,
    HOOK_OUR_PED_CAN_SEE_THIS_ENTITY,
    HOOK_POSITION_ATTACHED_PED,
    HOOK_SHOULDER_BONE_ROTATION,
    HOOK_DEAD_PED_MAKES_TYRES_BLOODY,
    HOOK_POSITION_ANY_PED_OUT_OF_COLLISION,
    HOOK_POSITION_PED_OUT_OF_COLLISION,
];
