//! `CPhysical` construction, destruction, world sector lists, the moving list, collision records and
//! the small getters (PORT-PHYS). Ported from the 1.0 US bytes (specs: `docs/re/specs/PHYS/`);
//! gta-reversed `Entity/Physical.cpp` was a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::shared::*;
use crate::collision::fpu::Fpu;

orig! {
    /// `CEntity::CEntity()` (0x532A90, thiscall, returns this)
    pub fn entity_ctor(e: Ent) -> Ent = thiscall 0x532A90 => unported;
    /// `CEntity::~CEntity()` (0x535E90, thiscall)
    pub fn entity_dtor(e: Ent) -> u32 = thiscall 0x535E90 => unported;
    /// `CPlaceable::AllocateStaticMatrix()` (0x54F4C0, thiscall)
    pub fn allocate_static_matrix(e: Ent) = thiscall 0x54F4C0 => unported;
    /// `CMatrix::SetUnity()` (0x59AE70, thiscall)
    pub fn matrix_set_unity(m: Ent) = thiscall 0x59AE70 => unported;
    /// `CRealTimeShadowManager::ReturnRealTimeShadow(CRealTimeShadow*)` (0x705B30, thiscall on
    /// `g_realTimeShadowMan` 0xC40350)
    pub fn return_real_time_shadow(mgr: Ent, shadow: Ent) = thiscall 0x705B30 => unported;
    /// `CEntryInfoList::Flush()` (0x536E10, thiscall on the list head)
    pub fn entry_info_list_flush(list: GPtr<GPtr<u8>>) = thiscall 0x536E10 => sa_core::pools::entry_info_list_flush;
    /// `CEntity::Remove()` (0x534AE0, thiscall)
    pub fn entity_remove(e: Ent) = thiscall 0x534AE0 => unported;
    /// `CEntity::Add()` (0x533020, thiscall)
    pub fn entity_add(e: Ent) = thiscall 0x533020 => unported;
    /// `CPtrNodeDoubleLink::operator new(size)` (0x5523C0, cdecl)
    pub fn ptr_node_double_new(size: u32) -> Ent = cdecl 0x5523C0 => unported;
    /// `CPtrNodeDoubleLink::operator delete(void*)` (0x5523D0, cdecl)
    pub fn ptr_node_double_delete(p: Ent) = cdecl 0x5523D0 => unported;
    /// `CEntryInfoNode::operator new(size)` (0x536DC0, cdecl)
    pub fn entry_info_node_new(size: u32) -> Ent = cdecl 0x536DC0 => entry_info_node_new_port;
    /// `CEntryInfoNode::operator delete(void*)` (0x536DD0, cdecl)
    pub fn entry_info_node_delete(p: Ent) = cdecl 0x536DD0 => entry_info_node_delete_port;
    /// virtual `CEntity::GetBoundRect(CRect* out)` (vtable +0x24, thiscall, returns out)
    pub fn get_bound_rect_v(e: Ent, out: GPtr<[f32; 4]>) -> GPtr<[f32; 4]> = thiscall vslot 0x24 => unported;
    /// `CEntity::GetBoundCentre(CVector& out)` (0x534250, thiscall)
    pub fn get_bound_centre(e: Ent, out: GPtr<CVector>) = thiscall 0x534250 => unported;
    /// `CTimeCycle::GetAmbientRed_BeforeBrightness()` (0x560390, ST0)
    pub fn tc_ambient_red_bb() -> f64 = cdecl 0x560390 => unported;
    /// `CTimeCycle::GetAmbientGreen_BeforeBrightness()` (0x5603A0, ST0)
    pub fn tc_ambient_green_bb() -> f64 = cdecl 0x5603A0 => unported;
    /// `CTimeCycle::GetAmbientBlue_BeforeBrightness()` (0x5603B0, ST0)
    pub fn tc_ambient_blue_bb() -> f64 = cdecl 0x5603B0 => unported;
    /// `CPhysical::GetLightingFromCol(bool)` (0x5447B0; the dword the original pushes)
    pub fn get_lighting_from_col_call(e: Ent, interior: u32) -> f64 = thiscall 0x5447B0 => get_lighting_from_col_port;
    /// `operator delete(void*)` (CRT `free`, 0x8214BD)
    pub fn crt_free(p: Ent) = cdecl 0x8214BD => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    pub fn apply_move_force_call(e: Ent, force: CVector) = thiscall 0x5429F0 => crate::physical::forces::apply_move_force;
}

