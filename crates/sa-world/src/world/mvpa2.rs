//! WRITER-MVP-A batch `mvpa-world-2`: further WORLD / VEH leaves on the native loading path
//! (placement matrix, ground height probe, vehicle pointer validation, mission trains, special plates).
//! Ported from the 1.0 US bytes, SecuROM fragments followed. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr, x};

/// `CPools::ms_pVehiclePool`
const VEHICLE_POOL: usize = 0xB74494;

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(p: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, `ret 4`)
    fn matrix_assign(m: GPtr<u8>, o: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)`
    /// (0x5674E0)
    #[allow(clippy::too_many_arguments)]
    fn world_process_vertical_line(p: GPtr<u8>, min_z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b0: u32, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn world_find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// virtual deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CQuadTreeNode::GetAllMatching(const CRect&, CPtrListSingleLink&)` (0x5528C0, `ret 8`)
    fn quad_tree_get_all_matching_rect(q: GPtr<u8>, rect: GPtr<u8>, list: GPtr<u8>) = thiscall 0x5528C0 => unported;
    /// `CPtrListSingleLink::Flush()` (0x552400)
    fn single_link_flush(l: GPtr<u8>) = thiscall 0x552400 => crate::world::mvpa_math::single_link_flush;
    /// `CPointRoutePool::New()` (0x41B5B0)
    fn point_route_pool_new(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x41B5B0 => unported;
}

/// `CPlaceable::SetMatrix(const CMatrix& m)` (0x54F610, thiscall, `ret 4`). Without a matrix (+0x14) and
/// when `m.at.z` is exactly 1.0f (bit compare): the simple transform (+4) takes `m.pos` (x as a dword, y
/// and z through `fld`/`fstp`) and heading `fpatan(-m.up.x, m.up.y)` (atan2, rounded by the f32 store).
/// Otherwise the matrix is allocated when missing (`AllocateMatrix`) and assigned.
#[hook(addr = 0x54F610, conv = thiscall, name = "CPlaceable::SetMatrix", expect = "83 EC 10 56 8B 74 24 18",
       this = "CPlaceable", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk")]
pub fn placeable_set_matrix(this: GPtr<u8>, m: GPtr<u8>) {
    if rd::<GPtr<u8>>(this, 0x14).is_null() {
        if rd::<u32>(m, 0x28) == 0x3F80_0000 {
            let y = x(rd::<f32>(m, 0x34)).to_f32();
            let z = x(rd::<f32>(m, 0x38)).to_f32();
            let h = x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)));
            wr::<u32>(this, 4, rd::<u32>(m, 0x30));
            wr::<f32>(this, 8, y);
            wr::<f32>(this, 0xC, z);
            wr::<f32>(this, 0x10, h.to_f32());
            return;
        }
        placeable_allocate_matrix(this);
    }
    matrix_assign(rd::<GPtr<u8>>(this, 0x14), m);
}

/// `CWorld::FindGroundZFor3DCoord(float x, float y, float z, bool* found, CEntity** entity)` (0x5696C0,
/// cdecl, ST0): `ProcessVerticalLine((x, y, z), -1000.0f, colPoint, outEntity, buildings only)`. Hit:
/// `*found = true`, `*entity = outEntity` (when the pointers are set) and the colpoint's z; miss: `*found
/// = false`, `*entity = null`, 0.0. The original keeps `outEntity` in its own `x` argument slot (residue
/// in the caller's argument area); here it is a local.
#[hook(addr = 0x5696C0, conv = cdecl, name = "CWorld::FindGroundZFor3DCoord", expect = "83 EC 38 8B 44 24 3C",
       unverified = "mvpa-world-2", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the original writes the hit entity over its own first argument slot")]
pub fn find_ground_z_for_3d_coord(px: f32, py: f32, pz: f32, found: GPtr<u8>, entity: GPtr<u8>) -> X87 {
    let mut point = [px, py, pz];
    let mut colpoint = [0u32; 11];
    let mut out = [0u32; 1];
    let hit = world_process_vertical_line(
        GPtr::<f32>::from_raw(point.as_mut_ptr()).cast(),
        0xC47A_0000,
        GPtr::<u32>::from_raw(colpoint.as_mut_ptr()).cast(),
        GPtr::<u32>::from_raw(out.as_mut_ptr()).cast(),
        1,
        0,
        0,
        0,
        0,
        0,
        0,
    );
    if hit == 0 {
        if !found.is_null() {
            found.write(0);
        }
        if !entity.is_null() {
            entity.cast::<u32>().write(0);
        }
        return x(g::<f32>(0x858B50).read());
    }
    if !found.is_null() {
        found.write(1);
    }
    if !entity.is_null() {
        entity.cast::<u32>().write(out[0]);
    }
    x(f32::from_bits(colpoint[2]))
}

