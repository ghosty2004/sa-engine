//! Small table helpers of the script system on the loading path (batch `mvpi-script-2`): streamed
//! script lookups, resource lookups, the connect-lod / special-anim-group / building-swap /
//! invisibility tables. Ported from the 1.0 US bytes (`.HOODLUM` bodies behind the stubs); unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// The connect-lod table (10 pairs of int32 handles)
const CONNECT_LODS: usize = 0xA44800;
/// The special anim groups attached to char models (8 of 0x14: model int32 +0, group name char[] +4)
const ATTACHED_ANIM_GROUPS: usize = 0xA44850;
/// The building swaps (0x19 of 0xC: building +0, new model +4, old model +8)
const BUILDING_SWAPS: usize = 0xA44A30;
/// The invisibility settings (0x14 entity pointers)
const INVISIBILITY_SETTINGS: usize = 0xA449E0;
/// The int16 id counters of thing type 8 (see `GetNewUniqueScriptThingIndex`)
const THING8_COUNTERS: u32 = 0xC098D0;

orig! {
    /// `CBuilding::ReplaceWithNewModel(int32 model)` (0x403EC0)
    fn building_replace_with_new_model(b: GPtr<u8>, model: u32) = thiscall 0x403EC0 => unported;
    /// `C3dMarkers::PlaceMarkerSet(uint32 id, uint16 type, CVector& pos, float size, uint8 r, uint8 g,
    /// uint8 b, uint8 a, uint16 pulsePeriod, float pulseFraction, int16 rotateRate)` (0x725BA0)
    fn place_marker_set(id: u32, ty: u32, pos: GPtr<u8>, size: u32, r: u32, g: u32, b: u32, a: u32, period: u32, frac: u32, rot: u32) = cdecl 0x725BA0 => unported;
}

/// The script spheres (16 of 0x18: active byte +0, marker id +4, position +8, radius +0x14)
const SCRIPT_SPHERES: usize = 0xA91268;

fn ga<T: sa_core::mem::GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// `CStreamedScripts::GetStreamedScriptWithThisStartAddress(uint8* ip)` (0x470910, thiscall, `ret 4`):
/// the first of the 0x52 streamed scripts (0x20 bytes, data pointer +0) whose data is `ip`, else 0x52.
#[hook(addr = 0x470910, conv = thiscall, name = "CStreamedScripts::GetStreamedScriptWithThisStartAddress",
       expect = "8B 54 24 04 56", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_streamed_script_with_this_start_address(this: GPtr<u8>, ip: u32) -> u32 {
    let mut i = 0u32;
    while i < 0x52 {
        if this.byte_add((i << 5) as isize).cast::<u32>().read() == ip {
            break;
        }
        i += 1;
    }
    i
}

/// Phase B/C path of the `GetStreamedScriptWithThisStartAddress` callers that take an `i32`.
pub fn get_streamed_script_with_this_start_address_host(this: GPtr<u8>, ip: u32) -> i32 {
    get_streamed_script_with_this_start_address(this, ip) as i32
}

/// `CTheScripts::GetUniqueScriptThingIndex(int32 index, uint8 type)` (stub 0x4810C0 → 0x1561250,
/// cdecl): type 8 → `counter[index] << 16 | index` (int16 counters at 0xC098D0), else -1.
#[hook(addr = 0x4810c0, conv = cdecl, name = "CTheScripts::GetUniqueScriptThingIndex", expect = "E9 8B 01 0E 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_unique_script_thing_index(index: i32, ty: u32) -> i32 {
    if ty as u8 != 8 {
        return -1;
    }
    let c = GPtr::<u16>::from_addr(THING8_COUNTERS.wrapping_add((index as u32).wrapping_mul(2)) as usize).read();
    ((u32::from(c) << 16) | index as u32) as i32
}

/// `CStreamedScripts::ReInitialise()` (stub 0x4706A0 → 0x1565AD0, thiscall): the status byte (+4) of
/// each of the 0x52 streamed scripts = 0.
#[hook(addr = 0x4706a0, conv = thiscall, name = "CStreamedScripts::ReInitialise", expect = "E9 2B 54 0F 01",
       unverified = "mvpi-script-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn streamed_scripts_reinitialise(this: GPtr<u8>) {
    for i in 0..0x52isize {
        this.byte_add(4 + i * 0x20).write(0);
    }
}

