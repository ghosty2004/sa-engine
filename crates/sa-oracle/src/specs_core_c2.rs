//! L1 specs for PORT-CORE fork C2 (`sa_core::input`, `sa_core::light`): functions whose only state
//! is a few globals (key states, the game clock, the OS version) and one import. Their global reads
//! are made L1 inputs by patching the absolute addresses in the original's instructions to point
//! into scratch regions (same instructions, redirected operands); `MapVirtualKeyA` is replaced by a
//! call to a stub (`mov eax, [scratch]; ret 8`) written over the catalogued start 0x747000, which is
//! allow-listed as a callee. See `docs/arch/core-port.md`.

use crate::call::{Conv::*, SCRATCH};
use sa_core::mem::GPtr;
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

#[cfg(test)]
fn nz16(w: u32) -> bool {
    w & 0xFFFF != 0
}

/// Byte offset of a key word in `CKeyboardState` from the absolute operand at `site + 3` of the
/// unpatched exe (`cmp word ptr [abs], 0`), and whether it is the new (0xB73190) or old (0xB72F20)
/// state. The offsets come from the original's bytes, not from the port.
fn key_word_of_site(site: u32) -> (bool, usize) {
    let img = crate::image_cached().expect("gta_sa.exe 1.0 US");
    let a = img.u32(site + 3).expect("operand");
    let (new, base) = if (0xB73190..0xB73190 + 0x270).contains(&a) {
        (true, 0xB73190)
    } else {
        assert!((0xB72F20..0xB72F20 + 0x270).contains(&a), "{site:#x}: {a:#x} is no key state word");
        (false, 0xB72F20)
    };
    (new, (a - base) as usize)
}

fn set_key(state: &sa_core::mem::Global<input::KeyboardState>, byte_off: usize, v: u16) {
    let mut s = state.get();
    s[byte_off / 2] = v as i16;
    state.set(s);
}