/// `CVehicle::IsVehiclePointerValid(CVehicle*)` (0x6E38F0, cdecl): the signed slot index `(v - objects) /
/// 0xA18` must be in `[0, size)` with the slot used; then true for trains (+0x590 == 8) or when +0xB0
/// (the entity's collision/link pointer) is set. EAX is 0 or 1.
#[hook(addr = 0x6E38F0, conv = cdecl, name = "CVehicle::IsVehiclePointerValid", expect = "56 90 E9 32 DE D1 FF",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn is_vehicle_pointer_valid(v: GPtr<u8>) -> u32 {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let idx = (v.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32 / 0xA18;
    if idx < 0 || idx >= rd::<i32>(pool, 8) || (rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize).read() as i8) < 0 {
        return 0;
    }
    if rd::<u32>(v, 0x590) == 8 || rd::<u32>(v, 0xB0) != 0 {
        1
    } else {
        0
    }
}

/// `CTrain::ReleaseMissionTrains()` (0x6F6B60): for every used, non-null vehicle (pool from the last slot
/// down) whose type (+0x590) is 6 (train) and which is not `FindPlayerVehicle(-1, false)`, clears bit 5
/// of byte +0x5B8 (the mission-train flag).
#[hook(addr = 0x6F6B60, conv = cdecl, name = "CTrain::ReleaseMissionTrains", expect = "53 8B 1D 94 44 B7 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn release_mission_trains() {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i.wrapping_mul(0xA18) as isize);
        if v.is_null() || rd::<u32>(v, 0x590) != 6 {
            continue;
        }
        if v == world_find_player_vehicle(-1, 0) {
            continue;
        }
        wr::<u8>(v, 0x5B8, rd::<u8>(v, 0x5B8) & 0xDF);
    }
}

/// `CSpecialPlateHandler::Add(int32 carGenId, const char* text)` (0x6F2D90, thiscall, `ret 8`): when the
/// count (+0xF0) is not 15, entry `count` (0x10 bytes) gets the id and the text (byte copy from +4), then
/// count + 1.
#[hook(addr = 0x6F2D90, conv = thiscall, name = "CSpecialPlateHandler::Add", expect = "8B 81 F0 00 00 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn special_plate_handler_add(this: GPtr<u8>, id: i32, text: GPtr<u8>) {
    let n = rd::<i32>(this, 0xF0);
    if n == 15 {
        return;
    }
    wr::<i32>(this, (n << 4) as isize, id);
    let n = rd::<i32>(this, 0xF0);
    let mut d = this.byte_add(((n << 4) + 4) as isize);
    let mut s = text;
    loop {
        let b = s.read();
        d.write(b);
        s = s.byte_add(1);
        d = d.byte_add(1);
        if b == 0 {
            break;
        }
    }
    wr::<i32>(this, 0xF0, rd::<i32>(this, 0xF0).wrapping_add(1));
}

/// `CPlane::SwitchAmbientPlanes(bool on)` (0x6CCC50, cdecl): when ambient planes are currently on
/// (`GenPlanesActive`... byte 0x8D33BC) and `on` is false, every used, non-null vehicle (pool from the last
/// slot down) with subtype (+0x594) 3 or 4 created randomly (+0x4A4 == 1) is removed from the world and
/// deleted. The flag becomes `on` in every case.
#[hook(addr = 0x6CCC50, conv = cdecl, name = "CPlane::SwitchAmbientPlanes", expect = "A0 BC 33 8D 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "deletes vehicles (pool frees)")]
pub fn switch_ambient_planes(on: u32) {
    if g::<u8>(0x8D33BC).read() != 0 && on as u8 == 0 {
        let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
        let mut i = rd::<i32>(pool, 8);
        while i != 0 {
            i -= 1;
            if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
                continue;
            }
            let v = rd::<GPtr<u8>>(pool, 0).byte_add(i.wrapping_mul(0xA18) as isize);
            if v.is_null() {
                continue;
            }
            let t = rd::<u32>(v, 0x594);
            if (t == 3 || t == 4) && rd::<u8>(v, 0x4A4) == 1 {
                world_remove(v);
                v_delete(v, 1);
            }
        }
    }
    g::<u8>(0x8D33BC).write(on as u8);
}

/// `CEntryExitManager::FindNearestEntryExit(const CVector2D& pos, float range, int32 ignoreArea)`
/// (0x43F4B0, cdecl): the entry-exits of the quad tree (0x96A7D0) in the rect (x - r, y + r, x + r,
/// y - r) are collected into a local list; the nearest whose area (its link's when linked, byte +0x32)
/// is not `ignoreArea` wins, by `fsqrt(dx*dx + dy*dy)` to the entrance centre (`(right+left)*0.5`,
/// `(top+bottom)*0.5`) against a best distance that starts at `2 * range` (the original doubles its own
/// `range` argument slot and compares against it, kept as a value here). Returns the winner's pool index
/// (signed division by 0x3C) or -1; the list is flushed either way.
#[hook(addr = 0x43F4B0, conv = cdecl, name = "CEntryExitManager::FindNearestEntryExit", expect = "6A FF 68 38 77 83 00",
       unverified = "mvpa-world-2", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk",
       note = "fsqrt; the original overwrites its range argument slot with 2*range and then the best distance")]
