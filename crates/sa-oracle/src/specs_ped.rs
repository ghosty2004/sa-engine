//! L1 specs for PORT-WEAPON-PED `sa_ped` (see `docs/arch/weapon-ped-port.md`): the pure PED leaves whose
//! memory fits the hosted scratch image (a `CPed` prefix, a `CWanted`, a `CPlayerInfo` prefix, a
//! `CPedClothesDesc`), and every hook that preserves registers (globals read or written through
//! absolute operands are patched to scratch regions and mirrored into sa-ped's host globals).

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::abi::{PRESERVE_ECX, PRESERVE_EDX};
use sa_core::mem::GPtr;
use sa_ped::{clothes, damage, ped, ped_type, player_info, wanted};

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x9E_D0_57;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// Scratch offset of region `r` for regions of the given word counts (the `B::layout` rule).
fn off(words: &[usize], r: usize) -> u32 {
    let mut o = 0x10usize;
    for w in &words[..r] {
        o += (4 * w + 0x10 + 0xF) & !0xF;
    }
    (SCRATCH as usize + o) as u32
}

/// Sets word `w` of region `r` to a value below `n` in most cases (kept raw otherwise).
fn small(v: &mut [u32], l: &Layout, r: usize, w: usize, n: u32) {
    if let Some(s) = l.word_slot[r][w] {
        let mut g = Rng::new(seed(v) ^ (w as u64) << 8);
        if g.below(100) < 92 {
            v[s] = g.below(n);
        }
    }
}

/// Sets the byte at `o` of region `r` below `n` (other bytes of the word kept).
fn small_byte(v: &mut [u32], l: &Layout, r: usize, o: usize, n: u32) {
    if let Some(s) = l.word_slot[r][o / 4] {
        let mut g = Rng::new(seed(v) ^ (o as u64) << 16);
        if g.below(100) < 92 {
            let sh = 8 * (o % 4);
            v[s] = (v[s] & !(0xFF << sh)) | (g.below(n) << sh);
        }
    }
}

/// Clears each word of region `r` with probability `pct` (sparse key tables).
fn sparse(v: &mut [u32], l: &Layout, r: usize, pct: u32) {
    let mut g = Rng::new(seed(v));
    for s in l.word_slot[r].iter().flatten() {
        if g.below(100) < pct {
            v[*s] = 0;
        }
    }
}

fn g_type(r: &mut Rng) -> u32 {
    if r.chance(90) { r.below(40) } else { r.next_u32() }
}

fn g_level(r: &mut Rng) -> u32 {
    if r.chance(90) {
        (r.below(11) as i32 - 2) as u32
    } else {
        r.next_u32()
    }
}

const NAMES: [&[u8]; 11] = [
    b"torso\0",
    b"head\0",
    b"hands\0",
    b"legs\0",
    b"feet\0",
    b"necklace\0",
    b"watch\0",
    b"glasses\0",
    b"hat\0",
    b"extra1\0",
    b"torsx\0",
];

/// A thiscall predicate on a `CWanted` returning EAX.
fn wanted_eax(key: &'static str, name: &'static str, addr: u32, f: fn(GPtr<u8>) -> u32) -> Spec {
    b(key, name, addr, Thiscall, Eax)
        .regions(vec![raw(0xA7)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            small(v, l, 0, 11, 9);
            small_byte(v, l, 0, 0x1E, 0x40);
        })
        .rust(move |c| ExpRet::Eax(f(tp(c, 0))))
}

