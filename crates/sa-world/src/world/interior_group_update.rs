//! PORT-VEH-3 (batch `world3-2`, SA:MP-executed rows): `InteriorGroup_c` update / setup / ped
//! upkeep / exit, `BreakObject_c::Exit`, `Interior_c::Shop_AddShelfInfo` and three static
//! initialisers (`CReplay::PlayerInfo`, `gMatrixList`, the furniture sub-group store). Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, wr, xi};

orig! {
    /// `InteriorGroup_c::CalcIsVisible()` (0x595200)
    fn ig_calc_is_visible(ig: GPtr<u8>) = thiscall 0x595200 => unported;
    /// `InteriorGroup_c::SetupPaths()` (0x595590)
    fn ig_setup_paths(ig: GPtr<u8>) = thiscall 0x595590 => unported;
    /// `InteriorGroup_c::SetupPeds()` (0x596890)
    fn ig_setup_peds(ig: GPtr<u8>) = thiscall 0x596890 => unported;
    /// `InteriorGroup_c::UpdatePeds()` (0x596830)
    fn ig_update_peds(ig: GPtr<u8>) = thiscall 0x596830 => interior_group_update_peds;
    /// `InteriorGroup_c::ReferenceAnims()` (0x5950D0)
    fn ig_reference_anims(ig: GPtr<u8>) = thiscall 0x5950D0 => unported;
    /// `InteriorGroup_c::DereferenceAnims()` (0x595160)
    fn ig_dereference_anims(ig: GPtr<u8>) = thiscall 0x595160 => unported;
    /// `InteriorGroup_c::ContainsInteriorType(int32)` (0x595250)
    fn ig_contains_interior_type(ig: GPtr<u8>, t: i32) -> u8 = thiscall 0x595250 => unported;
    /// `InteriorGroup_c::RemovePed(CPed*)` (0x594E30)
    fn ig_remove_ped(ig: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x594E30 => unported;
    /// `InteriorGroup_c::UpdateOfficePeds()` (0x594E90)
    fn ig_update_office_peds(ig: GPtr<u8>) = thiscall 0x594E90 => unported;
    /// `CPed::IsPointerValid()` (0x5E4220)
    fn ped_is_pointer_valid(p: GPtr<u8>) -> u8 = thiscall 0x5E4220 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20)
    fn population_remove_ped(p: GPtr<u8>) = cdecl 0x610F20 => unported;
    /// `Interior_c::Exit()` (0x592230)
    fn interior_exit(i: GPtr<u8>) = thiscall 0x592230 => unported;
    /// `InteriorManager_c::ReturnInteriorToPool(Interior_c*)` (0x5984B0)
    fn interior_manager_return_interior_to_pool(m: GPtr<u8>, i: GPtr<u8>) = thiscall 0x5984B0 => unported;
    /// `CPathFind::RemoveInterior(uint32)` (0x44E1A0)
    fn path_find_remove_interior(pf: GPtr<u8>, id: i32) = thiscall 0x44E1A0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// CRT `free` (0x8214BD)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `Interior_c::AddInteriorInfo(int32, float, float, int32, CEntity*)` (0x591E40)
    fn interior_add_interior_info(i: GPtr<u8>, kind: i32, x: f32, y: f32, dir: u32, ent: u32) = thiscall 0x591E40 => unported;
    /// `CPlayerInfo::CPlayerInfo()` (0x571920)
    fn player_info_ctor(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x571920 => unported;
    /// `0x54F440`: the `CMatrixLinkList` constructor
    fn matrix_link_list_ctor(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x54F440 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `_atexit` (0x821D1E)
    fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
}

/// `InteriorGroup_c::Update()` (0x5968E0): visibility; paths set up until +0xB8 is set; peds set up
/// until +0xB9 is set (only once the paths are); peds updated once +0xB9 is set; then
/// `ReferenceAnims` (a tail call).
#[hook(addr = 0x5968E0, conv = thiscall, name = "InteriorGroup_c::Update", this = "InteriorGroup_c",
       expect = "56 8B F1 E8 18 E9 FF FF",
       unverified = "world3-2", route = "L2", hazards = "heap-writes",
       note = "the path/ped setup creates peds and path nodes")]
pub fn interior_group_update(this: GPtr<u8>) {
    ig_calc_is_visible(this);
    let mut peds = true;
    if rd::<u8>(this, 0xB8) == 0 {
        ig_setup_paths(this);
        if rd::<u8>(this, 0xB8) == 0 {
            peds = false;
        }
    }
    if peds && rd::<u8>(this, 0xB9) == 0 {
        ig_setup_peds(this);
    }
    if rd::<u8>(this, 0xB9) != 0 {
        ig_update_peds(this);
    }
    ig_reference_anims(this);
}

