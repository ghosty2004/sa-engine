//! `CPlayerInfo` and `CPlayerPedData` helpers (PORT-WEAPON-PED). Ported from the 1.0 US bytes
//! (gta-reversed `PlayerInfo.cpp`, `PlayerPedData.cpp`, `docs/re/specs/PED/CPlayerInfo*.md`).
//!
//! `CPlayerInfo` (0x190): +0 ped, +4 `CPlayerPedData` (0xAC), +0xB0 remote vehicle, +0xB4 special
//! collectable, +0xB8 money, +0xD8 last target vehicle, +0xDC player state, +0xDD remote flag, +0x14C,
//! +0x14F max health, +0x164 skin name[32], +0x184 skin texture, +0x188 parachute model requested,
//! +0x18C parachute timer.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

const _: () = assert!(sa_core::layouts::size::CPlayerInfo == 0x190);
const _: () = assert!(sa_core::layouts::size::CPlayerPedData == 0xAC);

const THE_CAMERA: usize = 0xB6F028;
/// `MODEL_GUN_PARA` (0x173)
const MI_PARACHUTE: i32 = 0x173;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn register_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571B70 => unported;
    /// `CGeneral::GetATanOfXY(float x, float y)` (0x53CC70)
    pub fn get_atan_of_xy(x: f32, y: f32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0)
    pub fn get_fat_and_muscle_modifier(a: i32) -> f64 = cdecl 0x559AF0 => unported;
    /// `CPlayerPedData::SetInitialState` (0x56F210)
    pub fn ped_data_set_initial_state(d: GPtr<u8>) = thiscall 0x56F210 => set_initial_state;
    /// `RwTextureDestroy` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `CPlayerSkin::GetSkinTexture(const char*)` (0x6FFA10)
    pub fn get_skin_texture(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x6FFA10 => unported;
    /// CRT `strncpy` (0x821F40)
    pub fn crt_strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CGameLogic::IsCoopGameGoingOn` (0x441390)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CCamera::CalculateGroundHeight(eGroundHeightType)` (0x514B80)
    pub fn camera_calculate_ground_height(cam: GPtr<u8>, ty: i32) -> f64 = thiscall 0x514B80 => unported;
    /// `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CWanted::Initialise` (0x562390)
    pub fn wanted_initialise(w: GPtr<u8>) = thiscall 0x562390 => crate::wanted::initialise;
    /// `CPedClothesDesc::CPedClothesDesc` (0x5A8020)
    pub fn clothes_desc_ctor(d: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A8020 => crate::clothes::desc_constructor;
    /// `CPedClothesDesc::Initialise` (0x5A78F0)
    pub fn clothes_desc_initialise(d: GPtr<u8>) = thiscall 0x5A78F0 => crate::clothes::desc_initialise;
}

/// `CPlayerInfo::IsPlayerInRemoteMode` (0x56DAB0, AL): a remote vehicle or the remote flag (+0xDD).
#[hook(addr = 0x56DAB0, conv = thiscall, name = "CPlayerInfo::IsPlayerInRemoteMode", expect = "8B 81 B0 00 00 00", this = "CPlayerInfo")]
pub fn is_player_in_remote_mode(this: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0xB0) != 0 {
        return 1;
    }
    (rd::<u8>(this, 0xDD) != 0) as u8
}

fn vehicle_or_ped(this: GPtr<u8>) -> GPtr<u8> {
    let p = rd::<GPtr<u8>>(this, 0);
    if rd::<u32>(p, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(p, 0x58C);
        if !v.is_null() {
            return v;
        }
    }
    p
}

fn copy3(dst: GPtr<u32>, src: GPtr<u32>) {
    dst.write(src.read());
    dst.wrapping_index(1).write(src.wrapping_index(1).read());
    dst.wrapping_index(2).write(src.wrapping_index(2).read());
}