pub fn find_nearest_entry_exit(pos: GPtr<u8>, range: f32, ignore_area: i32) -> i32 {
    let mut list = [0u32; 1];
    let lp = GPtr::<u32>::from_raw(list.as_mut_ptr()).cast::<u8>();
    let px = rd::<f32>(pos, 0);
    let py = rd::<f32>(pos, 4);
    let mut rect = [0f32; 4];
    rect[0] = x(px).fsub_m(range).to_f32();
    rect[3] = x(py).fsub_m(range).to_f32();
    rect[2] = x(px).fadd_m(range).to_f32();
    rect[1] = x(py).fadd_m(range).to_f32();
    let mut best = x(range).fadd(x(range)).to_f32();
    quad_tree_get_all_matching_rect(g::<GPtr<u8>>(0x96A7D0).read(), GPtr::<f32>::from_raw(rect.as_mut_ptr()).cast(), lp);
    let mut found = GPtr::<u8>::NULL;
    let mut node = rd::<GPtr<u8>>(lp, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        let link = rd::<GPtr<u8>>(e, 0x38);
        let area = if link.is_null() { rd::<u8>(e, 0x32) } else { rd::<u8>(link, 0x32) };
        if i32::from(area) == ignore_area {
            continue;
        }
        let cx = x(rd::<f32>(e, 0x10)).fadd_m(rd::<f32>(e, 8)).fmul_m(0.5);
        let cy = x(rd::<f32>(e, 0x14)).fadd_m(rd::<f32>(e, 0xC)).fmul_m(0.5);
        let dy = x(py).fsub(cy);
        let dx = x(px).fsub(cx);
        let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
        if !d.fcom(x(best)).test_parity(5) {
            best = d.to_f32();
            found = e;
        }
    }
    if found.is_null() {
        single_link_flush(lp);
        return -1;
    }
    let base = rd::<u32>(g::<GPtr<u8>>(0x96A7D8).read(), 0);
    let idx = (found.addr() as u32).wrapping_sub(base) as i32 / 0x3C;
    single_link_flush(lp);
    idx
}

/// `CPool::Clear()` (0x49C9A0, `.HOODLUM` stub; body 0x1562930; thiscall on a `CPool`): every byte-map
/// entry `0..size` (size re-read each round) gets bit 7 set and then everything but bit 7 cleared (free,
/// id 0).
#[hook(addr = 0x49C9A0, conv = thiscall, name = "Clear", expect = "E9 8B 5F 0C 01",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "CPool<T>::Clear instance (catalogue name 'Clear')")]
pub fn pool_clear(this: GPtr<u8>) {
    let mut i = 0i32;
    while i < rd::<i32>(this, 8) {
        let m = rd::<GPtr<u8>>(this, 4).byte_add(i as isize);
        m.write(m.read() | 0x80);
        let m = rd::<GPtr<u8>>(this, 4).byte_add(i as isize);
        m.write(m.read() & 0x80);
        i += 1;
    }
}

/// `CPointRoute::operator delete(void*)` (0x41B7D0, `.HOODLUM` stub; body 0x1564FD0): frees the slot in
/// `CPools::ms_pPointRoutePool` (0xB744B0, 0x64-byte objects; signed index) inline: byte-map bit 7 set,
/// first-free index lowered.
#[hook(addr = 0x41B7D0, conv = cdecl, name = "CPointRoute::operator delete", expect = "E9 FB 97 14 01",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn point_route_delete(p: GPtr<u8>) {
    let pool = g::<GPtr<u8>>(0xB744B0).read();
    let idx = (p.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32 / 0x64;
    let m = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize);
    m.write(m.read() | 0x80);
    if idx < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, idx);
    }
}

/// `CPointRoute::operator new(uint32)` (0x41B7C0, cdecl): a tail call of `CPointRoutePool::New()` on
/// `CPools::ms_pPointRoutePool` (0xB744B0, loaded in a SecuROM fragment); the size is ignored.
#[hook(addr = 0x41B7C0, conv = cdecl, name = "CPointRoute::operator new", expect = "90 E9 27 E3 FE FF",
       unverified = "mvpa-world-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn point_route_new(_size: u32) -> GPtr<u8> {
    point_route_pool_new(g::<GPtr<u8>>(0xB744B0).read())
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PLACEABLE_SET_MATRIX,
    HOOK_FIND_GROUND_Z_FOR_3D_COORD,
    HOOK_IS_VEHICLE_POINTER_VALID,
    HOOK_RELEASE_MISSION_TRAINS,
    HOOK_SPECIAL_PLATE_HANDLER_ADD,
    HOOK_SWITCH_AMBIENT_PLANES,
    HOOK_FIND_NEAREST_ENTRY_EXIT,
    HOOK_POOL_CLEAR,
    HOOK_POINT_ROUTE_DELETE,
    HOOK_POINT_ROUTE_NEW,
];
