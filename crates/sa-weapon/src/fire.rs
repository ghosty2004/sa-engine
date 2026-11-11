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

pub const HOOKS: &[HookReg] = &[
    HOOK_INITIALISE,
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
