//! `CFire`, `CFireManager`, `CCreepingFire` (PORT-WEAPON-PED). Ported from the 1.0 US bytes
//! (gta-reversed `Fire.cpp`/`FireManager.cpp`/`CreepingFire.cpp` and `docs/re/specs/WEAPON/CFire*.md`
//! as reading aids).
//!
//! `CFire` (0x28): +0 flags (0x01 active, 0x02 created by script, 0x04 makes noise, 0x08 being
//! extinguished, 0x10 first generation), +2 script reference (int16), +4 position, +0x10 entity on
//! fire, +0x14 entity that started it, +0x18 time to burn (ms), +0x1C strength (f32), +0x20
//! generations allowed, +0x21 removal distance, +0x24 `FxSystem_c*`. `CFireManager` (0x964): 60 fires,
//! +0x960 max fires. `gFireManager` at 0xB71F80. `CCreepingFire::m_aFireStatus[32][32]` at 0xB71B68.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_world::physical::shared::{clean_up_old_reference, register_reference};

use crate::common::*;

pub const FIRE_SIZE: usize = 0x28;
pub const NUM_FIRES: usize = 60;
pub const FIRE_MANAGER_SIZE: usize = 0x964;

const _: () = assert!(FIRE_SIZE == sa_core::layouts::size::CFire);
const _: () = assert!(FIRE_MANAGER_SIZE == sa_core::layouts::size::CFireManager);

/// `gFireManager`
pub const FIRE_MANAGER: usize = 0xB71F80;
/// `CFireManager::m_nMaxFireGenerationsAllowed` (int32)
const MAX_FIRE_GENERATIONS: usize = 0xB728E0;
/// `g_fxMan` (`FxManager_c`)
const FX_MAN: usize = 0xA9AE80;

sa_core::globals! {
    list GLOBALS;
    /// `CCreepingFire::m_aFireStatus[32][32]`
    pub static CREEPING_FIRE_STATUS @ 0xB71B68: [u8; 0x400] = [0; 0x400];
}

/// 0x863A3C: FLT_MAX
const K_FLT_MAX: u32 = 0x7F7F_FFFF;
/// 0x863A40: 0.3 (bits 0x3E999998)
const K_863A40: u32 = 0x3E99_9998;
/// 0x858C7C: 1/32767
const K_RAND: u32 = 0x3800_0100;
/// 0x858624: 1.0
const K_ONE: u32 = 0x3F80_0000;
/// 0x858CA0: 2.0
const K_TWO: u32 = 0x4000_0000;
/// 0x858C54: 2^32
const K_2P32: u32 = 0x4F80_0000;
/// 0x858B38: 0.02
const K_858B38: u32 = 0x3CA3_D70A;
/// 0x858C58: 0.01
const K_858C58: u32 = 0x3C23_D70A;
/// 0x858B4C: 0.005
const K_858B4C: u32 = 0x3BA3_D70A;

orig! {
    /// `CFire::Extinguish()` (0x5393F0)
    pub fn extinguish_call(fire: GPtr<u8>) = thiscall 0x5393F0 => extinguish;
    /// `CFire::CreateFxSysForStrength(const CVector&, RwMatrix*)` (0x539360)
    pub fn create_fx_sys_for_strength_call(fire: GPtr<u8>, pos: GPtr<u8>, m: GPtr<u8>) = thiscall 0x539360 => create_fx_sys_for_strength;
    /// `CFire::Initialise()` (0x538B30)
    pub fn initialise_call(fire: GPtr<u8>) = thiscall 0x538B30 => initialise;
    /// `CFireManager::Init()` (0x538BC0)
    pub fn manager_init_call(this: GPtr<u8>) = thiscall 0x538BC0 => manager_init;
    /// `CFireManager::GetNextFreeFire(bool)` (0x539E50)
    pub fn get_next_free_fire_call(this: GPtr<u8>, allow_script: u8) -> GPtr<u8> = thiscall 0x539E50 => get_next_free_fire;
    /// `CFireManager::StartFire(CVector, float, uint8, CEntity*, uint32, int8, uint8)` (0x539F00)
    pub fn start_fire_no_target_call(this: GPtr<u8>, x: f32, y: f32, z: f32, size: f32, unk: u8, creator: GPtr<u8>, time: u32, gens: i8, unk2: u8) -> GPtr<u8> = thiscall 0x539F00 => start_fire_no_target;
    /// `FxManager_c::CreateFxSystem(const char*, CVector*, RwMatrix*, bool)` (0x4A9BE0, thiscall)
    pub fn create_fx_system(man: GPtr<u8>, name: GPtr<u8>, pos: GPtr<u8>, m: GPtr<u8>, flag: u8) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810, thiscall)
    pub fn destroy_fx_system(man: GPtr<u8>, fx: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(float, float, float, float*, float*, float*)` (0x6E8580)
    pub fn get_water_level_no_waves(x: f32, y: f32, z: f32, out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> u8 = cdecl 0x6E8580 => unported;
    /// `CDamageManager::GetEngineStatus()` (0x6C22C0, thiscall)
    pub fn get_engine_status(dm: GPtr<u8>) -> u32 = thiscall 0x6C22C0 => unported;
    /// `CDamageManager::SetEngineStatus(uint32)` (0x6C22A0, thiscall)
    pub fn set_engine_status(dm: GPtr<u8>, s: u32) = thiscall 0x6C22A0 => unported;
    /// `CPed::IsPedInControl()` (0x5E3960, thiscall)
    pub fn ped_is_ped_in_control(ped: GPtr<u8>) -> u8 = thiscall 0x5E3960 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0, thiscall)
    pub fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CCrime::ReportCrime(eCrimeType, CEntity* victim, CPed* commitedBy)` (0x532010)
    pub fn report_crime(ty: i32, victim: GPtr<u8>, by: GPtr<u8>) = cdecl 0x532010 => unported;
    /// `CTheScripts::GetNewUniqueScriptThingIndex(int32 index, uint8 type)` (0x483720)
    pub fn get_new_unique_script_thing_index(i: i32, ty: u8) -> i32 = cdecl 0x483720 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float r, g, b, a, size, spin, life)` (0x4AB290, thiscall)
    pub fn fx_prt_mult(this: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(CVector*, CVector*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    pub fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, t: f32, mult: GPtr<u8>, a: f32, b: f32, c: f32, d: u8) = thiscall 0x4AA440 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool, bool, bool, bool, bool, bool, CStoredCollPoly*)` (0x5674E0)
    pub fn process_vertical_line(p: GPtr<u8>, z: f32, cp: GPtr<u8>, ent: GPtr<GPtr<u8>>, b: u8, v: u8, pd: u8, o: u8, d: u8, s: u8, poly: GPtr<u8>) -> u8 = cdecl 0x5674E0 => unported;
}

/// `CFireManager::m_aFires[i]` of the manager at `this`.
#[inline]
pub fn fire(this: GPtr<u8>, i: usize) -> GPtr<u8> {
    this.byte_add((i * FIRE_SIZE) as isize)
}

/// `(float)(uint32)v` as MSVC's `fild` + `fadd 2^32` for negative ints.
#[inline]
fn u32_to_x87(v: u32) -> X87 {
    let f = xi(v as i32);
    if (v as i32) < 0 { f.fadd_m(cf(K_2P32)) } else { f }
}

/// `CFire::Initialise()` (0x538B30, thiscall): position 0 (copied from zeroed stack temporaries),
/// flags `& 0xF4 | 0x14`, reference 1, entities 0, time 0, strength 1.0, generations 100, removal
/// distance 60, fx null. ECX is left unchanged (`CFire::Constructor` returns it as `this`).
#[hook(addr = 0x538B30, conv = thiscall, name = "CFire::Initialise", expect = "83 EC 0C C7 44 24 00 00 00 00 00", this = "CFire", preserve = "ecx")]
pub fn initialise(this: GPtr<u8>) {
    wu(this, 4, 0);
    wu(this, 8, 0);
    wu(this, 0xC, 0);
    let fl = (rb(this, 0) & 0xF4) | 0x14;
    wh(this, 2, 1);
    wu(this, 0x18, 0);
    wu(this, 0x10, 0);
    wu(this, 0x14, 0);
    wu(this, 0x1C, K_ONE);
    wb(this, 0, fl);
    wu(this, 0x24, 0);
    wb(this, 0x20, 100);
    wb(this, 0x21, 60);
}

/// `CFire::Constructor()` (0x539D90, thiscall): `Initialise()`, returns `this` (ECX after the call).
#[hook(addr = 0x539D90, conv = thiscall, name = "CFire::Constructor", expect = "E8 9B ED FF FF", this = "CFire")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    initialise_call(this);
    this
}

/// The `Extinguish` body the original inlines (into `RemoveAllScriptFires`, `Shutdown`,
/// `GetNextFreeFire`) and has as a function (0x5393F0), for an active fire with the flags `fl` it
/// has already read: time 0, flags `& 0xF6 | 0x10`, kill the fx, unlink the entity on fire.
fn extinguish_body(f: GPtr<u8>, fl: u8) {
    wu(f, 0x18, 0);
    wb(f, 0, (fl & 0xF6) | 0x10);
    let fx = rp(f, 0x24);
    if !fx.is_null() {
        fx_kill(fx);
        wp(f, 0x24, GPtr::NULL);
    }
    let e = rp(f, 0x10);
    if e.is_null() {
        return;
    }
    match etype(e) {
        3 => wu(e, 0x730, 0),
        2 => wu(e, 0x490, 0),
        _ => {}
    }
    let e = rp(f, 0x10);
    if !e.is_null() {
        clean_up_old_reference(e, f.byte_add(0x10).cast());
    }
    wp(f, 0x10, GPtr::NULL);
}