pub fn all() -> Vec<Spec> {
    // SetMaximumWantedLevel: the two globals' stores, patched to regions 0 / 1
    let max_words = [1usize, 1];
    let (max_lvl, max_chaos) = (off(&max_words, 0), off(&max_words, 1));
    let mut max_patch = Vec::new();
    for (k, a) in [
        0x561E84u32,
        0x561E8E,
        0x561E99,
        0x561EA3,
        0x561EAE,
        0x561EB8,
        0x561EC3,
        0x561ECD,
        0x561ED8,
        0x561EE2,
        0x561EED,
        0x561EF7,
    ]
    .iter()
    .enumerate()
    {
        max_patch.push((
            a + 2,
            (if k % 2 == 0 { max_lvl } else { max_chaos })
                .to_le_bytes()
                .to_vec(),
        ));
    }
    max_patch.push((0x561F05, max_lvl.to_le_bytes().to_vec()));
    max_patch.push((0x561F0A, max_chaos.to_le_bytes().to_vec()));

    let v = vec![
        b(
            "PedCrimeFindImmediateDetectionRange",
            "CCrime::FindImmediateDetectionRange",
            0x531FC0,
            Cdecl,
            St0,
        )
        .args(vec![A::I(g_type)])
        .preserve(PRESERVE_ECX)
        .rust(|c| st0(wanted::find_immediate_detection_range(c.i(0) as i32))),
        b(
            "PedWantedSetMaximumWantedLevel",
            "CWanted::SetMaximumWantedLevel",
            0x561E70,
            Cdecl,
            Void,
        )
        .regions(vec![raw(1), raw(1)])
        .args(vec![A::I(g_level)])
        .patch(max_patch)
        .preserve(PRESERVE_ECX)
        .rust(|c| {
            let _g = crate::host_globals();
            wanted::MAXIMUM_WANTED_LEVEL.set(c.u(0, 0) as i32);
            wanted::MAXIMUM_CHAOS_LEVEL.set(c.u(1, 0) as i32);
            wanted::set_maximum_wanted_level(c.i(0) as i32);
            c.set_u(0, 0, wanted::MAXIMUM_WANTED_LEVEL.get() as u32);
            c.set_u(1, 0, wanted::MAXIMUM_CHAOS_LEVEL.get() as u32);
            ExpRet::Void
        }),
        b(
            "PedClothesGetTextureDependency",
            "CClothes::GetTextureDependency",
            0x5A7EA0,
            Cdecl,
            Eax,
        )
        .args(vec![A::I(g_type)])
        .preserve(PRESERVE_ECX | PRESERVE_EDX)
        .rust(|c| ExpRet::Eax(clothes::get_texture_dependency(c.i(0) as i32) as u32)),
        b(
            "PedClothesGetDependentTexture",
            "CClothes::GetDependentTexture",
            0x5A7F30,
            Cdecl,
            Eax,
        )
        .args(vec![A::I(g_type)])
        .rust(|c| ExpRet::Eax(clothes::get_dependent_texture(c.i(0) as i32) as u32)),
        b(
            "PedGetClothesModelFromName",
            "GetClothesModelFromName",
            0x5A7A20,
            Cdecl,
            Eax,
        )
        .regions(vec![raw(4)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            let s0 = l.word_slot[0][0].unwrap();
            let pick = (v[s0] % 13) as usize;
            if pick < NAMES.len() {
                let mut bytes = [0u8; 16];
                bytes[..NAMES[pick].len()].copy_from_slice(NAMES[pick]);
                for w in 0..4 {
                    let s = l.word_slot[0][w].unwrap();
                    v[s] = u32::from_le_bytes(bytes[4 * w..4 * w + 4].try_into().unwrap());
                }
            }
        })
        .rust(|c| ExpRet::Eax(clothes::get_clothes_model_from_name(tp(c, 0)) as u32)),
        b("PedTypeGetPedFlag", "CPedType::GetPedFlag", 0x608830, Cdecl, Eax)
            .args(vec![A::I(g_type)])
            .rust(|c| ExpRet::Eax(ped_type::get_ped_flag(c.i(0) as i32))),
        b(
            "PedTypePoliceDontCare",
            "CPedType::PoliceDontCareAboutCrimesAgainstPedType",
            0x608A40,
            Cdecl,
            Al,
        )
        .args(vec![A::I(g_type)])
        .rust(|c| {
            ExpRet::Al(ped_type::police_dont_care_about_crimes_against_ped_type(
                c.i(0) as i32
            ))
        }),
        b(
            "PedCanBeDeletedEvenInVehicle",
            "CPed::CanBeDeletedEvenInVehicle",
            0x5DF150,
            Thiscall,
            Al,
        )
        .regions(vec![raw(0x122)])
        .args(vec![A::P(0)])
        .fix(|v, l| small_byte(v, l, 0, 0x484, 6))
        .rust(|c| ExpRet::Al(ped::can_be_deleted_even_in_vehicle(tp(c, 0)))),
        b(
            "PedSetStayInSamePlace",
            "CPed::SetStayInSamePlace",
            0x481090,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x11C)])
        .args(vec![A::P(0), A::I(g_bool)])
        .rust(|c| {
            ped::set_stay_in_same_place(tp(c, 0), c.i(1));
            ExpRet::Void
        }),
        wanted_eax(
            "PedWantedAreSwatRequired",
            "CWanted::AreSwatRequired",
            0x561F40,
            wanted::are_swat_required,
        ),
        wanted_eax(
            "PedWantedAreFbiRequired",
            "CWanted::AreFbiRequired",
            0x561F60,
            wanted::are_fbi_required,
        ),
        wanted_eax(
            "PedWantedAreArmyRequired",
            "CWanted::AreArmyRequired",
            0x561F80,
            wanted::are_army_required,
        ),
        wanted_eax(
            "PedWantedAreMiamiViceRequired",
            "CWanted::AreMiamiViceRequired",
            0x561F30,
            wanted::are_miami_vice_required,
        ),
        wanted_eax(
            "PedWantedNumOfHelisRequired",
            "CWanted::NumOfHelisRequired",
            0x561FA0,
            wanted::num_of_helis_required,
        ),
        b(
            "PedPlayerInfoIsRestartingAfterDeath",
            "CPlayerInfo::IsRestartingAfterDeath",
            0x56E550,
            Thiscall,
            Al,
        )
        .regions(vec![raw(0x38)])
        .args(vec![A::P(0)])
        .fix(|v, l| small_byte(v, l, 0, 0xDC, 5))
        .rust(|c| ExpRet::Al(player_info::is_restarting_after_death(tp(c, 0)))),
        b(
            "PedPlayerInfoIsPlayerInRemoteMode",
            "CPlayerInfo::IsPlayerInRemoteMode",
            0x56DAB0,
            Thiscall,
            Al,
        )
        .regions(vec![raw(0x38)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            let mut g = Rng::new(seed(v));
            if g.chance(60)
                && let Some(s) = l.word_slot[0][0x2C]
            {
                v[s] = 0;
            }
            small_byte(v, l, 0, 0xDD, 3);
        })
        .rust(|c| ExpRet::Al(player_info::is_player_in_remote_mode(tp(c, 0)))),
        b(
            "PedDamageCalculatorCtor",
            "CPedDamageResponseCalculator::Constructor",
            0x4AD3F0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(5)])
        .args(vec![
            A::P(0),
            A::I(g_rand),
            A::F,
            A::I(g_type),
            A::I(g_small_int),
            A::I(g_rand),
        ])
        .rust(|c| {
            damage::constructor(tp(c, 0), c.i(1), c.f(2).to_bits(), c.i(3), c.i(4), c.i(5));
            ExpRet::Eax(c.addr(0))
        }),
        b(
            "PedClothesDescInitialise",
            "CPedClothesDesc::Initialise",
            0x5A78F0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(30)])
        .args(vec![A::P(0)])
        .rust(|c| {
            clothes::desc_initialise(tp(c, 0));
            ExpRet::Void
        }),
        b(
            "PedClothesDescHasVisibleTattoo",
            "CPedClothesDesc::HasVisibleTattoo",
            0x5A79D0,
            Thiscall,
            Al,
        )
        .regions(vec![raw(30)])
        .args(vec![A::P(0)])
        .fix(|v, l| sparse(v, l, 0, 93))
        .rust(|c| ExpRet::Al(clothes::has_visible_tattoo(tp(c, 0)))),
        b(
            "PedClothesDescSetModelHash",
            "CPedClothesDesc::SetModel-modelHash",
            0x5A7910,
            Thiscall,
            Void,
        )
        .regions(vec![raw(30)])
        .args(vec![A::P(0), A::I(g_rand), A::I(|r| r.below(10))])
        .rust(|c| {
            clothes::set_model_hash(tp(c, 0), c.i(1), c.i(2) as i32);
            ExpRet::Void
        }),
    ];
    v
}
