//! L1 specs for PORT-COLL `SurfaceInfos_c` (`sa_world::collision::surface`, see
//! `docs/arch/coll-port.md`): the table getters, `GetTyreGrip`, `GetWetMultiplier`,
//! `GetAdhesiveLimit` and `GetSurfaceIdFromName`. Every spec runs the hooked entry on the host on a copy
//! of the scratch image (`this` = region 0, a 0x100-byte slice of the table: the adhesive-limit matrix
//! and entries 0..8). Surface ids are kept in 0..8 so every read stays in the region.
//! `CWeather::WetRoads` (0xC81308) is read through the `fmul dword [abs]` operand at 0x55E61D, patched
//! to a scratch word (region 1), and copied into sa-world's host global for the port.

use crate::call::{Conv::*, SCRATCH};
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_world::collision::surface::{self as s, SiPtr};

type Getter = fn(SiPtr, u32) -> u32;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn rng(seed: u32) -> Rng {
    Rng::new(u64::from(seed) | 1 << 41)
}

fn g_id(r: &mut Rng) -> u32 {
    r.below(9)
}

/// The getters of the form `(table word >> shift) & mask` (one spec each).
pub fn getters() -> Vec<(u32, &'static str, &'static str, Getter)> {
    vec![
        (
            0x55E5C0,
            "SurfaceInfos_c::GetAdhesionGroup",
            "SurfaceInfosGetAdhesionGroup",
            s::get_adhesion_group as Getter,
        ),
        (
            0x55E630,
            "SurfaceInfos_c::GetSkidmarkType",
            "SurfaceInfosGetSkidmarkType",
            s::get_skidmark_type as Getter,
        ),
        (
            0x55E650,
            "SurfaceInfos_c::GetFrictionEffect",
            "SurfaceInfosGetFrictionEffect",
            s::get_friction_effect as Getter,
        ),
        (
            0x55E670,
            "SurfaceInfos_c::GetBulletFx",
            "SurfaceInfosGetBulletFx",
            s::get_bullet_fx as Getter,
        ),
        (
            0x55E690,
            "SurfaceInfos_c::IsSoftLanding",
            "SurfaceInfosIsSoftLanding",
            s::is_soft_landing as Getter,
        ),
        (
            0x55E6B0,
            "SurfaceInfos_c::IsSeeThrough",
            "SurfaceInfosIsSeeThrough",
            s::is_see_through as Getter,
        ),
        (
            0x55E6D0,
            "SurfaceInfos_c::IsShootThrough",
            "SurfaceInfosIsShootThrough",
            s::is_shoot_through as Getter,
        ),
        (
            0x55E6F0,
            "SurfaceInfos_c::IsSand",
            "SurfaceInfosIsSand",
            s::is_sand as Getter,
        ),
        (
            0x55E710,
            "SurfaceInfos_c::IsWater",
            "SurfaceInfosIsWater",
            s::is_water as Getter,
        ),
        (
            0x55E730,
            "SurfaceInfos_c::IsShallowWater",
            "SurfaceInfosIsShallowWater",
            s::is_shallow_water as Getter,
        ),
        (
            0x55E750,
            "SurfaceInfos_c::IsBeach",
            "SurfaceInfosIsBeach",
            s::is_beach as Getter,
        ),
        (
            0x55E770,
            "SurfaceInfos_c::IsSteepSlope",
            "SurfaceInfosIsSteepSlope",
            s::is_steep_slope as Getter,
        ),
        (
            0x55E790,
            "SurfaceInfos_c::IsGlass",
            "SurfaceInfosIsGlass",
            s::is_glass as Getter,
        ),
        (
            0x55E7B0,
            "SurfaceInfos_c::IsStairs",
            "SurfaceInfosIsStairs",
            s::is_stairs as Getter,
        ),
        (
            0x55E7D0,
            "SurfaceInfos_c::IsSkateable",
            "SurfaceInfosIsSkateable",
            s::is_skateable as Getter,
        ),
        (
            0x55E7F0,
            "SurfaceInfos_c::IsPavement",
            "SurfaceInfosIsPavement",
            s::is_pavement as Getter,
        ),
        (
            0x55E810,
            "SurfaceInfos_c::GetRoughness",
            "SurfaceInfosGetRoughness",
            s::get_roughness as Getter,
        ),
        (
            0x55E830,
            "SurfaceInfos_c::GetFlammability",
            "SurfaceInfosGetFlammability",
            s::get_flammability as Getter,
        ),
        (
            0x55E850,
            "SurfaceInfos_c::CreatesSparks",
            "SurfaceInfosCreatesSparks",
            s::creates_sparks as Getter,
        ),
        (
            0x55E870,
            "SurfaceInfos_c::CantSprintOn",
            "SurfaceInfosCantSprintOn",
            s::cant_sprint_on as Getter,
        ),
        (
            0x55E890,
            "SurfaceInfos_c::LeavesFootsteps",
            "SurfaceInfosLeavesFootsteps",
            s::leaves_footsteps as Getter,
        ),
        (
            0x55E8B0,
            "SurfaceInfos_c::ProducesFootDust",
            "SurfaceInfosProducesFootDust",
            s::produces_foot_dust as Getter,
        ),
        (
            0x55E8D0,
            "SurfaceInfos_c::MakesCarDirty",
            "SurfaceInfosMakesCarDirty",
            s::makes_car_dirty as Getter,
        ),
        (
            0x55E8F0,
            "SurfaceInfos_c::MakesCarClean",
            "SurfaceInfosMakesCarClean",
            s::makes_car_clean as Getter,
        ),
        (
            0x55E910,
            "SurfaceInfos_c::CreatesWheelGrass",
            "SurfaceInfosCreatesWheelGrass",
            s::creates_wheel_grass as Getter,
        ),
        (
            0x55E930,
            "SurfaceInfos_c::CreatesWheelGravel",
            "SurfaceInfosCreatesWheelGravel",
            s::creates_wheel_gravel as Getter,
        ),
        (
            0x55E950,
            "SurfaceInfos_c::CreatesWheelMud",
            "SurfaceInfosCreatesWheelMud",
            s::creates_wheel_mud as Getter,
        ),
        (
            0x55E970,
            "SurfaceInfos_c::CreatesWheelDust",
            "SurfaceInfosCreatesWheelDust",
            s::creates_wheel_dust as Getter,
        ),
        (
            0x55E990,
            "SurfaceInfos_c::CreatesWheelSand",
            "SurfaceInfosCreatesWheelSand",
            s::creates_wheel_sand as Getter,
        ),
        (
            0x55E9B0,
            "SurfaceInfos_c::CreatesWheelSpray",
            "SurfaceInfosCreatesWheelSpray",
            s::creates_wheel_spray as Getter,
        ),
        (
            0x55E9D0,
            "SurfaceInfos_c::CreatesPlants",
            "SurfaceInfosCreatesPlants",
            s::creates_plants as Getter,
        ),
        (
            0x55E9F0,
            "SurfaceInfos_c::CreatesObjects",
            "SurfaceInfosCreatesObjects",
            s::creates_objects as Getter,
        ),
        (
            0x55EA10,
            "SurfaceInfos_c::CanClimb",
            "SurfaceInfosCanClimb",
            s::can_climb as Getter,
        ),
        (
            0x55EA30,
            "SurfaceInfos_c::IsAudioConcrete",
            "SurfaceInfosIsAudioConcrete",
            s::is_audio_concrete as Getter,
        ),
        (
            0x55EA50,
            "SurfaceInfos_c::IsAudioGrass",
            "SurfaceInfosIsAudioGrass",
            s::is_audio_grass as Getter,
        ),
        (
            0x55EA70,
            "SurfaceInfos_c::IsAudioSand",
            "SurfaceInfosIsAudioSand",
            s::is_audio_sand as Getter,
        ),
        (
            0x55EA90,
            "SurfaceInfos_c::IsAudioGravel",
            "SurfaceInfosIsAudioGravel",
            s::is_audio_gravel as Getter,
        ),
        (
            0x55EAB0,
            "SurfaceInfos_c::IsAudioWood",
            "SurfaceInfosIsAudioWood",
            s::is_audio_wood as Getter,
        ),
        (
            0x55EAD0,
            "SurfaceInfos_c::IsAudioWater",
            "SurfaceInfosIsAudioWater",
            s::is_audio_water as Getter,
        ),
        (
            0x55EAF0,
            "SurfaceInfos_c::IsAudioMetal",
            "SurfaceInfosIsAudioMetal",
            s::is_audio_metal as Getter,
        ),
        (
            0x55EB10,
            "SurfaceInfos_c::IsAudioLongGrass",
            "SurfaceInfosIsAudioLongGrass",
            s::is_audio_long_grass as Getter,
        ),
        (
            0x55EB30,
            "SurfaceInfos_c::IsAudioTile",
            "SurfaceInfosIsAudioTile",
            s::is_audio_tile as Getter,
        ),
    ]
}