/// `CFire::Extinguish()` (0x5393F0, thiscall): only for an active fire (see [`extinguish_body`]).
#[hook(addr = 0x5393F0, conv = thiscall, name = "CFire::Extinguish", expect = "56 8B F1 8A 06", this = "CFire")]
pub fn extinguish(this: GPtr<u8>) {
    let fl = rb(this, 0);
    if fl & 1 != 0 {
        extinguish_body(this, fl);
    }
}

/// `CFire::CreateFxSysForStrength(const CVector& pos, RwMatrix* m)` (0x539360, thiscall `ret 8`; split
/// chunk 0x405B72): kills an existing fx (field cleared), then `g_fxMan.CreateFxSystem("fire" /
/// "fire_med" / "fire_large", pos, m, true)` for strength <= 1 / <= 2 / above (NaN: "fire_large"),
/// and `Play` it.
#[hook(addr = 0x539360, conv = thiscall, name = "CFire::CreateFxSysForStrength", expect = "56 8B F1 8B 4E 24", this = "CFire")]
pub fn create_fx_sys_for_strength(this: GPtr<u8>, pos: GPtr<u8>, m: GPtr<u8>) {
    let fx = rp(this, 0x24);
    if !fx.is_null() {
        fx_kill(fx);
        wu(this, 0x24, 0);
    }
    let s = rf(this, 0x1C);
    let mut f = Fpu::new();
    f.fld(s);
    f.fcomp_m(cf(K_ONE));
    let name = if !f.jp(0x41) {
        0x85AA8C
    } else {
        f.fld(s);
        f.fcomp_m(cf(K_TWO));
        if !f.jp(0x41) { 0x85AA80 } else { 0x85AA74 }
    };
    let fx = create_fx_system(g(FX_MAN), g(name), pos, m, 1);
    wp(this, 0x24, fx);
    if !fx.is_null() {
        fx_play(fx);
    }
}

/// `CFireManager::Init()` (0x538BC0, thiscall): every fire as `CFire::Initialise` (inlined; position
/// from zeroed temporaries), then `m_nMaxFires = 999999`.
#[hook(addr = 0x538BC0, conv = thiscall, name = "CFireManager::Init", expect = "83 EC 14 53 55", this = "CFireManager")]
pub fn manager_init(this: GPtr<u8>) {
    for i in 0..NUM_FIRES {
        let f = fire(this, i);
        wu(f, 4, 0);
        wu(f, 8, 0);
        wu(f, 0xC, 0);
        wb(f, 0, (rb(f, 0) & 0xF4) | 0x14);
        wh(f, 2, 1);
        wu(f, 0x18, 0);
        wu(f, 0x10, 0);
        wu(f, 0x14, 0);
        wu(f, 0x1C, K_ONE);
        wu(f, 0x24, 0);
        wb(f, 0x20, 100);
        wb(f, 0x21, 60);
    }
    wu(this, 0x960, 999_999);
}

/// `CFireManager::Destructor()` (0x538BB0, thiscall): `eh vector destructor iterator(this, 0x28, 60,
/// CFire::~CFire 0x538BA0)`.
#[hook(addr = 0x538BB0, conv = thiscall, name = "CFireManager::Destructor", expect = "68 A0 8B 53 00", this = "CFireManager")]
pub fn manager_destructor(this: GPtr<u8>) {
    eh_vector_dtor(this, 0x28, 0x3C, 0x538BA0);
}

/// `CFireManager::Constructor()` (0x539DA0, thiscall): the 60 `CFire` constructors through the CRT
/// iterator, then `Init()`; returns `this`.
#[hook(addr = 0x539DA0, conv = thiscall, name = "CFireManager::Constructor", expect = "56 68 A0 8B 53 00", this = "CFireManager")]
pub fn manager_constructor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 0x28, 0x3C, 0x539D90, 0x538BA0);
    manager_init_call(this);
    this
}

/// `CFireManager::GetNumOfNonScriptFires()` (0x538F10, thiscall): active and not script.
#[hook(addr = 0x538F10, conv = thiscall, name = "CFireManager::GetNumOfNonScriptFires", expect = "56 33 C0 8B D1", this = "CFireManager")]
pub fn get_num_of_non_script_fires(this: GPtr<u8>) -> u32 {
    (0..NUM_FIRES).filter(|&i| rb(fire(this, i), 0) & 3 == 1).count() as u32
}

/// `CFireManager::FindNearestFire(const CVector& pos, bool checkExtinguished, bool checkScript)`
/// (0x538F40, thiscall `ret 0xC`): the active fire (skipping script fires when `check_script`, fires
/// being extinguished when `check_extinguished`, and fires on peds) with the smallest 2D distance
/// squared, strictly below the previous best (starting at FLT_MAX), or null.
#[hook(addr = 0x538F40, conv = thiscall, name = "CFireManager::FindNearestFire", expect = "51 D9 05 3C 3A 86 00", this = "CFireManager")]
pub fn find_nearest_fire(this: GPtr<u8>, pos: GPtr<f32>, check_extinguished: u8, check_script: u8) -> GPtr<u8> {
    let mut f = Fpu::new();
    f.fld(cf(K_FLT_MAX));
    let mut best = GPtr::NULL;
    let (px, py) = (pos.read(), pos.wrapping_index(1).read());
    for i in 0..NUM_FIRES {
        let fi = fire(this, i);
        let fl = rb(fi, 0);
        if fl & 1 == 0 || (check_script != 0 && fl & 2 != 0) || (check_extinguished != 0 && fl & 8 != 0) {
            continue;
        }
        let e = rp(fi, 0x10);
        if !e.is_null() && etype(e) == 3 {
            continue;
        }
        f.fld(rf(fi, 4));
        f.fsub_m(px);
        f.fld(rf(fi, 8));
        f.fsub_m(py);
        f.fld_st(1);
        f.fmulp(2);
        f.fld_st(0);
        f.fmul_st(1);
        f.faddp(2);
        f.fxch(1);
        let tmp = f.fst();
        f.fstp_st(1);
        f.fcomp_st(1);
        if !f.jp(5) {
            f.fstp_st(0);
            best = fi;
            f.fld(tmp);
        }
    }
    best
}

/// `CFireManager::PlentyFiresAvailable()` (0x539340, thiscall): at least 6 inactive fires.
// not registered: a loop branches back into the 5 patched bytes (0x539344)
pub fn plenty_fires_available(this: GPtr<u8>) -> u8 {
    let mut n = 0;
    for i in 0..NUM_FIRES {
        if rb(fire(this, i), 0) & 1 == 0 {
            n += 1;
            if n >= 6 {
                return 1;
            }
        }
    }
    0
}

/// `CFireManager::IsScriptFireExtinguished(int16 id)` (0x5396E0, thiscall `ret 4`): `!active`.
#[hook(addr = 0x5396E0, conv = thiscall, name = "CFireManager::IsScriptFireExtinguished", expect = "0F BF 44 24 04", this = "CFireManager")]
pub fn is_script_fire_extinguished(this: GPtr<u8>, id: i16) -> u8 {
    !rb(fire_at(this, id), 0) & 1
}

/// `m_aFires[(int16)id]` (signed index, no bounds check).
fn fire_at(this: GPtr<u8>, id: i16) -> GPtr<u8> {
    this.byte_add(isize::from(id) * FIRE_SIZE as isize)
}

/// `CFireManager::ClearAllScriptFireFlags()` (0x5397A0, thiscall).
#[hook(addr = 0x5397A0, conv = thiscall, name = "CFireManager::ClearAllScriptFireFlags", expect = "B8 3C 00 00 00", this = "CFireManager")]
pub fn clear_all_script_fire_flags(this: GPtr<u8>) {
    for i in 0..NUM_FIRES {
        let f = fire(this, i);
        wb(f, 0, rb(f, 0) & 0xFD);
    }
}

/// `CFireManager::SetScriptFireAudio(int16 id, bool on)` (0x5397B0, thiscall `ret 8`): flag 0x04.
#[hook(addr = 0x5397B0, conv = thiscall, name = "CFireManager::SetScriptFireAudio", expect = "8A 44 24 08 84 C0", this = "CFireManager")]
pub fn set_script_fire_audio(this: GPtr<u8>, id: i16, on: u8) {
    let f = fire_at(this, id);
    if on != 0 {
        wb(f, 0, rb(f, 0) | 4);
    } else {
        wb(f, 0, rb(f, 0) & 0xFB);
    }
}

/// `CFireManager::GetScriptFireCoords(int16 id)` (0x5397E0, thiscall `ret 4`): `&m_aFires[id].pos`.
#[hook(addr = 0x5397E0, conv = thiscall, name = "CFireManager::GetScriptFireCoords", expect = "0F BF 44 24 04", this = "CFireManager")]
pub fn get_script_fire_coords(this: GPtr<u8>, id: i16) -> GPtr<u8> {
    fire_at(this, id).byte_add(4)
}

/// `CFireManager::GetNumFiresInRange(const CVector& pos, float range)` (0x5397F0, thiscall `ret 8`):
/// active non-script fires with `sqrt(dy*dy + dx*dx) <= range` (dx stored as f32 first).
#[hook(addr = 0x5397F0, conv = thiscall, name = "CFireManager::GetNumFiresInRange", expect = "83 EC 10 56 8B 74 24 18", this = "CFireManager")]
pub fn get_num_fires_in_range(this: GPtr<u8>, pos: GPtr<f32>, range: f32) -> u32 {
    let mut n = 0;
    for i in 0..NUM_FIRES {
        let fi = fire(this, i);
        if rb(fi, 0) & 3 != 1 {
            continue;
        }
        let mut f = Fpu::new();
        f.fld(pos.read());
        f.fld(pos.wrapping_index(1).read());
        f.fld(rf(fi, 4));
        f.fld(rf(fi, 8));
        let fy = f.fstp();
        f.fsub_st(2);
        let dx = f.fstp();
        f.fld(fy);
        f.fsub_st(1);
        f.fstp_st(2);
        f.fstp_st(0);
        f.fld_st(0);
        f.fmulp(1);
        f.fld(dx);
        f.fmul_m(dx);
        f.faddp(1);
        f.fsqrt();
        f.fcomp_m(range);
        if !f.jp(0x41) {
            n += 1;
        }
    }
    n
}

