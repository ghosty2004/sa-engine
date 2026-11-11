//! `CRunningScript::ProcessCommands1800To1899` (0x46D050): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1800To1899.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x070A, 0x070D, 0x0713, 0x0729, 0x0750, 0x075A];

orig! {
    /// `CRunningScript::ReadTextLabelFromScript(char*, uint8)` (0x463D50)
    fn read_label(s: Scr, buf: GPtr<u8>, len: u32) = thiscall 0x463D50 => crate::vm::read_text_label_from_script;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    fn get_animation_block(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CAnimManager::GetAnimation(const char*, const CAnimBlock*)` (0x4D42F0)
    fn get_animation(name: GPtr<u8>, block: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D42F0 => unported;
    /// `RpAnimBlendClumpIsInitialized(RpClump*)` (0x4D6760), AL
    fn rp_anim_blend_clump_is_initialized(clump: GPtr<u8>) -> u8 = cdecl 0x4D6760 => unported;
    /// `RpAnimBlendClumpInit(RpClump*)` (0x4D6720)
    fn rp_anim_blend_clump_init(clump: GPtr<u8>) = cdecl 0x4D6720 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, CAnimBlendHierarchy*, int32 flags, float blendDelta)` (0x4D4410)
    fn blend_animation(clump: GPtr<u8>, anim: GPtr<u8>, flags: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4410 => unported;
    /// `CRunningScript::ScriptTaskPickUpObject(int32 command)` (0x46AF50; reads its own parameters)
    fn script_task_pick_up_object(s: Scr, command: u32) = thiscall 0x46AF50 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// A virtual `CTask` method without arguments returning EAX (`GetTaskType`, vtable +0x10), read at run time
    fn vcall_task_type(f: u32, this: GPtr<u8>) -> u32 = thiscall dyn => unported;
    /// The drive-by task constructor 0x6217D0 (`CTaskSimpleGangDriveBy`: target entity, `const CVector*` target
    /// coords, abort range, frequency, style, seat RHS), returns this
    fn task_drive_by_ctor(t: GPtr<u8>, target: GPtr<u8>, pos: GPtr<u32>, range: u32, freq: u32, style: u32, rhs: u32) -> GPtr<u8> = thiscall 0x6217D0 => unported;
    /// The mobile phone task constructor 0x6348A0 (`CTaskComplexUseMobilePhone(int32 duration)`), returns this
    fn task_use_mobile_phone_ctor(t: GPtr<u8>, duration: u32) -> GPtr<u8> = thiscall 0x6348A0 => unported;
    /// `CTaskManager::FindTaskByType(int32 slot, int32 type)` (0x6817D0)
    fn find_task_by_type(tm: GPtr<u8>, slot: u32, ty: u32) -> GPtr<u8> = thiscall 0x6817D0 => unported;
    /// 0x634A40 (thiscall on the phone task, `CPed*`): ends the mobile phone task
    fn phone_task_stop(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x634A40 => unported;
    /// `CClothes::RebuildPlayer(CPlayerPed*, bool bIgnoreFatAndMuscle)` (0x5A82C0)
    fn rebuild_player(ped: GPtr<u8>, ignore: u32) = cdecl 0x5A82C0 => unported;
    /// `CReplay::Init()` (0x45E220)
    fn replay_init() = cdecl 0x45E220 => unported;
}

/// `CRunningScript::ProcessCommands1800To1899(int32 command)` (0x46D050, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x46D050, conv = thiscall, name = "CRunningScript::ProcessCommands1800To1899", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_1800_to_1899(s: Scr, command: u32) -> i8 {
    match command {
        // 070A TASK_PICK_UP_OBJECT (0x46D166): `this->ScriptTaskPickUpObject(command)`
        0x070A => script_task_pick_up_object(s, command),
        // 0713 TASK_DRIVE_BY (0x46D3A7)
        0x0713 => task_drive_by(s, command),
        // 0729 TASK_USE_MOBILE_PHONE (0x46DA35)
        0x0729 => task_use_mobile_phone(s, command),
        // 070D BUILD_PLAYER_MODEL (0x46D21A): `CClothes::RebuildPlayer(CWorld::Players[SP0].m_pPed, false)`;
        // `CReplay::Init()` (player index inlined: `imul SP0,0x190` + 0xB7CD98)
        0x070D => {
            collect(s, 1);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            rebuild_player(ped, 0);
            replay_init();
        }
        // 0750 SET_OBJECT_VISIBLE (0x46E802): `OBJ(SP0)` +0x1C bit 7 (bIsVisible) = SP1 != 0 (no null check)
        0x0750 => {
            collect(s, 2);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let fl = o.byte_add(0x1C).cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 { v | 0x80 } else { v & 0xFFFF_FF7F });
        }
        // 075A PLAY_OBJECT_ANIM (0x46E9FC)
        0x075A => play_object_anim(s),
        _ => {
            return fallback(
                process_commands_1800_to_1899_original,
                "CRunningScript::ProcessCommands1800To1899",
                s,
                command,
            );
        }
    }
    0
}

/// 0x0713 TASK_DRIVE_BY (0x46D3A7): `CollectParameters(10)`: SP0 ped, SP1 target ped, SP2 target car, SP3..SP5 coords,
/// SP6 abort range, SP7 style, SP8 seat RHS, SP9 frequency.
/// - `target = 0`, `tPed = 0`; `SP1 >= 0` -> `target = tPed = PED(SP1)`; `SP2 >= 0` -> `target = VEH(SP2)`.
/// - If `SP0 != -1`: `cur = PED(SP0)->m_pIntelligence (+0x47C) +0x10` (no null check of the ped); if `cur` and
///   `cur->GetTaskType() == 0x3FE` and `cur+0x34 == tPed`: nothing is given (AL = 0).
/// - `t = new(0x44)`; ctor 0x6217D0(target, &(SP3..SP5), SP6, SP9, style, SP8 != 0); `t+0xE = 1` (t not null-checked);
///   `GivePedScriptedTask(SP0, t, command)`.
///
/// The style and seat dwords are stack slots whose low byte only is written (`mov [esp+0x20],dl` /
/// `mov [esp+0x14],cl`); the callee takes them as byte arguments, so the port passes the byte zero-extended
/// (`(u8)SP7`, 0/1). The SEH state writes are not ported.
fn task_drive_by(s: Scr, command: u32) {
    collect(s, 10);
    let ped = sp(0);
    let pos = [sp(3), sp(4), sp(5)];
    let (range, style, rhs, freq) = (sp(6), sp(7) & 0xFF, u32::from(sp(8) != 0), sp(9));
    let mut target = GPtr::NULL;
    let mut t_ped = GPtr::NULL;
    if spi(1) >= 0 {
        t_ped = ped_at_ref(pool(PED_POOL), spi(1));
        target = t_ped;
    }
    if spi(2) >= 0 {
        target = vehicle_at_ref(pool(VEHICLE_POOL), spi(2));
    }
    if ped as i32 != -1 {
        let p = ped_at_ref(pool(PED_POOL), ped as i32);
        let cur = p
            .byte_add(0x47C)
            .cast::<GPtr<u8>>()
            .read()
            .byte_add(0x10)
            .cast::<GPtr<u8>>()
            .read();
        if !cur.is_null() {
            let f = cur.cast::<GPtr<u32>>().read().byte_add(0x10).read();
            if vcall_task_type(f, cur) == 0x3FE && cur.byte_add(0x34).cast::<GPtr<u8>>().read() == t_ped {
                return;
            }
        }
    }
    let t = task_new(0x44);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        task_drive_by_ctor(
            t,
            target,
            GPtr::from_raw(pos.as_ptr().cast_mut()),
            range,
            freq,
            style,
            rhs,
        )
    };
    t.byte_add(0xE).write(1);
    give_ped_scripted_task(s, ped, t, command);
}

