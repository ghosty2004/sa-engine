//! `CRunningScript::ProcessCommands0To99` (0x465E60): the basic commands (WAIT, GOTO, variable
//! assignment and arithmetic, comparisons, GOTO_IF_FALSE, TERMINATE_THIS_SCRIPT, START_NEW_SCRIPT,
//! GOSUB/RETURN, LINE, CREATE_PLAYER, SHAKE_CAM). Ported as a whole switch from the 1.0 US bytes
//! (jump table 0x466C50, 100 entries; gta-reversed `BasicCommands.cpp` as reading aid).
//!
//! Every case calls the VM helpers (`GetPointerToScriptVariable`, `CollectParameters`,
//! `StoreParameters`, `UpdateCompareFlag`, `UpdatePC`, `ReadParametersForNewlyStartedScript`) through
//! `orig!` in the original's order, so their hooks govern the calls from here too. Float arithmetic is
//! the x87 sequence the case compiles to (`fld a; fop b; fstp`, PC=24, memory-operand forms);
//! float comparisons are `fcomp` + `fnstsw` tests: `test ah,0x41; je` = greater, `test ah,1; je` =
//! greater or equal, `test ah,0x44; jnp` = equal (all false when unordered), i.e. the IEEE relations.
//! Integer division is `cdq; idiv` (a zero divisor or INT_MIN / -1 faults in the original; the port
//! panics there).
//!
//! The float compares store the 0/1 flag in the low byte of the command argument's stack slot
//! (`mov byte [esp+0x2c], 1`) before pushing it; the slot is dead after `ret 4`, the port does not
//! write it.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::vm::{
    self, Scr, collect_parameters_call as collect, get_pointer_to_script_variable_call as var_ptr, param,
    read_parameters_for_newly_started_script_call, ss, store_parameters_call,
    update_compare_flag_call as flag, update_pc_call,
};

sa_core::globals! {
    list GLOBALS;
    /// `CTheScripts::bAlreadyRunningAMissionScript`
    pub static ALREADY_RUNNING_A_MISSION_SCRIPT @ 0xA444B1: u8 = 0;
    /// `CTheScripts::pIdleScripts`
    pub static IDLE_SCRIPTS @ 0xA8B428: GPtr<u8> = GPtr::NULL;
    /// `CTheScripts::pActiveScripts`
    pub static ACTIVE_SCRIPTS @ 0xA8B42C: GPtr<u8> = GPtr::NULL;
}

/// `TheCamera` (owned by the camera subsystem; addressed directly as sa-stream does).
const THE_CAMERA: usize = 0xB6F028;
/// `CStreaming::ms_aInfoForModel[0].m_LoadState` (0x8E4CC0 + 0x10; owned by sa-stream's streaming).
const MODEL0_LOAD_STATE: usize = 0x8E4CD0;
/// "player" (.rdata 0x859D54)
const PLAYER_NAME: usize = 0x859D54;
/// `.rdata` 0x858CDC: 0.001f (SHAKE_CAM scale)
const K_SHAKE_SCALE: u32 = 0x3A83_126F;
/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;

