//! PORT-RENDER: `CCullZones` (attribute, tunnel and mirror zones, the per-frame player / camera flags).
//! Ported from the 1.0 US bytes; gta-reversed `CullZones.cpp` as a reading aid.
//!
//! Zone records: `CZoneDef` (0x10 bytes: i16 x, y, vec1 x/y, vec2 x/y, minZ, maxZ) followed by an i16 flags
//! word (`CCullZoneReflection` / attribute zones, stride 0x12) or, for mirror zones (stride 0x18), by
//! `f32 mirrorV` (+0x10), `i8 normal x/y/z` (+0x14..+0x16) and the u8 flags (+0x17).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::FRAME_COUNTER;
use sa_core::{hook, orig};

use crate::common::{Fpu, ftol32, k, rd, x};
use sa_world::world::common::{find_player_coors, find_player_ped};

sa_core::globals! {
    list GLOBALS;
    /// `CCullZones::aMirrorAttributeZones[72]` (0x18 bytes each)
    pub static MIRROR_ZONES @ 0xC815C0: [u8; 72 * 0x18] = [0; 72 * 0x18];
    /// `CCullZones::aTunnelAttributeZones[40]` (0x12 bytes each)
    pub static TUNNEL_ZONES @ 0xC81C80: [u8; 40 * 0x12] = [0; 40 * 0x12];
    /// `CCullZones::aAttributeZones[1300]` (0x12 bytes each)
    pub static ATTRIBUTE_ZONES @ 0xC81F50: [u8; 1300 * 0x12] = [0; 1300 * 0x12];
    /// `CCullZones::CurrentFlags_Player`
    pub static CURRENT_FLAGS_PLAYER @ 0xC87AB8: i32 = 0;
    /// `CCullZones::CurrentFlags_Camera`
    pub static CURRENT_FLAGS_CAMERA @ 0xC87ABC: i32 = 0;
    /// `CCullZones::NumTunnelAttributeZones`
    pub static NUM_TUNNEL_ZONES @ 0xC87AC0: i32 = 0;
    /// `CCullZones::NumMirrorAttributeZones`
    pub static NUM_MIRROR_ZONES @ 0xC87AC4: i32 = 0;
    /// `CCullZones::NumAttributeZones`
    pub static NUM_ATTRIBUTE_ZONES @ 0xC87AC8: i32 = 0;
    /// `CCullZones::bMilitaryZonesDisabled`
    pub static MILITARY_ZONES_DISABLED @ 0xC87ACD: u8 = 0;
}

const ATTRIBUTE_BASE: usize = 0xC81F50;
const TUNNEL_BASE: usize = 0xC81C80;
const MIRROR_BASE: usize = 0xC815C0;
const ZONE_STRIDE: isize = 0x12;
const MIRROR_STRIDE: isize = 0x18;

/// 0.0 (0x858B50), 100.0 (0x858628)
const ZERO: u32 = 0x0000_0000;
const HUNDRED: u32 = 0x42C8_0000;

