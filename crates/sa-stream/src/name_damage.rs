//! `RwHelper::GetNameAndDamage` (STD-6, batch `std6-stream-1`). Ported from the 1.0 US bytes
//! (0x5370A0).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// CRT `strncpy` (0x821F40)
    fn crt_strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
}

fn at(p: GPtr<u8>, i: isize) -> u8 {
    p.byte_add(i).read()
}

/// `RwHelper::GetNameAndDamage(nodeName, outName, outDamage)` (0x5370A0): a name ending in "_dam"
/// (lower case) is a damaged part: `*outDamage = 1`, the name without the suffix; one ending in "_L0" /
/// "_l0" is copied without that suffix (`*outDamage = 0`); any other name is copied whole
/// (`*outDamage = 0`).
#[hook(addr = 0x5370A0, conv = cdecl, name = "RwHelper::GetNameAndDamage", expect = "8B 44 24 04 56 8B F0 57",
       unverified = "std6-stream-1", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn get_name_and_damage(name: GPtr<u8>, out: GPtr<u8>, damage: GPtr<u8>) {
    let mut len = 0isize;
    while at(name, len) != 0 {
        len += 1;
    }
    if at(name, len - 4) == b'_' && at(name, len - 3) == b'd' && at(name, len - 2) == b'a' && at(name, len - 1) == b'm' {
        damage.write(1);
        crt_strncpy(out, name, (len - 4) as u32);
        out.byte_add(len - 4).write(0);
        return;
    }
    let c = at(name, len - 2);
    if at(name, len - 3) == b'_' && (c == b'L' || c == b'l') && at(name, len - 1) == b'0' {
        damage.write(0);
        crt_strncpy(out, name, (len - 3) as u32);
        out.byte_add(len - 3).write(0);
        return;
    }
    damage.write(0);
    let mut i = 0isize;
    loop {
        let b = at(name, i);
        out.byte_add(i).write(b);
        if b == 0 {
            break;
        }
        i += 1;
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_GET_NAME_AND_DAMAGE];
