//! `CScriptResourceManager::RemoveFromResourceManager` (batch `mvpi-script-1`). Ported from the 1.0 US
//! `.HOODLUM` body behind the stub; unverified (docs/verify/HOWTO.md).
//!
//! Resource entries: 0x4B of 0xC bytes at `this` (id +0, owner script +4, type u16 +8).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

fn at<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast::<T>()
}

/// `CScriptResourceManager::RemoveFromResourceManager(int32 id, uint16 type, CRunningScript* owner)`
/// (stub 0x470510 → 0x156B210, thiscall, `ret 0xC`): over the 0x4B entries, every one with this id and
/// type: owned by `owner` → remembered (the last such index wins), else counted. A remembered entry is
/// cleared (id -1, type 0, owner 0, in that order). Returns AL = no entry of another owner. The
/// "nothing remembered" test compares the index with the value of the no-CD routine 0xCB73E0 (called
/// with 0x156001C and the stack pointer); it is taken as -1, the index's initial value (gta-reversed's
/// `foundIndex != -1`). EAX's upper bytes are the original's: the cleared entry's address, else the
/// loop pointer `this + 0x388`.
#[hook(addr = 0x470510, conv = thiscall, name = "CScriptResourceManager::RemoveFromResourceManager",
       expect = "E9 FB AC 0F 01", unverified = "mvpi-script-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the 'not found' constant comes from the no-CD routine 0xCB73E0 (assumed -1)")]
pub fn remove_from_resource_manager(this: GPtr<u8>, id: i32, ty: u32, owner: u32) -> u32 {
    let mut found: i32 = -1;
    let mut others: u32 = 0;
    for i in 0..0x4Bisize {
        let e = this.byte_add(i * 0xC);
        if at::<i32>(e, 0).read() == id && u32::from(at::<u16>(e, 8).read()) == ty {
            if at::<u32>(e, 4).read() == owner {
                found = i as i32;
            } else {
                others = others.wrapping_add(1);
            }
        }
    }
    let mut eax = (this.addr() as u32).wrapping_add(0x388);
    if found != -1 {
        let e = this.byte_add(found as isize * 0xC);
        at::<u32>(e, 0).write(0xFFFF_FFFF);
        at::<u16>(e, 8).write(0);
        at::<u32>(e, 4).write(0);
        eax = e.addr() as u32;
    }
    (eax & 0xFFFF_FF00) | u32::from(others == 0)
}

pub const HOOKS: &[HookReg] = &[HOOK_REMOVE_FROM_RESOURCE_MANAGER];
