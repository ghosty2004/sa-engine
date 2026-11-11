//! PORT-CAM: `CHandShaker` (0x94 bytes; `gHandShaker[6]` at 0xB6ECA0). Ported from the 1.0 US bytes;
//! gta-reversed `HandShaker.cpp` and `docs/re/specs/CAM/CHandShaker.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{k, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `gHandShaker[6]` (CHandShaker, 0x94 bytes each)
    pub static G_HAND_SHAKER @ 0xB6ECA0: [u8; 6 * 0x94] = [0; 6 * 0x94];
}

/// `CHandShaker::~CHandShaker` (0x509810), the `eh vector` destructor argument of `__init_gHandShaker`.
pub const HAND_SHAKER_DTOR: u32 = 0x509810;
/// `CHandShaker::CHandShaker` (0x517740), the `eh vector` constructor argument.
pub const HAND_SHAKER_CTOR: u32 = 0x517740;

/// 1/32767 (0x858C7C), the `rand()` scale of `Reset`.
const RAND_SCALE: u32 = 0x3800_0100;

orig! {
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CHandShaker::SetDefaults()` (0x517330)
    pub fn set_defaults_call(this: GPtr<u8>) = thiscall 0x517330 => hand_shaker_set_defaults;
    /// `CHandShaker::Reset()` (0x50D860)
    pub fn reset_call(this: GPtr<u8>) = thiscall 0x50D860 => hand_shaker_reset;
}

/// `CHandShaker::~CHandShaker()` (0x509810): `add ecx, 0x3C; jmp CMatrix::~CMatrix` (the matrix member).
#[hook(addr = 0x509810, conv = thiscall, name = "CHandShaker::~CHandShaker", expect = "83 C1 3C E9 B8 14 09 00", this = "CHandShaker")]
pub fn hand_shaker_destructor(this: GPtr<u8>) {
    matrix_dtor(this.byte_add(0x3C));
}

/// `CHandShaker::CHandShaker()` (0x517740): the matrix member's attach pointer and owns flag (+0x7C,
/// +0x80) cleared, `SetDefaults()`, returns `this`.
#[hook(addr = 0x517740, conv = thiscall, name = "CHandShaker::CHandShaker", expect = "56 8B F1 33 C0", this = "CHandShaker")]
pub fn hand_shaker_constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x7C, 0);
    wr::<u32>(this, 0x80, 0);
    set_defaults_call(this);
    this
}

/// `CHandShaker::SetDefaults()` (0x517330): the tuning vectors +0xC (0.02, 0.02, 0.01), +0x18
/// (0.0002, 0.0002, 0.0001), +0x30 (1.3, 1.3, 1.4), +0x84 0.3, +0x88 1.0, +0x8C 20, +0x90 0.001, then
/// tail-jumps to `Reset()`.
#[hook(addr = 0x517330, conv = thiscall, name = "CHandShaker::SetDefaults", expect = "83 EC 0C C7 44 24 00 0A D7 A3 3C", this = "CHandShaker")]
pub fn hand_shaker_set_defaults(this: GPtr<u8>) {
    for (o, v) in [
        (0x0C, 0x3CA3_D70A),
        (0x10, 0x3CA3_D70A),
        (0x14, 0x3C23_D70A),
        (0x18, 0x3951_B717),
        (0x1C, 0x3951_B717),
        (0x20, 0x38D1_B717),
        (0x30, 0x3FA6_6666),
        (0x34, 0x3FA6_6666),
        (0x38, 0x3FB3_3333),
        (0x84, 0x3E99_999A),
        (0x88, 0x3F80_0000),
        (0x8C, 0x14),
        (0x90, 0x3A83_126F),
    ] {
        wr::<u32>(this, o, v);
    }
    reset_call(this);
}

/// `CHandShaker::Reset()` (0x50D860): +0 = (0, 0, 0); three `rand()` (return addresses 0x50D8A0,
/// 0x50D8BE, 0x50D8DC) scaled by 1/32767: `a` and `b` stored as f32, the third kept on the stack;
/// +0x24 = `r3 * k * [+0x18]`, +0x28 = `b * [+0x1C]`, +0x2C = `a * [+0x20]`.
#[hook(addr = 0x50D860, conv = thiscall, name = "CHandShaker::Reset", expect = "83 EC 24 56 8B F1", this = "CHandShaker")]
pub fn hand_shaker_reset(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    let s20: f32 = rd(this, 0x20);
    let r1 = sa_core::orig::at_site(0x50D8A0, sa_core::crt::rand);
    let a = xi(r1).fmul_m(k(RAND_SCALE)).to_f32();
    let s1c: f32 = rd(this, 0x1C);
    let r2 = sa_core::orig::at_site(0x50D8BE, sa_core::crt::rand);
    let b = xi(r2).fmul_m(k(RAND_SCALE)).to_f32();
    let s18: f32 = rd(this, 0x18);
    let r3 = sa_core::orig::at_site(0x50D8DC, sa_core::crt::rand);
    let c = xi(r3).fmul_m(k(RAND_SCALE)).fmul_m(s18).to_f32();
    let d = x(b).fmul_m(s1c).to_f32();
    let e = x(a).fmul_m(s20).to_f32();
    wr(this, 0x24, c);
    wr(this, 0x28, d);
    wr(this, 0x2C, e);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_HAND_SHAKER_DESTRUCTOR,
    HOOK_HAND_SHAKER_CONSTRUCTOR,
    HOOK_HAND_SHAKER_SET_DEFAULTS,
    HOOK_HAND_SHAKER_RESET,
];
