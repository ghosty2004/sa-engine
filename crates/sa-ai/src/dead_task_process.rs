//! `CTaskSimpleDead::ProcessPed`, executed by the original under SA:MP 0.3.7 (`catalog/samp-priority.csv`)
//! (PORT-AI-8). Ported from the 1.0 US bytes (gta-reversed does not reverse it). Every call to another catalogued
//! function goes through `orig!` at the original address; `_CIasin` (0x821E70) through
//! `sa_core::orig::x87_unary`.
//!
//! `CTaskSimpleDead` (0x10 bytes): +8 time of death (ms), +0xC flags (bit 0 first frame, bit 1 drowning, bit 2
//! blood pool done).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CTimer::m_snPreviousTimeInMilliseconds`
const PREV_TIME_MS: usize = 0xB7CB78;
/// -1.0f
const K_MINUS_ONE: usize = 0x858C1C;
/// 1.0f
const K_ONE: usize = 0x858624;
/// `_CIasin`
const CI_ASIN: usize = 0x821E70;
/// `gpBloodPoolTex`
const BLOOD_POOL_TEX: usize = 0xC40400;

orig! {
    /// `CCarEnterExit::ComputeTargetDoorToExit(const CVehicle*, const CPed*)` (0x64F110, cdecl)
    fn car_enter_exit_compute_target_door_to_exit(v: u32, ped: GPtr<u8>) -> i32 = cdecl 0x64F110 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610, cdecl)
    fn anim_blend_animation(clump: u32, group: u32, anim: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CPed::SetPedState(ePedState)` (0x5E4500, `ret 4`)
    fn ped_set_ped_state(p: GPtr<u8>, s: u32) = thiscall 0x5E4500 => unported;
    /// `FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(i: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60, cdecl)
    fn weapon_info_get(wt: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPed::RemoveWeaponModel(int32 modelIndex)` (0x5E3990, `ret 4`)
    fn ped_remove_weapon_model(p: GPtr<u8>, mi: u32) = thiscall 0x5E3990 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::RemoveWeaponAnims(int32, float delta)` (0x5F0250, `ret 8`)
    fn ped_remove_weapon_anims(p: GPtr<u8>, a: u32, delta: u32) = thiscall 0x5F0250 => unported;
    /// `CPed::CreateDeadPedWeaponPickups` (0x4591D0)
    fn ped_create_dead_ped_weapon_pickups(p: GPtr<u8>) = thiscall 0x4591D0 => unported;
    /// `CPed::CreateDeadPedMoney` (0x4590F0)
    fn ped_create_dead_ped_money(p: GPtr<u8>) = thiscall 0x4590F0 => unported;
    /// `CEventDeadPed::CEventDeadPed(CPed*, bool, uint32 time)` (0x4ADEA0, `ret 0xC`)
    fn event_dead_ped_ctor(e: GPtr<u8>, ped: GPtr<u8>, b: u32, time: u32) -> GPtr<u8> = thiscall 0x4ADEA0 => unported;
    /// `CEventDeadPed::~CEventDeadPed` (0x4ADF70)
    fn event_dead_ped_dtor(e: GPtr<u8>) = thiscall 0x4ADF70 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50, cdecl)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CAccidentManager::GetInstance()` (0x56CF20, cdecl)
    fn accident_manager_get_instance() -> GPtr<u8> = cdecl 0x56CF20 => unported;
    /// `CAccidentManager::ReportAccident(CPed*)` (0x56CE80, `ret 4`)
    fn accident_manager_report_accident(m: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x56CE80 => unported;
    /// `CPed::DeadPedMakesTyresBloody` (0x6B4200)
    fn ped_dead_ped_makes_tyres_bloody(p: GPtr<u8>) = thiscall 0x6B4200 => unported;
    /// `CLocalisation::Blood()` (0x56D230, cdecl)
    fn localisation_blood() -> u8 = cdecl 0x56D230 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(float x, float y, float z, float* level, float*, float*)` (0x6E8580, cdecl)
    fn water_get_water_level_no_waves(x: u32, y: u32, z: u32, out: GPtr<u8>, a: u32, b: u32) -> u8 = cdecl 0x6E8580 => unported;
    /// `CShadows::StoreStaticShadow(uint32 id, uint8 type, RwTexture*, const CVector*, float frontX, float frontY,
    /// float sideX, float sideY, int16 intensity, uint8 r, uint8 g, uint8 b, float zDistance, float scale, float
    /// drawDistance, bool temporary, float upDistance)` (0x70BA00, cdecl)
    #[allow(clippy::too_many_arguments)]
    fn shadows_store_static_shadow(id: u32, ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, i: u32, r: u32, g: u32, b: u32, zd: u32, sc: u32, dd: u32, tmp: u32, up: u32) -> u32 = cdecl 0x70BA00 => unported;
    /// `CShadows::AddPermanentShadow(uint8 type, RwTexture*, const CVector*, float frontX, float frontY, float sideX,
    /// float sideY, int16 intensity, uint8 r, uint8 g, uint8 b, float zDistance, uint32 time, float scale)`
    /// (0x706F60, cdecl)
    #[allow(clippy::too_many_arguments)]
    fn shadows_add_permanent_shadow(ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, i: u32, r: u32, g: u32, b: u32, zd: u32, t: u32, sc: u32) = cdecl 0x706F60 => unported;
}

