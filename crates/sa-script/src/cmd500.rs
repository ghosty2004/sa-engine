//! `CRunningScript::ProcessCommands500To599` (0x47E090): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands500To599.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x01F5, 0x01F6, 0x01FA, 0x01FB, 0x0202, 0x0203, 0x0204, 0x0205, 0x0206, 0x0207, 0x0208, 0x0209, 0x020A,
    0x020B, 0x0213, 0x0214, 0x0215, 0x0224, 0x0226, 0x0227, 0x0229, 0x0235, 0x0236, 0x0237, 0x0241, 0x0244,
    0x0247, 0x0248, 0x0249, 0x0253, 0x0254, 0x0256,
];

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// `.rdata` 0x858B8C: 0.5f
const K_HALF: u32 = 0x3F00_0000;
/// `CTheScripts::UsedObjectArray[0].nModelIndex` 0xA44B88 (0x1C bytes per entry; a negative model
/// operand `m` reads `0xA44B88 - m * 0x1C`).
const USED_OBJECT_MODEL: u32 = 0xA44B88;
/// `CStreaming::ms_aInfoForModel[0].m_LoadState` (0x8E4CC0 + 0x10, 0x14 bytes per model).
const MODEL0_LOAD_STATE: u32 = 0x8E4CD0;
/// `CTheScripts::ScriptResourceManager` 0xA485A8.
const SCRIPT_RESOURCE_MANAGER: usize = 0xA485A8;

