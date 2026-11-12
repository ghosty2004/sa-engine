//! `CPed::IsPointerValid` (batch `samp-ped-1`), ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
}

/// `CPools::ms_pPedPool` (0xB74490): +0 objects, +4 byte map; ped size 0x7C4.
const PED_POOL: usize = 0xB7_4490;

/// `CPed::IsPointerValid()` (0x5E4220, thiscall, AL): the slot index `(this - pool.objects) / 0x7C4` (signed, the
/// compiler's multiply-by-reciprocal) goes through the byte map as `(byteMap[i] + (i << 8)) >> 8` (the byte is read
/// before the range test, as the bytes do); valid when it lies in 0..0x8C and either `this +0xB0` is set or `this`
/// is `FindPlayerPed(-1)`. The entry `push esi; push edi; nop; jmp 0x401E54` reaches a no-CD chunk that loads EDI =
/// the pool pointer `[ [0x60E533] ^ [0x15606F4] ]` = `[0xB74490]` (decoded from the exe image) and returns to
/// 0x5E4228.
#[hook(addr = 0x5E4220, conv = thiscall, name = "CPed::IsPointerValid", expect = "56 57 90 E9 2C DC E1 FF",
       unverified = "samp-ped-1", route = "L2", hazards = "entry-protection-read", recs = "weapon-combat",
       note = "AL result (the upper EAX bits are internal values in the original); 0x403968 reads the entry's rel32 0x5E4224 as a key (keycells reader)")]
pub fn ped_is_pointer_valid(this: GPtr<u8>) -> u8 {
    let pool = GPtr::<GPtr<u8>>::from_addr(PED_POOL).read();
    let objects = pool.cast::<u32>().read();
    let byte_map = pool.byte_add(4).cast::<u32>().read();
    let diff = (this.addr() as u32).wrapping_sub(objects) as i32;
    let mut q = ((i64::from(diff) * 0x41EE_7CA7) >> 32) as i32 >> 9;
    q += (q as u32 >> 31) as i32;
    let b = GPtr::<u8>::from_addr(byte_map.wrapping_add(q as u32) as usize).read();
    let idx = (i32::from(b).wrapping_add(q << 8)) >> 8;
    if !(0..0x8C).contains(&idx) {
        return 0;
    }
    if this.byte_add(0xB0).cast::<u32>().read() != 0 || this == find_player_ped(-1) {
        1
    } else {
        0
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PED_IS_POINTER_VALID];