orig! {
    /// `CZoneDef::IsPointWithin(CVector)` (0x72D850, thiscall, vector by value)
    pub fn zone_is_point_within(zone: GPtr<u8>, px: f32, py: f32, pz: f32) -> u8 = thiscall 0x72D850 => sa_world::world::zones::zone_def_is_point_within;
    /// `CCamera::GetGameCamPosition()` (0x50AE50, returns `CVector*`)
    pub fn get_game_cam_position(cam: GPtr<u8>) -> GPtr<f32> = thiscall 0x50AE50 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    pub fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPlayerPed::SetWantedLevelNoDrop(int)` (0x609F30)
    pub fn set_wanted_level_no_drop(ped: GPtr<u8>, level: i32) = thiscall 0x609F30 => unported;
    /// `CCullZones::FindAttributesForCoors(CVector)` (0x72D970)
    pub fn find_attributes_for_coors_call(px: f32, py: f32, pz: f32) -> i32 = cdecl 0x72D970 => find_attributes_for_coors;
    /// `CCullZones::AddTunnelAttributeZone(...)` (0x72DB50)
    pub fn add_tunnel_attribute_zone_call(c: GPtr<f32>, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32, a6: f32, flags: u32) = cdecl 0x72DB50 => add_tunnel_attribute_zone;
}

/// The `CZoneDef` part of `AddCullZone` / `AddTunnelAttributeZone` / `AddMirrorAttributeZone`, in the
/// original store order: +0 `ftol(c.x - a1 - a4)`, +2 `ftol(c.y - a2 - a5)`, +4 `ftol(a1 + a1)`,
/// +6 `ftol(a2 + a2)`, +0xC `ftol(a3)`, +8 `ftol(a4 + a4)`, +0xA `ftol(a5 + a5)`, +0xE `ftol(a6)` (low
/// words of `_ftol`).
#[allow(clippy::too_many_arguments)]
fn store_zone_def(z: GPtr<u8>, c: GPtr<f32>, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32, a6: f32) {
    let w = |o: isize, v: sa_math::x87::X87| sa_world::world::common::wr::<u16>(z, o, ftol32(v) as u16);
    let mut f = Fpu::new();
    f.fld(c.read());
    f.fsub_m(a1);
    f.fsub_m(a4);
    w(0, f.pop());
    f.fld(c.wrapping_index(1).read());
    f.fsub_m(a2);
    f.fsub_m(a5);
    w(2, f.pop());
    for (o, v, dbl) in [
        (4, a1, true),
        (6, a2, true),
        (0xC, a3, false),
        (8, a4, true),
        (0xA, a5, true),
        (0xE, a6, false),
    ] {
        let mut v = x(v);
        if dbl {
            v = v.fadd(v);
        }
        w(o, v);
    }
}

/// `CCullZones::Init()` (0x72D6B0): `NumAttributeZones = CurrentFlags_Player = CurrentFlags_Camera = 0`
/// (the tunnel / mirror counts are left alone).
#[hook(addr = 0x72D6B0, conv = cdecl, name = "CCullZones::Init", expect = "33 C0 A3 C8 7A C8 00")]
pub fn init() {
    NUM_ATTRIBUTE_ZONES.set(0);
    CURRENT_FLAGS_PLAYER.set(0);
    CURRENT_FLAGS_CAMERA.set(0);
}

/// The OR of the sign-extended flags words of every zone of `base[0..n]` that contains the point (the
/// count is read once).
fn or_flags_of_zones(base: usize, n: i32, px: f32, py: f32, pz: f32) -> i32 {
    let mut flags = 0i32;
    let mut z = GPtr::<u8>::from_addr(base);
    let mut left = n;
    if left <= 0 {
        return 0;
    }
    loop {
        if zone_is_point_within(z, px, py, pz) != 0 {
            flags |= i32::from(rd::<i16>(z, 0x10));
        }
        z = z.byte_add(ZONE_STRIDE);
        left -= 1;
        if left == 0 {
            return flags;
        }
    }
}

/// `CCullZones::FindAttributesForCoors(CVector)` (0x72D970, vector by value): the OR of the flags of
/// every attribute zone containing the point.
#[hook(addr = 0x72D970, conv = cdecl, name = "CCullZones::FindAttributesForCoors", expect = "83 EC 08 A1 C8 7A C8 00")]
pub fn find_attributes_for_coors(px: f32, py: f32, pz: f32) -> i32 {
    or_flags_of_zones(ATTRIBUTE_BASE, NUM_ATTRIBUTE_ZONES.get(), px, py, pz)
}

/// `CCullZones::FindTunnelAttributesForCoors(CVector)` (0x72D9F0): the same over the tunnel zones. The
/// entry jumps into the protection chunk 0x4017AD, which loads `NumTunnelAttributeZones` (0xC87AC0) and
/// jumps back. Not hooked: the protection reads the entry's jump displacement (0x72D9F4) as data
/// (0x401C18 `adc`, 0x1566FC3 `xor`); callers reach the original through `orig!`.
pub fn find_tunnel_attributes_for_coors(px: f32, py: f32, pz: f32) -> i32 {
    or_flags_of_zones(TUNNEL_BASE, NUM_TUNNEL_ZONES.get(), px, py, pz)
}

/// `CCullZones::FindMirrorAttributesForCoors(CVector)` (0x72DA70): the first mirror zone containing the
/// point, or null (count read once).
#[hook(addr = 0x72DA70, conv = cdecl, name = "CCullZones::FindMirrorAttributesForCoors", expect = "53 8B 1D C4 7A C8 00")]
pub fn find_mirror_attributes_for_coors(px: f32, py: f32, pz: f32) -> GPtr<u8> {
    let n = NUM_MIRROR_ZONES.get();
    let mut i = 0;
    while i < n {
        let z = GPtr::<u8>::from_addr(MIRROR_BASE).byte_add(i as isize * MIRROR_STRIDE);
        if zone_is_point_within(z, px, py, pz) != 0 {
            return z;
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CCullZones::FindZoneWithStairsAttributeForPlayer()` (0x72DAD0): the player's coordinates
/// (`FindPlayerCoors(&v, -1)`), then the first attribute zone with flag bit 1 (byte +0x10 & 2) containing
/// them, or null; the count is re-read every iteration.
#[hook(addr = 0x72DAD0, conv = cdecl, name = "CCullZones::FindZoneWithStairsAttributeForPlayer", expect = "83 EC 0C 53 55")]
pub fn find_zone_with_stairs_attribute_for_player() -> GPtr<u8> {
    let mut v = [0f32; 3];
    find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
    let mut i = 0;
    while i < NUM_ATTRIBUTE_ZONES.get() {
        let z = GPtr::<u8>::from_addr(ATTRIBUTE_BASE).byte_add(i as isize * ZONE_STRIDE);
        if rd::<u8>(z, 0x10) & 2 != 0 && zone_is_point_within(z, v[0], v[1], v[2]) != 0 {
            return z;
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CCullZones::AddTunnelAttributeZone(const CVector& c, a1..a6, u16 flags)` (0x72DB50): appends a
/// tunnel zone (no bounds check), flags word at +0x10.
#[hook(addr = 0x72DB50, conv = cdecl, name = "CCullZones::AddTunnelAttributeZone", expect = "53 8B 5C 24 08")]
#[allow(clippy::too_many_arguments)]
pub fn add_tunnel_attribute_zone(
    c: GPtr<f32>,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    a6: f32,
    flags: u32,
) {
    let n = NUM_TUNNEL_ZONES.get();
    let z = GPtr::<u8>::from_addr(TUNNEL_BASE).byte_add(n as isize * ZONE_STRIDE);
    store_zone_def(z, c, a1, a2, a3, a4, a5, a6);
    NUM_TUNNEL_ZONES.set(n.wrapping_add(1));
    sa_world::world::common::wr::<u16>(z, 0x10, flags as u16);
}

/// `CCullZones::AddMirrorAttributeZone(const CVector& c, a1..a6, u8 flags, f32 mirrorV, f32 nx, f32 ny,
/// f32 nz)` (0x72DC10): appends a mirror zone; the normal is stored as `(i8)_ftol(n * 100.0f)`.
#[hook(addr = 0x72DC10, conv = cdecl, name = "CCullZones::AddMirrorAttributeZone", expect = "53 8B 5C 24 08")]
#[allow(clippy::too_many_arguments)]
pub fn add_mirror_attribute_zone(
    c: GPtr<f32>,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    a6: f32,
    flags: u32,
    mirror_v: u32,
    nx: f32,
    ny: f32,
    nz: f32,
) {
    let n = NUM_MIRROR_ZONES.get();
    let z = GPtr::<u8>::from_addr(MIRROR_BASE).byte_add(n as isize * MIRROR_STRIDE);
    store_zone_def(z, c, a1, a2, a3, a4, a5, a6);
    let wr8 = |o: isize, v: u8| sa_world::world::common::wr::<u8>(z, o, v);
    wr8(0x17, flags as u8);
    sa_world::world::common::wr::<u32>(z, 0x10, mirror_v);
    wr8(0x14, ftol32(x(nx).fmul_m(k(HUNDRED))) as u8);
    wr8(0x15, ftol32(x(ny).fmul_m(k(HUNDRED))) as u8);
    NUM_MIRROR_ZONES.set(n.wrapping_add(1));
    wr8(0x16, ftol32(x(nz).fmul_m(k(HUNDRED))) as u8);
}

/// `(flags >> bit) & 1`, the AL every flag getter returns.
#[inline]
fn bit(flags: i32, b: u32) -> u8 {
    ((flags as u32 >> b) & 1) as u8
}

/// `CCullZones::CamCloseInForPlayer()` (0x72DD20): player flags bit 0.
#[hook(addr = 0x72DD20, conv = cdecl, name = "CCullZones::CamCloseInForPlayer", expect = "A0 B8 7A C8 00")]
pub fn cam_close_in_for_player() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 0)
}

/// `CCullZones::CamStairsForPlayer()` (0x72DD30): player flags bit 1.
#[hook(addr = 0x72DD30, conv = cdecl, name = "CCullZones::CamStairsForPlayer", expect = "A0 B8 7A C8 00")]
pub fn cam_stairs_for_player() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 1)
}

/// `CCullZones::Cam1stPersonForPlayer()` (0x72DD40): player flags bit 2.
#[hook(addr = 0x72DD40, conv = cdecl, name = "CCullZones::Cam1stPersonForPlayer", expect = "A0 B8 7A C8 00")]
pub fn cam_1st_person_for_player() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 2)
}