orig! {
    /// `CRestart::CancelOverrideRestart()` (0x460800)
    fn cancel_override_restart() = cdecl 0x460800 => unported;
    /// `CDarkel::ReadStatus()` (0x43D1E0), AX
    fn darkel_read_status() -> u16 = cdecl 0x43D1E0 => unported;
    /// `CRunningScript::LocateCharCarCommand(int32 command)` (0x487420)
    fn locate_char_car_command(s: Scr, command: u32) = thiscall 0x487420 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90), floats as raw dwords, ST0 (a PC=24
    /// result: exact as f64)
    fn random_float_in_range(lo: u32, hi: u32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CVehicle::BlowUpCar(CEntity*, bool)` (vtable slot 0xA4)
    fn blow_up_car(v: GPtr<u8>, culprit: GPtr<u8>, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `CGangs::SetGangWeapons(int16 gang, int32, int32, int32)` (0x5DE550)
    fn set_gang_weapons(gang: u32, w1: i32, w2: i32, w3: i32) = cdecl 0x5DE550 => unported;
    /// `CPlayerInfo::IsPlayerInRemoteMode()` (0x56DAB0), AL
    fn is_player_in_remote_mode(info: GPtr<u8>) -> u8 = thiscall 0x56DAB0 => unported;
    /// Unnamed cdecl function taking `const CVector*` (0x47E070, a `.HOODLUM` stub; SET_CUTSCENE_OFFSET's
    /// callee, presumably `CCutsceneMgr::SetCutsceneOffset`)
    fn set_cutscene_offset(v: GPtr<u32>) = cdecl 0x47E070 => unported;
    /// `CClock::StoreClock()` (0x52D020)
    fn store_clock() = cdecl 0x52D020 => unported;
    /// `CClock::RestoreClock()` (0x52D070)
    fn restore_clock() = cdecl 0x52D070 => unported;
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    fn collect_next(s: Scr) -> i32 = thiscall 0x464250 => crate::vm::collect_next_parameter_without_increasing_pc;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0); the result is discarded here
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CPickups::GenerateNewOne(CVector pos, uint32 model, uint8 type, uint32 ammo, uint32 money,
    /// bool isEmpty, const char* message)` (0x456F20); the by-value vector as three raw dwords
    fn generate_new_one(x: u32, y: u32, z: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> i32 = cdecl 0x456F20 => unported;
    /// `CPickups::IsPickUpPickedUp(int32)` (0x454B40), AL
    fn is_pick_up_picked_up(h: i32) -> u8 = cdecl 0x454B40 => unported;
    /// `CPickups::RemovePickUp(int32)` (0x4573D0)
    fn remove_pick_up(h: i32) = cdecl 0x4573D0 => unported;
    /// `CStreaming::RequestModel(int32 model, int32 flags)` (0x4087E0)
    fn request_model(model: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90)
    fn set_mission_doesnt_require_model(model: i32) = cdecl 0x409C90 => unported;
    /// `CScriptResourceManager::AddToResourceManager(int32, uint32 type, CRunningScript*)` (0x4704B0)
    fn add_to_resource_manager(m: GPtr<u8>, model: i32, ty: u32, s: Scr) = thiscall 0x4704B0 => unported;
    /// `CScriptResourceManager::RemoveFromResourceManager(int32, uint32 type, CRunningScript*)` (0x470510), AL
    fn remove_from_resource_manager(m: GPtr<u8>, model: i32, ty: u32, s: Scr) -> u8 = thiscall 0x470510 => unported;
}

/// A model operand as these cases decode it: `test v,v; jge keep; imul v,0x1C; mov r,0xA44B88; sub r,v;
/// mov v,[r]` (a negative value indexes `CTheScripts::UsedObjectArray` backwards).
#[inline]
fn model_operand(v: u32) -> u32 {
    if (v as i32) < 0 {
        GPtr::<u32>::from_addr(USED_OBJECT_MODEL.wrapping_sub(v.wrapping_mul(0x1C)) as usize).read()
    } else {
        v
    }
}

/// `CRunningScript::ProcessCommands500To599(int32 command)` (0x47E090, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47E090, conv = thiscall, name = "CRunningScript::ProcessCommands500To599", expect = "64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_500_to_599(s: Scr, command: u32) -> i8 {
    match command {
        // 01F5 GET_PLAYER_CHAR (0x47E150): P0 = PedPool.GetRef(CWorld::Players[P0].m_pPed).
        0x01F5 => {
            collect(s, 1);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let h = ped_get_ref(pool(PED_POOL), ped);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 01F6 CANCEL_OVERRIDE_RESTART (0x47E19B)
        0x01F6 => cancel_override_restart(),
        // 01FA READ_KILL_FRENZY_STATUS (0x47E304): P0 = (u16)CDarkel::ReadStatus() (`movzx`).
        0x01FA => {
            let st = darkel_read_status();
            set_sp(0, u32::from(st));
            store(s, 1);
        }
        // 01FB SQRT (0x47E331): `fld P0; fsqrt; fstp P0` (PC=24).
        0x01FB => {
            collect(s, 1);
            set_sp(0, X87::from_f32(spf(0)).fsqrt().to_f32().to_bits());
            store(s, 1);
        }
        // 0202..0207 LOCATE_CHAR_*_CAR_2D/3D (0x47E367): LocateCharCarCommand(command) (ECX holds the
        // command from the prologue).
        0x0202..=0x0207 => locate_char_car_command(s, command),
        // 0208 GENERATE_RANDOM_FLOAT_IN_RANGE (0x47E385): P0 = GetRandomNumberInRange(P0, P1) (`fstp`).
        0x0208 => {
            collect(s, 2);
            let r = random_float_in_range(sp(0), sp(1));
            set_sp(0, X87::from_f64_exact(r).to_f32().to_bits());
            store(s, 1);
        }
        // 0209 GENERATE_RANDOM_INT_IN_RANGE (0x47E3C8)
        0x0209 => {
            collect(s, 2);
            let r = random_int_in_range(spi(0), spi(1));
            set_sp(0, r as u32);
            store(s, 1);
        }
        // 020A LOCK_CAR_DOORS (0x47E40A): null -> nothing; `m_eDoorLock` (+0x4F8) = P1.
        0x020A => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if !veh.is_null() {
                veh.byte_add(0x4F8).cast::<u32>().write(sp(1));
            }
        }
        // 020B EXPLODE_CAR (0x47E44F): no null check; virtual BlowUpCar(nullptr, false).
        0x020B => {
            collect(s, 1);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            blow_up_car(veh, GPtr::NULL, 0);
        }
        // 0213 CREATE_PICKUP (0x47E57A): P0 model (negative: UsedObjectArray), P1 type, P2..P4 x, y,
        // z. `fld z; fst local; fcomp -100.0; test ah,0x41; jp`: when `z <= -100` (ordered) local z =
        // FindGroundZForCoord(x, y) `fadd [0.5]`. Discarded GetActualPickupIndex of the output operand;
        // P0 = GenerateNewOne((x, y, z), model, (u8)P1, 0, 0, 0, nullptr); StoreParameters(1).
        0x0213 => {
            collect(s, 5);
            let model = model_operand(sp(0));
            let (x, y) = (sp(2), sp(3));
            let st = X87::from_f32(f(sp(4)));
            let mut z = st.to_f32().to_bits();
            if !st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
                z = X87::from_f64_exact(find_ground_z_for_coord(x, y))
                    .fadd_m(f(K_HALF))
                    .to_f32()
                    .to_bits();
            }
            get_actual_pickup_index(collect_next(s));
            let h = generate_new_one(x, y, z, model, sp(1) & 0xFF, 0, 0, 0, 0);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 0214 HAS_PICKUP_BEEN_COLLECTED (0x47E662): flag = IsPickUpPickedUp(P0) != 0 (`setne` into the
        // dead command-argument slot, then pushed).
        0x0214 => {
            collect(s, 1);
            let b = is_pick_up_picked_up(spi(0)) != 0;
            set_flag(s, b);
        }
        // 0215 REMOVE_PICKUP (0x47E6AB)
        0x0215 => {
            collect(s, 1);
            remove_pick_up(spi(0));
        }
        // 0224 SET_CAR_HEALTH (0x47E9E0): `fild P1; fstp local` (int -> f32, round to nearest) before
        // the lookup; no null check; `m_fHealth` (+0x4C0) = local.
        0x0224 => {
            collect(s, 2);
            let h = X87::from_i32(spi(1)).to_f32().to_bits();
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            veh.byte_add(0x4C0).cast::<u32>().write(h);
        }
        // 0226 GET_CHAR_HEALTH (0x47EA2B) / 0227 GET_CAR_HEALTH (0x47EA74, shared tail 0x47EA4B): no null
        // check; P0 = `_ftol2(health)` (ped +0x540, vehicle +0x4C0; EAX = the low dword).
        0x0226 | 0x0227 => {
            collect(s, 1);
            let (e, off) = if command == 0x0226 {
                (ped_at_ref(pool(PED_POOL), spi(0)), 0x540)
            } else {
                (vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), 0x4C0)
            };
            let health = f(e.byte_add(off).cast::<u32>().read());
            set_sp(0, X87::from_f32(health).ftol() as u32);
            store(s, 1);
        }
        // 0229 CHANGE_CAR_COLOUR (0x47EA97): no null check; primary/secondary colour bytes +0x434/+0x435 =
        // (u8)P1, (u8)P2.
        0x0229 => {
            collect(s, 3);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            veh.byte_add(0x434).write(sp(1) as u8);
            veh.byte_add(0x435).write(sp(2) as u8);
        }
        // 0235 SET_GANG_PED_MODELS (0x47ECC2) / 0236 SET_GANG_CAR_MODEL (0x47ECE1): operands read only.
        0x0235 => collect(s, 3),
        0x0236 => collect(s, 2),
        // 0237 SET_GANG_WEAPONS (0x47ED00): CGangs::SetGangWeapons((u16)P0, P1, P2, P3).
        0x0237 => {
            collect(s, 4);
            set_gang_weapons(sp(0) & 0xFFFF, spi(1), spi(2), spi(3));
        }
        // 0241 IS_PLAYER_IN_REMOTE_MODE (0x47EE83): flag = CWorld::Players[P0].IsPlayerInRemoteMode() != 0.
        0x0241 => {
            collect(s, 1);
            let b = is_player_in_remote_mode(player_info(sp(0))) != 0;
            set_flag(s, b);
        }
        // 0244 SET_CUTSCENE_OFFSET (0x47EEB8): 0x47E070(&local (P0, P1, P2)).
        0x0244 => {
            collect(s, 3);
            let v = [sp(0), sp(1), sp(2)];
            set_cutscene_offset(GPtr::from_raw(v.as_ptr().cast_mut()));
        }
        // 0247 REQUEST_MODEL (0x47EF90): RequestModel(m, 0xC) (KEEP_IN_MEMORY | MISSION_REQUIRED);
        // ScriptResourceManager.AddToResourceManager(m, 2, this).
        0x0247 => {
            collect(s, 1);
            let m = model_operand(sp(0)) as i32;
            request_model(m, 0xC);
            add_to_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), m, 2, s);
        }
        // 0248 HAS_MODEL_LOADED (0x47EFDE): flag = ms_aInfoForModel[m].m_LoadState == 1 (LOADED).
        0x0248 => {
            collect(s, 1);
            let m = model_operand(sp(0));
            let st =
                GPtr::<u8>::from_addr(MODEL0_LOAD_STATE.wrapping_add(m.wrapping_mul(20)) as usize).read();
            set_flag(s, st == 1);
        }
        // 0249 MARK_MODEL_AS_NO_LONGER_NEEDED (0x47F03E): SetMissionDoesntRequireModel(m) only when
        // RemoveFromResourceManager(m, 2, this) returns true.
        0x0249 => {
            collect(s, 1);
            let m = model_operand(sp(0)) as i32;
            if remove_from_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), m, 2, s) != 0 {
                set_mission_doesnt_require_model(m);
            }
        }
        // 0253 STORE_CLOCK (0x47F1AC) / 0254 RESTORE_CLOCK (0x47F1C7)
        0x0253 => store_clock(),
        0x0254 => restore_clock(),
        // 0256 IS_PLAYER_PLAYING (0x47F1E2): flag = CWorld::Players[P0].m_nPlayerState (+0xDC) == 0.
        0x0256 => {
            collect(s, 1);
            let st = player_info(sp(0)).byte_add(0xDC).read();
            set_flag(s, st == 0);
        }
        _ => {
            return fallback(
                process_commands_500_to_599_original,
                "CRunningScript::ProcessCommands500To599",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_500_TO_599];