fn getter_spec(addr: u32, name: &'static str, key: &'static str, f: Getter) -> Spec {
    b(key, name, addr, Thiscall, Eax)
        .regions(vec![raw(64)])
        .args(vec![A::P(0), A::I(g_id)])
        .rust(move |c| {
            let id = c.i(1);
            ExpRet::Eax(f(tp(c, 0), id))
        })
}

/// Region-1 name: a table name (sometimes mutated: a changed byte, a cut, an extra byte), NUL-padded.
fn fix_name(v: &mut [u32], lay: &Layout) {
    let w = &lay.word_slot[1];
    let mut r = rng(v[w[0].unwrap()] ^ v[w[3].unwrap()].rotate_left(7));
    let mut buf = [0u8; 64];
    if r.chance(90) {
        let n = s::SURFACE_NAMES[r.below(s::SURFACE_NAMES.len() as u32) as usize];
        buf[..n.len()].copy_from_slice(n);
        let len = n.len();
        match r.below(8) {
            0 => buf[r.below(len as u32) as usize] ^= 1 << r.below(7),
            1 => buf[r.below(len as u32) as usize] = 0,
            2 if len < 60 => buf[len] = b'A' + r.below(26) as u8,
            3 => buf[r.below(len as u32) as usize] = r.next_u32() as u8,
            _ => {}
        }
    } else {
        for b in buf.iter_mut().take(r.below(40) as usize) {
            *b = b'A' + r.below(30) as u8;
        }
    }
    for (k, slot) in w.iter().enumerate().take(16) {
        v[slot.unwrap()] = u32::from_le_bytes(buf[4 * k..4 * k + 4].try_into().unwrap());
    }
}