/// `CCullZones::NoPolice()` (0x72DD50): player flags bit 4.
#[hook(addr = 0x72DD50, conv = cdecl, name = "CCullZones::NoPolice", expect = "A0 B8 7A C8 00")]
pub fn no_police() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 4)
}

/// `CCullZones::PoliceAbandonCars()` (0x72DD60): player flags bit 8.
#[hook(addr = 0x72DD60, conv = cdecl, name = "CCullZones::PoliceAbandonCars", expect = "A1 B8 7A C8 00")]
pub fn police_abandon_cars() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 8)
}

/// `CCullZones::InRoomForAudio()` (0x72DD70): camera flags bit 9.
#[hook(addr = 0x72DD70, conv = cdecl, name = "CCullZones::InRoomForAudio", expect = "A1 BC 7A C8 00")]
pub fn in_room_for_audio() -> u8 {
    bit(CURRENT_FLAGS_CAMERA.get(), 9)
}

/// `CCullZones::FewerCars()` (0x72DD80): player flags bit 15.
#[hook(addr = 0x72DD80, conv = cdecl, name = "CCullZones::FewerCars", expect = "A1 B8 7A C8 00")]
pub fn fewer_cars() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 15)
}

/// `CCullZones::FewerPeds()` (0x72DD90): player flags bit 10.
#[hook(addr = 0x72DD90, conv = cdecl, name = "CCullZones::FewerPeds", expect = "A1 B8 7A C8 00")]
pub fn fewer_peds() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 10)
}

