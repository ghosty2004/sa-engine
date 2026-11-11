//! PORT-CAM-2: `CCamera::UpdateTargetEntity` (the per-frame choice of the camera's target entity, +0x958).
//! Ported from the 1.0 US bytes; gta-reversed `Camera.cpp` and `docs/re/specs/CAM/CCamera.md` as reading
//! aids (gta-reversed reads the player once and tests `FindPlayerVehicle()` where the bytes read
//! `CWorld::Players[PlayerInFocus].m_pPed->m_pVehicle`; the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_world::world::common::{find_player_ped, find_player_vehicle};
use sa_world::world::cworld::{PLAYER_IN_FOCUS, PLAYER_INFO_SIZE, PLAYERS};

use crate::camctl::{clean_up_old_reference, register_reference};
use crate::common::{ACTIVE_CAM, CAMS, CCAM_SIZE, Fpu, g, rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `.data` f32 0.3: the squared target-vehicle speed above which `UpdateTargetEntity` sets
    /// `m_bPlayerWasOnBike` (+0x27)
    pub static TARGET_MOVING_SPEED_SQR @ 0x8CCC80: f32 = 0.3;
}

/// `CPed::m_nPedState` (+0x530), `CPed::m_pVehicle` (+0x58C), `CPed::m_pIntelligence` (+0x47C).
const PED_STATE: isize = 0x530;
const PED_VEHICLE: isize = 0x58C;
const PED_INTELLIGENCE: isize = 0x47C;
/// `CCam::m_pCamTargetEntity` (+0x21C).
const CAM_TARGET: isize = 0x21C;