pub fn entry_info_node_new_port(size: u32) -> Ent {
    sa_core::pools::entry_info_node_operator_new(size).cast()
}
pub fn entry_info_node_delete_port(p: Ent) {
    sa_core::pools::entry_info_node_operator_delete(p.cast())
}
pub fn get_lighting_from_col_port(e: Ent, interior: u32) -> f64 {
    get_lighting_from_col(e, interior as u8).to_f64()
}

sa_core::globals! {
    list GLOBALS;
    /// `CTimeCycle::m_CurrentColours` RGB sum 1 (0xB7C518..0xB7C520, f32; `SumOfCurrentRGB1`)
    pub static TC_CURRENT_RGB1 @ 0xB7C518: [f32; 3] = [0.0; 3];
    /// `CTimeCycle::m_CurrentColours` RGB sum 2 (0xB7C528..0xB7C530, f32; `SumOfCurrentRGB2`)
    pub static TC_CURRENT_RGB2 @ 0xB7C528: [f32; 3] = [0.0; 3];
}

/// `CRealTimeShadowManager g_realTimeShadowMan` (0xC40350)
const REAL_TIME_SHADOW_MAN: usize = 0xC40350;
/// `CPhysical`'s vtable (0x863BA0)
const VTABLE: u32 = 0x863BA0;
/// `CWorld::ms_aRepeatSectors` (0xB992B8, 16x16 `CRepeatSector` of 0xC bytes)
const REPEAT_SECTORS: usize = 0xB992B8;

/// `CTimer::UpdateTimeStep(float)` (0x541F40, cdecl): `ms_fTimeStep = ts` when `ts > 1e-5f` or NaN
/// (raw copy), else the dword of 1e-5f (spec CTimer.md).
#[hook(addr = 0x541F40, conv = cdecl, name = "FUN_00541f40", expect = "D9 44 24 04 D8 1D 14 8C 85 00")]
pub fn update_time_step(ts: u32) {
    let mut f = Fpu::new();
    f.fld(f32::from_bits(ts));
    f.fcomp_m(cf(k::K_858C14));
    if f.jp(0x41) {
        sa_core::timer::TIME_STEP.ptr().cast::<u32>().write(ts);
    } else {
        sa_core::timer::TIME_STEP.ptr().cast::<u32>().write(k::K_858C14);
    }
}

/// `CPhysical::SetVelocity(CVector)` (stub 0x441130, `ret 0xC`): `m_vecMoveSpeed = v` (dword copies).
#[hook(addr = 0x441130, conv = thiscall, name = "CPhysical::SetVelocity", expect = "E9 0B B6 12 01", this = "CPhysical")]
pub fn set_velocity(this: Ent, x: u32, y: u32, z: u32) {
    wv_bits(this, MOVE_SPEED, [x, y, z]);
}

/// `CPhysical::ApplyMoveForce(float x, float y, float z)` (0x4ABBA0, `ret 0xC`): passes the three
/// dwords on as a `CVector` to `ApplyMoveForce(CVector)` (0x5429F0).
#[hook(addr = 0x4ABBA0, conv = thiscall, name = "CPhysical::ApplyMoveForce", expect = "83 EC 0C 8B 44 24 10", this = "CPhysical")]
pub fn apply_move_force_xyz(this: Ent, x: u32, y: u32, z: u32) {
    apply_move_force_call(this, CVector::new(f32::from_bits(x), f32::from_bits(y), f32::from_bits(z)));
}