/// `CCullZones::DoINeedToLoadCollision()` (0x72DDA0): player flags bit 6.
#[hook(addr = 0x72DDA0, conv = cdecl, name = "CCullZones::DoINeedToLoadCollision", expect = "A0 B8 7A C8 00")]
pub fn do_i_need_to_load_collision() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 6)
}

/// `CCullZones::CamNoRain()` (0x72DDB0): camera flags bit 3.
#[hook(addr = 0x72DDB0, conv = cdecl, name = "CCullZones::CamNoRain", expect = "A0 BC 7A C8 00")]
pub fn cam_no_rain() -> u8 {
    bit(CURRENT_FLAGS_CAMERA.get(), 3)
}

/// `CCullZones::PlayerNoRain()` (0x72DDC0): player flags bit 3.
#[hook(addr = 0x72DDC0, conv = cdecl, name = "CCullZones::PlayerNoRain", expect = "A0 B8 7A C8 00")]
pub fn player_no_rain() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 3)
}

/// `CCullZones::DoExtraAirResistanceForPlayer()` (0x72DDD0): player flags bit 14.
#[hook(addr = 0x72DDD0, conv = cdecl, name = "CCullZones::DoExtraAirResistanceForPlayer", expect = "A1 B8 7A C8 00")]
pub fn do_extra_air_resistance_for_player() -> u8 {
    bit(CURRENT_FLAGS_PLAYER.get(), 14)
}

/// One edge test of `IsPointWithinArbitraryArea`: `(a - b) * (c - d) - (e - f) * (g - h)` compared with
/// 0.0; returns the `test ah, 5` parity (true = even = not less).
#[allow(clippy::too_many_arguments)]
fn edge(a: f32, b: f32, c: f32, d: f32, e: f32, f_: f32, g: f32, h: f32) -> bool {
    let mut f = Fpu::new();
    f.fld(a);
    f.fsub_m(b);
    f.fld(c);
    f.fsub_m(d);
    f.fmulp(1);
    f.fld(e);
    f.fsub_m(f_);
    f.fld(g);
    f.fsub_m(h);
    f.fmulp(1);
    f.fsubp(1);
    f.fcomp_m(k(ZERO));
    f.jp(0x05)
}

/// `CCullZones::IsPointWithinArbitraryArea(px, py, x1, y1, x2, y2, x3, y3, x4, y4)` (0x72DDE0): the point
/// is on the non-negative side of the four edges 1-2, 2-3, 3-4, 4-1 (a negative edge value returns 0;
/// NaN passes).
#[hook(addr = 0x72DDE0, conv = cdecl, name = "CCullZones::IsPointWithinArbitraryArea", expect = "D9 44 24 04 D8 64 24 0C")]
#[allow(clippy::too_many_arguments)]
pub fn is_point_within_arbitrary_area(
    px: f32,
    py: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
    x4: f32,
    y4: f32,
) -> u8 {
    if !edge(px, x1, y2, y1, py, y1, x2, x1) {
        return 0;
    }
    if !edge(px, x2, y3, y2, py, y2, x3, x2) {
        return 0;
    }
    if !edge(px, x3, y4, y3, py, y3, x4, x3) {
        return 0;
    }
    // last edge: (px - x4) * (y1 - y4) - (x1 - x4) * (py - y4)
    u8::from(edge(px, x4, y1, y4, x1, x4, py, y4))
}

