//! L1 specs for PORT-CORE fork C2 (`sa_core::input`, `sa_core::light`): functions whose only state
//! is a few globals (key states, the game clock, the OS version) and one import. Their global reads
//! are made L1 inputs by patching the absolute addresses in the original's instructions to point
//! into scratch regions (same instructions, redirected operands); `MapVirtualKeyA` is replaced by a
//! call to a stub (`mov eax, [scratch]; ret 8`) written over the catalogued start 0x747000, which is
//! allow-listed as a callee. See `docs/arch/core-port.md`.

use crate::call::{Conv::*, SCRATCH};
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::{input, light};
use sa_math::x87::X87;

/// Region 0 starts at scratch + 0x10; regions are 0x10-aligned with 0x10 bytes of slack.
const R0: u32 = SCRATCH + 0x10;

fn addr_patch(at: u32, target: u32) -> (u32, Vec<u8>) {
    (at, target.to_le_bytes().to_vec())
}

fn st0(v: X87) -> ExpRet {
    ExpRet::St0 {
        f32: v.to_f32().to_bits(),
        ext: v.to_ext(),
    }
}

fn nz16(w: u32) -> bool {
    w & 0xFFFF != 0
}

/// `CPad::Is*JustPressed` over `n` (new, old) word pairs whose `cmp word ptr [abs], 0` operands are
/// at `sites` (address bytes at site + 3), in the original's order new0, old0, new1, old1, ...
fn key_spec(key: &'static str, name: &'static str, addr: u32, ret_al: bool, sites: &'static [u32]) -> Spec {
    let patches = sites
        .iter()
        .enumerate()
        .map(|(k, s)| addr_patch(s + 3, R0 + 4 * k as u32))
        .collect();
    b(key, name, addr, Cdecl, if ret_al { Al } else { Eax })
        .regions(vec![raw(sites.len())])
        .patch(patches)
        .rust(move |c| {
            let any = (0..sites.len() / 2).any(|p| nz16(c.u(0, 2 * p)) && !nz16(c.u(0, 2 * p + 1)));
            if ret_al {
                ExpRet::Al(any as u8)
            } else {
                ExpRet::Eax(any as u32)
            }
        })
}

fn g_key(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => r.next_u32(),
        _ => r.below(0x100),
    }
}

fn g_vk(r: &mut Rng) -> u32 {
    match r.below(10) {
        0 => r.next_u32(),
        1 => r.below(0x10000),
        _ => r.below(0xA0),
    }
}

fn g_lparam(r: &mut Rng) -> u32 {
    r.next_u32() & if r.chance(50) { !0x0100_0000 } else { !0 }
}

/// Stub over 0x747000: `mov eax, [R0 + 4]; ret 8` (MapVirtualKeyA's result = region 0 word 1).
fn map_vk_stub() -> Vec<(u32, Vec<u8>)> {
    let mut stub = vec![0xA1];
    stub.extend_from_slice(&(R0 + 4).to_le_bytes());
    stub.extend_from_slice(&[0xC2, 0x08, 0x00]);
    let site = 0x747B46u32;
    let rel = 0x747000u32.wrapping_sub(site + 5);
    let mut call = vec![0xE8];
    call.extend_from_slice(&rel.to_le_bytes());
    call.push(0x90);
    vec![(0x747000, stub), (site, call), addr_patch(0x74784D, R0)]
}