/// `CPlayerInfo::GetSpeed_Hook(CVector* out)` (0x56DF50, `ret 4`): the vehicle's move speed when the ped
/// is in one, else the ped's; returns `out`.
#[hook(addr = 0x56DF50, conv = thiscall, name = "CPlayerInfo::GetSpeed_Hook", expect = "8B 01 8B 88 6C 04 00 00", this = "CPlayerInfo")]
pub fn get_speed(this: GPtr<u8>, out: GPtr<u32>) -> GPtr<u32> {
    copy3(out, vehicle_or_ped(this).byte_add(0x44).cast());
    out
}

/// `CPlayerInfo::GetPos_Hook(CVector* out)` (0x56DFB0, entry `mov eax, [ecx]; nop; jmp 0x4013C9`, `ret
/// 4`): the vehicle's or the ped's position; returns `out`.
#[hook(addr = 0x56DFB0, conv = thiscall, name = "CPlayerInfo::GetPos_Hook", expect = "8B 01 90 E9 11 34 E9 FF", this = "CPlayerInfo")]
pub fn get_pos(this: GPtr<u8>, out: GPtr<u32>) -> GPtr<u32> {
    copy3(out, ent_pos(vehicle_or_ped(this)).cast());
    out
}

/// `CPlayerInfo::IsRestartingAfterDeath` (0x56E550, AL)
#[hook(addr = 0x56E550, conv = thiscall, name = "CPlayerInfo::IsRestartingAfterDeath", expect = "80 B9 DC 00 00 00 01", this = "CPlayerInfo")]
pub fn is_restarting_after_death(this: GPtr<u8>) -> u8 {
    (rd::<u8>(this, 0xDC) == 1) as u8
}

/// `CPlayerInfo::IsRestartingAfterArrest` (0x56E560, AL)
#[hook(addr = 0x56E560, conv = thiscall, name = "CPlayerInfo::IsRestartingAfterArrest", expect = "80 B9 DC 00 00 00 02", this = "CPlayerInfo")]
pub fn is_restarting_after_arrest(this: GPtr<u8>) -> u8 {
    (rd::<u8>(this, 0xDC) == 2) as u8
}

/// `CPlayerInfo::IsRestartingAfterMissionFailed` (0x56E570, AL)
#[hook(addr = 0x56E570, conv = thiscall, name = "CPlayerInfo::IsRestartingAfterMissionFailed", expect = "80 B9 DC 00 00 00 03", this = "CPlayerInfo")]
pub fn is_restarting_after_mission_failed(this: GPtr<u8>) -> u8 {
    (rd::<u8>(this, 0xDC) == 3) as u8
}

/// `CPlayerInfo::AddHealth(int32)` (0x56EAB0, `ret 4`): `v = health + amount` capped at the max health
/// (u8 +0x14F, as f32); health only rises (a capped value below the current health keeps it).
#[hook(addr = 0x56EAB0, conv = thiscall, name = "CPlayerInfo::AddHealth", expect = "DB 44 24 04 0F B6 81 4F 01 00 00", this = "CPlayerInfo")]
pub fn add_health(this: GPtr<u8>, amount: i32) {
    let ped = rd::<GPtr<u8>>(this, 0);
    let h = rd::<f32>(ped, 0x540);
    let sum = xi(amount).fadd_m(h);
    let max = xi(i32::from(rd::<u8>(this, 0x14F))).to_f32();
    let below = !sum.fcom(x(max)).test_parity(5);
    let v = if below { sum } else { x(max) };
    if !v.fcom(x(h)).test_parity(5) {
        wr::<f32>(ped, 0x540, h);
        return;
    }
    if below {
        wr::<f32>(ped, 0x540, sum.to_f32());
    } else {
        wr::<f32>(ped, 0x540, max);
    }
}