orig! {
    /// `CCamera::CamShakeNoPos(CCamera*, float)` (0x50A970, cdecl)
    pub fn cam_shake_no_pos(cam: GPtr<u8>, strength: f32) = cdecl 0x50A970 => unported;
    /// `CTheScripts::StartNewScript(uint8* ip)` (0x464C20)
    pub fn start_new_script(ip: GPtr<u8>) -> Scr = cdecl 0x464C20 => crate::script_brains::start_new_script;
    /// `CRunningScript::RemoveScriptFromList(CRunningScript**)` (0x464BD0)
    pub fn remove_script_from_list(s: Scr, list: GPtr<GPtr<u8>>) = thiscall 0x464BD0 => unported;
    /// `CRunningScript::AddScriptToList(CRunningScript**)` (0x464C00)
    pub fn add_script_to_list(s: Scr, list: GPtr<GPtr<u8>>) = thiscall 0x464C00 => unported;
    /// `CRunningScript::ShutdownThisScript()` (0x465AA0)
    pub fn shutdown_this_script(s: Scr) = thiscall 0x465AA0 => unported;
    /// `CStreaming::RequestSpecialModel(int32 slot, const char* name, int32 flags)` (0x409D10)
    pub fn request_special_model(slot: i32, name: GPtr<u8>, flags: i32) = cdecl 0x409D10 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn load_all_requested_models(only_priority: i32) = cdecl 0x40EA10 => unported;
    /// `CPlayerPed::SetupPlayerPed(int32)` (0x60D790)
    pub fn setup_player_ped(idx: i32) = cdecl 0x60D790 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    pub fn set_char_created_by(ped: GPtr<u8>, by: i32) = thiscall 0x5E47E0 => unported;
    /// `CPlayerPed::DeactivatePlayerPed(int32)` (0x609520)
    pub fn deactivate_player_ped(idx: i32) = cdecl 0x609520 => unported;
    /// `CPlayerPed::ReactivatePlayerPed(int32)` (0x609540)
    pub fn reactivate_player_ped(idx: i32) = cdecl 0x609540 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), ST0 (read as f64: exact)
    pub fn find_ground_z_for_coord(x: f32, y: f32) -> f64 = cdecl 0x569660 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0), ST0
    pub fn get_distance_from_centre_of_mass_to_base_of_model(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CPlaceable::SetPosn(float, float, float)` (0x420B80)
    pub fn set_posn(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x420B80 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    pub fn clear_space_for_mission_entity(pos: GPtr<f32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    pub fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimplePlayerOnFoot::CTaskSimplePlayerOnFoot()` (0x685750), returns this
    pub fn task_simple_player_on_foot_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x685750 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0)
    pub fn task_manager_set_task(m: GPtr<u8>, task: GPtr<u8>, slot: i32, force: i32) = thiscall 0x681AF0 => unported;
}

#[inline]
fn sp0() -> u32 {
    param(0).read()
}

#[inline]
fn f(v: u32) -> f32 {
    f32::from_bits(v)
}

/// `fld a; fop b; fstp` with `b` a memory operand.
#[inline]
fn fop(a: u32, b: u32, op: fn(X87, f32) -> X87) -> u32 {
    op(X87::from_f32(f(a)), f(b)).to_f32().to_bits()
}

/// `p = GetPointerToScriptVariable(t); CollectParameters(1)` (the `VAR op= VALUE` forms).
#[inline]
fn var_then_value(s: Scr, t: i32) -> (GPtr<u32>, u32) {
    let p = var_ptr(s, t);
    collect(s, 1);
    (p, sp0())
}

/// `CollectParameters(1); p = GetPointerToScriptVariable(t)` (the `VALUE cmp VAR` forms).
#[inline]
fn value_then_var(s: Scr, t: i32) -> (u32, GPtr<u32>) {
    collect(s, 1);
    let v = sp0();
    (v, var_ptr(s, t))
}

/// Two variable operands, `a` first.
#[inline]
fn two_vars(s: Scr, ta: i32, tb: i32) -> (GPtr<u32>, GPtr<u32>) {
    let a = var_ptr(s, ta);
    let b = var_ptr(s, tb);
    (a, b)
}

#[inline]
fn gt(a: u32, b: u32) -> bool {
    (a as i32) > (b as i32)
}

#[inline]
fn ge(a: u32, b: u32) -> bool {
    (a as i32) >= (b as i32)
}

#[inline]
fn fgt(a: u32, b: u32) -> bool {
    f(a) > f(b)
}

#[inline]
fn fge(a: u32, b: u32) -> bool {
    f(a) >= f(b)
}

#[inline]
fn feq(a: u32, b: u32) -> bool {
    f(a) == f(b)
}

#[inline]
fn set_flag(s: Scr, b: bool) {
    flag(s, u32::from(b));
}