/// A dword-aligned stack object of `N` dwords, as a game pointer.
fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The f32 constant at `a`.
fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// The ped's position: its matrix +0x30, or the placement at +4 without a matrix.
fn pos_of(ped: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) }
}

/// MSVC's u32 → x87: `fild dword` and + 2^32 (0x858C54) when the sign bit is set.
fn x_of_u32(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(0x858C54)) } else { r }
}

/// Whether player `i` (`FindPlayerPed`) is the ped whose +0x71C is `ped`.
fn player_targets(ped: GPtr<u8>) -> bool {
    if rd::<GPtr<u8>>(find_player_ped(0), 0x71C) == ped {
        return true;
    }
    !find_player_ped(1).is_null() && rd::<GPtr<u8>>(find_player_ped(1), 0x71C) == ped
}

/// `asin` of `v` clamped to [-1, 1] the original's way (`fld -1.0; fcomp`: `-1 > v` gives -1.0; else `fld 1.0;
/// fcomp`: `1 < v` gives 1.0; else `v` through an f32 stack slot), stored as an f32.
fn clamped_asin(v: X87) -> f32 {
    let a = if !x(k(K_MINUS_ONE)).fcom(v).test_any(0x41) {
        -1.0f32
    } else if x(k(K_ONE)).fcom(v).test_parity(0x05) {
        f32_of(v)
    } else {
        1.0
    };
    f32_of(X87::from_f64_exact(sa_core::orig::x87_unary(CI_ASIN, f64::from(a))))
}

/// `CTaskSimpleDead::ProcessPed(CPed*)` (0x630600, `ret 4`; always AL = 0).
///
/// First frame (+0xC bit 0): a ped in a vehicle (flag 0x100 of +0x46C) blends the slumped anim 0xBB (exit doors
/// 10 / 11) or 0xBC at 4.0; ped state 0x37 (dead); the frame flag is cleared; a ped that was standing or ducking
/// (+0x46C bits 0 / 1) remembers it; collision (+0x1C bit 0) is turned off unless drowning, attached (+0x568) or
/// targeted by a player (`FindPlayerPed(i)` +0x71C); +0x540 = 0; the current weapon model is removed and slot 0
/// selected; a non-player loses its weapon anims (-1000.0) and drops weapon pickups and money; a `CEventDeadPed`
/// (drowning flag, time +8) goes to the global event group and the accident manager is told.
///
/// Then a standing drowning ped is laid down (collision off, drowning flag cleared, anim 0x27 at 8.0 without flag
/// bit 3 of +0x2E); a standing ped with collision that is not drowning, attached or targeted loses its collision;
/// either case, or the remembered standing flag, sets the lean angles +0x520 / +0x518 to `asin` of the ped's
/// +0x578 vector dotted with its matrix right / forward (clamped to [-1, 1]).
///
/// `DeadPedMakesTyresBloody`; with blood on and not drowning: peds in the ped's scanner list (intelligence +0x130,
/// 16 entries) closer than the pool radius (0 before 2 s, 0.00015 per ms after that, 0.75 after 7 s) get flag
/// 0x10000000 of +0x46C and +0x750 = 200; after 2 s and before the pool is done (+0xC bit 2), a body in or below
/// still water during this frame's 2 s mark marks it done; else (blood still on) a growing static shadow (id
/// `this + 0x11`, blood texture 0xC40400, red 200) for 5 s, then a permanent one (40 s) and the pool is done.
///
/// Finally a drowning ped loses +0x46C bits 0..1; any other ped gets flag 8 of +0x528 and a zero move speed.
#[hook(addr = 0x630600, conv = thiscall, name = "CTaskSimpleDead::ProcessPed", expect = "6A FF 68 68 04 84 00",
       unverified = "ai8-3", route = "L2", hazards = "ax-al-return,heap-writes,x87-transcendental",
       recs = "weapon-combat,police-gang", note = "the original keeps the elapsed time and the asin arguments in its CPed* argument slot")]