/// `CPlayerInfo::EvaluateCarPosition(CEntity* car, CPed* ped, float pedToVehDist, float* outDistance,
/// CVehicle** outVehicle)` (0x56DAD0, `ret 0x14`): score `(10 - dist) * (1 - |angle| / 2pi)` where angle =
/// the ped's heading (`GetATanOfXY` of its forward vector, stored as f32) minus the direction to the car
/// (`GetATanOfXY`, used from ST0), wrapped into [-pi, pi]; a score >= the best (ordered) replaces it and
/// the vehicle.
#[hook(addr = 0x56DAD0, conv = thiscall, name = "CPlayerInfo::EvaluateCarPosition", expect = "53 55 56 8B 74 24 14", this = "CPlayerInfo")]
pub fn evaluate_car_position(_this: GPtr<u8>, car: GPtr<u8>, ped: GPtr<u8>, dist: f32, out: GPtr<f32>, out_veh: GPtr<GPtr<u8>>) {
    const PI: f32 = f32::from_bits(0x4049_0FDB);
    const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let heading = X87::from_f64_exact(get_atan_of_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14))).to_f32();
    let c = ent_pos(car);
    let p = ent_pos(ped);
    let dy = x(c.wrapping_index(1).read()).fsub_m(p.wrapping_index(1).read()).to_f32();
    let dx = x(c.read()).fsub_m(p.read()).to_f32();
    let a = X87::from_f64_exact(get_atan_of_xy(dx, dy));
    let mut d = x(heading).fsub(a);
    if !d.fcom(x(PI)).test_any(0x41) {
        loop {
            d = d.fsub_m(TWO_PI);
            if d.fcom(x(PI)).test_any(0x41) {
                break;
            }
        }
    }
    if !d.fcom(x(-PI)).test_parity(5) {
        loop {
            d = d.fadd_m(TWO_PI);
            if d.fcom(x(-PI)).test_parity(5) {
                break;
            }
        }
    }
    if !d.fcom(x(0.0)).test_parity(5) {
        d = d.fchs();
    }
    let k = x(1.0).fsub(d.fmul_m(f32::from_bits(0x3E22_F983)));
    let s = k.fmul(x(10.0).fsub_m(dist));
    if !s.fcom(x(out.read())).test_any(1) {
        out.write(s.to_f32());
        out_veh.write(car);
    }
}

/// `CPlayerInfo::SetLastTargetVehicle(CVehicle*)` (0x56DA80, `ret 4`)
#[hook(addr = 0x56DA80, conv = thiscall, name = "CPlayerInfo::SetLastTargetVehicle", expect = "56 8D B1 D8 00 00 00", this = "CPlayerInfo")]
pub fn set_last_target_vehicle(this: GPtr<u8>, v: GPtr<u8>) {
    let r: GPtr<GPtr<u8>> = this.byte_add(0xD8).cast();
    let e = r.read();
    if !e.is_null() {
        clean_up_old_reference(e, r);
    }
    r.write(v);
    if !v.is_null() {
        register_reference(v, r);
    }
}

/// `CPlayerPedData::SetInitialState` (0x56F210)
#[hook(addr = 0x56F210, conv = thiscall, name = "CPlayerPedData::SetInitialState", expect = "83 EC 0C 53 33 DB", this = "CPlayerPedData")]
pub fn set_initial_state(this: GPtr<u8>) {
    for o in [8, 0xC, 0x10, 0x74, 0x78] {
        wr::<u32>(this, o, 0);
    }
    let f = (rd::<u32>(this, 0x34) & 0xFFFF_F7B0) | 0x1010;
    wr::<u32>(this, 0x14, 0);
    wr::<u8>(this, 0x20, 0);
    wr::<u32>(this, 0x24, 0);
    wr::<u8>(this, 0x21, 0);
    for o in [0x30, 0x28, 0x2C, 0x3C] {
        wr::<u32>(this, o, 0);
    }
    wr::<u8>(this, 0x40, 0);
    wr::<u8>(this, 0x41, 0);
    wr::<u8>(this, 0x42, 0);
    wr::<u8>(this, 0x43, 0x63);
    for o in [0x50, 0x54, 0x58, 0x64, 0x68, 0x6C, 0x70, 0x7C, 0x80] {
        wr::<u32>(this, o, 0);
    }
    wr::<u8>(this, 0x84, 0);
    wr::<u8>(this, 0x85, 0);
    wr::<u8>(this, 0x86, 0);
    wr::<u16>(this, 0x88, 0);
    wr::<u16>(this, 0x8A, 0);
    wr::<u32>(this, 0x34, f);
    wr::<u8>(this, 0x8D, 1);
    wr::<u8>(this, 0x8C, 0);
    wr::<f32>(this, 0x18, X87::from_f64_exact(get_fat_and_muscle_modifier(7)).to_f32());
    wr::<u32>(this, 0x1C, 0);
    wr::<f32>(this, 0x44, X87::from_f64_exact(get_fat_and_muscle_modifier(8)).to_f32());
    let f = rd::<u32>(this, 0x34) & 0xFFFF_FF7F;
    for o in [0x94, 0x98, 0xA4, 0xA8] {
        wr::<u32>(this, o, 0);
    }
    wr::<u32>(this, 0x34, f);
    wr::<u32>(this, 0xA0, 0x8000_0000);
    wr::<u32>(this, 0x9C, 0xFFFF_FFFF);
}

