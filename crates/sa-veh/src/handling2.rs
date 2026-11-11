//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `cHandlingDataMgr`. Ported from the 1.0 US bytes;
//! gta-reversed and docs/re/specs/VEH as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jne_mask, jp5, rd, wr, x, xi};

orig! {
    /// `cTransmission::cTransmission()` (0x6D0450)
    pub fn transmission_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x6D0450 => unported;
    /// `cTransmission::InitGearRatios()` (0x6D0460)
    pub fn init_gear_ratios(t: GPtr<u8>) = thiscall 0x6D0460 => unported;
}

/// CRT `_CIasin` (0x821E70): operand and result in ST0.
const CI_ASIN: usize = 0x821E70;

/// `fld dword [addr]` of a `.rdata` constant.
fn k(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `fld qword [addr]` of a `.rdata` double constant.
fn kq(addr: usize) -> X87 {
    X87::from_f64_exact(g::<f64>(addr).read())
}

/// `__init_gHandlingDataMgr` (0x8543D0): `cTransmission()` for the 210 car entries' transmissions
/// (0xC2BA08, stride 0xE0), then `rep stosd` zeroes the whole manager (0x3189 dwords from 0xC2B9C8),
/// which undoes the constructors' stores. Kept.
#[hook(addr = 0x8543D0, conv = cdecl, name = "__init_gHandlingDataMgr", expect = "56 57 BE 08 BA C2 00")]
pub fn init_g_handling_data_mgr() {
    for i in 0..0xD2usize {
        transmission_ctor(GPtr::from_addr(0xC2BA08 + 0xE0 * i));
    }
    for i in 0..0x3189usize {
        g::<u32>(0xC2B9C8 + 4 * i).write(0);
    }
}

/// `cHandlingDataMgr::ConvertBikeDataToWorldUnits(tBikeHandlingData*)` (0x6F5240): `+0x14`, `+0x2C`,
/// `+0x30` = `asin(v) * [0x859878]` (CRT `_CIasin` with the operand in ST0), `+0x18 = v * [0x859878]`.
#[hook(addr = 0x6F5240, conv = thiscall, name = "cHandlingDataMgr::ConvertBikeDataToWorldUnits", expect = "56 8B 74 24 08", this = "cHandlingDataMgr")]
pub fn convert_bike_data_to_world_units(_mgr: GPtr<u8>, data: GPtr<u8>) {
    let m = k(0x859878);
    let asin = |v: f32| X87::from_f64_exact(sa_core::orig::x87_unary(CI_ASIN, x(v).to_f64()));
    wr::<f32>(data, 0x14, asin(rd::<f32>(data, 0x14)).fmul_m(m).to_f32());
    wr::<f32>(data, 0x18, x(rd::<f32>(data, 0x18)).fmul_m(m).to_f32());
    wr::<f32>(data, 0x2C, asin(rd::<f32>(data, 0x2C)).fmul_m(m).to_f32());
    wr::<f32>(data, 0x30, asin(rd::<f32>(data, 0x30)).fmul_m(m).to_f32());
}

/// `cHandlingDataMgr::ConvertDataToGameUnits(tHandlingData*)` (0x6F5080): scales the engine
/// acceleration `+0x7C` and inertia `+0x94` by [0xC2B9B4], the max velocity `+0x84` by [0xC2B9BC]
/// (`T`), `+8 = 1 / mass`, `+0x24 = mass * [0x858C98] / gears`, `+0xC8 = (1 / mass) * +0xC8 *
/// [0x8631F0]`. Then iterates the top speed: `v = T`; while `v > 0`: `v -= [0x85A308]` (double) and
/// with `w = accel * [0x872350]` (double) and the drag `+0x10`, `r = -(1/(1 + v² drag) - 1) * v` when
/// `drag < [0x858C58]`, else `r = drag / [0x872348] * [0x858B8C] * v * v`; continue while `w < r`.
/// `+0x88`/`+0x8C`/`+0x84` from v and T by vehicle id (0x26, flag 0x1000000 of `+0xD0`, boats
/// 0xA2..=0xAE, others; `+0x8C` clamped to [0x858FDC]), `+0x7C = accel * (4WD ? [0x858C84] :
/// [0x858B8C])`, `cTransmission::InitGearRatios(+0x2C)`. All values at register precision as the
/// original keeps them on the FPU stack.
#[hook(addr = 0x6F5080, conv = thiscall, name = "cHandlingDataMgr::ConvertDataToGameUnits", expect = "51 8B 4C 24 08", this = "cHandlingDataMgr")]
pub fn convert_data_to_game_units(_mgr: GPtr<u8>, d: GPtr<u8>) {
    let s7c = x(k(0xC2B9B4)).fmul_m(rd::<f32>(d, 0x7C));
    let gears = i32::from(rd::<u8>(d, 0x20));
    wr::<f32>(d, 0x7C, s7c.to_f32());
    let s84 = x(k(0xC2B9BC)).fmul_m(rd::<f32>(d, 0x84));
    let t = s84.to_f32();
    wr::<f32>(d, 0x84, t);
    wr::<f32>(d, 0x94, x(k(0xC2B9B4)).fmul_m(rd::<f32>(d, 0x94)).to_f32());
    let inv = x(k(0x858624)).fdiv_m(rd::<f32>(d, 4));
    wr::<f32>(d, 8, inv.to_f32());
    let v24 = x(rd::<f32>(d, 4)).fmul_m(k(0x858C98)).fdiv(xi(gears));
    wr::<f32>(d, 0x24, v24.to_f32());
    let vc8 = inv.fmul_m(rd::<f32>(d, 0xC8)).fmul_m(k(0x8631F0));
    wr::<f32>(d, 0xC8, vc8.to_f32());

    let zero = x(k(0x858B50));
    let one = k(0x858624);
    let mut v = s84;
    loop {
        if jne_mask(v, zero, 0x41) {
            break;
        }
        v = v.fsub(kq(0x85A308));
        let w = s7c.fmul(kq(0x872350));
        let r = if !jp5(x(rd::<f32>(d, 0x10)), x(k(0x858C58))) {
            v.fmul(v)
                .fmul_m(rd::<f32>(d, 0x10))
                .fadd_m(one)
                .fdivr_m(one)
                .fsub_m(one)
                .fmul(v)
                .fchs()
        } else {
            x(rd::<f32>(d, 0x10))
                .fdiv_m(k(0x872348))
                .fmul_m(k(0x858B8C))
                .fmul(v)
                .fmul(v)
        };
        // fcomp w, r; continue while w < r (jnp)
        if jp5(w, r) {
            break;
        }
    }

    let id = rd::<i32>(d, 0);
    // The value left for `+0x8C` (None: `+0x8C` already written, skip the store).
    let y: Option<X87> = if id == 0x26 {
        wr::<f32>(d, 0x88, t);
        Some(x(t).fchs())
    } else if rd::<u32>(d, 0xD0) & 0x0100_0000 != 0 {
        let a = x(t).fmul_m(k(0x863278));
        wr::<f32>(d, 0x88, a.to_f32());
        Some(clamp_8c(a.fmul_m(k(0x85C654))))
    } else if (0xA2..=0xAE).contains(&id) {
        wr::<f32>(d, 0x88, v.to_f32());
        wr::<u32>(d, 0x8C, 0xBD4C_CCCD);
        wr::<f32>(d, 0x84, v.fmul_m(k(0x858F08)).to_f32());
        None
    } else {
        wr::<f32>(d, 0x88, v.to_f32());
        wr::<f32>(d, 0x84, v.fmul_m(k(0x858F08)).to_f32());
        Some(clamp_8c(x(rd::<f32>(d, 0x88)).fmul_m(k(0x858EE4))))
    };
    if let Some(y) = y {
        wr::<f32>(d, 0x8C, y.to_f32());
    }
    let f = if rd::<u8>(d, 0x74) == 0x34 {
        k(0x858C84)
    } else {
        k(0x858B8C)
    };
    wr::<f32>(d, 0x7C, s7c.fmul_m(f).to_f32());
    init_gear_ratios(d.byte_add(0x2C));
}

/// `fld [0x858FDC]; fcomp st(1); test ah, 5; jp`: `y` replaced by the constant when the constant < y.
fn clamp_8c(y: X87) -> X87 {
    let m = x(k(0x858FDC));
    if jp5(m, y) { y } else { m }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_HANDLING_DATA_MGR,
    HOOK_CONVERT_BIKE_DATA_TO_WORLD_UNITS,
    HOOK_CONVERT_DATA_TO_GAME_UNITS,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