pub fn dead_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let mut standing = false;
    let mut targeted = false;
    if rd::<u8>(this, 0xC) & 1 != 0 {
        if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
            let door = car_enter_exit_compute_target_door_to_exit(rd::<u32>(ped, 0x58C), ped);
            let anim = if door == 10 || door == 11 { 0xBB } else { 0xBC };
            anim_blend_animation(rd::<u32>(ped, 0x18), 0, anim, 0x4080_0000);
        }
        ped_set_ped_state(ped, 0x37);
        wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) & 0xFE);
        let fl = rd::<u32>(ped, 0x46C);
        if fl & 1 != 0 || (fl >> 1) & 1 != 0 {
            standing = true;
        }
        targeted = player_targets(ped);
        if rd::<u8>(this, 0xC) & 2 == 0 && rd::<u32>(ped, 0x568) == 0 && !targeted {
            wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) & 0xFFFF_FFFE);
        }
        let slot = i32::from(rd::<i8>(ped, 0x718)) as isize * 0x1C;
        wr::<u32>(ped, 0x540, 0);
        let info = weapon_info_get(rd::<u32>(ped, slot + 0x5A0), 1);
        ped_remove_weapon_model(ped, rd::<u32>(info, 0xC));
        wr::<u8>(ped, 0x718, 0);
        if ped_is_player(ped) == 0 {
            ped_remove_weapon_anims(ped, 0, 0xC47A_0000);
            ped_create_dead_ped_weapon_pickups(ped);
            ped_create_dead_ped_money(ped);
        }
        let drown = u32::from(rd::<u8>(this, 0xC) >> 1) & 0xFFFF_FF01;
        let mut ev = [0u32; 10];
        event_dead_ped_ctor(local(&mut ev), ped, drown, rd::<u32>(this, 8));
        event_group_add(get_event_global_group(), local(&mut ev), 0);
        accident_manager_report_accident(accident_manager_get_instance(), ped);
        event_dead_ped_dtor(local(&mut ev));
    }
    let fl = rd::<u8>(this, 0xC);
    let lean = if fl & 1 == 0 && fl & 2 != 0 && rd::<u8>(ped, 0x46C) & 1 != 0 {
        wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) & 0xFFFF_FFFE);
        wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) & 0xFD);
        let a = anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x27, 0x4100_0000);
        wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) & 0xF7);
        true
    } else if rd::<u8>(ped, 0x1C) & 1 != 0
        && fl & 2 == 0
        && rd::<u8>(ped, 0x46C) & 1 != 0
        && rd::<u32>(ped, 0x568) == 0
        && !player_targets(ped)
        && !targeted
    {
        wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) & 0xFFFF_FFFE);
        true
    } else {
        standing
    };
    if lean {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let (a, b, c) = (rd::<f32>(ped, 0x578), rd::<f32>(ped, 0x57C), rd::<f32>(ped, 0x580));
        let fdot = f32_of(
            x(c).fmul_m(rd::<f32>(m, 0x18)).fadd(x(b).fmul_m(rd::<f32>(m, 0x14))).fadd(x(a).fmul_m(rd::<f32>(m, 0x10))),
        );
        let rdot = x(c).fmul_m(rd::<f32>(m, 8)).fadd(x(b).fmul_m(rd::<f32>(m, 4))).fadd(x(a).fmul_m(rd::<f32>(m, 0)));
        wr::<f32>(ped, 0x520, clamped_asin(rdot));
        wr::<f32>(ped, 0x518, clamped_asin(x(fdot)));
    }
    ped_dead_ped_makes_tyres_bloody(ped);
    if localisation_blood() != 0 && rd::<u8>(this, 0xC) & 2 == 0 {
        blood_pool(this, ped);
    }
    if rd::<u8>(this, 0xC) & 2 != 0 {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FFFC);
    } else {
        wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
        wr::<u32>(ped, 0x44, 0);
        wr::<u32>(ped, 0x48, 0);
        wr::<u32>(ped, 0x4C, 0);
    }
    0
}

