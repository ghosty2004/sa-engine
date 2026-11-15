//! Small script helpers the ported command switches still reached through the original (FRONTIER-2 lane C, batch
//! `frontier2-c2`). Ported from the 1.0 US bytes (the `.HOODLUM` bodies included). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::field::at;
use sa_core::hook;
use sa_core::mem::GPtr;

/// `CRunningScript::ThisIsAValidRandomPed(ePedType type, bool civilian, bool gang, bool criminal)` (0x489490,
/// thiscall, `ret 0x10`; byte table 0x4894E8 over `type - 4`, dword table 0x4894D8): types 4 / 5 → `civilian`, 7..15
/// → `gang`, 20 / 22 → `criminal` (each as a whole dword, non-zero → 1); every other type, and anything outside 4..22
/// → 0. AL.
#[hook(addr = 0x489490, conv = thiscall, name = "CRunningScript::ThisIsAValidRandomPed", expect = "8B 44 24 04 83 C0 FC",
       unverified = "frontier2-c2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn this_is_a_valid_random_ped(_this: GPtr<u8>, ty: u32, civilian: u32, gang: u32, criminal: u32) -> u8 {
    let t = ty.wrapping_sub(4);
    if t > 0x12 {
        return 0;
    }
    const CASE: [u8; 0x13] = [0, 0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 2, 3, 2];
    let v = match CASE[t as usize] {
        0 => civilian,
        1 => gang,
        2 => criminal,
        _ => 0,
    };
    u8::from(v != 0)
}

/// `CScriptsForBrains::SwitchAllObjectBrainsWithThisID(int8 id, bool on)` (stub 0x46A900 → .HOODLUM 0x156FA50,
/// thiscall, `ret 8`): for a non-negative id, every one of the 0x46 brains whose id byte (+3) equals it gets its
/// active byte (+4) = `on`.
#[hook(addr = 0x46A900, conv = thiscall, name = "CScriptsForBrains::SwitchAllObjectBrainsWithThisID",
       expect = "E9 4B 51 10 01", unverified = "frontier2-c2", route = "L2", hazards = "none")]
pub fn switch_all_object_brains_with_this_id(this: GPtr<u8>, id: u32, on: u32) {
    let id = id as u8;
    if (id as i8) < 0 {
        return;
    }
    for i in 0..0x46isize {
        let e = this.byte_add(i * 0x14);
        if at::<u8>(e, 3).read() == id {
            at::<u8>(e, 4).write(on as u8);
        }
    }
}

/// The script attractor record's ped-type slot claim (stub 0x492CB0 → .HOODLUM 0x156E990, thiscall, `ret 4`; unnamed in
/// gta-reversed): `+0x20 = 1`; the first of the four handle slots (+0) holding -1 becomes -2 and its ped type (+0x10
/// + 4i) the argument; the others are left alone.
#[hook(addr = 0x492CB0, conv = thiscall, name = "thunk_FUN_0156e990", expect = "E9 DB BC 0D 01",
       unverified = "frontier2-c2", route = "L2", hazards = "none")]
pub fn script_attractor_add_ped_type(this: GPtr<u8>, ped_type: u32) {
    at::<u8>(this, 0x20).write(1);
    for i in 0..4isize {
        if at::<i32>(this, 4 * i).read() == -1 {
            at::<i32>(this, 4 * i).write(-2);
            at::<u32>(this, 0x10 + 4 * i).write(ped_type);
            return;
        }
    }
}

/// `CStreamedScripts::FindStreamedScriptQuiet(const char* name)` (0x4706F0, thiscall, `ret 4`): the index of the first
/// of the `+0xA44` (word) streamed scripts whose name (`+8 + 0x20 i`) `_stricmp`s equal, else -1.
#[hook(addr = 0x4706F0, conv = thiscall, name = "CStreamedScripts::FindStreamedScriptQuiet", expect = "53 55 56 8B D9 33 F6",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn find_streamed_script_quiet(this: GPtr<u8>, name: GPtr<u8>) -> i32 {
    let mut i = 0i32;
    while i < i32::from(at::<u16>(this, 0xA44).read()) {
        if stricmp(this.byte_add(8 + 0x20 * i as isize), name) == 0 {
            return i;
        }
        i += 1;
    }
    -1
}