/// `CScriptResourceManager::HasResourceBeenRequested(int32 id, uint16 type)` (stub 0x470620 →
/// 0x156F2A0, thiscall, `ret 8`): AL = 1 at the first of the 0x4B entries (0xC bytes: id +0, type u16
/// +8) with this id and type, else 0. EAX's upper bytes are the original's: that entry's address, else
/// `this + 0x384`.
#[hook(addr = 0x470620, conv = thiscall, name = "CScriptResourceManager::HasResourceBeenRequested",
       expect = "E9 7B EC 0F 01", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn has_resource_been_requested(this: GPtr<u8>, id: i32, ty: u32) -> u32 {
    for i in 0..0x4Bisize {
        let e = this.byte_add(i * 0xC);
        if e.cast::<i32>().read() == id && u32::from(e.byte_add(8).cast::<u16>().read()) == ty {
            return (e.addr() as u32 & 0xFFFF_FF00) | 1;
        }
    }
    (this.addr() as u32).wrapping_add(0x384) & 0xFFFF_FF00
}

/// `CTheScripts::InitialiseAllConnectLodObjects()` (stub 0x470960 → 0x1562910, cdecl): the 10 pairs
/// at 0xA44800 = (-1, -1).
#[hook(addr = 0x470960, conv = cdecl, name = "CTheScripts::InitialiseAllConnectLodObjects", expect = "E9 AB 1F 0F 01",
       unverified = "mvpi-script-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn initialise_all_connect_lod_objects() {
    for i in 0..10usize {
        ga::<i32>(CONNECT_LODS + i * 8).write(-1);
        ga::<i32>(CONNECT_LODS + i * 8 + 4).write(-1);
    }
}

/// `CTheScripts::InitialiseSpecialAnimGroupsAttachedToCharModels()` (stub 0x474730 → 0x1562530,
/// cdecl): the 8 entries at 0xA44850: model -1, first name byte 0.
#[hook(addr = 0x474730, conv = cdecl, name = "CTheScripts::InitialiseSpecialAnimGroupsAttachedToCharModels",
       expect = "E9 FB DD 0E 01", unverified = "mvpi-script-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn initialise_special_anim_groups_attached_to_char_models() {
    for i in 0..8usize {
        ga::<i32>(ATTACHED_ANIM_GROUPS + i * 0x14).write(-1);
        ga::<u8>(ATTACHED_ANIM_GROUPS + i * 0x14 + 4).write(0);
    }
}

/// `CTheScripts::UndoBuildingSwaps()` (0x481290, cdecl): every used entry of the 0x19 building swaps
/// (building +0) gets `building->ReplaceWithNewModel(old model +8)`, then building 0 and both models
/// -1.
#[hook(addr = 0x481290, conv = cdecl, name = "CTheScripts::UndoBuildingSwaps", expect = "53 56 57 BE 30 4A A4 00",
       unverified = "mvpi-script-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn undo_building_swaps() {
    for i in 0..0x19usize {
        let e = BUILDING_SWAPS + i * 0xC;
        let b = ga::<GPtr<u8>>(e).read();
        if !b.is_null() {
            building_replace_with_new_model(b, ga::<u32>(e + 8).read());
            ga::<u32>(e).write(0);
            ga::<i32>(e + 4).write(-1);
            ga::<i32>(e + 8).write(-1);
        }
    }
}