/// `CPad::Is*JustPressed` over `n` (new, old) word pairs whose `cmp word ptr [abs], 0` operands are
/// at `sites` (address bytes at site + 3), in the original's order new0, old0, new1, old1, ...
/// The Rust side is the hooked entry `entry`, run on sa-core's host key states filled from the
/// scratch words (zero elsewhere).
fn key_spec(key: &'static str, name: &'static str, addr: u32, ret_al: bool, sites: &'static [u32], entry: fn() -> u32) -> Spec {
    let patches = sites
        .iter()
        .enumerate()
        .map(|(k, s)| addr_patch(s + 3, R0 + 4 * k as u32))
        .collect();
    b(key, name, addr, Cdecl, if ret_al { Al } else { Eax })
        .regions(vec![raw(sites.len())])
        .patch(patches)
        .rust(move |c| {
            let words: Vec<(bool, usize)> = sites.iter().map(|&s| key_word_of_site(s)).collect();
            let _g = crate::host_globals();
            input::NEW_KEY_STATE.set([0; 0x138]);
            input::OLD_KEY_STATE.set([0; 0x138]);
            for (k, &(new, off)) in words.iter().enumerate() {
                let st = if new { &input::NEW_KEY_STATE } else { &input::OLD_KEY_STATE };
                set_key(st, off, c.u(0, k) as u16);
            }
            let r = entry();
            if ret_al {
                ExpRet::Al(r as u8)
            } else {
                ExpRet::Eax(r)
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

/// Scratch address of `GTATranslateKey`'s region 2 (the recorded `MapVirtualKeyA` arguments):
/// regions 0 (2 words) and 1 (1 word) take 0x20 bytes each from 0x10.
const MAP_VK_ARGS: u32 = SCRATCH + 0x50;

/// Stub over 0x747000 (stdcall, `ret 8`): records MapVirtualKeyA's two arguments at region 2 and
/// returns region 0 word 1:
/// `mov eax, [esp+4]; mov [R2], eax; mov eax, [esp+8]; mov [R2+4], eax; mov eax, [R0+4]; ret 8`.
fn map_vk_stub() -> Vec<(u32, Vec<u8>)> {
    let mut stub = vec![0x8B, 0x44, 0x24, 0x04, 0xA3];
    stub.extend_from_slice(&MAP_VK_ARGS.to_le_bytes());
    stub.extend_from_slice(&[0x8B, 0x44, 0x24, 0x08, 0xA3]);
    stub.extend_from_slice(&(MAP_VK_ARGS + 4).to_le_bytes());
    stub.push(0xA1);
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
                // the hooked entry on the host clock globals: hours = byte 3, minutes = byte 2,
                // seconds = the low word (the patched operands above)
                let w = c.u(0, 0);
                let _g = crate::host_globals();
                light::CLOCK_HOURS.set((w >> 24) as u8);
                light::CLOCK_MINUTES.set((w >> 16) as u8);
                light::CLOCK_SECONDS.set(w as u16);
                st0(light::get_day_night_balance())
            }),
        // ------------------------------------------------------------------ CPad
        key_spec(
            "CPadIsEnterJustPressed",
            "CPad::IsEnterJustPressed",
            0x4D5980,
            true,
            &[0x4D5980, 0x4D598A, 0x4D5994, 0x4D599E],
            || input::is_enter_just_pressed() as u32,
        ),
        key_spec(
            "CPadIsMenuKeyJustPressed",
            "CPad::IsMenuKeyJustPressed",
            0x744D50,
            false,
            &[0x744D50, 0x744D5A, 0x744D64, 0x744D6E],
            input::is_menu_key_just_pressed,
        ),
        key_spec(
            "CPadIsTabJustPressed",
            "CPad::IsTabJustPressed",
            0x744D90,
            false,
            &[0x744D90, 0x744D9A],
            input::is_tab_just_pressed,
        ),
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
            // `cmp word ptr [eax*2 + abs], 0`: the abs operands (key 0's word in the new / old
            // state) are patched to regions 0 / 1, which hold the 256 key words
            let (new0, off_new) = key_word_of_site(0x4D59BA - 3);
            let (new1, off_old) = key_word_of_site(0x4D59C4 - 3);
            assert!(new0 && !new1);
            let _g = crate::host_globals();
            let mut new = [0i16; 0x138];
            let mut old = [0i16; 0x138];
            for k in 0..0x100 {
                new[off_new / 2 + k] = (c.u(0, k / 2) >> (16 * (k % 2))) as i16;
                old[off_old / 2 + k] = (c.u(1, k / 2) >> (16 * (k % 2))) as i16;
            }
            input::NEW_KEY_STATE.set(new);
            input::OLD_KEY_STATE.set(old);
            ExpRet::Eax(input::is_standard_key_just_pressed(GPtr::from_addr(c.i(0) as usize), c.i(1)))
        }),
        // ------------------------------------------------------------------ Win
        b("GTATranslateKey", "Win::GTATranslateKey", 0x747820, Cdecl, Al)
            .regions(vec![raw(2), out1(), raw(2)])
            .args(vec![A::P(1), A::I(g_lparam), A::I(g_vk)])
            .fix(|i, l| {
                // region 0 word 0 = OS version: mostly 0 or 1 (Windows 98)
                let os = l.word_slot[0][0].unwrap();
                i[os] = match i[os] % 4 {
                    0 | 1 => 0,
                    2 => 1,
                    _ => i[os],
                };
                // region 0 word 1 = MapVirtualKeyA's result: the stored char must be `< 0xFF`
                // (gta-reversed: `<= 0xFF`), so the low half is biased to the boundary
                // (VER-2, verifier finding V-11); the upper half stays random or zero
                let chr = l.word_slot[0][1].unwrap();
                let v = i[chr];
                let hi = if v & 0x8000_0000 != 0 { v & 0xFFFF_0000 } else { 0 };
                i[chr] = match (v >> 8) % 16 {
                    0..=4 => hi | 0xFF,
                    5 | 6 => hi | 0xFE,
                    7 | 8 => hi | 0x100,
                    9 => hi | 0xFFFF,
                    10 => hi,
                    _ => v,
                };
            })
            .callees(&[0x747000])
            .patch(map_vk_stub())
            .rust(|c| {
                // the hooked entry: OS version from region 0 word 0, MapVirtualKeyA through a
                // stand-in that records its arguments at region 2 and returns region 0 word 1
                assert_eq!(c.addr(2), MAP_VK_ARGS);
                thread_local! {
                    static MAP_VK: std::cell::Cell<(u32, usize)> = const { std::cell::Cell::new((0, 0)) };
                }
                fn map_vk(code: u32, kind: u32) -> u32 {
                    let (chr, rec) = MAP_VK.with(std::cell::Cell::get);
                    let rec = GPtr::<[u32; 2]>::from_raw(rec as *mut [u32; 2]);
                    rec.write([code, kind]);
                    chr
                }
                sa_core::orig::host::set::<fn(u32, u32) -> u32>("sa_core::input::map_virtual_key_a", map_vk);
                let rec = c.mem.as_mut_ptr().wrapping_add(c.lay.off[2]) as usize;
                MAP_VK.with(|m| m.set((c.u(0, 1), rec)));
                let ck = GPtr::<i32>::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[1]).cast());
                let (lp, vk) = (c.i(1), c.i(2));
                let _g = crate::host_globals();
                input::OS_VER.set(c.u(0, 0) as i32);
                let r = input::gta_translate_key(ck, lp, vk);
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