/// `CFireManager::GetNumFiresInArea(minX, minY, minZ, maxX, maxY, maxZ)` (0x539860, thiscall `ret
/// 0x18`): active non-script fires with `min <= pos <= max` per axis (NaN fails).
#[hook(addr = 0x539860, conv = thiscall, name = "CFireManager::GetNumFiresInArea", expect = "56 33 D2 83 C1 08", this = "CFireManager")]
pub fn get_num_fires_in_area(this: GPtr<u8>, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> u32 {
    let mut n = 0;
    for i in 0..NUM_FIRES {
        let fi = fire(this, i);
        if rb(fi, 0) & 3 != 1 {
            continue;
        }
        if in_range(rf(fi, 4), x1, x2) && in_range(rf(fi, 8), y1, y2) && in_range(rf(fi, 0xC), z1, z2) {
            n += 1;
        }
    }
    n
}

/// `fcomp lo; test ah,1; jne skip` then `fcomp hi; test ah,0x41; jp skip`: `lo <= v <= hi`.
pub fn in_range(v: f32, lo: f32, hi: f32) -> bool {
    let mut f = Fpu::new();
    f.fld(v);
    f.fcomp_m(lo);
    if f.jne(1) {
        return false;
    }
    f.fld(v);
    f.fcomp_m(hi);
    !f.jp(0x41)
}

/// `CFireManager::RemoveAllScriptFires()` (0x539720, thiscall): script fires lose the flag; the
/// active ones are extinguished (inlined `Extinguish`).
#[hook(addr = 0x539720, conv = thiscall, name = "CFireManager::RemoveAllScriptFires", expect = "53 56 57 8D 71 10", this = "CFireManager")]
pub fn remove_all_script_fires(this: GPtr<u8>) {
    for i in 0..NUM_FIRES {
        let f = fire(this, i);
        let fl = rb(f, 0);
        if fl & 2 == 0 {
            continue;
        }
        let fl = fl & 0xFD;
        wb(f, 0, fl);
        if fl & 1 != 0 {
            extinguish_body(f, fl);
        }
    }
}

/// `CFireManager::Shutdown()` (0x539DD0, thiscall): every fire loses the script flag; the active ones
/// are extinguished (inlined `Extinguish`).
#[hook(addr = 0x539DD0, conv = thiscall, name = "CFireManager::Shutdown", expect = "55 56 57 8D 71 10", this = "CFireManager")]
pub fn manager_shutdown(this: GPtr<u8>) {
    for i in 0..NUM_FIRES {
        let f = fire(this, i);
        let fl = rb(f, 0) & 0xFD;
        wb(f, 0, fl);
        if fl & 1 != 0 {
            extinguish_body(f, fl);
        }
    }
}

/// `CFireManager::GetNextFreeFire(bool allowScriptFires)` (0x539E50, thiscall `ret 4`): the first fire
/// that is neither active nor script; else (with `allow`) the first script fire that is not first
/// generation... precisely: the first fire without flag 0x02 or without flag 0x10, which loses the
/// script flag and is extinguished (inlined); else null.
#[hook(addr = 0x539E50, conv = thiscall, name = "CFireManager::GetNextFreeFire", expect = "53 33 DB 33 C0", this = "CFireManager")]
pub fn get_next_free_fire(this: GPtr<u8>, allow: u8) -> GPtr<u8> {
    for i in 0..NUM_FIRES {
        if rb(fire(this, i), 0) & 3 == 0 {
            return fire(this, i);
        }
    }
    if allow == 0 {
        return GPtr::NULL;
    }
    for i in 0..NUM_FIRES {
        let f = fire(this, i);
        let fl = rb(f, 0);
        if fl & 2 == 0 || fl & 0x10 == 0 {
            let fl = fl & 0xFD;
            wb(f, 0, fl);
            if fl & 1 != 0 {
                extinguish_body(f, fl);
            }
            return f;
        }
    }
    GPtr::NULL
}

/// `CFireManager::ExtinguishPoint(CVector point, float range)` (0x539450, thiscall `ret 0x10`): every
/// active fire with `range*range > dx*dx + dy*dy + dz*dz` (summed as the bytes: (y²+x²)+z²) loses the
/// script flag and is extinguished (`CFire::Extinguish` call).
#[hook(addr = 0x539450, conv = thiscall, name = "CFireManager::ExtinguishPoint", expect = "56 57 8B F1 BF 3C 00 00 00", this = "CFireManager")]
pub fn extinguish_point(this: GPtr<u8>, px: f32, py: f32, pz: f32, range: f32) {
    for i in 0..NUM_FIRES {
        let fi = fire(this, i);
        let fl = rb(fi, 0);
        if fl & 1 == 0 {
            continue;
        }
        if within(fi, px, py, pz, range) {
            wb(fi, 0, fl & 0xFD);
            extinguish_call(fi);
        }
    }
}

/// The squared-distance test of `ExtinguishPoint` / `ExtinguishPointWithWater`.
fn within(fi: GPtr<u8>, px: f32, py: f32, pz: f32, range: f32) -> bool {
    let mut f = Fpu::new();
    f.fld(pz);
    f.fsub_m(rf(fi, 0xC));
    f.fld(py);
    f.fsub_m(rf(fi, 8));
    f.fld(px);
    f.fsub_m(rf(fi, 4));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld(range);
    f.fmul_m(range);
    f.fcompp();
    !f.jne(0x41)
}

/// `CFireManager::RemoveScriptFire(int16 id)` (0x539700, thiscall `ret 4`): a script fire loses the
/// flag and is extinguished.
#[hook(addr = 0x539700, conv = thiscall, name = "CFireManager::RemoveScriptFire", expect = "0F BF 44 24 04", this = "CFireManager")]
pub fn remove_script_fire(this: GPtr<u8>, id: i16) {
    let f = fire_at(this, id);
    let fl = rb(f, 0);
    if fl & 2 != 0 {
        wb(f, 0, fl & 0xFD);
        extinguish_call(f);
    }
}

/// `CFireManager::DestroyAllFxSystems()` (0x539D10, thiscall).
#[hook(addr = 0x539D10, conv = thiscall, name = "CFireManager::DestroyAllFxSystems", expect = "56 57 8D 71 24", this = "CFireManager")]
pub fn destroy_all_fx_systems(this: GPtr<u8>) {
    for i in 0..NUM_FIRES {
        let f = fire(this, i);
        if rb(f, 0) & 1 == 0 {
            continue;
        }
        let fx = rp(f, 0x24);
        if !fx.is_null() {
            destroy_fx_system(g(FX_MAN), fx);
            wu(f, 0x24, 0);
        }
    }
}

/// `CFireManager::CreateAllFxSystems()` (0x539D50, thiscall): active fires without fx get one.
#[hook(addr = 0x539D50, conv = thiscall, name = "CFireManager::CreateAllFxSystems", expect = "56 57 8B F1 BF 3C 00 00 00", this = "CFireManager")]
pub fn create_all_fx_systems(this: GPtr<u8>) {
    for i in 0..NUM_FIRES {
        let f = fire(this, i);
        if rb(f, 0) & 1 != 0 && rp(f, 0x24).is_null() {
            create_fx_sys_for_strength_call(f, f.byte_add(4), GPtr::NULL);
        }
    }
}

/// `CFireManager::ExtinguishPointWithWater(CVector point, float range, float strength)` (0x5394C0,
/// thiscall `ret 0x14`): for every active fire within range: strength -= `water * 0.02 * timeStep`,
/// two "smoke" particles at the fire with three `rand()` jitters, flag 0x08; strength <= 0 →
/// `Extinguish`; else a new fx when `_ftol(strength)` changed. True if any fire was hit.
#[hook(addr = 0x5394C0, conv = thiscall, name = "CFireManager::ExtinguishPointWithWater", expect = "83 EC 4C 53 55", this = "CFireManager")]
pub fn extinguish_point_with_water(this: GPtr<u8>, px: f32, py: f32, pz: f32, range: f32, water: f32) -> u8 {
    let mut any = false;
    for i in 0..NUM_FIRES {
        let fi = fire(this, i);
        if rb(fi, 0) & 1 == 0 || !within(fi, px, py, pz, range) {
            continue;
        }
        let old = rf(fi, 0x1C);
        let mut pos = rv_bits(fi, 4);
        let s = x(water).fmul_m(cf(K_858B38)).fmul_m(time_step()).fsubr_m(old).to_f32();
        wf(fi, 0x1C, s);
        let r = (sa_core::orig::at_site(0x539559, rand) & 0xFF) - 0x80;
        pos[0] = xi(r).fmul_m(cf(K_858C58)).fadd_m(f32::from_bits(pos[0])).to_f32().to_bits();
        let r = (sa_core::orig::at_site(0x53957E, rand) & 0xFF) - 0x80;
        pos[1] = xi(r).fmul_m(cf(K_858C58)).fadd_m(f32::from_bits(pos[1])).to_f32().to_bits();
        let r = sa_core::orig::at_site(0x5395A3, rand) & 0xFF;
        let mut mult = [0u8; 0x1C];
        let mut vel: [u32; 3] = [0, 0, 0x3F4C_CCCD];
        let mut vel2: [u32; 3] = [0, 0, 0x3FB3_3333];
        pos[2] = xi(r).fmul_m(cf(K_858B4C)).fadd_m(f32::from_bits(pos[2])).to_f32().to_bits();
        fx_prt_mult(
            lp(&mut mult).cast(),
            1.0,
            1.0,
            1.0,
            f32::from_bits(0x3F19_999A),
            0.75,
            0.0,
            f32::from_bits(0x3ECC_CCCD),
        );
        let smoke = g::<GPtr<u8>>(0xA9AE24).read();
        let pp = lp(&mut pos).cast();
        fx_add_particle(smoke, pp, lp(&mut vel).cast(), 0.0, lp(&mut mult).cast(), -1.0, f32::from_bits(0x3F99_999A), f32::from_bits(0x3F19_999A), 0);
        let smoke = g::<GPtr<u8>>(0xA9AE24).read();
        fx_add_particle(smoke, pp, lp(&mut vel2).cast(), 0.0, lp(&mut mult).cast(), -1.0, f32::from_bits(0x3F99_999A), f32::from_bits(0x3F19_999A), 0);
        wb(fi, 0, rb(fi, 0) | 8);
        let mut f = Fpu::new();
        f.fld(rf(fi, 0x1C));
        f.fcomp_m(0.0);
        if !f.jp(5) {
            extinguish_call(fi);
        } else if ftol32(x(rf(fi, 0x1C))) != ftol32(x(old)) {
            create_fx_sys_for_strength_call(fi, fi.byte_add(4), GPtr::NULL);
        }
        any = true;
    }
    any as u8
}

/// `CFireManager::StartFire(CVector pos, float size, uint8, CEntity* creator, uint32 timeToBurn,
/// int8 generations, uint8)` (0x539F00, thiscall `ret 0x24`): null when the point is under water
/// (water level above `z`) or no fire is free; else the first inactive non-script fire: position,
/// flags `& 0xFD | 0x15`, time `_ftol((rand()/32767*0.3 + 1) * (float)time + (float)now)`, not being
/// extinguished, no entity on fire, creator registered, generations `min(gens, max)`, strength 1, fx.
#[hook(addr = 0x539F00, conv = thiscall, name = "CFireManager::StartFire-NoTarget", expect = "51 55 8B 6C 24 10", this = "CFireManager")]
#[allow(clippy::too_many_arguments)]
pub fn start_fire_no_target(this: GPtr<u8>, px: f32, py: f32, pz: f32, _size: f32, _unk: u8, creator: GPtr<u8>, time: u32, gens: i8, _unk2: u8) -> GPtr<u8> {
    let mut level = 0f32;
    if get_water_level_no_waves(px, py, pz, lp(&mut level), GPtr::NULL, GPtr::NULL) != 0 {
        let mut f = Fpu::new();
        f.fld(level);
        f.fcomp_m(pz);
        if !f.jne(0x41) {
            return GPtr::NULL;
        }
    }
    let Some(fi) = (0..NUM_FIRES).map(|i| fire(this, i)).find(|&f| rb(f, 0) & 3 == 0) else {
        return GPtr::NULL;
    };
    let fl = rb(fi, 0);
    wf(fi, 4, px);
    wf(fi, 8, py);
    wb(fi, 0, (fl & 0xFD) | 0x15);
    wf(fi, 0xC, pz);
    let r = sa_core::orig::at_site(0x539F8C, rand);
    let t = xi(r).fmul_m(cf(K_RAND)).fmul_m(cf(K_863A40)).fadd_m(cf(K_ONE));
    let t = t.fmul(u32_to_x87(time)).fadd(u32_to_x87(time_ms()));
    wu(fi, 0x18, ftol32(t) as u32);
    if !rp(fi, 0x10).is_null() {
        clean_up_old_reference(rp(fi, 0x10), fi.byte_add(0x10).cast());
    }
    wb(fi, 0, rb(fi, 0) & 0xF7);
    wu(fi, 0x10, 0);
    if !rp(fi, 0x14).is_null() {
        clean_up_old_reference(rp(fi, 0x14), fi.byte_add(0x14).cast());
    }
    wp(fi, 0x14, creator);
    let max = g::<i32>(MAX_FIRE_GENERATIONS).read();
    let gn = i32::from(gens);
    wb(fi, 0x20, if gn < max { gn } else { max } as u8);
    wu(fi, 0x1C, K_ONE);
    if !creator.is_null() {
        register_reference(creator, fi.byte_add(0x14).cast());
    }
    create_fx_sys_for_strength_call(fi, fi.byte_add(4), GPtr::NULL);
    fi
}

/// `CFireManager::StartScriptFire(const CVector& pos, CEntity* target, float, uint8, int8 generations,
/// int32 strength)` (0x53A270, thiscall `ret 0x18`): puts out a fire the target already has (vehicle
/// engine status >= 225 lowered to 215 when it had none and no... as the bytes), takes
/// `GetNextFreeFire(true)`, sets it up as a script fire and returns its unique script index (-1 when
/// none is free).
#[hook(addr = 0x53A270, conv = thiscall, name = "CFireManager::StartScriptFire", expect = "83 EC 10 53 56", this = "CFireManager")]
pub fn start_script_fire(this: GPtr<u8>, pos: GPtr<u8>, target: GPtr<u8>, _a: f32, _b: u8, gens: i8, strength: i32) -> i32 {
    if !target.is_null() {
        match etype(target) {
            3 => {
                let f = rp(target, 0x730);
                if !f.is_null() {
                    wb(f, 0, rb(f, 0) & 0xFD);
                    extinguish_call(rp(target, 0x730));
                }
            }
            2 => {
                let f = rp(target, 0x490);
                if !f.is_null() {
                    wb(f, 0, rb(f, 0) & 0xFD);
                    extinguish_call(rp(target, 0x490));
                }
                if ru(target, 0x590) == 0 {
                    let dm = target.byte_add(0x5A0);
                    if get_engine_status(dm) >= 0xE1 {
                        set_engine_status(dm, 0xD7);
                    }
                }
            }
            _ => {}
        }
    }
    let fi = get_next_free_fire_call(this, 1);
    if fi.is_null() {
        return -1;
    }
    let fl = rb(fi, 0) | 7;
    wb(fi, 0, fl);
    wv_bits(fi, 4, rv_bits(pos, 0));
    wb(fi, 0, (fl & 0xF7) | 0x10);
    let max = g::<i32>(MAX_FIRE_GENERATIONS).read();
    let gn = i32::from(gens);
    wb(fi, 0x20, if gn < max { gn } else { max } as u8);
    if !rp(fi, 0x10).is_null() {
        clean_up_old_reference(rp(fi, 0x10), fi.byte_add(0x10).cast());
    }
    wp(fi, 0x10, target);
    if !target.is_null() {
        register_reference(target, fi.byte_add(0x10).cast());
    }
    if !rp(fi, 0x14).is_null() {
        clean_up_old_reference(rp(fi, 0x14), fi.byte_add(0x14).cast());
    }
    wu(fi, 0x14, 0);
    wf(fi, 0x1C, xi(strength).to_f32());
    if target.is_null() {
        create_fx_sys_for_strength_call(fi, pos, GPtr::NULL);
    } else {
        let ty = etype(target);
        let slot = match ty {
            3 => Some(0x730),
            2 => Some(0x490),
            _ => None,
        };
        if let Some(off) = slot {
            wp(target, off, fi);
            let mut p = rv_bits(pos_ptr(target), 0);
            create_fx_sys_for_strength_call(fi, lp(&mut p).cast(), GPtr::NULL);
        }
    }
    // `(fire - this) / 0x28` by the magic multiply 0x66666667 >> 4 (`sar`, rounding toward zero)
    let d = (fi.addr() as i32).wrapping_sub(this.addr() as i32);
    let q = ((i64::from(d) * 0x6666_6667) >> 32) as i32 >> 4;
    let idx = q + ((q as u32) >> 31) as i32;
    get_new_unique_script_thing_index(idx, 5)
}

/// `CCreepingFire::SetReadyToBurn()` (0x539CC0): clears the 32x32 status grid.
#[hook(addr = 0x539CC0, conv = cdecl, name = "CCreepingFire::SetReadyToBurn", expect = "57 B9 00 01 00 00")]
pub fn set_ready_to_burn() {
    CREEPING_FIRE_STATUS.set([0; 0x400]);
}

fn status_cell(i: u32) -> GPtr<u8> {
    CREEPING_FIRE_STATUS.ptr().cast::<u8>().byte_add(i as isize)
}

/// `CCreepingFire::Update()` (0x539CE0): the cell `[(n & 31)][(n >> 5) & 31]` for frame `n`: 4 → 0;
/// 5, 6 → decremented.
#[hook(addr = 0x539CE0, conv = cdecl, name = "CCreepingFire::Update", expect = "8B 0D 4C CB B7 00")]
pub fn creeping_fire_update() {
    let n = frame_counter();
    let c = status_cell(((n & 0x1F) << 5) + ((n >> 5) & 0x1F));
    let v = c.read();
    if v == 4 {
        c.write(0);
    } else if v > 4 && v <= 6 {
        c.write(v - 1);
    }
}

/// `CCreepingFire::TryToStartFireAtCoors(CVector pos, int8 generations, bool, bool scriptFire, float
/// zDist)` (0x53A450, cdecl): for a free grid cell (`_ftol(x) & 31`, `_ftol(y) & 31`) and plenty of
/// free fires (inlined over `gFireManager`), a vertical line from `pos` down to `z - zDist`; on a hit
/// the cell becomes 6 and a fire starts at the hit (20 s, `!scriptFire` as the last argument), whose
/// script flag becomes `scriptFire & 1` and first-generation flag is cleared.
#[hook(addr = 0x53A450, conv = cdecl, name = "CCreepingFire::TryToStartFireAtCoors", expect = "D9 44 24 04 83 EC 38")]
pub fn try_to_start_fire_at_coors(px: f32, py: f32, pz: f32, gens: i8, _a: u8, script: u8, zdist: f32) -> u8 {
    let ix = ftol32(x(px)) as u32 & 0x1F;
    let iy = ftol32(x(py)) as u32 & 0x1F;
    let cell = status_cell((ix << 5) + iy);
    if cell.read() != 0 {
        return 0;
    }
    let man: GPtr<u8> = g(FIRE_MANAGER);
    let mut n = 0;
    let mut plenty = false;
    for i in 0..NUM_FIRES {
        if rb(fire(man, i), 0) & 1 == 0 {
            n += 1;
            if n >= 6 {
                plenty = true;
                break;
            }
        }
    }
    if !plenty {
        return 0;
    }
    let mut p = [px.to_bits(), py.to_bits(), pz.to_bits()];
    let mut cp = [0u8; 0x2C];
    let mut ent = GPtr::NULL;
    let z = x(pz).fsub_m(zdist).to_f32();
    if process_vertical_line(lp(&mut p).cast(), z, lp(&mut cp).cast(), lp(&mut ent), 1, 0, 0, 0, 0, 0, GPtr::NULL) == 0 {
        return 0;
    }
    cell.write(6);
    let hit_z = f32::from_le_bytes([cp[8], cp[9], cp[10], cp[11]]);
    let fi = start_fire_no_target_call(man, px, py, hit_z, f32::from_bits(0x3F4C_CCCD), 1, GPtr::NULL, 20_000, gens, (script == 0) as u8);
    if fi.is_null() {
        return 0;
    }
    wb(fi, 0, ((script & 1) << 1) | (rb(fi, 0) & 0xED));
    1
}

/// `CFireManager::StartFire(CEntity* target, CEntity* creator, float size, uint8, uint32 timeToBurn,
/// int8 generations)` (0x53A050, thiscall `ret 0x18`; split chunk 0x409BC8): null for a target that
/// already burns, a ped not in control, a vehicle with engine status >= 225 (cars only), a fire-proof
/// ped/vehicle, or no free fire. Else the target's fire pointer is set (with a crime report 13/14/15
/// for a creator; `FindPlayerPed(-1)` is called for peds and unused), the fire sits at the target,
/// flags `& 0xF5 | 0x11`, strength 1, burn time: player 2333 ms, vehicle 3000 + rand(0, 1000), else
/// `time + rand(0, 1000)`; entity on fire / creator registered, flag 0x04, generations `min(gens,
/// max)`, fx at the fire.
#[hook(addr = 0x53A050, conv = thiscall, name = "CFireManager::StartFire-", expect = "56 57 8B 7C 24 0C", this = "CFireManager")]
pub fn start_fire_target(this: GPtr<u8>, target: GPtr<u8>, creator: GPtr<u8>, _size: f32, _unk: u8, time: u32, gens: i8) -> GPtr<u8> {
    let ty = etype(target);
    let fireproof_check = match ty {
        3 => {
            if !rp(target, 0x730).is_null() || ped_is_ped_in_control(target) == 0 {
                return GPtr::NULL;
            }
            true
        }
        2 => {
            if !rp(target, 0x490).is_null() {
                return GPtr::NULL;
            }
            if ru(target, 0x590) == 0 && get_engine_status(target.byte_add(0x5A0)) >= 0xE1 {
                return GPtr::NULL;
            }
            true
        }
        4 => {
            if !rp(target, 0x164).is_null() {
                return GPtr::NULL;
            }
            false
        }
        _ => false,
    };
    if fireproof_check && ru(target, 0x40) & 0x80000 != 0 {
        return GPtr::NULL;
    }
    let Some(fi) = (0..NUM_FIRES).map(|i| fire(this, i)).find(|&f| rb(f, 0) & 3 == 0) else {
        return GPtr::NULL;
    };
    match etype(target) {
        3 => {
            wp(target, 0x730, fi);
            find_player_ped(-1);
            if !creator.is_null() {
                report_crime(if ru(target, 0x598) == 6 { 0xE } else { 0xD }, target, creator);
            }
        }
        2 => {
            wp(target, 0x490, fi);
            if !creator.is_null() {
                report_crime(0xF, target, creator);
            }
        }
        4 => wp(target, 0x164, fi),
        _ => {}
    }
    let pos = rv_bits(pos_ptr(target), 0);
    wb(fi, 0, (rb(fi, 0) & 0xF5) | 0x11);
    wv_bits(fi, 4, pos);
    wu(fi, 0x1C, K_ONE);
    let t = if etype(target) == 3 && ped_is_player(target) != 0 {
        time_ms().wrapping_add(0x91D)
    } else if etype(target) == 2 {
        let r = random_int(0, 1000) as u32;
        r.wrapping_add(time_ms()).wrapping_add(0xBB8)
    } else {
        let r = random_int(0, 1000) as u32;
        r.wrapping_add(time_ms()).wrapping_add(time)
    };
    wu(fi, 0x18, t);
    if !rp(fi, 0x10).is_null() {
        clean_up_old_reference(rp(fi, 0x10), fi.byte_add(0x10).cast());
    }
    wp(fi, 0x10, target);
    register_reference(target, fi.byte_add(0x10).cast());
    if !rp(fi, 0x14).is_null() {
        clean_up_old_reference(rp(fi, 0x14), fi.byte_add(0x14).cast());
    }
    wp(fi, 0x14, creator);
    if !creator.is_null() {
        register_reference(creator, fi.byte_add(0x14).cast());
    }
    wb(fi, 0, rb(fi, 0) | 4);
    let max = g::<i32>(MAX_FIRE_GENERATIONS).read();
    let gn = i32::from(gens);
    wb(fi, 0x20, if gn < max { gn } else { max } as u8);
    create_fx_sys_for_strength_call(fi, fi.byte_add(4), GPtr::NULL);
    fi
}

// ---- CFireManager::Update (PORT-WEAPON-2) --------------------------------------------------------

orig! {
    /// `CFire::ProcessFire()` (0x53A570, thiscall)
    pub fn process_fire_call(fire: GPtr<u8>) = thiscall 0x53A570 => process_fire;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    pub fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// `CWorld::FindGroundZFor3DCoord(CVector, bool*, CEntity**)` (0x5696C0, ST0)
    pub fn find_ground_z_for_3d_coord(x: f32, y: f32, z: f32, hit: GPtr<u8>, e: GPtr<GPtr<u8>>) -> f64 = cdecl 0x5696C0 => unported;
    /// `CVector::Normalise()` (0x59C910, thiscall)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// CRT `ceil(double)` (0x823820, ST0)
    pub fn crt_ceil(v: f64) -> f64 = cdecl 0x823820 => unported;
    /// `CShadows::StoreStaticShadow(uint32 id, uint8 type, RwTexture*, const CVector*, float frontX, float frontY,
    /// float sideX, float sideY, int16 intensity, uint8 r, g, b, float zDistance, float scale, float drawDistance,
    /// bool temporary, float upDistance)` (0x70BA00; every argument a pushed dword)
    pub fn store_static_shadow(id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: f32, fy: f32, sx: f32, sy: f32, intensity: u32, r: u32, g: u32, b: u32, zd: f32, scale: f32, dd: f32, temp: u32, up: f32) -> u8 = cdecl 0x70BA00 => unported;
    /// `CCoronas::RegisterCorona(uint32 id, CEntity*, uint8 r, g, b, a, const CVector&, float radius, float farClip,
    /// eCoronaType, eCoronaFlareType, eCoronaReflType, eCoronaLOSCheck, eCoronaTrail, float normalAngle, bool neonFade,
    /// float pullTowardsCam, bool fullBrightAtStart, float fadeSpeed, bool onlyFromBelow, bool reflectionDelay)`
    /// (0x6FC580; every argument a pushed dword)
    pub fn register_corona(id: u32, e: u32, r: u32, g: u32, b: u32, a: u32, pos: GPtr<u8>, radius: f32, far: f32, ty: u32, flare: u32, refl: u32, los: u32, trail: u32, normal_angle: f32, neon: u32, pull: f32, bright: u32, fade: f32, below: u32, delay: u32) = cdecl 0x6FC580 => unported;
}

/// `CReplay::Mode`
const REPLAY_MODE: usize = 0xA43088;
/// `CTimer::m_snPreviousTimeInMilliseconds`
const PREV_TIME_MS: usize = 0xB7CB78;
/// `TheCamera` matrix pointer / placement / heading / right vector
const CAM_MATRIX: usize = 0xB6F03C;
const CAM_PLACEMENT: usize = 0xB6F02C;
const CAM_HEADING: usize = 0xB6F038;
const CAM_RIGHT: usize = 0xB6F99C;
/// `gpShadowExplosionTex`
const SHADOW_EXPLOSION_TEX: usize = 0xC403F4;
/// The fire glow base colour bytes (r, g, b)
const FIRE_COLOUR: usize = 0x8CCFF0;
/// "riot_smoke"
const RIOT_SMOKE: usize = 0x863A54;

fn cam_matrix_pos() -> GPtr<u8> {
    let m = g::<GPtr<u8>>(CAM_MATRIX).read();
    if m.is_null() { g(CAM_PLACEMENT) } else { m.byte_add(0x30) }
}

/// `(int)((float)colour[i] * m)` as the original computes it: `fild` of the byte, `fmul` by the f32
/// multiplier, `_ftol2`.
fn colour(i: usize, m: f32) -> u32 {
    ftol32(xi(i32::from(g::<u8>(FIRE_COLOUR + i).read())).fmul_m(m)) as u32
}

/// `CFireManager::Update()` (0x53AF00), ported from the 1.0 US bytes. Kept from the original
/// (`docs/re/specs/WEAPON/CFireManager.deviations.md`): the riot block's inline random formulas (two raw
/// `rand()`s), its gates (`hitEntity && (hitEntity->+0x1E & 1) && hit && z > 0.5f`), the direction
/// `point - camera` (gr: camera - point) compared with the f64 0.2; the group search (strongest =
/// first strictly greater strength than the running best, which starts at -1.0 on the x87 stack;
/// group = unvisited active fires with `sqrt(dx*dx + dy*dy) < 6.0`); the shadow's front/side
/// vectors `(1.2*d, 0)` / `(0, -1.2*d)` (gr: `-1.2*d` front, `1.2*d` side); the colour multipliers
/// `rand()*(1/32767)*0.39999998f + 0.6f` and `* 0.8f`; the corona offsets (x and y of the camera
/// offset and of the right vector in extended precision, z rounded to f32 first).
/// Not reproduced (stack-only): the dead accumulator `+= pos * ceil(strength)` (the multiplications
/// cannot fault and the locals are never read); the stack scratch slots.
#[hook(addr = 0x53AF00, conv = thiscall, name = "CFireManager::Update", expect = "55 8B EC 83 E4 F8", this = "CFireManager")]
pub fn manager_update(this: GPtr<u8>) {
    if g::<u8>(REPLAY_MODE).read() == 1 {
        return;
    }
    for i in 0..NUM_FIRES {
        let f = fire(this, i);
        if rb(f, 0) & 1 != 0 {
            process_fire_call(f);
        }
    }
    if la_riots_active_here() != 0 && g::<u32>(PREV_TIME_MS).read() / 500 != time_ms() / 500 {
        riot(this);
    }
    let mut visited = [false; NUM_FIRES];
    let mut n = (0..NUM_FIRES).filter(|&i| rb(fire(this, i), 0) & 1 != 0).count() as i32;
    while n > 0 {
        // strongest unvisited active fire
        let mut best = X87::constant(0xBF80_0000);
        let mut idx: isize = -1;
        for (i, &v) in visited.iter().enumerate() {
            let f = fire(this, i);
            if !v && rb(f, 0) & 1 != 0 && !best.fcom(x(rf(f, 0x1C))).test_parity(0x05) {
                best = x(rf(f, 0x1C));
                idx = i as isize;
            }
        }
        // original out-of-bounds when nothing beat -1.0: fire[-1]
        let s = this.byte_add(idx * FIRE_SIZE as isize);
        let mut sum = 0f32;
        let mut ceil_sum = 0i32;
        for (i, v) in visited.iter_mut().enumerate() {
            let f = fire(this, i);
            if *v || rb(f, 0) & 1 == 0 {
                continue;
            }
            let dx = x(rf(f, 4)).fsub_m(rf(s, 4));
            let dy = x(rf(f, 8)).fsub_m(rf(s, 8));
            let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
            if d.fcom(X87::constant(0x40C0_0000)).test_parity(0x05) {
                continue;
            }
            sum = x(sum).fadd_m(rf(f, 0x1C)).to_f32();
            ceil_sum = ceil_sum.wrapping_add(ftol32(st0(crt_ceil(f64::from(rf(f, 0x1C))))));
            n -= 1;
            *v = true;
        }
        // `fcomp 4.0; test ah, 0x41; jne`: only `sum > 4` continues
        if x(sum).fcom(X87::constant(0x4080_0000)).test_any(0x41) || ceil_sum == 0 {
            continue;
        }
        fire_glow(s, sum);
    }
}

/// The shadow and corona part of [`manager_update`] for the group around fire `s` (0x53B565..0x53BAAA).
fn fire_glow(s: GPtr<u8>, sum: f32) {
    let t = x(sum).fsub_m(cf(0x40C0_0000)).fadd_m(cf(0x4040_0000));
    // `fld 7.0; fcomp st1; test ah, 5; jp`: 7.0 when `7 < t`, else t
    let dir = if X87::constant(0x40E0_0000).fcom(t).test_parity(0x05) { t.to_f32() } else { 7.0 };
    let r = sa_core::orig::at_site(0x53B599, rand);
    let m = xi(r).fmul_m(cf(0x3800_0100)).fmul_m(cf(0x3ECC_CCCC)).fadd_m(cf(0x3F19_999A)).to_f32();
    let p = rv(s, 4);
    let mut pos = [p[0], p[1], x(p[2]).fadd_m(5.0).to_f32()];
    let (b, gr, rr) = (colour(2, m), colour(1, m), colour(0, m));
    let tex = g::<u32>(SHADOW_EXPLOSION_TEX).read();
    let front = x(dir).fmul_m(cf(0x3F99_999A)).to_f32();
    let side = x(dir).fmul_m(cf(0xBF99_999A)).to_f32();
    store_static_shadow(s.addr() as u32, 2, tex, lp(&mut pos).cast(), front, 0.0, 0.0, side, 0, rr, gr, b, 10.0, 1.0, 40.0, 0, 0.0);
    // `fcomp 6.0; test ah, 0x41; jne`: only `sum > 6` continues
    if x(sum).fcom(X87::constant(0x40C0_0000)).test_any(0x41) {
        return;
    }
    let m2 = x(m).fmul_m(cf(0x3F4C_CCCD)).to_f32();
    let mut pt = [p[0], p[1], x(p[2]).fadd_m(cf(0x4026_6666)).to_f32()];
    let c = rv(cam_matrix_pos(), 0);
    let mut d = [x(c[0]).fsub_m(pt[0]).to_f32(), x(c[1]).fsub_m(pt[1]).to_f32(), x(c[2]).fsub_m(pt[2]).to_f32()];
    vector_normalise(lp(&mut d).cast());
    let k = cf(0x4060_0000);
    let oz = x(d[2]).fmul_m(k).to_f32();
    pt[0] = x(d[0]).fmul_m(k).fadd_m(pt[0]).to_f32();
    pt[1] = x(d[1]).fmul_m(k).fadd_m(pt[1]).to_f32();
    pt[2] = x(oz).fadd_m(pt[2]).to_f32();
    let radius = x(dir).fmul_m(0.5).to_f32();
    let id = s.addr() as u32;
    let corona = |pt: &mut [f32; 3], id: u32, flare: u32| {
        let (b, g_, r) = (colour(2, m2), colour(1, m2), colour(0, m2));
        register_corona(id, 0, r, g_, b, 0xFF, lp(pt).cast(), radius, 70.0, 1, flare, 0, 0, 0, 0.0, 0, 1.5, 0, 15.0, 0, 0);
    };
    corona(&mut pt, id, 2);
    pt[2] = x(pt[2]).fadd_m(2.0).to_f32();
    corona(&mut pt, id.wrapping_add(1), 0);
    pt[2] = x(pt[2]).fsub_m(2.0).to_f32();
    let cr = rv(g(CAM_RIGHT), 0);
    let mut rt = [cr[0], cr[1], 0.0f32];
    vector_normalise(lp(&mut rt).cast());
    let r2z = x(rt[2]).fdouble().to_f32();
    pt[0] = x(rt[0]).fdouble().fadd_m(pt[0]).to_f32();
    pt[1] = x(rt[1]).fdouble().fadd_m(pt[1]).to_f32();
    pt[2] = x(r2z).fadd_m(pt[2]).to_f32();
    corona(&mut pt, id.wrapping_add(2), 0);
    let r4z = x(rt[2]).fmul_m(4.0).to_f32();
    pt[0] = x(pt[0]).fsub(x(rt[0]).fmul_m(4.0)).to_f32();
    pt[1] = x(pt[1]).fsub(x(rt[1]).fmul_m(4.0)).to_f32();
    pt[2] = x(pt[2]).fsub_m(r4z).to_f32();
    corona(&mut pt, id.wrapping_add(3), 0);
}

/// The LA riots block of [`manager_update`] (0x53AF75..0x53B179).
fn riot(this: GPtr<u8>) {
    let r1 = sa_core::orig::at_site(0x53AF7A, rand);
    let a = xi(r1).fmul_m(cf(0x3800_0100)).fmul_m(cf(0x40C9_0FDB)).to_f32();
    let r2 = sa_core::orig::at_site(0x53AF97, rand);
    let d = xi(r2).fmul_m(cf(0x3800_0100)).fmul_m(25.0).fadd_m(35.0);
    let sx = x(a).fsin().fmul(d).to_f32();
    let cy = x(a).fcos().fmul(d).to_f32();
    let c = rv(cam_matrix_pos(), 0);
    let mut pt = [x(sx).fadd_m(c[0]).to_f32(), x(cy).fadd_m(c[1]).to_f32(), x(c[2]).fadd_m(10.0).to_f32()];
    let mut hit_entity: GPtr<u8> = GPtr::NULL;
    let mut hit = 0u8;
    pt[2] = st0(find_ground_z_for_3d_coord(pt[0], pt[1], pt[2], lp(&mut hit), lp(&mut hit_entity))).to_f32();
    let c = rv(cam_matrix_pos(), 0);
    let mut dir = [x(pt[0]).fsub_m(c[0]).to_f32(), x(pt[1]).fsub_m(c[1]).to_f32(), x(pt[2]).fsub_m(c[2]).to_f32()];
    vector_normalise(lp(&mut dir).cast());
    let m = g::<GPtr<u8>>(CAM_MATRIX).read();
    let fwd = if m.is_null() {
        let h = g::<f32>(CAM_HEADING).read();
        [x(h).fsin().fchs().to_f32(), x(h).fcos().to_f32(), 0.0]
    } else {
        rv(m, 0x10)
    };
    if hit_entity.is_null() || rb(hit_entity, 0x1E) & 1 == 0 || hit == 0 {
        return;
    }
    // `fcomp 0.5; test ah, 0x41; jne`: only `z > 0.5` continues
    if x(pt[2]).fcom(X87::constant(0x3F00_0000)).test_any(0x41) {
        return;
    }
    let dot = x(fwd[2]).fmul_m(dir[2]).fadd(x(fwd[1]).fmul_m(dir[1])).fadd(x(fwd[0]).fmul_m(dir[0]));
    // `fcomp qword 0.2; test ah, 0x41; je`: smoke when `dot > 0.2`
    let smoke = !dot.fcom(X87::from_f64_exact(0.2)).test_any(0x41) || sa_core::orig::at_site(0x53B112, rand) < 0x3FFF;
    if smoke {
        let fx = create_fx_system(g(FX_MAN), g(RIOT_SMOKE), lp(&mut pt).cast(), GPtr::NULL, 1);
        if !fx.is_null() {
            fx_play_and_kill(fx);
        }
    } else {
        let size = st0(sa_core::orig::at_site(0x53B135, || random_float(0.5, 2.5))).to_f32();
        start_fire_no_target_call(this, pt[0], pt[1], pt[2], size, 1, GPtr::NULL, 30000, 5, 1);
    }
}

// ---- CFire::ProcessFire (PORT-WEAPON-2) ------------------------------------------------------------

orig! {
    /// `CFireManager::StartFire(CEntity* target, CEntity* creator, float, uint8, uint32 time, int8 gens)` (0x53A050)
    pub fn start_fire_target_call(man: GPtr<u8>, target: GPtr<u8>, creator: GPtr<u8>, size: f32, unk: u8, time: u32, gens: i8) -> GPtr<u8> = thiscall 0x53A050 => start_fire_target;
    /// `CPed::IsAlive()` (0x5E0170, thiscall)
    pub fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CVehicle::InflictDamage(CEntity*, eWeaponType, float, CVector)` (0x6D7C90, thiscall)
    pub fn pf_vehicle_inflict_damage(veh: GPtr<u8>, by: GPtr<u8>, ty: i32, dmg: f32, x: f32, y: f32, z: f32) = thiscall 0x6D7C90 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990, thiscall)
    pub fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `operator*(CVector* out, const CMatrix&, const CVector&)` (0x59C890, returns out)
    pub fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `FxSystem_c::SetOffsetPos(CVector*)` (0x4AA660, thiscall)
    pub fn fx_set_offset_pos(fx: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4AA660 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, b: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CPlayerPed::DoStuffToGoOnFire()` (0x60A020, thiscall)
    pub fn do_stuff_to_go_on_fire(ped: GPtr<u8>) = thiscall 0x60A020 => unported;
    /// `CVehicle::FindTyreNearestPoint(float x, float y)` (0x6D7BC0, thiscall; EAX)
    pub fn find_tyre_nearest_point(veh: GPtr<u8>, x: f32, y: f32) -> u32 = thiscall 0x6D7BC0 => unported;
    /// `CVehicle::BurstTyre(uint8, bool)` (vtable +0xB0; the whole EAX dword is pushed)
    pub fn pf_burst_tyre(veh: GPtr<u8>, piece: u32, b: u32) = thiscall vslot 0xB0 => unported;
    /// `CObject::ObjectFireDamage(float, CEntity*)` (0x5A1580, thiscall)
    pub fn object_fire_damage(obj: GPtr<u8>, dmg: f32, by: GPtr<u8>) = thiscall 0x5A1580 => unported;
    /// `CCreepingFire::TryToStartFireAtCoors(CVector, int8 gens, bool, bool script, float zDist)` (0x53A450; dwords as pushed)
    pub fn try_to_start_fire_at_coors_call(x: f32, y: f32, z: f32, gens: u32, a: u32, script: u32, zd: f32) -> u8 = cdecl 0x53A450 => unported;
    /// `VectorSub(CVector* out, const CVector*, const CVector*)` (0x40FE60)
    pub fn pf_vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator*(CVector* out, float, const CVector*)` (0x40FE90)
    pub fn pf_vector_scale(out: GPtr<u8>, k: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `operator+(CVector* out, const CVector*, const CVector*)` (0x40FE30)
    pub fn pf_vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CVector::Magnitude()` (0x4082C0, thiscall, ST0)
    pub fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `FxSystem_c::SetConstTime(bool, float)` (0x4AA6C0, thiscall)
    pub fn pf_fx_set_const_time(fx: GPtr<u8>, b: u32, t: f32) = thiscall 0x4AA6C0 => unported;
    /// `CPointLights::AddLight(type, CVector pos, CVector dir, radius, r, g, b, fog, shadows, attachedTo)` (0x7000E0)
    pub fn point_lights_add_light(ty: u32, px: f32, py: f32, pz: f32, dx: f32, dy: f32, dz: f32, radius: f32, r: f32, g: f32, b: f32, fog: u32, shadows: u32, attached: u32) = cdecl 0x7000E0 => unported;
}

/// `CPools::ms_pVehiclePool` / `ms_pObjectPool`
const VEHICLE_POOL: usize = 0xB74494;
const OBJECT_POOL: usize = 0xB7449C;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rp(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `sqrt((dz*dz + dy*dy) + dx*dx)` of `p - fire.pos` as the pool loops compute it (x87).
fn dist3(p: GPtr<u8>, f: GPtr<u8>) -> X87 {
    let dx = x(rf(p, 0)).fsub_m(rf(f, 4));
    let dy = x(rf(p, 4)).fsub_m(rf(f, 8));
    let dz = x(rf(p, 8)).fsub_m(rf(f, 0xC));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// Walks a pool from slot `size - 1` down to 0 (the original order), calling `f` on every used slot.
fn pool_desc(pool_cell: usize, stride: usize, mut f: impl FnMut(GPtr<u8>)) {
    let pool = g::<GPtr<u8>>(pool_cell).read();
    let n = ru(pool, 8) as usize;
    for i in (0..n).rev() {
        if (rb(rp(pool, 4), i as isize) as i8) < 0 {
            continue;
        }
        let e = rp(pool, 0).byte_add((i * stride) as isize);
        if e.is_null() {
            continue;
        }
        f(e);
    }
}

/// `CFire::ProcessFire()` (0x53A570), ported from the 1.0 US bytes. Kept from the original
/// (`docs/re/specs/WEAPON/CFire.deviations.md` and this port's reading of the bytes):
/// - strength: `n = min3(s + ts*0.002)` is stored, then the OLD strength is written back when
///   `(int)s != (int)n` (both `_ftol2` on the x87 values);
/// - a ped's fire sets a fire-truck-free automobile's health to 75 (`+0x590 == 0`, model != 407);
/// - vehicle pool and object pool walked from the last slot down; vehicles already on fire and fire
///   trucks skipped; the non-BMX `StartFire` passes 0 generations; distances `sqrt((dz²+dy²)+dx²)`;
/// - creeping fire: `(int8)gens > 0`, `rand() & 0x7F == 0`, fewer than 25 active fires; the first random
///   value goes to y, `dir` is normalised, z gets `+ 2.0f`, `gens - 1` and the script flag are passed;
/// - merging a weak nearby fire: index `ftol((rand() & 0xFFFF) * (1/32768) * 60)`, the merged position
///   `0.7*this + 0.3*other` is written to THIS fire, generations become the signed MIN, the burn time
///   `max(old, now + 7000)` (unsigned);
/// - the FX const time is `min(frac(s), (u32)(burn - now) * 0.00028571428f)`;
/// - the removal test compares `(float)removalDistance > sqrt(dy² + dx²)` (2D, camera - fire).
///
/// Not reproduced (stack-only): the stack temporaries the original reuses.
#[hook(addr = 0x53A570, conv = thiscall, name = "CFire::ProcessFire", expect = "83 EC 2C 53 55 56", this = "CFire")]
pub fn process_fire(this: GPtr<u8>) {
    // strength
    let s = x(rf(this, 0x1C));
    let n = x(time_step()).fmul_m(cf(0x3B03_126F)).fadd_m(rf(this, 0x1C));
    let n = if X87::constant(0x4040_0000).fcom(n).test_parity(0x05) { n } else { X87::constant(0x4040_0000) };
    wf(this, 0x1C, n.to_f32());
    if ftol32(s) != ftol32(n) {
        wf(this, 0x1C, s.to_f32());
    }
    let e = rp(this, 0x10);
    if !e.is_null() {
        wv(this, 4, rv(entity_pos(e), 0));
        match etype(e) {
            3 => {
                if rp(e, 0x730) != this {
                    extinguish_call(this);
                    return;
                }
                let st = ru(e, 0x530);
                if st == 0x36 || st == 0x37 {
                    wf(this, 0xC, x(rf(this, 0xC)).fsub_m(1.0).to_f32());
                }
                let veh = rp(e, 0x58C);
                if ru(e, 0x46C) & 0x100 != 0 && !veh.is_null() {
                    if rh(veh, 0x22) != 0x197 && ru(veh, 0x590) == 0 {
                        wf(veh, 0x4C0, 75.0);
                    }
                } else if ped_is_player(e) == 0 && ped_is_alive(e) == 0 {
                    wu(e, 0x40, ru(e, 0x40) | 0x2000_0000);
                }
            }
            2 => {
                if rp(e, 0x490) != this {
                    extinguish_call(this);
                    return;
                }
                if rb(this, 0) & 2 == 0 {
                    let dmg = x(time_step()).fmul_m(cf(0x3F99_999A)).to_f32();
                    pf_vehicle_inflict_damage(e, rp(this, 0x14), 0x25, dmg, 0.0, 0.0, 0.0);
                }
                let v = rp(this, 0x10);
                if ru(v, 0x590) == 0 {
                    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * (i32::from(rh(v, 0x22) as i16) as usize)).read();
                    let d = rv(rp(mi, 0x5C), 0);
                    let mut local = [d[0], d[1], x(d[2]).fadd_m(cf(0x3E19_999A)).to_f32()];
                    let lp_ = lp(&mut local).cast::<u8>();
                    let m = placeable_get_matrix(v);
                    let mut out = [0f32; 3];
                    let r = matrix_mul_vec(lp(&mut out).cast(), m, lp_);
                    wv(this, 4, rv(r, 0));
                }
            }
            _ => {}
        }
        let fx = rp(this, 0x24);
        if !fx.is_null() {
            let t2 = x(time_step()).fdouble();
            let mv = rv(rp(this, 0x10), 0x44);
            let oy = t2.fmul_m(mv[1]).to_f32();
            let oz = t2.fmul_m(mv[2]).to_f32();
            let p = rv(this, 4);
            let mut out = [t2.fmul_m(mv[0]).fadd_m(p[0]).to_f32(), x(oy).fadd_m(p[1]).to_f32(), x(oz).fadd_m(p[2]).to_f32()];
            fx_set_offset_pos(fx, lp(&mut out).cast());
        }
    }
    let creator = || rp(this, 0x14);
    let fm = g::<u8>(FIRE_MANAGER);
    // the player
    let e = rp(this, 0x10);
    if (e.is_null() || etype(e) != 2)
        && find_player_vehicle(-1, 0).is_null()
        && rp(find_player_ped(-1), 0x730).is_null()
        && ru(find_player_ped(-1), 0x40) & 0x80000 == 0
        && rp(find_player_ped(-1), 0xFC).is_null()
    {
        let mut d = [0f32; 3];
        pf_vector_sub(lp(&mut d).cast(), entity_pos(find_player_ped(-1)), this.byte_add(4));
        let q = x(d[0]).fmul_m(d[0]).fadd(x(d[2]).fmul_m(d[2])).fadd(x(d[1]).fmul_m(d[1]));
        if !q.fcom(X87::constant(0x3F99_999A)).test_parity(0x05) {
            do_stuff_to_go_on_fire(find_player_ped(-1));
            let c = creator();
            start_fire_target_call(fm, find_player_ped(-1), c, cf(0x3F4C_CCCD), 1, 7000, 100);
        }
    }
    // vehicles
    if sa_core::orig::at_site(0x53A8CA, rand) & 0x1F == 0 {
        pool_desc(VEHICLE_POOL, 0xA18, |v| {
            if !rp(v, 0x490).is_null() || rh(v, 0x22) == 0x197 {
                return;
            }
            if dist3(entity_pos(v), this).fcom(X87::constant(0x4000_0000)).test_parity(0x05) {
                return;
            }
            if ru(v, 0x594) == 10 {
                do_stuff_to_go_on_fire(find_player_ped(-1));
                let c = creator();
                start_fire_target_call(fm, find_player_ped(-1), c, cf(0x3F4C_CCCD), 1, 7000, 100);
                let t = find_tyre_nearest_point(v, rf(this, 4), rf(this, 8));
                pf_burst_tyre(v, (t & 0xFFFF_FF00) | u32::from((t as u8).wrapping_add(0xD)), 0);
            } else {
                start_fire_target_call(fm, v, creator(), cf(0x3F4C_CCCD), 1, 7000, 0);
            }
        });
    }
    // objects
    if sa_core::orig::at_site(0x53AA0A, rand) & 3 == 0 {
        pool_desc(OBJECT_POOL, 0x19C, |o| {
            if dist3(entity_pos(o), this).fcom(X87::constant(0x4040_0000)).test_parity(0x05) {
                return;
            }
            object_fire_damage(o, x(time_step()).fmul_m(8.0).to_f32(), creator());
        });
    }
    // creeping fire
    if (rb(this, 0x20) as i8) > 0 && sa_core::orig::at_site(0x53AABB, rand) & 0x7F == 0 {
        let active = (0..NUM_FIRES).filter(|&i| rb(fire(fm, i), 0) & 1 != 0).count();
        if active < 25 {
            let t1 = xi(sa_core::orig::at_site(0x53AB3E, rand)).fmul_m(cf(0x3800_0100)).to_f32();
            let dxv = xi(sa_core::orig::at_site(0x53AB55, rand)).fmul_m(cf(0x3800_0100)).fdouble().fsub_m(1.0).to_f32();
            let mut dir = [dxv, x(t1).fdouble().fsub_m(1.0).to_f32(), 0.0f32];
            vector_normalise(lp(&mut dir).cast());
            let d = xi(sa_core::orig::at_site(0x53AB95, rand)).fmul_m(cf(0x3800_0100)).fadd_m(2.0);
            let script = u32::from(rb(this, 0) >> 1) & 1;
            let gens = u32::from(rb(this, 0x20).wrapping_sub(1));
            let ty = x(dir[1]).fmul(d).to_f32();
            let tz = d.fmul_m(dir[2]).to_f32();
            let p = rv(this, 4);
            let vx = x(dir[0]).fmul(d).fadd_m(p[0]).to_f32();
            let vy = x(ty).fadd_m(p[1]).to_f32();
            let vz = x(tz).fadd_m(p[2]).fadd_m(2.0).to_f32();
            try_to_start_fire_at_coors_call(vx, vy, vz, gens, 0, script, 10.0);
        }
    }
    // merge a weak nearby fire
    if !x(rf(this, 0x1C)).fcom(X87::constant(0x4000_0000)).test_parity(0x41)
        && rb(this, 0x20) != 0
        && sa_core::orig::at_site(0x53AC46, rand) & 0xF == 0
    {
        let r = sa_core::orig::at_site(0x53AC53, rand) & 0xFFFF;
        let idx = ftol32(xi(r).fmul_m(cf(0x3800_0000)).fmul_m(60.0));
        let other = fm.byte_add(idx as isize * FIRE_SIZE as isize);
        let fl = rb(other, 0);
        if other != this
            && fl & 1 != 0
            && fl & 2 == 0
            && !x(rf(other, 0x1C)).fcom(X87::one()).test_parity(0x41)
        {
            let mut d = [0f32; 3];
            let dp = lp(&mut d).cast::<u8>();
            pf_vector_sub(dp, this.byte_add(4), other.byte_add(4));
            if !st0(vector_magnitude(dp)).fcom(X87::constant(0x4060_0000)).test_parity(0x05) {
                let (mut a, mut b, mut out) = ([0f32; 3], [0f32; 3], [0f32; 3]);
                let ap = pf_vector_scale(lp(&mut a).cast(), cf(0x3E99_999A), other.byte_add(4));
                let bp = pf_vector_scale(lp(&mut b).cast(), cf(0x3F33_3333), this.byte_add(4));
                let r = pf_vector_add(lp(&mut out).cast(), bp, ap);
                let s1 = x(rf(this, 0x1C)).fadd_m(1.0).to_f32();
                wv(this, 4, rv(r, 0));
                wf(this, 0x1C, s1);
                let t = time_ms().wrapping_add(7000);
                let old = ru(this, 0x18);
                wu(this, 0x18, if old > t { old } else { t });
                create_fx_sys_for_strength_call(this, this.byte_add(4), GPtr::NULL);
                let (og, tg) = (rb(other, 0x20) as i8, rb(this, 0x20) as i8);
                wb(this, 0x20, if tg >= og { og as u8 } else { tg as u8 });
                extinguish_call(other);
            }
        }
    }
    // FX const time
    let fx = rp(this, 0x24);
    if !fx.is_null() {
        let s = x(rf(this, 0x1C));
        let frac = s.fsub(xi(ftol32(s)));
        let diff = ru(this, 0x18).wrapping_sub(time_ms());
        let mut k = xi(diff as i32);
        if (diff as i32) < 0 {
            k = k.fadd_m(cf(0x4F80_0000));
        }
        let k = k.fmul_m(cf(0x3995_CBEC)).to_f32();
        let v = if frac.fcom(x(k)).test_parity(0x05) { k } else { frac.to_f32() };
        pf_fx_set_const_time(fx, 1, v);
    }
    // light, or burn down
    let now = time_ms();
    let light = if now < ru(this, 0x18) {
        let c = rv(cam_matrix_pos(), 0);
        let dx = x(c[0]).fsub_m(rf(this, 4));
        let dy = x(c[1]).fsub_m(rf(this, 8));
        let dist = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        // `fild removal; fcompp; test ah, 0x41; je`: light when removal > dist
        !xi(i32::from(rb(this, 0x21))).fcom(dist).test_any(0x41) || rb(this, 0) & 2 != 0
    } else {
        rb(this, 0) & 2 != 0
    };
    if light {
        let r = (sa_core::orig::at_site(0x53AE36, rand) & 0x7F) as i16;
        let c = xi(i32::from(r)).fmul_m(cf(0x3B00_0000)).to_f32();
        let p = rv(this, 4);
        point_lights_add_light(0, p[0], p[1], p[2], 0.0, 0.0, 0.0, 8.0, c, c, 0.0, 0, 0, 0);
        return;
    }
    if !x(rf(this, 0x1C)).fcom(X87::one()).test_parity(0x41) {
        extinguish_call(this);
        return;
    }
    wf(this, 0x1C, x(rf(this, 0x1C)).fsub_m(1.0).to_f32());
    wu(this, 0x18, time_ms().wrapping_add(7000));
    create_fx_sys_for_strength_call(this, this.byte_add(4), GPtr::NULL);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INITIALISE,
    HOOK_MANAGER_UPDATE,
    HOOK_PROCESS_FIRE,
    HOOK_CONSTRUCTOR,
    HOOK_EXTINGUISH,
    HOOK_CREATE_FX_SYS_FOR_STRENGTH,
    HOOK_MANAGER_INIT,
    HOOK_MANAGER_DESTRUCTOR,
    HOOK_MANAGER_CONSTRUCTOR,
    HOOK_GET_NUM_OF_NON_SCRIPT_FIRES,
    HOOK_FIND_NEAREST_FIRE,
    HOOK_IS_SCRIPT_FIRE_EXTINGUISHED,
    HOOK_CLEAR_ALL_SCRIPT_FIRE_FLAGS,
    HOOK_SET_SCRIPT_FIRE_AUDIO,
    HOOK_GET_SCRIPT_FIRE_COORDS,
    HOOK_GET_NUM_FIRES_IN_RANGE,
    HOOK_GET_NUM_FIRES_IN_AREA,
    HOOK_REMOVE_ALL_SCRIPT_FIRES,
    HOOK_MANAGER_SHUTDOWN,
    HOOK_GET_NEXT_FREE_FIRE,
    HOOK_EXTINGUISH_POINT,
    HOOK_REMOVE_SCRIPT_FIRE,
    HOOK_DESTROY_ALL_FX_SYSTEMS,
    HOOK_CREATE_ALL_FX_SYSTEMS,
    HOOK_EXTINGUISH_POINT_WITH_WATER,
    HOOK_START_FIRE_NO_TARGET,
    HOOK_START_SCRIPT_FIRE,
    HOOK_START_FIRE_TARGET,
    HOOK_SET_READY_TO_BURN,
    HOOK_CREEPING_FIRE_UPDATE,
    HOOK_TRY_TO_START_FIRE_AT_COORS,
];