/// Every C2 spec.
pub fn all() -> Vec<Spec> {
    vec![
        // ------------------------------------------------------------------ Light
        b("GetDayNightBalance", "Light::GetDayNightBalance", 0x6FAB30, Cdecl, St0)
            .regions(vec![raw(1)])
            .patch(vec![
                addr_patch(0x6FAB34, R0 + 3),
                addr_patch(0x6FAB3B, R0 + 2),
                addr_patch(0x6FAB45, R0),
            ])
            .rust(|c| {
                let w = c.u(0, 0);
                st0(light::day_night_balance((w >> 24) as u8, (w >> 16) as u8, w as u16))
            }),
        // ------------------------------------------------------------------ CPad
        key_spec(
            "CPadIsEnterJustPressed",
            "CPad::IsEnterJustPressed",
            0x4D5980,
            true,
            &[0x4D5980, 0x4D598A, 0x4D5994, 0x4D599E],
        ),
        key_spec(
            "CPadIsMenuKeyJustPressed",
            "CPad::IsMenuKeyJustPressed",
            0x744D50,
            false,
            &[0x744D50, 0x744D5A, 0x744D64, 0x744D6E],
        ),
        key_spec("CPadIsTabJustPressed", "CPad::IsTabJustPressed", 0x744D90, false, &[0x744D90, 0x744D9A]),
        b(
            "CPadIsStandardKeyJustPressed",
            "CPad::IsStandardKeyJustPressed",
            0x4D59B0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(128), raw(128)])
        .args(vec![A::I(|r| r.next_u32()), A::I(g_key)])
        .patch(vec![addr_patch(0x4D59BA, R0), addr_patch(0x4D59C4, R0 + 0x210)])
        .rust(|c| {
            let k = (c.i(1) & 0xFF) as usize;
            let (w, h) = (k / 2, 16 * (k % 2));
            let new = (c.u(0, w) >> h) & 0xFFFF;
            let old = (c.u(1, w) >> h) & 0xFFFF;
            ExpRet::Eax((new != 0 && old == 0) as u32)
        }),
        // ------------------------------------------------------------------ Win
        b("GTATranslateKey", "Win::GTATranslateKey", 0x747820, Cdecl, Al)
            .regions(vec![raw(2), out1()])
            .args(vec![A::P(1), A::I(g_lparam), A::I(g_vk)])
            .fix(|i, l| {
                // region 0 word 0 = OS version: mostly 0 or 1 (Windows 98)
                let os = l.word_slot[0][0].unwrap();
                i[os] = match i[os] % 4 {
                    0 | 1 => 0,
                    2 => 1,
                    _ => i[os],
                };
            })
            .callees(&[0x747000])
            .patch(map_vk_stub())
            .rust(|c| {
                let (vk, lp, os) = (c.i(2), c.i(1), c.u(0, 0) as i32);
                let (code, r) = input::translate_key_case(vk, lp, os)
                    .unwrap_or_else(|| input::translate_key_default(c.u(0, 1)));
                c.set_u(1, 0, code as u32);
                ExpRet::Al(r as u8)
            }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{self, Backend, Cfg};
    use crate::specs::Expect;
    use std::sync::Arc;

    fn quick() -> Cfg {
        Cfg {
            cases: 20_000,
            backend: Backend::Unicorn,
            threads: 4,
            fixtures: false,
            keep: 1,
            shrink: false,
            snan_native: false,
            ..Cfg::default()
        }
    }

    fn spec(key: &str) -> Spec {
        all().into_iter().find(|s| s.key == key).unwrap()
    }

    /// The specs pass, and gta-reversed's `IsMenuKeyJustPressed` (`lalt` only) is caught. Skipped
    /// without the exe.
    #[test]
    fn c2_specs_pass_and_discriminate() {
        let Some(img) = crate::image_cached() else { return };
        for s in all() {
            assert_eq!(runner::run(&s, &img, &quick()).unwrap().mismatches, 0, "{}", s.key);
        }
        let mut s = spec("CPadIsMenuKeyJustPressed");
        s.rust = Arc::new(|_, sc| {
            let w = |k: usize| u32::from_le_bytes(sc[0x10 + 4 * k..0x14 + 4 * k].try_into().unwrap());
            Expect::Returns {
                ret: ExpRet::Eax((nz16(w(0)) && !nz16(w(1))) as u32),
                scratch: *sc,
            }
        });
        assert!(runner::run(&s, &img, &quick()).unwrap().mismatches > 0);
    }
}