/// `CRunningScript::ProcessCommands0To99(int32 command)` (0x465E60, thiscall, `ret 4`): AL = 0
/// continue, 1 wait (WAIT, TERMINATE_THIS_SCRIPT), -1 for the 15 opcodes without a case (003D..0041,
/// 0047..004C, 0054..0057). The default is taken for any command above 0x63 too.
#[hook(addr = 0x465E60, conv = thiscall, name = "CRunningScript::ProcessCommands0To99", expect = "64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_0_to_99(s: Scr, command: u32) -> i8 {
    match command {
        // NOP
        0x00 => {}
        // WAIT
        0x01 => {
            collect(s, 1);
            let t = sa_core::timer::TIME_MS.get().wrapping_add(sp0());
            s.byte_add(vm::WAKE_TIME).cast::<u32>().write(t);
            return 1;
        }
        // GOTO
        0x02 => {
            collect(s, 1);
            update_pc_call(s, sp0() as i32);
        }
        // SHAKE_CAM: fild SP0; fmul 0.001f; fstp
        0x03 => {
            collect(s, 1);
            let v = X87::from_i32(sp0() as i32).fmul_m(f(K_SHAKE_SCALE)).to_f32();
            cam_shake_no_pos(GPtr::from_addr(THE_CAMERA), v);
        }
        // SET_VAR_INT, SET_VAR_FLOAT, SET_LVAR_INT, SET_LVAR_FLOAT
        0x04..=0x07 => {
            let t = if command <= 0x05 { 2 } else { 1 };
            let (p, v) = var_then_value(s, t);
            p.write(v);
        }
        // ADD_VAL_TO_INT_VAR / _INT_LVAR
        0x08 | 0x0A => {
            let (p, v) = var_then_value(s, if command == 0x08 { 2 } else { 1 });
            p.write(p.read().wrapping_add(v));
        }
        // ADD_VAL_TO_FLOAT_VAR / _FLOAT_LVAR: fld SP0; fadd [p]
        0x09 | 0x0B => {
            let (p, v) = var_then_value(s, if command == 0x09 { 2 } else { 1 });
            p.write(fop(v, p.read(), X87::fadd_m));
        }
        // SUB_VAL_FROM_INT_VAR / _INT_LVAR
        0x0C | 0x0E => {
            let (p, v) = var_then_value(s, if command == 0x0C { 2 } else { 1 });
            p.write(p.read().wrapping_sub(v));
        }
        // SUB_VAL_FROM_FLOAT_VAR / _FLOAT_LVAR: fld [p]; fsub SP0
        0x0D | 0x0F => {
            let (p, v) = var_then_value(s, if command == 0x0D { 2 } else { 1 });
            p.write(fop(p.read(), v, X87::fsub_m));
        }
        // MULT_INT_VAR_BY_VAL / MULT_INT_LVAR_BY_VAL
        0x10 | 0x12 => {
            let (p, v) = var_then_value(s, if command == 0x10 { 2 } else { 1 });
            p.write((p.read() as i32).wrapping_mul(v as i32) as u32);
        }
        // MULT_FLOAT_VAR_BY_VAL / _FLOAT_LVAR: fld SP0; fmul [p]
        0x11 | 0x13 => {
            let (p, v) = var_then_value(s, if command == 0x11 { 2 } else { 1 });
            p.write(fop(v, p.read(), X87::fmul_m));
        }
        // DIV_INT_VAR_BY_VAL / DIV_INT_LVAR_BY_VAL: cdq; idiv
        0x14 | 0x16 => {
            let (p, v) = var_then_value(s, if command == 0x14 { 2 } else { 1 });
            p.write(idiv(p.read(), v));
        }
        // DIV_FLOAT_VAR_BY_VAL / _FLOAT_LVAR: fld [p]; fdiv SP0
        0x15 | 0x17 => {
            let (p, v) = var_then_value(s, if command == 0x15 { 2 } else { 1 });
            p.write(fop(p.read(), v, X87::fdiv_m));
        }
        // IS_INT_VAR_GREATER_THAN_NUMBER / IS_INT_LVAR_GREATER_THAN_NUMBER
        0x18 | 0x19 => {
            let (p, v) = var_then_value(s, if command == 0x18 { 2 } else { 1 });
            set_flag(s, gt(p.read(), v));
        }
        // IS_NUMBER_GREATER_THAN_INT_VAR / _INT_LVAR
        0x1A | 0x1B => {
            let (v, p) = value_then_var(s, if command == 0x1A { 2 } else { 1 });
            set_flag(s, gt(v, p.read()));
        }
        // IS_INT_(L)VAR_GREATER_THAN_INT_(L)VAR
        0x1C..=0x1F => {
            let (ta, tb) = pair(command - 0x1C);
            let (a, b) = two_vars(s, ta, tb);
            set_flag(s, gt(a.read(), b.read()));
        }
        // IS_FLOAT_VAR / _LVAR_GREATER_THAN_NUMBER
        0x20 | 0x21 => {
            let (p, v) = var_then_value(s, if command == 0x20 { 2 } else { 1 });
            set_flag(s, fgt(p.read(), v));
        }
        // IS_NUMBER_GREATER_THAN_FLOAT_VAR / _LVAR
        0x22 | 0x23 => {
            let (v, p) = value_then_var(s, if command == 0x22 { 2 } else { 1 });
            set_flag(s, fgt(v, p.read()));
        }
        // IS_FLOAT_(L)VAR_GREATER_THAN_FLOAT_(L)VAR
        0x24..=0x27 => {
            let (ta, tb) = pair(command - 0x24);
            let (a, b) = two_vars(s, ta, tb);
            set_flag(s, fgt(a.read(), b.read()));
        }
        // IS_INT_VAR / _LVAR_GREATER_OR_EQUAL_TO_NUMBER
        0x28 | 0x29 => {
            let (p, v) = var_then_value(s, if command == 0x28 { 2 } else { 1 });
            set_flag(s, ge(p.read(), v));
        }
        // IS_NUMBER_GREATER_OR_EQUAL_TO_INT_VAR / _LVAR
        0x2A | 0x2B => {
            let (v, p) = value_then_var(s, if command == 0x2A { 2 } else { 1 });
            set_flag(s, ge(v, p.read()));
        }
        // IS_INT_(L)VAR_GREATER_OR_EQUAL_TO_INT_(L)VAR
        0x2C..=0x2F => {
            let (ta, tb) = pair(command - 0x2C);
            let (a, b) = two_vars(s, ta, tb);
            set_flag(s, ge(a.read(), b.read()));
        }
        // IS_FLOAT_VAR / _LVAR_GREATER_OR_EQUAL_TO_NUMBER
        0x30 | 0x31 => {
            let (p, v) = var_then_value(s, if command == 0x30 { 2 } else { 1 });
            set_flag(s, fge(p.read(), v));
        }
        // IS_NUMBER_GREATER_OR_EQUAL_TO_FLOAT_VAR / _LVAR
        0x32 | 0x33 => {
            let (v, p) = value_then_var(s, if command == 0x32 { 2 } else { 1 });
            set_flag(s, fge(v, p.read()));
        }
        // IS_FLOAT_(L)VAR_GREATER_OR_EQUAL_TO_FLOAT_(L)VAR
        0x34..=0x37 => {
            let (ta, tb) = pair(command - 0x34);
            let (a, b) = two_vars(s, ta, tb);
            set_flag(s, fge(a.read(), b.read()));
        }
        // IS_INT_VAR / _LVAR_EQUAL_TO_NUMBER
        0x38 | 0x39 => {
            let (p, v) = var_then_value(s, if command == 0x38 { 2 } else { 1 });
            set_flag(s, p.read() == v);
        }
        // IS_INT_VAR_EQUAL_TO_INT_VAR, _LVAR_.._LVAR, _VAR_.._LVAR
        0x3A..=0x3C => {
            let (ta, tb) = [(2, 2), (1, 1), (2, 1)][(command - 0x3A) as usize];
            let (a, b) = two_vars(s, ta, tb);
            set_flag(s, a.read() == b.read());
        }
        // IS_FLOAT_VAR / _LVAR_EQUAL_TO_NUMBER
        0x42 | 0x43 => {
            let (p, v) = var_then_value(s, if command == 0x42 { 2 } else { 1 });
            set_flag(s, feq(p.read(), v));
        }
        // IS_FLOAT_VAR_EQUAL_TO_FLOAT_VAR, _LVAR_.._LVAR, _VAR_.._LVAR
        0x44..=0x46 => {
            let (ta, tb) = [(2, 2), (1, 1), (2, 1)][(command - 0x44) as usize];
            let (a, b) = two_vars(s, ta, tb);
            set_flag(s, feq(a.read(), b.read()));
        }
        // GOTO_IF_FALSE
        0x4D => {
            collect(s, 1);
            if s.byte_add(vm::COND_RESULT).read() == 0 {
                update_pc_call(s, sp0() as i32);
            }
        }
        // TERMINATE_THIS_SCRIPT
        0x4E => {
            if s.byte_add(vm::IS_MISSION).read() != 0 {
                ALREADY_RUNNING_A_MISSION_SCRIPT.set(0);
            }
            remove_script_from_list(s, ACTIVE_SCRIPTS.ptr());
            add_script_to_list(s, IDLE_SCRIPTS.ptr());
            shutdown_this_script(s);
            return 1;
        }
        // START_NEW_SCRIPT: a negative offset is replaced by the command id (0x4F)
        0x4F => {
            collect(s, 1);
            let off = sp0() as i32;
            let off = if off < 0 { command as i32 } else { off };
            let n = start_new_script(ss(off));
            read_parameters_for_newly_started_script_call(s, n);
        }
        // GOSUB: m_IPStack[m_StackDepth++] = m_IP (no bound check)
        0x50 => {
            collect(s, 1);
            let d = s.byte_add(vm::STACK_DEPTH).cast::<u16>();
            let depth = d.read();
            let ip = s.byte_add(vm::IP).cast::<u32>().read();
            s.byte_add(vm::IP_STACK)
                .cast::<u32>()
                .wrapping_index(i32::from(depth))
                .write(ip);
            d.write(depth.wrapping_add(1));
            update_pc_call(s, sp0() as i32);
        }
        // RETURN: m_IP = m_IPStack[--m_StackDepth]
        0x51 => {
            let d = s.byte_add(vm::STACK_DEPTH).cast::<u16>();
            let depth = d.read().wrapping_sub(1);
            d.write(depth);
            let ip = s
                .byte_add(vm::IP_STACK)
                .cast::<u32>()
                .wrapping_index(i32::from(depth))
                .read();
            s.byte_add(vm::IP).cast::<u32>().write(ip);
        }
        // LINE: 6 parameters skipped (gta-reversed leaves them unread)
        0x52 => collect(s, 6),
        // CREATE_PLAYER
        0x53 => create_player(s),
        // ADD_INT_VAR_TO_INT_VAR (2,2), ADD_INT_LVAR_TO_INT_LVAR (1,1), ADD_INT_VAR_TO_INT_LVAR (1,2),
        // ADD_INT_LVAR_TO_INT_VAR (2,1): *a += *b
        0x58 | 0x5A | 0x5C | 0x5E => {
            let (ta, tb) = [(2, 2), (1, 1), (1, 2), (2, 1)][((command - 0x58) / 2) as usize];
            let (a, b) = two_vars(s, ta, tb);
            a.write(a.read().wrapping_add(b.read()));
        }
        // ADD_FLOAT_VAR_TO_FLOAT_VAR, _LVAR_TO_.._LVAR, _VAR_TO_.._LVAR: fld [a]; fadd [b]
        0x59 | 0x5B | 0x5D => {
            let (ta, tb) = [(2, 2), (1, 1), (1, 2)][((command - 0x59) / 2) as usize];
            let (a, b) = two_vars(s, ta, tb);
            a.write(fop(a.read(), b.read(), X87::fadd_m));
        }
        // ADD_FLOAT_LVAR_TO_FLOAT_VAR: fld [b]; fadd [a] (operand order swapped)
        0x5F => {
            let (a, b) = two_vars(s, 2, 1);
            a.write(fop(b.read(), a.read(), X87::fadd_m));
        }
        // SUB_INT_VAR_FROM_INT_VAR (2,2), SUB_INT_LVAR_FROM_INT_LVAR (1,1): *a -= *b
        0x60 | 0x62 => {
            let t = if command == 0x60 { 2 } else { 1 };
            let (a, b) = two_vars(s, t, t);
            a.write(a.read().wrapping_sub(b.read()));
        }
        // SUB_FLOAT_VAR_FROM_FLOAT_VAR, SUB_FLOAT_LVAR_FROM_FLOAT_LVAR: fld [a]; fsub [b]
        0x61 | 0x63 => {
            let t = if command == 0x61 { 2 } else { 1 };
            let (a, b) = two_vars(s, t, t);
            a.write(fop(a.read(), b.read(), X87::fsub_m));
        }
        _ => return -1,
    }
    0
}