/// `CPlayerInfo::DeletePlayerSkin` (0x56EA80)
#[hook(addr = 0x56EA80, conv = thiscall, name = "CPlayerInfo::DeletePlayerSkin", expect = "56 8B F1 8B 86 84 01 00 00", this = "CPlayerInfo")]
pub fn delete_player_skin(this: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(this, 0x184);
    if !t.is_null() {
        rw_texture_destroy(t);
        wr::<u32>(this, 0x184, 0);
    }
}

fn ped_data_ctor_fields(d: GPtr<u8>) {
    wr::<u32>(d, 0, 0);
    wr::<u32>(d, 4, 0);
    wr::<u32>(d, 0x48, 0);
    wr::<u32>(d, 0x4C, 0);
    ped_data_set_initial_state(d);
    wr::<u32>(d, 0x60, 0);
    wr::<u32>(d, 0x94, 0);
    wr::<u32>(d, 0x98, 0);
    wr::<u32>(d, 0xA0, 0x8000_0000);
}

/// `CPlayerPedData::CPlayerPedData` (0x56F810): returns `this`.
#[hook(addr = 0x56F810, conv = thiscall, name = "CPlayerPedData::Constructor", expect = "56 8B F1 57 33 FF", this = "CPlayerPedData")]
pub fn ped_data_constructor(this: GPtr<u8>) -> GPtr<u8> {
    ped_data_ctor_fields(this);
    this
}

/// `CPlayerInfo::CPlayerInfo` (0x571920): the player data inline at +4, skin texture, parachute fields;
/// returns `this`.
#[hook(addr = 0x571920, conv = thiscall, name = "CPlayerInfo::Constructor", expect = "53 56 57 8B F9", this = "CPlayerInfo")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    ped_data_ctor_fields(this.byte_add(4));
    wr::<u32>(this, 0x184, 0);
    wr::<u8>(this, 0x188, 0);
    wr::<u32>(this, 0x18C, 0);
    this
}

/// `CPlayerInfo::LoadPlayerSkin` (0x56F7D0)
#[hook(addr = 0x56F7D0, conv = thiscall, name = "CPlayerInfo::LoadPlayerSkin", expect = "56 8B F1 8B 86 84 01 00 00", this = "CPlayerInfo")]
pub fn load_player_skin(this: GPtr<u8>) {
    delete_player_skin(this);
    wr::<GPtr<u8>>(this, 0x184, get_skin_texture(this.byte_add(0x164)));
}

/// `CPlayerInfo::SetPlayerSkin(const char*)` (0x5717F0, `ret 4`): `strncpy(skin, name, 32)` then reload.
#[hook(addr = 0x5717F0, conv = thiscall, name = "CPlayerInfo::SetPlayerSkin", expect = "8B 44 24 04 56", this = "CPlayerInfo")]
pub fn set_player_skin(this: GPtr<u8>, name: GPtr<u8>) {
    crt_strncpy(this.byte_add(0x164), name, 0x20);
    delete_player_skin(this);
    wr::<GPtr<u8>>(this, 0x184, get_skin_texture(this.byte_add(0x164)));
}

fn release_parachute(this: GPtr<u8>) {
    if rd::<u8>(this, 0x188) != 0 {
        set_model_is_deletable(MI_PARACHUTE);
        wr::<u8>(this, 0x188, 0);
        wr::<u32>(this, 0x18C, 0);
    }
}