/// The blood pool part of `CTaskSimpleDead::ProcessPed` (0x630954..0x630EAF).
fn blood_pool(this: GPtr<u8>, ped: GPtr<u8>) {
    let t = g::<u32>(TIME_MS).read().wrapping_sub(rd::<u32>(this, 8));
    let r = if t < 2000 {
        x(k(0x858B50))
    } else if t > 7000 {
        x(k(0x85B900))
    } else {
        x_of_u32(t - 2000).fmul_m(k(0x86DECC))
    };
    let r2 = r.fmul(r);
    let list = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x130);
    for i in 0..16 {
        let e = rd::<GPtr<u8>>(list, i * 4);
        if e.is_null() {
            continue;
        }
        let pp = pos_of(ped);
        let ep = pos_of(e);
        let dx = x(rd::<f32>(ep, 0)).fsub_m(rd::<f32>(pp, 0));
        let dy = x(rd::<f32>(ep, 4)).fsub_m(rd::<f32>(pp, 4));
        let dz = x(rd::<f32>(ep, 8)).fsub_m(rd::<f32>(pp, 8));
        let s = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        if !r2.fcom(s).test_any(0x41) {
            wr::<u32>(e, 0x750, 200);
            wr::<u32>(e, 0x46C, rd::<u32>(e, 0x46C) | 0x1000_0000);
        }
    }
    if t <= 2000 || rd::<u8>(this, 0xC) & 4 != 0 {
        return;
    }
    let p = pos_of(ped);
    let mut pos = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    let t2 = t - 2000;
    if t2 <= g::<u32>(TIME_MS).read().wrapping_sub(g::<u32>(PREV_TIME_MS).read()) {
        let mut level = [0u32; 1];
        let p = pos_of(ped);
        if water_get_water_level_no_waves(rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), local(&mut level), 0, 0) != 0
            && !x(rd::<f32>(pos_of(ped), 8)).fcom(x(f32::from_bits(level[0]))).test_parity(0x41)
        {
            wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) | 4);
        }
    }
    if rd::<u8>(this, 0xC) & 4 != 0 || localisation_blood() == 0 {
        return;
    }
    let tex = g::<u32>(BLOOD_POOL_TEX).read();
    if t2 < 5000 {
        let v = x_of_u32(t2);
        let side_y = f32_of(x(k(0x86DEC8)).fmul(v)).to_bits();
        let front_x = f32_of(v.fmul_m(k(0x86DECC))).to_bits();
        shadows_store_static_shadow((this.addr() as u32).wrapping_add(0x11), 1, tex, local(&mut pos), front_x, 0, 0, side_y,
                                    0xFF, 200, 0, 0, 0x4080_0000, 0x3F80_0000, 0x4220_0000, 0, 0);
    } else {
        shadows_add_permanent_shadow(1, tex, local(&mut pos), 0x3F40_0001, 0, 0, 0xBF40_0001, 0xFF, 200, 0, 0,
                                     0x4080_0000, 0x9C40, 0x3F80_0000);
        wr::<u8>(this, 0xC, rd::<u8>(this, 0xC) | 4);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_DEAD_PROCESS_PED];