/// `CCullZones::Update()` (0x72DEC0): on `(FrameCounter & 7) == 2` the camera flags are refreshed from
/// `TheCamera.GetGameCamPosition()`; on `== 6` the player flags from `FindPlayerCoors(-1)`, and then, unless
/// `bMilitaryZonesDisabled`, flag bit 12 (military zone) with a live player sets wanted level 5
/// (`SetWantedLevelNoDrop`).
#[hook(addr = 0x72DEC0, conv = cdecl, name = "CCullZones::Update", expect = "A1 4C CB B7 00")]
pub fn update() {
    match (FRAME_COUNTER.get() & 7).wrapping_sub(2) {
        0 => {
            let p = get_game_cam_position(GPtr::from_addr(crate::common::THE_CAMERA));
            let (a, b, c) = (p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
            CURRENT_FLAGS_CAMERA.set(find_attributes_for_coors_call(a, b, c));
        }
        4 => {
            let mut v = [0f32; 3];
            let p = find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
            let (a, b, c) = (p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
            let flags = find_attributes_for_coors_call(a, b, c);
            CURRENT_FLAGS_PLAYER.set(flags);
            if MILITARY_ZONES_DISABLED.get() != 0 || flags & 0x1000 == 0 {
                return;
            }
            if ped_is_alive(find_player_ped(-1)) == 0 {
                return;
            }
            set_wanted_level_no_drop(find_player_ped(-1), 5);
        }
        _ => {}
    }
}

/// `CCullZones::AddCullZone(const CVector& c, a1..a6, u32 flags, u32 unused)` (0x72DF70): flags with
/// bit 7 or bit 11 also add a tunnel zone (`AddTunnelAttributeZone`, all flags) and are masked with
/// 0xF77F; a non-zero low word then appends an attribute zone (no bounds check).
#[hook(addr = 0x72DF70, conv = cdecl, name = "CCullZones::AddCullZone", expect = "53 8B 5C 24 24")]
#[allow(clippy::too_many_arguments)]
pub fn add_cull_zone(
    c: GPtr<f32>,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    a6: f32,
    flags: u32,
    _unused: u32,
) {
    let mut flags = flags;
    if flags & 0x80 != 0 || flags & 0x800 != 0 {
        add_tunnel_attribute_zone_call(c, a1, a2, a3, a4, a5, a6, flags);
        flags &= 0xF77F;
    }
    if flags as u16 == 0 {
        return;
    }
    let n = NUM_ATTRIBUTE_ZONES.get();
    let z = GPtr::<u8>::from_addr(ATTRIBUTE_BASE).byte_add(n as isize * ZONE_STRIDE);
    store_zone_def(z, c, a1, a2, a3, a4, a5, a6);
    NUM_ATTRIBUTE_ZONES.set(n.wrapping_add(1));
    sa_world::world::common::wr::<u16>(z, 0x10, flags as u16);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT,
    HOOK_FIND_ATTRIBUTES_FOR_COORS,
    HOOK_FIND_MIRROR_ATTRIBUTES_FOR_COORS,
    HOOK_FIND_ZONE_WITH_STAIRS_ATTRIBUTE_FOR_PLAYER,
    HOOK_ADD_TUNNEL_ATTRIBUTE_ZONE,
    HOOK_ADD_MIRROR_ATTRIBUTE_ZONE,
    HOOK_CAM_CLOSE_IN_FOR_PLAYER,
    HOOK_CAM_STAIRS_FOR_PLAYER,
    HOOK_CAM_1ST_PERSON_FOR_PLAYER,
    HOOK_NO_POLICE,
    HOOK_POLICE_ABANDON_CARS,
    HOOK_IN_ROOM_FOR_AUDIO,
    HOOK_FEWER_CARS,
    HOOK_FEWER_PEDS,
    HOOK_DO_I_NEED_TO_LOAD_COLLISION,
    HOOK_CAM_NO_RAIN,
    HOOK_PLAYER_NO_RAIN,
    HOOK_DO_EXTRA_AIR_RESISTANCE_FOR_PLAYER,
    HOOK_IS_POINT_WITHIN_ARBITRARY_AREA,
    HOOK_UPDATE,
    HOOK_ADD_CULL_ZONE,
];