/// `CPhysical::CPhysical()` (0x542260, returns this): `CEntity::CEntity`, vtable, empty collision
/// list, `AllocateStaticMatrix` + `SetUnity`, then the field initialisation of the bytes. Not written
/// (kept from the slot, gr-dev Physical.cpp:128 keep-original): +0x3C, +0x9C, +0xA0, +0xBA,
/// +0x100..+0x117.
#[hook(addr = 0x542260, conv = thiscall, name = "CPhysical::Constructor", expect = "6A FF 68 76 C9 83 00", this = "CPhysical")]
pub fn constructor(this: Ent) -> Ent {
    entity_ctor(this);
    wu(this, 0, VTABLE);
    wu(this, COLLISION_LIST, 0);
    allocate_static_matrix(this);
    matrix_set_unity(matrix(this));
    for off in [MOVE_SPEED, TURN_SPEED, FRICTION_MOVE, FRICTION_TURN, FORCE, TORQUE] {
        wv_bits(this, off, [0; 3]);
    }
    wu(this, MASS, 0x3F80_0000);
    wu(this, TURN_MASS, 0x3F80_0000);
    wu(this, VELOCITY_FREQ, 0x3F80_0000);
    wu(this, AIR_RESISTANCE, 0x3DCC_CCCD);
    wu(this, MOVING_LIST, 0);
    wb(this, FAKE_PHYSICS, 0);
    wb(this, NUM_COLLIDED, 0);
    for i in 0..6 {
        wu(this, COLLIDED + 4 * i, 0);
    }
    wh(this, PIECE_TYPE, 0);
    wu(this, DAMAGE_INTENSITY, 0);
    wu(this, DAMAGE_ENTITY, 0);
    wv_bits(this, LAST_IMPACT_VELOCITY, [0; 3]);
    wv_bits(this, LAST_IMPACT_POS, [0; 3]);
    wu(this, EFLAGS, ru(this, EFLAGS) | 1);
    wv_bits(this, CENTRE_OF_MASS, [0; 3]);
    wu(this, MOVING_SPEED, 0);
    wu(this, ATTACHED_TO, 0);
    wu(this, IGNORED_COLLISION, 0);
    wu(this, ATTACH_QUAT + 0xC, 0);
    wu(this, ATTACH_QUAT, 0);
    wu(this, ATTACH_QUAT + 4, 0);
    wu(this, ATTACH_QUAT + 8, 0);
    wu(this, DYNAMIC_LIGHTING, 0);
    wu(this, SHADOW_DATA, 0);
    wu(this, 0x38, 0x42C8_0000);
    wu(this, PFLAGS, 2);
    wu(this, SURFACE_BRIGHTNESS, 0x3F80_0000);
    this
}

/// `CPhysical::~CPhysical()` (0x542450): vtable, the shadow returned to `g_realTimeShadowMan`
/// when set, the collision list flushed, `CEntity::~CEntity` (whose EAX is returned).
#[hook(addr = 0x542450, conv = thiscall, name = "CPhysical::Destructor", expect = "6A FF 68 96 C9 83 00", this = "CPhysical")]
pub fn destructor(this: Ent) -> u32 {
    wu(this, 0, VTABLE);
    let shadow = rp(this, SHADOW_DATA);
    if !shadow.is_null() {
        return_real_time_shadow(GPtr::from_addr(REAL_TIME_SHADOW_MAN), shadow);
    }
    entry_info_list_flush(this.byte_add(COLLISION_LIST).cast());
    entity_dtor(this)
}