/// `InteriorGroup_c::Setup()` (0x595320): the group type +0xE: 0 with a type-2 interior, 1 with a
/// type 0 or 6, else 2 with a type 1, else -1; then `ReferenceAnims` (a tail call).
#[hook(addr = 0x595320, conv = thiscall, name = "InteriorGroup_c::Setup", this = "InteriorGroup_c",
       expect = "56 6A 02 8B F1",
       unverified = "world3-2", route = "L2", hazards = "none")]
pub fn interior_group_setup(this: GPtr<u8>) {
    let t = if ig_contains_interior_type(this, 2) != 0 {
        0
    } else if ig_contains_interior_type(this, 0) != 0 || ig_contains_interior_type(this, 6) != 0 {
        1
    } else if ig_contains_interior_type(this, 1) != 0 {
        2
    } else {
        0xFF
    };
    wr::<u8>(this, 0xE, t);
    ig_reference_anims(this);
}

/// `InteriorGroup_c::UpdatePeds()` (0x596830): with an entry-exit (+0x30) and the flag byte 0xBB3DC2
/// set, every ped slot (+0x78, 16) is emptied (valid peds removed from the group); a group of type 2
/// then updates its office peds (a tail call).
#[hook(addr = 0x596830, conv = thiscall, name = "InteriorGroup_c::UpdatePeds", this = "InteriorGroup_c",
       expect = "57 8B F9 8B 47 30",
       unverified = "world3-2", route = "L2", hazards = "heap-writes")]
pub fn interior_group_update_peds(this: GPtr<u8>) {
    if rd::<u32>(this, 0x30) == 0 || g::<u8>(0xBB_3DC2).read() == 0 {
        return;
    }
    for k in 0..16isize {
        let o = 0x78 + 4 * k;
        let p = rd::<GPtr<u8>>(this, o);
        if p.is_null() {
            continue;
        }
        if ped_is_pointer_valid(p) != 0 {
            ig_remove_ped(this, rd::<GPtr<u8>>(this, o));
        }
        wr::<u32>(this, o, 0);
    }
    if rd::<u8>(this, 0xE) == 2 {
        ig_update_office_peds(this);
    }
}

/// `InteriorGroup_c::Exit()` (0x595290): every interior (+0x10, 8) exits and goes back to the pool
/// (`g_interiorMan` 0xBAF670); every ped (+0x38, 16): a dead or invalid one clears its slot flag
/// (0xBB3D9C + k), a valid one is removed from the population, the slot cleared; +0x36 = 0; the
/// interior's paths removed (`ThePaths`, id +0xC); then `DereferenceAnims` (a tail call).
#[hook(addr = 0x595290, conv = thiscall, name = "InteriorGroup_c::Exit", this = "InteriorGroup_c",
       expect = "53 55 56 8B E9",
       unverified = "world3-2", route = "L2", hazards = "heap-writes",
       note = "removes peds (pools) and path nodes")]
pub fn interior_group_exit(this: GPtr<u8>) {
    for k in 0..8isize {
        let o = 0x10 + 4 * k;
        if rd::<u32>(this, o) == 0 {
            continue;
        }
        interior_exit(rd::<GPtr<u8>>(this, o));
        interior_manager_return_interior_to_pool(GPtr::from_addr(0xBA_F670), rd::<GPtr<u8>>(this, o));
    }
    for k in 0..16usize {
        let o = 0x38 + 4 * k as isize;
        let p = rd::<GPtr<u8>>(this, o);
        if p.is_null() {
            continue;
        }
        if ped_is_pointer_valid(p) != 0 {
            if ped_is_alive(rd::<GPtr<u8>>(this, o)) == 0 {
                g::<u8>(0xBB_3D9C + k).write(0);
            }
            population_remove_ped(rd::<GPtr<u8>>(this, o));
        } else {
            g::<u8>(0xBB_3D9C + k).write(0);
        }
        wr::<u32>(this, o, 0);
    }
    wr::<u8>(this, 0x36, 0);
    path_find_remove_interior(GPtr::from_addr(0x96_F050), i32::from(rd::<i8>(this, 0xC)));
    ig_dereference_anims(this);
}

/// `BreakObject_c::Exit()` (0x59DDD0): with a group array (+8): per group (+4 of them, 0x74 each)
/// the texture (+0x54) destroyed and cleared and the vertex buffer (+0x50) freed; the array freed;
/// the active byte +1 cleared in every case.
#[hook(addr = 0x59DDD0, conv = thiscall, name = "BreakObject_c::Exit", this = "BreakObject_c",
       expect = "53 56 8B F1 8B 46 08",
       unverified = "world3-2", route = "L2", hazards = "heap-writes",
       note = "frees heap memory and RW textures")]