/// `CPlayerInfo::StreamParachuteWeapon(bool unk)` (0x56EB30, `ret 4`): keeps the parachute model requested
/// while the player flies a plane/heli (types 4/3) more than 50 above the ground (5 s timer, counted down
/// by `_ftol(timestep * 0.02 * 1000)`), releases it otherwise; in a co-op game it is released unless the
/// flag is set.
#[hook(addr = 0x56EB30, conv = thiscall, name = "CPlayerInfo::StreamParachuteWeapon", expect = "56 8B F1 E8 58 28 ED FF", this = "CPlayerInfo")]
pub fn stream_parachute_weapon(this: GPtr<u8>, unk: u32) {
    let coop = is_coop_game_going_on();
    let flag = unk as u8;
    if coop != 0 {
        if flag == 0 {
            release_parachute(this);
        }
        return;
    }
    if flag == 0 {
        release_parachute(this);
        return;
    }
    let ped = rd::<GPtr<u8>>(this, 0);
    if !ped.is_null() && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && matches!(rd::<i32>(v, 0x594), 3 | 4) {
            let t = ftol32(x(sa_core::timer::TIME_STEP.get()).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(1000.0)) as u32;
            let left = rd::<u32>(this, 0x18C);
            if left > t {
                wr::<u32>(this, 0x18C, left - t);
            } else {
                let z = ent_pos(v).wrapping_index(2).read();
                let h = X87::from_f64_exact(camera_calculate_ground_height(g::<u8>(THE_CAMERA), 0));
                let above = x(z).fsub(h);
                if !above.fcom(x(50.0)).test_any(0x41) {
                    wr::<u32>(this, 0x18C, 0x1388);
                } else {
                    wr::<u32>(this, 0x18C, 0);
                }
            }
        }
    }
    if rd::<u32>(this, 0x18C) == 0 {
        release_parachute(this);
        return;
    }
    request_model(MI_PARACHUTE, 4);
    wr::<u8>(this, 0x188, 1);
}

/// `CPlayerPedData::AllocateData` (0x571880): allocates the wanted (0x29C, police-scanner audio entity
/// vtable and +0x22C set) and the clothes description (0x78) when missing, then initialises both. The SEH
/// frame around the clothes constructor is not built (no exception can occur in it).
#[hook(addr = 0x571880, conv = thiscall, name = "CPlayerPedData::AllocateData", expect = "6A FF 68 EB CD 83 00", this = "CPlayerPedData")]
pub fn allocate_data(this: GPtr<u8>) {
    if rd::<u32>(this, 0) == 0 {
        let w = operator_new(0x29C);
        if !w.is_null() {
            wr::<u32>(w, 0x22C, 0);
            wr::<u32>(w, 0x21C, 0x85F368);
        }
        wr::<GPtr<u8>>(this, 0, w);
    }
    wanted_initialise(rd::<GPtr<u8>>(this, 0));
    if rd::<u32>(this, 4) == 0 {
        let c = operator_new(0x78);
        let c = if c.is_null() { c } else { clothes_desc_ctor(c) };
        wr::<GPtr<u8>>(this, 4, c);
    }
    clothes_desc_initialise(rd::<GPtr<u8>>(this, 4));
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_PLAYER_IN_REMOTE_MODE,
    HOOK_GET_SPEED,
    HOOK_GET_POS,
    HOOK_IS_RESTARTING_AFTER_DEATH,
    HOOK_IS_RESTARTING_AFTER_ARREST,
    HOOK_IS_RESTARTING_AFTER_MISSION_FAILED,
    HOOK_ADD_HEALTH,
    HOOK_EVALUATE_CAR_POSITION,
    HOOK_SET_LAST_TARGET_VEHICLE,
    HOOK_SET_INITIAL_STATE,
    HOOK_DELETE_PLAYER_SKIN,
    HOOK_PED_DATA_CONSTRUCTOR,
    HOOK_CONSTRUCTOR,
    HOOK_LOAD_PLAYER_SKIN,
    HOOK_SET_PLAYER_SKIN,
    HOOK_STREAM_PARACHUTE_WEAPON,
    HOOK_ALLOCATE_DATA,
];