/// `CScripted2dEffects::AddScripted2DEffect(float radius)` (0x6FA7C0, cdecl): the first free of the 64 slots (used byte
/// 0xC3A1A0 + i) is claimed: used = 1, `ms_userLists`-style dword 0xC3A020 + 4i = -1, the 36-byte user list at 0xC3A200
/// + 36i gets its eight dwords = -1 and +0x20 = 0, the effect pair 0xC3BB00 + 0x94 i is reset (0x6F9EB0), the byte
/// 0xC39FE0 + i = 0 and the radius (fld / fstp) at 0xC39EE0 + 4i; returns the slot. No free slot → -1.
#[hook(addr = 0x6FA7C0, conv = cdecl, name = "CScripted2dEffects::AddScripted2DEffect", expect = "83 CA FF 56 8B C2",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn add_scripted_2d_effect(radius: u32) -> i32 {
    for i in 0..0x40usize {
        let used = GPtr::<u8>::from_addr(0xC3A1A0 + i);
        if used.read() != 0 {
            continue;
        }
        used.write(1);
        GPtr::<i32>::from_addr(0xC3A020 + 4 * i).write(-1);
        let list = GPtr::<u8>::from_addr(0xC3A200 + 36 * i);
        for k in 0..4isize {
            at::<i32>(list, 4 * k).write(-1);
            at::<i32>(list, 0x10 + 4 * k).write(-1);
        }
        at::<u8>(list, 0x20).write(0);
        effect_2d_init(GPtr::from_addr(0xC3BB00 + 0x94 * i));
        GPtr::<u8>::from_addr(0xC39FE0 + i).write(0);
        GPtr::<u32>::from_addr(0xC39EE0 + 4 * i).write(sa_math::fpu::fld_fst(f32::from_bits(radius)).to_bits());
        return i as i32;
    }
    -1
}

/// `CScripted2dEffects::GetEffectPairs(const C2dEffect* effect)` (0x6FA840, cdecl): the effect's index among the 64
/// 0x40-byte effects at 0xC3AB00 selects the pair `0xC3BB00 + 0x94 i`; an unknown effect gives index -1 (0xC3BA6C).
#[hook(addr = 0x6FA840, conv = cdecl, name = "CScripted2dEffects::GetEffectPairs", expect = "56 8B 74 24 08 83 C8 FF",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn get_effect_pairs(effect: GPtr<u8>) -> GPtr<u8> {
    let i = (0..0x40u32).find(|i| effect.addr() as u32 == 0xC3AB00 + 0x40 * i).map_or(-1, |i| i as i32);
    GPtr::from_addr((i as u32).wrapping_mul(0x94).wrapping_add(0xC3BB00) as usize)
}