/// 0x0729 TASK_USE_MOBILE_PHONE (0x46DA35): `CollectParameters(2)`. `SP1 > 0` (signed): `t = new(0x20)` ctor
/// 0x6348A0(-1); `GivePedScriptedTask(SP0, t, command)`. Else `p = PED(SP0)` (no null check); `t =
/// p->m_pIntelligence->m_TaskMgr (+0x47C, +4).FindTaskByType(3, 0x640)`; if `t && t->GetTaskType() == 0x640`:
/// `0x634A40(t, p)`.
fn task_use_mobile_phone(s: Scr, command: u32) {
    collect(s, 2);
    let ped = sp(0);
    if spi(1) > 0 {
        let t = task_new(0x20);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            task_use_mobile_phone_ctor(t, u32::MAX)
        };
        give_ped_scripted_task(s, ped, t, command);
        return;
    }
    let p = ped_at_ref(pool(PED_POOL), ped as i32);
    let tm = p.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
    let t = find_task_by_type(tm, 3, 0x640);
    if t.is_null() {
        return;
    }
    let f = t.cast::<GPtr<u32>>().read().byte_add(0x10).read();
    if vcall_task_type(f, t) == 0x640 {
        phone_task_stop(t, p);
    }
}

/// 0x075A PLAY_OBJECT_ANIM (0x46E9FC): `CollectParameters(1)`; `obj = OBJ(SP0)` (no null check);
/// `ReadTextLabelFromScript(anim, 24)`; `ReadTextLabelFromScript(block, 16)`; `CollectParameters(3)`: SP0 blend delta
/// (raw f32), SP1 loop, SP2 keep last frame. `a = GetAnimation(anim, GetAnimationBlock(block))`; `flags = (SP1 != 0 ? 2
/// : 0) | (SP2 == 0 ? 8 : 0)`. If `obj->m_pRwObject (+0x18)`: `RpAnimBlendClumpInit` unless initialised;
/// `BlendAnimation(clump, a, flags, delta)`; flag 1. Else flag 0.
///
/// Divergence (not reproducible): the label buffers are stack arrays of 24 and 16 bytes; a name that fills its
/// buffer without a NUL makes the original callees read on into adjacent stack bytes. The port's buffers carry one
/// extra zero byte instead.
fn play_object_anim(s: Scr) {
    collect(s, 1);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    let mut anim = [0u8; 25];
    let mut block = [0u8; 17];
    let anim_p = GPtr::from_raw(anim.as_mut_ptr());
    let block_p = GPtr::from_raw(block.as_mut_ptr());
    read_label(s, anim_p, 0x18);
    read_label(s, block_p, 0x10);
    collect(s, 3);
    let delta = sp(0);
    let b = get_animation_block(block_p);
    let a = get_animation(anim_p, b);
    let mut flags = if sp(1) != 0 { 2 } else { 0 };
    if sp(2) == 0 {
        flags |= 8;
    }
    let rw = obj.byte_add(0x18).cast::<GPtr<u8>>();
    if rw.read().is_null() {
        set_flag(s, false);
        return;
    }
    if rp_anim_blend_clump_is_initialized(rw.read()) == 0 {
        rp_anim_blend_clump_init(rw.read());
    }
    blend_animation(rw.read(), a, flags, delta);
    set_flag(s, true);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1800_TO_1899];