/// Operand variable types of the four `(L)VAR op (L)VAR` forms in their opcode order:
/// VAR-VAR, LVAR-LVAR, VAR-LVAR, LVAR-VAR.
#[inline]
fn pair(k: u32) -> (i32, i32) {
    [(2, 2), (1, 1), (2, 1), (1, 2)][k as usize]
}

/// `cdq; idiv` (faults on a zero divisor and on INT_MIN / -1, as the original).
#[inline]
fn idiv(a: u32, b: u32) -> u32 {
    let (a, b) = (a as i32, b as i32);
    assert!(
        b != 0 && !(a == i32::MIN && b == -1),
        "ProcessCommands0To99: idiv fault"
    );
    (a / b) as u32
}

/// 0x0053 CREATE_PLAYER (0x4668DC): loads the special model "player" when model 0 is not loaded,
/// sets up and places the player ped (ground z when z <= -100, then + the distance from the centre of
/// mass to the base), stores the player index, and gives the ped a `CTaskSimplePlayerOnFoot` in the
/// primary slot 4.
fn create_player(s: Scr) {
    collect(s, 4);
    let idx = param(0).read() as i32;
    if GPtr::<u8>::from_addr(MODEL0_LOAD_STATE).read() != 1 {
        request_special_model(0, GPtr::from_addr(PLAYER_NAME), 0x1A);
        load_all_requested_models(1);
    }
    setup_player_ped(idx);
    let slot = sa_core::pools::PLAYER0_PED
        .ptr()
        .cast::<u8>()
        .byte_add(idx.wrapping_mul(0x190) as isize)
        .cast::<GPtr<u8>>();
    set_char_created_by(slot.read(), 2);
    deactivate_player_ped(idx);
    let x = param(1).read();
    let y = param(2).read();
    let mut z = f(param(3).read());
    // fld z; fcomp -100.0; test ah,0x41; jp skip: z <= -100 (ordered)
    if z <= f(K_MINUS_100) {
        z = X87::from_f64_exact(find_ground_z_for_coord(f(x), f(y))).to_f32();
    }
    let d = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(slot.read()));
    let z = d.fadd_m(z).to_f32();
    set_posn(slot.read(), f(x), f(y), z);
    let pos = [f(x), f(y), z];
    clear_space_for_mission_entity(GPtr::from_raw(pos.as_ptr().cast_mut()), slot.read());
    reactivate_player_ped(idx);
    param(0).write(idx as u32);
    store_parameters_call(s, 1);
    let t = task_new(0x1C);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        task_simple_player_on_foot_ctor(t)
    };
    let intel = slot.read().byte_add(0x47C).cast::<GPtr<u8>>().read();
    task_manager_set_task(intel.byte_add(4), t, 4, 0);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_0_TO_99];