/// `CTheScripts::UndoEntityInvisibilitySettings()` (stub 0x4812D0 → 0x15652D0, cdecl): every set entry
/// of the 0x14 at 0xA449E0: entity flags (+0x1C) |= 0x80, then |= 1 (re-read), entry = 0.
#[hook(addr = 0x4812d0, conv = cdecl, name = "CTheScripts::UndoEntityInvisibilitySettings", expect = "E9 FB 3F 0E 01",
       unverified = "mvpi-script-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn undo_entity_invisibility_settings() {
    for i in 0..0x14usize {
        let c = ga::<GPtr<u8>>(INVISIBILITY_SETTINGS + i * 4);
        let e = c.read();
        if e.is_null() {
            continue;
        }
        let f = e.byte_add(0x1C).cast::<u32>();
        f.write(f.read() | 0x80);
        let f = c.read().byte_add(0x1C).cast::<u32>();
        f.write(f.read() | 1);
        c.write(GPtr::NULL);
    }
}

/// `CTheScripts::DrawScriptSpheres()` (stub 0x4810E0 → 0x15647C0, cdecl): every active one of the 16
/// script spheres places marker set `(id, 1, pos, radius, 255, 0, 0, 228, 0x800, 0.1, 0)`.
#[hook(addr = 0x4810e0, conv = cdecl, name = "CTheScripts::DrawScriptSpheres", expect = "E9 DB 36 0E 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn draw_script_spheres() {
    for i in 0..16usize {
        let e = SCRIPT_SPHERES + i * 0x18;
        if ga::<u8>(e).read() == 0 {
            continue;
        }
        let radius = ga::<u32>(e + 0x14).read();
        let id = ga::<u32>(e + 4).read();
        place_marker_set(id, 1, GPtr::from_addr(e + 8), radius, 0xFF, 0, 0, 0xE4, 0x800, 0x3DCC_CCCD, 0);
    }
}

/// `CScriptsForBrains::IsObjectWithinBrainActivationRange(CObject* obj, const CVector& point)`
/// (0x46B3D0, thiscall, `ret 8`): the object's brain (int16 +0x168, 0x14-byte brains) must have attach
/// type 1, and the distance from the object's position (matrix or placement, copied) to `point`
/// (x87: `fsqrt(dz*dz + dy*dy + dx*dx)`) must be below the brain's radius (+8): AL = 1, else 0. The
/// original's EAX upper bytes are left over (index arithmetic / FPU status word).
#[hook(addr = 0x46b3d0, conv = thiscall, name = "CScriptsForBrains::IsObjectWithinBrainActivationRange",
       expect = "8B 54 24 04 0F BF 82 68 01 00 00", unverified = "mvpi-script-2", route = "L2", hazards = "ax-al-return,x87-transcendental",
       recs = "grove-walk")]
pub fn is_object_within_brain_activation_range(this: GPtr<u8>, obj: GPtr<u8>, point: GPtr<f32>) -> u8 {
    let b = this.byte_add(isize::from(obj.byte_add(0x168).cast::<i16>().read()) * 0x14);
    if b.byte_add(2).read() != 1 {
        return 0;
    }
    let m = obj.byte_add(0x14).cast::<GPtr<u8>>().read();
    let p: GPtr<f32> = if m.is_null() { obj.byte_add(4).cast() } else { m.byte_add(0x30).cast() };
    let pos = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
    let d = |i: usize| X87::from_f32(point.wrapping_index(i as i32).read()).fsub_m(pos[i]);
    let (dx, dy, dz) = (d(0), d(1), d(2));
    let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    let r = b.byte_add(8).cast::<f32>().read();
    u8::from(matches!(dist.fcom(X87::from_f32(r)), Cmp::Less))
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_OBJECT_WITHIN_BRAIN_ACTIVATION_RANGE,
    HOOK_DRAW_SCRIPT_SPHERES,
    HOOK_GET_STREAMED_SCRIPT_WITH_THIS_START_ADDRESS,
    HOOK_GET_UNIQUE_SCRIPT_THING_INDEX,
    HOOK_STREAMED_SCRIPTS_REINITIALISE,
    HOOK_HAS_RESOURCE_BEEN_REQUESTED,
    HOOK_INITIALISE_ALL_CONNECT_LOD_OBJECTS,
    HOOK_INITIALISE_SPECIAL_ANIM_GROUPS_ATTACHED_TO_CHAR_MODELS,
    HOOK_UNDO_BUILDING_SWAPS,
    HOOK_UNDO_ENTITY_INVISIBILITY_SETTINGS,
];