/// Region 0 for `GetAdhesiveLimit`: all 64 words are float inputs (so signalling NaNs are routed to
/// the CPU): groups 6 and 7 index past the 6x6 matrix into the entry words, which are then loaded with
/// `fld` too. The flag words are random bit patterns either way.
fn table_with_matrix() -> Region {
    floats(64)
}

/// The colpoint's surface bytes +0x20 (A) and +0x23 (B) in 0..8.
fn fix_cp(v: &mut [u32], lay: &Layout) {
    let w = &lay.word_slot[1];
    let s8 = w[8].unwrap();
    let x = v[s8];
    let a = x & 0xFF;
    let b3 = (x >> 24) & 0xFF;
    v[s8] = (x & 0x00FF_FF00) | (a % 9) | ((b3 % 9) << 24);
    // Only the word the original loads may be a signalling NaN (the others are quieted), so the
    // CPU-routed share stays small.
    let w0 = &lay.word_slot[0];
    let group = |v: &[u32], id: u32| v[w0[(0x25 + 3 * id) as usize].unwrap()] & 7;
    let (sa, sb) = (a % 9, b3 % 9);
    let read = (group(v, sa) + 6 * group(v, sb)) as usize;
    for (k, slot) in w0.iter().enumerate() {
        let i = slot.unwrap();
        if k != read && sa_math::x87::is_snan_f32(f32::from_bits(v[i])) {
            v[i] |= 0x40_0000;
        }
    }
}

pub fn all() -> Vec<Spec> {
    let mut out: Vec<Spec> = getters()
        .into_iter()
        .map(|(a, n, k, f)| getter_spec(a, n, k, f))
        .collect();
    out.push(
        b(
            "SurfaceInfosGetTyreGrip",
            "SurfaceInfos_c::GetTyreGrip",
            0x55E5E0,
            Thiscall,
            St0,
        )
        .regions(vec![raw(64)])
        .args(vec![A::P(0), A::I(g_id)])
        .rust(|c| {
            let id = c.i(1);
            st0(s::get_tyre_grip(tp(c, 0), id))
        }),
    );
    out.push(
        b(
            "SurfaceInfosGetWetMultiplier",
            "SurfaceInfos_c::GetWetMultiplier",
            0x55E600,
            Thiscall,
            St0,
        )
        .regions(vec![raw(64), floats(1)])
        .args(vec![A::P(0), A::I(g_id)])
        .patch(vec![(0x55E61F, (SCRATCH + 0x120).to_le_bytes().to_vec())])
        .rust(|c| {
            let id = c.i(1);
            let wet = c.fl(1, 0);
            let _g = crate::host_globals();
            s::WET_ROADS.set(wet);
            st0(s::get_wet_multiplier(tp(c, 0), id))
        }),
    );
    out.push(
        b(
            "SurfaceInfosGetAdhesiveLimit",
            "SurfaceInfos_c::GetAdhesiveLimit",
            0x55EB50,
            Thiscall,
            St0,
        )
        .regions(vec![table_with_matrix(), raw(11)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| fix_cp(v, l))
        .rust(|c| st0(s::get_adhesive_limit(tp(c, 0), tp(c, 1)))),
    );
    out.push(
        b(
            "SurfaceInfosGetSurfaceIdFromName",
            "SurfaceInfos_c::GetSurfaceIdFromName",
            0x55D220,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(1), raw(16)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| fix_name(v, l))
        .budget(200_000)
        .rust(|c| ExpRet::Eax(s::get_surface_id_from_name(tp(c, 0), tp(c, 1)))),
    );
    out
}
