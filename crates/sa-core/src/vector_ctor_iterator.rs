//! The compiler's `vector constructor iterator` (`??_H`) and `CMatrix`'s in-place translation, two unnamed helpers
//! SA:MP executes (UNK-game rows of `docs/samp/native-unported.csv`, batch `samp-unk-1`; names proposed in
//! `docs/samp/native-unported-names.csv`). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_math::x87::X87;

use crate::abi::HookReg;
use crate::mem::GPtr;
use crate::{hook, orig};

orig! {
    /// The element constructor passed to the iterator (thiscall, no argument; its result is ignored)
    fn call_element_ctor(f: u32, elem: GPtr<u8>) -> u32 = thiscall dyn => unported;
}

/// `FUN_00403d60` (0x403D60), proposed `` `vector constructor iterator' `` (`??_H`, stdcall `(void* array, size_t
/// size, int count, void (__thiscall* ctor)(void*))`, `ret 0x10`): `count - 1 < 0` (signed) does nothing; otherwise
/// `ctor` runs on `array`, `array + size`, ... `count` times. The non-EH twin of 0x821D30. The loop head is
/// reached through a no-CD disguised jump (0x403D7B → 0x4020DA: `ror [0x156071C], [0x422761] & 31` = 0x403D80,
/// decoded from the exe image); the port does not depend on it.
#[hook(addr = 0x403D60, conv = stdcall, name = "FUN_00403d60", expect = "8B 44 24 0C 48 78 26",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes", recs = "anim-input",
       note = "the element constructors write the array")]
pub fn vector_constructor_iterator(array: GPtr<u8>, size: u32, count: i32, ctor: u32) {
    if count.wrapping_sub(1) < 0 {
        return;
    }
    let mut p = array;
    for _ in 0..count {
        call_element_ctor(ctor, p);
        p = p.byte_add(size as i32 as isize);
    }
}

/// `FUN_0046ae00` (0x46AE00), proposed `CMatrix::Translate(const CVector&)` (`GetPosition() += v`; thiscall,
/// `ret 4`; callers the script command handler 0x46D050 and `CAutomobile::SpawnFlyingComponent`): position +0x30,
/// +0x34, +0x38 each `+= v` on the x87 stack, rounded to f32 by `fstp`.
#[hook(addr = 0x46AE00, conv = thiscall, name = "FUN_0046ae00", expect = "8B 44 24 04 D9 40 08",
       unverified = "samp-unk-1", route = "L2", hazards = "none")]
pub fn matrix_translate(m: GPtr<u8>, v: GPtr<f32>) {
    let (vx, vy, vz) = (v.read(), v.wrapping_index(1).read(), v.wrapping_index(2).read());
    for (off, d) in [(0x30, vx), (0x34, vy), (0x38, vz)] {
        let p = m.byte_add(off).cast::<f32>();
        p.write(X87::from_f32(d).fadd_m(p.read()).to_f32());
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_VECTOR_CONSTRUCTOR_ITERATOR, HOOK_MATRIX_TRANSLATE];
