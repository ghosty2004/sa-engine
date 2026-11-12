//! `CGangWars::ReleasePedsInAttackWave` (batch `std8-pop-1`), ported from the 1.0 US bytes (`.HOODLUM` body
//! 0x15609C0). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

orig! {
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0, `ret 4`)
    fn ped_set_char_created_by(ped: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32 handle)` (0x587C60)
    fn radar_clear_blip_for_entity(ty: u32, handle: u32) = cdecl 0x587C60 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00)
    fn wander_task_by_ped_type(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskIndex, CTask*, bool)` (0x4B0A00, `ret 0xC`)
    fn event_script_command_ctor(ev: GPtr<u8>, index: u32, task: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50)
    fn event_script_command_dtor(ev: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(group: GPtr<u8>, ev: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPedType::GetPedTypeAcquaintances(ePedType)` (0x6089B0, returns the 20-byte acquaintance block)
    fn ped_type_acquaintances(pt: u32) -> GPtr<u8> = cdecl 0x6089B0 => unported;
}

/// `CPools::ms_pPedPool` (0xB74490)
const PED_POOL: usize = 0xB74490;
/// `CPed` size in the pool
const PED_SIZE: u32 = 0x7C4;
/// The mask the no-CD VM (0xCB73E0, descriptor 0x155FF14) yields into EBP: `~bClearRadarBlipOnDeath` (bit 0x2000 of
/// the ped flags +0x474).
const CLEAR_BLIP_ON_DEATH_OFF: u32 = 0xFFFF_DFFF;

/// The ped's pool handle: `(index << 8) + the pool flag byte of index`.
fn ped_handle(pool: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let objects = rd::<u32>(pool, 0);
    let idx = (ped.addr() as u32).wrapping_sub(objects) as i32 / PED_SIZE as i32;
    let flags = rd::<GPtr<u8>>(pool, 4);
    ((idx as u32) << 8).wrapping_add(flags.byte_add(idx as isize).read() as u32)
}

/// `CGangWars::ReleasePedsInAttackWave(bool isEndOfWar, bool restoreAcquaintances)` (stub 0x445C30 → 0x15609C0,
/// cdecl). Walks the ped pool from the last slot down. A ped with `bPartOfAttackWave` (+0x474 bit 0x1000): the bit
/// cleared, `SetCharCreatedBy(1)`, counted, `CRadar::ClearBlipForEntity(2, handle)`, `bClearRadarBlipOnDeath`
/// (0x2000) cleared; with `restoreAcquaintances` also a `CEventScriptCommand(3, GetWanderTaskByPedType(ped),
/// false)` (a stack event) added to the ped's event group (`+0x47C` + 0x68) and the acquaintances (+0x4E0..+0x4F0)
/// reset from `CPedType::GetPedTypeAcquaintances(pedType +0x598)`. Then, with `isEndOfWar`, a (live) ped still
/// flagged 0x2000 gets the same blip clear and flag clear. Returns the count.
///
/// The 0x2000 clear mask comes out of the no-CD protection VM (into EBP); its use (clearing the flag that pairs with
/// the blip removal, as gta-reversed's source has it) decodes it as `!0x2000`, not by evaluating the VM.
#[hook(addr = 0x445C30, conv = cdecl, name = "CGangWars::ReleasePedsInAttackWave", expect = "E9 8B AD 11 01",
       route = "L2", hazards = "uninit-locals", recs = "grove-walk",
       note = "the original computes the 0x2000 clear mask in the no-CD VM 0xCB73E0; SEH frame not modelled")]
pub fn release_peds_in_attack_wave(is_end_of_war: u32, restore: u32) -> u32 {
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    let mut count = 0u32;
    while i != 0 {
        i -= 1;
        let pool = g::<GPtr<u8>>(PED_POOL).read();
        let free = rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() & 0x80 != 0;
        let ped = if free {
            GPtr::NULL
        } else {
            GPtr::<u8>::from_addr(rd::<u32>(pool, 0).wrapping_add(i as u32 * PED_SIZE) as usize)
        };
        if !ped.is_null() && rd::<u32>(ped, 0x474) & 0x1000 != 0 {
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & !0x1000);
            ped_set_char_created_by(ped, 1);
            count += 1;
            radar_clear_blip_for_entity(2, ped_handle(g::<GPtr<u8>>(PED_POOL).read(), ped));
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & CLEAR_BLIP_ON_DEATH_OFF);
            if restore as u8 != 0 {
                let task = wander_task_by_ped_type(ped);
                let mut ev = [0u32; 10];
                let e = GPtr::<u8>::from_raw(ev.as_mut_ptr().cast());
                event_script_command_ctor(e, 3, task, 0);
                event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
                let acq = ped_type_acquaintances(rd::<u32>(ped, 0x598));
                for o in [0, 4, 8, 0xC, 0x10] {
                    wr::<u32>(ped, 0x4E0 + o, rd::<u32>(acq, o));
                }
                event_script_command_dtor(e);
            }
        }
        if is_end_of_war as u8 != 0 && !ped.is_null() && rd::<u32>(ped, 0x474) & 0x2000 != 0 {
            radar_clear_blip_for_entity(2, ped_handle(g::<GPtr<u8>>(PED_POOL).read(), ped));
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & CLEAR_BLIP_ON_DEATH_OFF);
        }
    }
    count
}

pub const HOOKS: &[HookReg] = &[HOOK_RELEASE_PEDS_IN_ATTACK_WAVE];