/// `CTheScripts::AddScriptSphere(uint32 id, CVector pos, float radius)` (stub 0x483B30 → .HOODLUM 0x156E050, cdecl): the
/// first free of the 0x18-byte spheres at 0xA91268 (slot 0 tested first, then slots 1..16 by their used byte; a full
/// array ends on index 16, as the original) gets used = 1, id (+4) = `id + index`, the position (+8) and radius
/// (+0x14) as raw dwords; returns `GetNewUniqueScriptThingIndex(index, 0)`.
#[hook(addr = 0x483B30, conv = cdecl, name = "CTheScripts::AddScriptSphere", expect = "E9 1B A5 0E 01",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn add_script_sphere(id: u32, x: u32, y: u32, z: u32, radius: u32) -> i32 {
    const SPHERES: usize = 0xA91268;
    let mut index = 0u32;
    if GPtr::<u8>::from_addr(SPHERES).read() != 0 {
        let mut off = 0u32;
        loop {
            if off >= 0x180 {
                break;
            }
            let used = GPtr::<u8>::from_addr(SPHERES + 0x18 + off as usize).read();
            off += 0x18;
            index += 1;
            if used == 0 {
                break;
            }
        }
    }
    let e = GPtr::<u8>::from_addr(SPHERES + 0x18 * index as usize);
    at::<u32>(e, 4).write(id.wrapping_add(index));
    at::<u32>(e, 8).write(x);
    at::<u32>(e, 0xC).write(y);
    at::<u32>(e, 0x10).write(z);
    at::<u8>(e, 0).write(1);
    at::<u32>(e, 0x14).write(radius);
    get_new_unique_script_thing_index(index as i32, 0)
}

/// `CTheScripts::IsEntityWithinSearchLight(int32 handle, CEntity* entity)` (stub 0x493900 → .HOODLUM 0x156C000, cdecl):
/// the search light's index (`GetActualScriptThingIndex(handle, 2)`); a valid one → `IsPointWithinSearchLight(&pos,
/// index)` on the entity's position (a copy on the stack). AL.
#[hook(addr = 0x493900, conv = cdecl, name = "CTheScripts::IsEntityWithinSearchLight", expect = "E9 FB 86 0D 01",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn is_entity_within_search_light(handle: u32, entity: GPtr<u8>) -> u8 {
    let index = get_actual_script_thing_index(handle, 2);
    if index < 0 {
        return 0;
    }
    let p = sa_world::world::common::entity_pos(entity);
    let mut pos = [at::<u32>(p, 0).read(), at::<u32>(p, 4).read(), at::<u32>(p, 8).read()];
    u8::from(is_point_within_search_light(GPtr::from_raw(pos.as_mut_ptr()).cast(), index) != 0)
}

use crate::bindings::get_new_unique_script_thing_index;
use sa_core::bindings::stricmp;
use sa_render::bindings::effect_2d_init;
use sa_world::bindings::get_actual_script_thing_index;
sa_core::orig! {
    /// `CTheScripts::IsPointWithinSearchLight(const CVector& point, int32 index)` (0x493280, cdecl, AL)
    fn is_point_within_search_light(point: GPtr<u8>, index: i32) -> u8 = cdecl 0x493280 => unported;
}

/// The patrol-point anim record constructor (stub 0x46A340 → .HOODLUM 0x156E1D0, thiscall, `ret 8`; unnamed): `strcpy`
/// of the anim name to +0 and of the block name to +0x18 (byte loops, the terminator included).
#[hook(addr = 0x46A340, conv = thiscall, name = "thunk_FUN_0156e1d0", expect = "E9 8B 3E 10 01",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn patrol_anim_record_ctor(this: GPtr<u8>, anim: GPtr<u8>, block: GPtr<u8>) {
    for (dst, src) in [(this, anim), (this.byte_add(0x18), block)] {
        let mut k = 0isize;
        loop {
            let b = at::<u8>(src, k).read();
            at::<u8>(dst, k).write(b);
            k += 1;
            if b == 0 {
                break;
            }
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PATROL_ANIM_RECORD_CTOR,
    HOOK_FIND_STREAMED_SCRIPT_QUIET,
    HOOK_ADD_SCRIPTED_2D_EFFECT,
    HOOK_GET_EFFECT_PAIRS,
    HOOK_ADD_SCRIPT_SPHERE,
    HOOK_IS_ENTITY_WITHIN_SEARCH_LIGHT,
    HOOK_THIS_IS_A_VALID_RANDOM_PED,
    HOOK_SWITCH_ALL_OBJECT_BRAINS_WITH_THIS_ID,
    HOOK_SCRIPT_ATTRACTOR_ADD_PED_TYPE,
];