pub fn break_object_exit(this: GPtr<u8>) {
    if rd::<u32>(this, 8) != 0 {
        let mut k = 0i32;
        while k < rd::<i32>(this, 4) {
            let o = (k as isize) * 0x74;
            let tex = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), o + 0x54);
            if !tex.is_null() {
                rw_texture_destroy(tex);
                wr::<u32>(rd::<GPtr<u8>>(this, 8), o + 0x54, 0);
            }
            let buf = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), o + 0x50);
            if !buf.is_null() {
                crt_free(buf);
            }
            k += 1;
        }
        crt_free(rd::<GPtr<u8>>(this, 8));
    }
    wr::<u8>(this, 1, 0);
}

/// `Interior_c::Shop_AddShelfInfo(int32 x, int32 y, int32 dir)` (0x59A140): after more than one
/// shelf since the last one (counter 0x8D0948) a 40% chance (`ftol(rand frac * 100) > 60`) adds
/// interior info 8 at (x, y) and resets the counter to 1; otherwise the counter is incremented.
#[hook(addr = 0x59A140, conv = thiscall, name = "Interior_c::Shop_AddShelfInfo", this = "Interior_c",
       expect = "51 83 3D 48 09 8D 00 01",
       unverified = "world3-2", route = "L2", hazards = "heap-writes",
       note = "rand site 0x59A152; AddInteriorInfo writes the interior's info list")]
pub fn interior_shop_add_shelf_info(this: GPtr<u8>, px: i32, py: i32, dir: u32) {
    let cnt = g::<i32>(0x8D_0948);
    if cnt.read() > 1 {
        let r = sa_core::orig::at_site(0x59_A152, sa_core::crt::rand);
        let v = ftol32(xi(r & 0xFFFF).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(100.0));
        if v > 0x3C {
            interior_add_interior_info(this, 8, xi(px).to_f32(), xi(py).to_f32(), dir, 0);
            cnt.write(1);
            return;
        }
    }
    cnt.write(cnt.read().wrapping_add(1));
}

/// `__init_CReplay__PlayerInfo` (0x84A540): `CPlayerInfo` constructor on 0xA430B0, its destructor
/// registered with `atexit` (0x856160).
#[hook(addr = 0x84A540, conv = cdecl, name = "__init_CReplay__PlayerInfo",
       expect = "B9 B0 30 A4 00",
       unverified = "world3-2", route = "split", hazards = "none",
       note = "one-time static initialiser")]
pub fn init_replay_player_info() {
    player_info_ctor(GPtr::from_addr(0xA4_30B0));
    atexit(0x0085_6160);
}

/// `__init_gMatrixList` (0x84E300): the `CMatrixLinkList` constructor on 0xB74288, its destructor
/// registered with `atexit` (0x856490).
#[hook(addr = 0x84E300, conv = cdecl, name = "__init_gMatrixList",
       expect = "B9 88 42 B7 00",
       unverified = "world3-2", route = "split", hazards = "none",
       note = "one-time static initialiser")]
pub fn init_matrix_list() {
    matrix_link_list_ctor(GPtr::from_addr(0xB7_4288));
    atexit(0x0085_6490);
}

/// `__init_FurnitureManager_c__g_subGroupStore` (0x84F320): 128 `FurnitureSubGroup_c` (0x1C) at
/// 0xBAD3F8 through the CRT vector constructor, the array destructor registered with `atexit`
/// (0x856600).
#[hook(addr = 0x84F320, conv = cdecl, name = "__init_FurnitureManager_c__g_subGroupStore",
       expect = "68 70 0E 59 00",
       unverified = "world3-2", route = "split", hazards = "none",
       note = "one-time static initialiser")]
pub fn init_furniture_sub_group_store() {
    eh_vector_ctor(GPtr::from_addr(0xBA_D3F8), 0x1C, 0x80, 0x0059_0E20, 0x0059_0E70);
    atexit(0x0085_6600);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTERIOR_GROUP_UPDATE,
    HOOK_INTERIOR_GROUP_SETUP,
    HOOK_INTERIOR_GROUP_UPDATE_PEDS,
    HOOK_INTERIOR_GROUP_EXIT,
    HOOK_BREAK_OBJECT_EXIT,
    HOOK_INTERIOR_SHOP_ADD_SHELF_INFO,
    HOOK_INIT_REPLAY_PLAYER_INFO,
    HOOK_INIT_MATRIX_LIST,
    HOOK_INIT_FURNITURE_SUB_GROUP_STORE,
];