orig! {
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390, `.HOODLUM` stub; AL)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    pub fn get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    pub fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CVehicle::CanPedOpenLocks(const CPed*)` (0x6D1E20; AL)
    pub fn can_ped_open_locks(veh: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6D1E20 => unported;
}

/// `CWorld::Players[CWorld::PlayerInFocus].m_pPed` (both re-read at every use, as the original).
fn focus_ped() -> GPtr<u8> {
    let focus = usize::from(PLAYER_IN_FOCUS.get());
    g::<GPtr<u8>>(PLAYERS + focus * PLAYER_INFO_SIZE).read()
}

/// `&m_aCams[m_nActiveCam].m_pCamTargetEntity` (the active cam index re-read each time).
fn cam_target_slot(this: GPtr<u8>) -> GPtr<u8> {
    let i = isize::from(rd::<u8>(this, ACTIVE_CAM));
    this.byte_add(CAMS + i * CCAM_SIZE + CAM_TARGET)
}

/// The inlined reference hand-over on `slot`: `CleanUpOldReference` on the old entity when set, then
/// `slot = new()` (evaluated after the clean-up) and `RegisterReference` on it (no null check).
fn retarget(slot: GPtr<u8>, new: impl FnOnce() -> GPtr<u8>) {
    let old = slot.cast::<GPtr<u8>>().read();
    if !old.is_null() {
        clean_up_old_reference(old, slot);
    }
    let e = new();
    slot.cast::<GPtr<u8>>().write(e);
    register_reference(e, slot);
}

/// `CCamera::UpdateTargetEntity()` (0x50C360, thiscall). In order:
/// - +0x27 = target is a vehicle whose `|moveSpeed|^2` (`(x*x + y*y) + z*z` on the stack) is greater
///   than [`TARGET_MOVING_SPEED_SQR`] (`test ah,0x41; jne`: NaN clears it);
/// - script control (+0xB0 == 2): `ok = 1`; Obbe modes 7 / 8 clear it unless the player's state is 0x3F
///   and target the player ped when it has no vehicle;
/// - leave unless `((+0x2B == 1 || ok) && +0x58 == 0)`, the target is null, or +0x34 == 1;
/// - target the player ped when the player has no vehicle, or (no co-op game and the simplest active
///   task has type 0x3FE); then a ped state 0x3B / 0x39 / 0x35 marks `busy`, else the active cam's
///   target follows (re-registered when different); otherwise target the player vehicle;
/// - `locked = !(focus ped && its vehicle && vehicle->CanPedOpenLocks(FindPlayerPed))`: state 0x3B, not
///   locked, not busy and +0xB4 != 0 target the focus ped's vehicle (the ped when it has none); states
///   0x39 / 0x35, not locked: the vehicle when not busy and +0xB4 != 0, the ped when the vehicle is null;
///   states 0x3D and 0x3A target the ped; a vehicle target with +0xB4 == 0 and player state 0x3F too.
#[hook(addr = 0x50C360, conv = thiscall, name = "CCamera::UpdateTargetEntity", expect = "51 53 56 57 8B F9", this = "CCamera")]
pub fn update_target_entity(this: GPtr<u8>) {
    let slot = this.byte_add(0x958);
    let target = || rd::<GPtr<u8>>(this, 0x958);
    let mut busy = false;
    let mut ok = false;
    wr::<u8>(this, 0x27, 0);
    let e = target();
    if !e.is_null() && rd::<u8>(e, 0x36) & 7 == 2 {
        let mut f = Fpu::new();
        f.fld(rd::<f32>(e, 0x4C));
        f.fld(rd::<f32>(e, 0x48));
        f.fld(rd::<f32>(e, 0x44));
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fcomp_m(TARGET_MOVING_SPEED_SQR.get());
        if !f.jne(0x41) {
            wr::<u8>(this, 0x27, 1);
        }
    }
    if rd::<i32>(this, 0xB0) == 2 {
        let mode = rd::<i32>(this, 0xC34);
        ok = true;
        if mode == 8 || mode == 7 {
            if rd::<i32>(find_player_ped(-1), PED_STATE) != 0x3F {
                ok = false;
            }
            if find_player_vehicle(-1, 0).is_null() {
                retarget(slot, || find_player_ped(-1));
            }
        }
    }
    let go = (rd::<u8>(this, 0x2B) == 1 || ok) && rd::<u8>(this, 0x58) == 0;
    if !go && !target().is_null() && rd::<u8>(this, 0x34) != 1 {
        return;
    }

    let ped_path = if find_player_vehicle(-1, 0).is_null() {
        true
    } else if is_coop_game_going_on() != 0 {
        false
    } else {
        let tm = || rd::<GPtr<u8>>(find_player_ped(-1), PED_INTELLIGENCE).byte_add(4);
        if get_simplest_active_task(tm()).is_null() {
            false
        } else {
            v_task_type(get_simplest_active_task(tm())) == 0x3FE
        }
    };
    if ped_path {
        retarget(slot, || find_player_ped(-1));
        let state = || rd::<i32>(find_player_ped(-1), PED_STATE);
        if state() == 0x3B || state() == 0x39 || state() == 0x35 {
            busy = true;
        } else {
            let cs = cam_target_slot(this);
            let cur = cs.cast::<GPtr<u8>>().read();
            if target() != cur {
                if !cur.is_null() {
                    clean_up_old_reference(cur, cs);
                }
                cam_target_slot(this).cast::<GPtr<u8>>().write(target());
                let cs = cam_target_slot(this);
                register_reference(cs.cast::<GPtr<u8>>().read(), cs);
            }
        }
    } else {
        retarget(slot, || find_player_vehicle(-1, 0));
    }

    let mut locked = true;
    let p = focus_ped();
    if !p.is_null() && !rd::<GPtr<u8>>(p, PED_VEHICLE).is_null() {
        let ped = find_player_ped(-1);
        let veh = rd::<GPtr<u8>>(find_player_ped(-1), PED_VEHICLE);
        if can_ped_open_locks(veh, ped) != 0 {
            locked = false;
        }
    }
    if rd::<i32>(focus_ped(), PED_STATE) == 0x3B && !locked && !busy && rd::<u32>(this, 0xB4) != 0 {
        let old = target();
        if !old.is_null() {
            clean_up_old_reference(old, slot);
        }
        slot.cast::<GPtr<u8>>()
            .write(rd::<GPtr<u8>>(focus_ped(), PED_VEHICLE));
        if rd::<GPtr<u8>>(focus_ped(), PED_VEHICLE).is_null() {
            slot.cast::<GPtr<u8>>().write(find_player_ped(-1));
        }
        register_reference(target(), slot);
    }
    let s = rd::<i32>(focus_ped(), PED_STATE);
    if (s == 0x39 || s == 0x35) && !locked {
        if !busy && rd::<u32>(this, 0xB4) != 0 {
            retarget(slot, || rd::<GPtr<u8>>(focus_ped(), PED_VEHICLE));
        }
        if rd::<GPtr<u8>>(focus_ped(), PED_VEHICLE).is_null() {
            retarget(slot, || find_player_ped(-1));
        }
    }
    if rd::<i32>(focus_ped(), PED_STATE) == 0x3D {
        retarget(slot, || find_player_ped(-1));
    }
    if rd::<i32>(focus_ped(), PED_STATE) == 0x3A {
        retarget(slot, || find_player_ped(-1));
    }
    if rd::<u8>(target(), 0x36) & 7 == 2
        && rd::<u32>(this, 0xB4) == 0
        && rd::<i32>(find_player_ped(-1), PED_STATE) == 0x3F
    {
        retarget(slot, || find_player_ped(-1));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_UPDATE_TARGET_ENTITY];