/// `CPhysical::~CPhysical` scalar deleting destructor (0x544990, `ret 4`, returns this).
#[hook(addr = 0x544990, conv = thiscall, name = "CPhysical::`scalar deleting destructor'", expect = "56 8B F1 E8 B8 DA FF FF", this = "CPhysical")]
pub fn scalar_deleting_destructor(this: Ent, flags: u8) -> Ent {
    destructor_call(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

orig! {
    /// `CPhysical::~CPhysical` (0x542450)
    pub fn destructor_call(e: Ent) -> u32 = thiscall 0x542450 => destructor;
}

/// Unlinks `link` from the list whose head `list` points at (`if (*list == link) *list =
/// link->next; prev->next = next; next->prev = prev`, each re-read).
fn unlink_double(list: GPtr<Ent>, link: Ent) {
    if list.read() == link {
        list.write(rp(link, 4));
    }
    let prev = rp(link, 8);
    if !prev.is_null() {
        wp(prev, 4, rp(link, 4));
    }
    let next = rp(link, 4);
    if !next.is_null() {
        wp(next, 8, rp(link, 8));
    }
}

/// Pushes `link` at the front of `*list` (`prev = 0; next = *list; (*list)->prev = link; *list =
/// link`).
fn push_double(list: GPtr<Ent>, link: Ent) {
    wu(link, 8, 0);
    wp(link, 4, list.read());
    let head = list.read();
    if !head.is_null() {
        wp(head, 8, link);
    }
    list.write(link);
}

/// Unlinks and frees every node from `cursor` on (RemoveAndAdd's tail and `Remove`).
fn free_nodes(this: Ent, mut cursor: Ent) {
    while !cursor.is_null() {
        let list: GPtr<Ent> = rp(cursor, 0).cast();
        let link = rp(cursor, 4);
        unlink_double(list, link);
        ptr_node_double_delete(link);
        let head = rp(this, COLLISION_LIST);
        let next = rp(cursor, 0x10);
        if head == cursor {
            wp(this, COLLISION_LIST, next);
        }
        let prev = rp(cursor, 0xC);
        if !prev.is_null() {
            wp(prev, 0x10, rp(cursor, 0x10));
        }
        let n = rp(cursor, 0x10);
        if !n.is_null() {
            wp(n, 0xC, rp(cursor, 0xC));
        }
        entry_info_node_delete(cursor);
        cursor = next;
    }
}

/// `CPhysical::Remove()` (0x5424C0, virtual): a big building goes to `CEntity::Remove`; otherwise
/// every collision-list node is unlinked from its sector list and freed.
#[hook(addr = 0x5424C0, conv = thiscall, name = "CPhysical::Remove", expect = "57 8B F9 8B 47 1C", this = "CPhysical")]
pub fn remove(this: Ent) {
    if ru(this, EFLAGS) & 0x100 != 0 {
        entity_remove(this);
        return;
    }
    free_nodes(this, rp(this, COLLISION_LIST));
}

/// `_ftol(floor((double)(v * 0.02f + 60.0f)))` (PC = 24; `floor` 0x8219F0 and `_ftol` 0x821B40 are
/// CRT, exact on doubles).
pub fn sector_of(v: f32) -> i32 {
    let s = X87::from_f32(v).fmul_m(cf(k::K_858B38)).fadd_m(cf(k::K_858B34));
    X87::from_f64_exact(floor_f64(s.to_f64())).ftol() as u32 as i32
}

/// CRT `floor` on a double (exact).
pub fn floor_f64(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() || v.abs() >= 4_503_599_627_370_496.0 {
        return v;
    }
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}

/// The sector rect of `this->GetBoundRect()` in the original order: x0 (left), y0 (bottom), x1
/// (right), y1 (top).
fn sector_rect(this: Ent) -> (i32, i32, i32, i32) {
    let mut rect = [0f32; 4];
    get_bound_rect_v(this, lp(&mut rect));
    (sector_of(rect[0]), sector_of(rect[3]), sector_of(rect[2]), sector_of(rect[1]))
}

/// The repeat sector of (x, y) and the list `type` selects in it (other types keep `list`).
fn sector_list(this: Ent, x: i32, y: i32, list: Ent) -> (Ent, Ent) {
    let idx = ((y & 15) << 4) + (x & 15);
    let sector: Ent = GPtr::from_addr(REPEAT_SECTORS).byte_add(12 * idx as isize);
    let list = match etype(this) {
        ty::VEHICLE => sector,
        ty::PED => sector.byte_add(4),
        ty::OBJECT => sector.byte_add(8),
        _ => list,
    };
    (sector, list)
}

/// `CPhysical::Add()` (0x544A30, virtual; spec CPhysical.md): a big building goes to
/// `CEntity::Add`; otherwise one node + link per covered repeat sector (node allocated first),
/// pushed at the front of the collision list. `list` starts as the integer `y1` (only used when the
/// type matches no list).
#[hook(addr = 0x544A30, conv = thiscall, name = "CPhysical::Add", expect = "64 A1 00 00 00 00", this = "CPhysical")]
pub fn add(this: Ent) {
    if ru(this, EFLAGS) & 0x100 != 0 {
        entity_add(this);
        return;
    }
    let (x0, y0, x1, y1) = sector_rect(this);
    let mut list: Ent = GPtr::from_addr(y1 as u32 as usize);
    let mut y = y0;
    while y <= y1 {
        let mut x = x0;
        while x <= x1 {
            let (sector, l) = sector_list(this, x, y, list);
            list = l;
            let mut node = entry_info_node_new(0x14);
            if !node.is_null() {
                let link = ptr_node_double_new(0xC);
                if !link.is_null() {
                    wp(link, 0, this);
                }
                push_double(list.cast(), link);
                wp(node, 4, link);
                wp(node, 8, sector);
                wp(node, 0, list);
            } else {
                node = GPtr::NULL;
            }
            wu(node, 0xC, 0);
            wp(node, 0x10, rp(this, COLLISION_LIST));
            let head = rp(this, COLLISION_LIST);
            if !head.is_null() {
                wp(head, 0xC, node);
            }
            wp(this, COLLISION_LIST, node);
            x += 1;
        }
        y += 1;
    }
}

/// `CPhysical::RemoveAndAdd()` (0x542560; spec CPhysical.md): reuses the existing nodes in order
/// (moving their links to the new sector lists), allocates new ones (link first) at the front when
/// the old list runs out, frees the rest.
#[hook(addr = 0x542560, conv = thiscall, name = "CPhysical::RemoveAndAdd", expect = "83 EC 2C 55 8B E9", this = "CPhysical")]
pub fn remove_and_add(this: Ent) {
    if ru(this, EFLAGS) & 0x100 != 0 {
        entity_remove(this);
        entity_add(this);
        return;
    }
    let mut cursor = rp(this, COLLISION_LIST);
    let (x0, y0, x1, y1) = sector_rect(this);
    let mut list: Ent = GPtr::from_addr(y1 as u32 as usize);
    let mut y = y0;
    while y <= y1 {
        let mut x = x0;
        while x <= x1 {
            let (sector, l) = sector_list(this, x, y, list);
            list = l;
            if !cursor.is_null() {
                let old: GPtr<Ent> = rp(cursor, 0).cast();
                let link = rp(cursor, 4);
                unlink_double(old, link);
                push_double(list.cast(), link);
                wp(cursor, 8, sector);
                wp(cursor, 0, list);
                cursor = rp(cursor, 0x10);
            } else {
                let link = ptr_node_double_new(0xC);
                if !link.is_null() {
                    wp(link, 0, this);
                }
                push_double(list.cast(), link);
                let mut node = entry_info_node_new(0x14);
                if !node.is_null() {
                    wp(node, 8, sector);
                    wp(node, 0, list);
                    wp(node, 4, link);
                } else {
                    node = GPtr::NULL;
                }
                wu(node, 0xC, 0);
                wp(node, 0x10, rp(this, COLLISION_LIST));
                let head = rp(this, COLLISION_LIST);
                if !head.is_null() {
                    wp(head, 0xC, node);
                }
                wp(this, COLLISION_LIST, node);
            }
            x += 1;
        }
        y += 1;
    }
    free_nodes(this, cursor);
}

/// `CPhysical::AddToMovingList()` (0x542800): when not in the list and not `bDontStream`-like entity
/// flag 0x40000: a new `CPtrNodeDoubleLink` (item = this) pushed at the front of
/// `CWorld::ms_listMovingEntityPtrs`; a failed allocation still writes through the null link.
#[hook(addr = 0x542800, conv = thiscall, name = "CPhysical::AddToMovingList", expect = "56 8B F1 90 E9 E1 37 EC FF", this = "CPhysical")]
pub fn add_to_moving_list(this: Ent) {
    if !rp(this, MOVING_LIST).is_null() || ru(this, EFLAGS) & 0x40000 != 0 {
        return;
    }
    let link = ptr_node_double_new(0xC);
    if !link.is_null() {
        wp(link, 0, this);
    }
    wp(this, MOVING_LIST, link);
    wu(link, 8, 0);
    wp(link, 4, MOVING_LIST_HEAD.get());
    let head = MOVING_LIST_HEAD.get();
    if !head.is_null() {
        wp(head, 8, link);
    }
    MOVING_LIST_HEAD.set(link);
}

/// `CPhysical::RemoveFromMovingList()` (0x542860): unlinks and frees the moving-list node.
#[hook(addr = 0x542860, conv = thiscall, name = "CPhysical::RemoveFromMovingList", expect = "56 8B F1 8B 86 B4 00 00 00", this = "CPhysical")]
pub fn remove_from_moving_list(this: Ent) {
    let link = rp(this, MOVING_LIST);
    if link.is_null() {
        return;
    }
    unlink_double(MOVING_LIST_HEAD.ptr(), link);
    ptr_node_double_delete(rp(this, MOVING_LIST));
    wu(this, MOVING_LIST, 0);
}

/// `CPhysical::AddCollisionRecord(CEntity*)` (0x543490, `ret 4`): `bOnSolidSurface`, the last
/// collision time; two vehicles get their +0x45C word 0xFFFF replaced by 15000; then, when
/// `bCanBeCollidedWith`, the entity is appended unless present or the list holds 6.
#[hook(addr = 0x543490, conv = thiscall, name = "CPhysical::AddCollisionRecord", expect = "8A 51 36 57 81 49 40 00 02 00 00", this = "CPhysical")]
pub fn add_collision_record(this: Ent, other: Ent) {
    wu(this, PFLAGS, ru(this, PFLAGS) | 0x200);
    wu(this, LAST_COLLISION_TIME, sa_core::timer::TIME_MS.get());
    if etype(this) == ty::VEHICLE && etype(other) == ty::VEHICLE {
        if rh(this, 0x45C) == 0xFFFF {
            wh(this, 0x45C, 15000);
        }
        if rh(other, 0x45C) == 0xFFFF {
            wh(other, 0x45C, 15000);
        }
    }
    if (ru(this, PFLAGS) >> 28) & 1 == 0 {
        return;
    }
    let n = rb(this, NUM_COLLIDED);
    let mut i = 0u32;
    while (i as i32) < n as i32 {
        if rp(this, COLLIDED + 4 * i as isize) == other {
            return;
        }
        i += 1;
        if i as i32 >= rb(this, NUM_COLLIDED) as i32 {
            break;
        }
    }
    if n < 6 {
        wp(this, COLLIDED + 4 * n as isize, other);
        wb(this, NUM_COLLIDED, rb(this, NUM_COLLIDED).wrapping_add(1));
    }
}

/// `CPhysical::GetHasCollidedWith(CEntity*)` (0x543540, `ret 4`, AL).
#[hook(addr = 0x543540, conv = thiscall, name = "CPhysical::GetHasCollidedWith", expect = "8B 41 40 C1 E8 1C", this = "CPhysical")]
pub fn get_has_collided_with(this: Ent, other: Ent) -> u8 {
    if (ru(this, PFLAGS) >> 28) & 1 == 0 {
        return 0;
    }
    let n = rb(this, NUM_COLLIDED) as i32;
    (0..n).any(|i| rp(this, COLLIDED + 4 * i as isize) == other) as u8
}

/// `CPhysical::GetHasCollidedWithAnyObject()` (0x543580, AL): an object (type 4) in the list.
#[hook(addr = 0x543580, conv = thiscall, name = "CPhysical::GetHasCollidedWithAnyObject", expect = "8B 41 40 C1 E8 1C", this = "CPhysical")]
pub fn get_has_collided_with_any_object(this: Ent) -> u8 {
    if (ru(this, PFLAGS) >> 28) & 1 == 0 {
        return 0;
    }
    let n = rb(this, NUM_COLLIDED) as i32;
    (0..n).any(|i| {
        let e = rp(this, COLLIDED + 4 * i as isize);
        !e.is_null() && etype(e) == ty::OBJECT
    }) as u8
}

/// `CPhysical::RemoveRefsToEntity(CEntity*)` (0x544280, `ret 4`): removes every occurrence of the
/// entity from the collided list, shifting the rest down (int16 indices, the count re-read).
#[hook(addr = 0x544280, conv = thiscall, name = "CPhysical::RemoveRefsToEntity", expect = "90 E9 9B 54 EC FF", this = "CPhysical")]
pub fn remove_refs_to_entity(this: Ent, other: Ent) {
    if rb(this, NUM_COLLIDED) == 0 {
        return;
    }
    let mut i: i16 = 0;
    loop {
        let a = i as i32;
        if rp(this, COLLIDED + 4 * a as isize) == other {
            let mut j = i;
            let mut b = a;
            while b < rb(this, NUM_COLLIDED) as i32 - 1 {
                let v = ru(this, COLLIDED + 4 * (b + 1) as isize);
                wu(this, COLLIDED + 4 * b as isize, v);
                j = j.wrapping_add(1);
                b = j as i32;
            }
            wb(this, NUM_COLLIDED, rb(this, NUM_COLLIDED).wrapping_sub(1));
        } else {
            i = i.wrapping_add(1);
        }
        if i >= rb(this, NUM_COLLIDED) as i16 {
            break;
        }
    }
}

/// `CPhysical::CanPhysicalBeDamaged(eWeaponType, bool* bDamagedDueToFireOrExplosionOrBullet)`
/// (0x5448B0, `ret 8`, AL): the proof flags per weapon class (jump table 0x544938/0x544954).
#[hook(addr = 0x5448B0, conv = thiscall, name = "CPhysical::CanPhysicalBeDamaged", expect = "8B 44 24 08 85 C0", this = "CPhysical")]
pub fn can_physical_be_damaged(this: Ent, weapon: u32, out: GPtr<u8>) -> u8 {
    if !out.is_null() {
        out.write(0);
    }
    let f = ru(this, PFLAGS);
    let set_out = || {
        if !out.is_null() {
            out.write(1);
            1
        } else {
            1
        }
    };
    if weapon > 0x36 {
        return 1;
    }
    match DAMAGE_CLASS[weapon as usize] {
        // 0x5448D2: melee
        0 => (f & 0x20_0000 == 0) as u8,
        // 0x544903: explosion
        1 => {
            if (f >> 23) & 1 != 0 {
                0
            } else {
                set_out()
            }
        }
        // 0x544913: explosion and fire
        2 => {
            if (f >> 23) & 1 != 0 || f & 0x8_0000 != 0 {
                0
            } else {
                set_out()
            }
        }
        // 0x5448E0: bullet
        3 => {
            if f & 0x4_0000 != 0 {
                0
            } else {
                set_out()
            }
        }
        // 0x5448F5: fire (out not set)
        4 => (f & 0x8_0000 == 0) as u8,
        // 0x54492A: collision
        5 => (f & 0x10_0000 == 0) as u8,
        // 0x544933
        _ => 1,
    }
}

/// The jump-table index per weapon type 0..=0x36 (bytes at 0x544954; targets at 0x544938: 0 melee
/// 0x5448D2, 1 explosion 0x544903, 2 explosion+fire 0x544913, 3 bullet 0x5448E0, 4 fire 0x5448F5,
/// 5 collision 0x54492A, 6 true 0x544933).
const DAMAGE_CLASS: [u8; 0x37] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 6, 2, 1, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
    1, 1, 4, 3, 1, 1, 6, 6, 6, 6, 6, 6, 6, 6, 5, 5, 1, 3, 6, 5,
];

/// `CPhysical::GetBoundRect()` (0x5449B0, `CRect* (CRect* out)`, `ret 4`): the bound centre +- the
/// col model's bounding-sphere radius (`ms_modelInfoPtrs[model]->colModel->radius`).
#[hook(addr = 0x5449B0, conv = thiscall, name = "CPhysical::GetBoundRect", expect = "83 EC 1C 56 8D 44 24 04", this = "CPhysical")]
pub fn get_bound_rect(this: Ent, out: GPtr<[f32; 4]>) -> GPtr<[f32; 4]> {
    let mut c = CVector::new(0.0, 0.0, 0.0);
    get_bound_centre(this, lp(&mut c));
    let model = rh(this, MODEL_INDEX) as i16 as i32;
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
    let col = rp(mi, 0x14);
    let mut f = Fpu::new();
    f.fld(rf(col, 0x24));
    f.fld(c.x);
    f.fsub_st(1);
    let left = f.fstp();
    f.fld(c.y);
    f.fsub_st(1);
    let bottom = f.fstp();
    f.fld(c.x);
    f.fadd_st(1);
    let right = f.fstp();
    f.fld(c.y);
    f.fadd_st(1);
    let top = f.fstp();
    let o = out.cast::<u8>();
    wf(o, 0, left);
    wf(o, 4, top);
    wf(o, 8, right);
    wf(o, 0xC, bottom);
    out
}

/// `CPhysical::GetLightingFromCol(bool bInteriorLighting)` (0x5447B0, `ret 4`, ST0):
/// `((green + (red + blue)) * 0.333333f + brightness)`; outside interiors scaled by
/// `RGB1sum/765*frac + (1 - frac)` and plus `RGB2sum/765*frac` (sum orders of the bytes; the
/// constant is 0x3EAAAA9F, not 1/3).
#[hook(addr = 0x5447B0, conv = thiscall, name = "CPhysical::GetLightingFromCol", expect = "83 EC 08 8B 81 2C 01 00 00", this = "CPhysical")]
pub fn get_lighting_from_col(this: Ent, interior: u8) -> X87 {
    let brightness = rf(this, SURFACE_BRIGHTNESS);
    let mut f = Fpu::new();
    f.push(X87::from_f64_exact(tc_ambient_blue_bb()));
    let s = f.fstp();
    f.push(X87::from_f64_exact(tc_ambient_red_bb()));
    f.fadd_m(s);
    let s = f.fstp();
    f.push(X87::from_f64_exact(tc_ambient_green_bb()));
    f.fadd_m(s);
    f.fmul_m(cf(k::K_863C10));
    f.fadd_m(brightness);
    if interior != 0 {
        return f.st(0);
    }
    let frac = TEST_ADD_AMBIENT_LIGHT_FRAC.get();
    let rgb1 = TC_CURRENT_RGB1.get();
    let rgb2 = TC_CURRENT_RGB2.get();
    f.fld(rgb1[2]);
    f.fadd_m(rgb1[1]);
    f.fadd_m(rgb1[0]);
    f.fmul_m(cf(k::K_863C0C));
    f.fmul_m(frac);
    f.fld(cf(k::K_858624));
    f.fsub_m(frac);
    f.faddp(1);
    f.fmulp(1);
    f.fld(rgb2[2]);
    f.fadd_m(rgb2[1]);
    f.fadd_m(rgb2[0]);
    f.fmul_m(cf(k::K_863C0C));
    f.fmul_m(frac);
    f.faddp(1);
    f.st(0)
}

/// `CPhysical::GetLightingTotal()` (0x544850, ST0): `GetLightingFromCol(interior) +
/// m_fDynamicLighting`; interior when the area code (+0x2F) is set, or for a player ped (+0x480)
/// when `CGame::currArea` is set or the player data's +0x86 flag. The pushed dword keeps the upper
/// bytes of the original's local (the `push ecx` slot = this).
#[hook(addr = 0x544850, conv = thiscall, name = "CPhysical::GetLightingTotal", expect = "51 56 8B F1 8A 46 2F", this = "CPhysical")]
pub fn get_lighting_total(this: Ent) -> X87 {
    let mut interior = false;
    if rb(this, 0x2F) != 0 {
        interior = true;
    } else if etype(this) == ty::PED {
        let pd = rp(this, 0x480);
        if !pd.is_null() && (sa_core::game::CURR_AREA.get() != 0 || rb(pd, 0x86) != 0) {
            interior = true;
        }
    }
    let arg = (this.addr() as u32 & 0xFFFF_FF00) | interior as u32;
    let l = get_lighting_from_col_call(this, arg);
    X87::from_f64_exact(l).fadd_m(rf(this, DYNAMIC_LIGHTING))
}

/// `CPhysical::SkipPhysics()` (0x5433B0): clears `bSubmergedInWater` (not for peds and vehicles) and
/// entity flags 0x1068; unless the status (+0x36 bits 3..7) is 2: clears `bOnSolidSurface`, the
/// collided count, the piece type and the damage intensity, releases the damage entity reference,
/// zeroes both friction speeds and the damage entity.
#[hook(addr = 0x5433B0, conv = thiscall, name = "CPhysical::SkipPhysics", expect = "83 EC 0C 56 8B F1", this = "CPhysical")]
pub fn skip_physics(this: Ent) {
    let t = rb(this, TYPE);
    if t & 7 != ty::PED && t & 7 != ty::VEHICLE {
        wu(this, PFLAGS, ru(this, PFLAGS) & 0xFFFF_FEFF);
    }
    wu(this, EFLAGS, ru(this, EFLAGS) & 0xFFFF_EF97);
    if t & 0xF8 == 0x10 {
        return;
    }
    wu(this, PFLAGS, ru(this, PFLAGS) & 0xFFFF_FDFF);
    wb(this, NUM_COLLIDED, 0);
    wh(this, PIECE_TYPE, 0);
    wu(this, DAMAGE_INTENSITY, 0);
    let de = rp(this, DAMAGE_ENTITY);
    if !de.is_null() {
        clean_up_old_reference(de, this.byte_add(DAMAGE_ENTITY).cast());
    }
    wv_bits(this, FRICTION_TURN, [0; 3]);
    wu(this, FRICTION_MOVE, 0);
    wu(this, FRICTION_MOVE + 4, 0);
    wu(this, DAMAGE_ENTITY, 0);
    wu(this, FRICTION_MOVE + 8, 0);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_UPDATE_TIME_STEP,
    HOOK_SET_VELOCITY,
    HOOK_APPLY_MOVE_FORCE_XYZ,
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_REMOVE,
    HOOK_ADD,
    HOOK_REMOVE_AND_ADD,
    HOOK_ADD_TO_MOVING_LIST,
    HOOK_REMOVE_FROM_MOVING_LIST,
    HOOK_ADD_COLLISION_RECORD,
    HOOK_GET_HAS_COLLIDED_WITH,
    HOOK_GET_HAS_COLLIDED_WITH_ANY_OBJECT,
    HOOK_REMOVE_REFS_TO_ENTITY,
    HOOK_CAN_PHYSICAL_BE_DAMAGED,
    HOOK_GET_BOUND_RECT,
    HOOK_GET_LIGHTING_FROM_COL,
    HOOK_GET_LIGHTING_TOTAL,
    HOOK_SKIP_PHYSICS,
];
