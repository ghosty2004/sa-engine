//! L1 specs for PORT-STREAM owner b (`sa_stream::modelinfo`; see `docs/arch/stream-port.md`): the
//! model-info functions whose result depends only on `this` (and argument) memory — flag getters and
//! setters, `Init`s, reduced destructors, `GetModelType`/`GetRwModelType`/`As*Ptr`/`GetTimeInfo`,
//! `CTimeInfo`, `CPedModelInfo::IncrementVoice`, the IDE flag setters, `Add2dEffect`,
//! `GetAtomicFromDistance` and `SetupVehicleUpgradeFlags`.
//!
//! `this` is a raw region; the Rust side runs the actual port on a copy of the scratch image through
//! a host pointer into it (fields read/written are integers and floats only; pointer-sized fields are
//! compared as 32-bit words in the expected values, because a 64-bit host `GPtr` is 8 bytes).
//! `GetAtomicFromDistance` reads `TheCamera.m_fLODDistMultiplier` (0xB6F118): its `fld` operand is
//! patched to region 0. `SetupVehicleUpgradeFlags` calls the CRT `strncmp` (allowed callee); the
//! Rust side passes a host prefix comparison. `Add2dEffect` calls `Get2dEffectStore` (allowed
//! callee, returns 0xB4C2D8).

use crate::call::{Conv::*, SCRATCH};
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_stream::modelinfo::{atomic, base, clump, model_info, ped, time, weapon};

/// A generator seeded from input words (used by `fix`).
fn rng(seed: u32) -> Rng {
    Rng::new(u64::from(seed) | 1 << 40)
}

/// Host pointer to region `r` of the expected image.
fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

/// The 1.0 US address of region `r` as a (never dereferenced) game pointer.
fn ga<T>(c: &Ctx, r: usize) -> GPtr<T> {
    GPtr::from_addr(c.addr(r) as usize)
}

/// A thiscall spec on a `words`-word `this` region with extra raw arguments.
fn this_spec(
    key: &'static str,
    name: &'static str,
    addr: u32,
    ret: crate::specs::RetKind,
    words: usize,
    extra: Vec<A>,
) -> B {
    let mut args = vec![A::P(0)];
    args.extend(extra);
    b(key, name, addr, Thiscall, ret)
        .regions(vec![raw(words)])
        .args(args)
}

/// A cdecl spec `f(this, arg)`.
fn cdecl_spec(key: &'static str, name: &'static str, addr: u32, words: usize, g: fn(&mut Rng) -> u32) -> B {
    b(key, name, addr, Cdecl, Void)
        .regions(vec![raw(words)])
        .args(vec![A::P(0), A::I(g)])
}

fn g_bool_word(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => 0,
        1 => 1,
        2 => r.below(0x100) << (8 * r.below(4)),
        _ => r.next_u32(),
    }
}

/// IDE flag words: random bits, mostly sparse.
fn g_ide_flags(r: &mut Rng) -> u32 {
    match r.below(3) {
        0 => r.next_u32(),
        1 => 1 << r.below(32),
        _ => r.next_u32() & r.next_u32() & r.next_u32(),
    }
}

/// A flags word with the special-type field set to one of its values most of the time.
fn g_flags_word(r: &mut Rng) -> u32 {
    let mut w = r.next_u32();
    if r.chance(70) {
        w = (w & !0x7800_0000 & !0x7800) | (r.below(16) << 11) << if r.chance(50) { 0 } else { 16 };
    }
    w
}

fn g_small_i16(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => r.next_u32(),
        _ => (r.below(40) as i32 - 5) as u32 & 0xFFFF,
    }
}

/// A constant-returning `this` function (AL).
fn al_const(key: &'static str, name: &'static str, addr: u32, f: fn(base::MiPtr) -> u8) -> Spec {
    this_spec(key, name, addr, Al, 1, vec![]).rust(move |c| ExpRet::Al(f(ga(c, 0))))
}

/// A `this`-only function returning a pointer computed from `this` (never dereferenced).
fn ptr_of_this(key: &'static str, name: &'static str, addr: u32, f: fn(base::MiPtr) -> base::MiPtr) -> Spec {
    this_spec(key, name, addr, Eax, 1, vec![]).rust(move |c| ExpRet::Eax(f(ga(c, 0)).addr() as u32))
}

fn time_ptr(
    key: &'static str,
    name: &'static str,
    addr: u32,
    f: fn(base::MiPtr) -> time::TimeInfoPtr,
) -> Spec {
    this_spec(key, name, addr, Eax, 1, vec![]).rust(move |c| ExpRet::Eax(f(ga(c, 0)).addr() as u32))
}

/// A `this` getter returning EAX.
fn eax_get(key: &'static str, name: &'static str, addr: u32, f: fn(base::MiPtr) -> u32) -> Spec {
    this_spec(key, name, addr, Eax, 8, vec![])
        .fix(|v, lay| {
            let s = lay.word_slot[0][4].unwrap();
            let mut r = rng(v[s] ^ 0x5EED);
            v[s] = g_flags_word(&mut r);
        })
        .rust(move |c| ExpRet::Eax(f(tp(c, 0))))
}

/// A `this` setter taking one raw word.
fn set1(key: &'static str, name: &'static str, addr: u32, f: fn(base::MiPtr, u32)) -> Spec {
    this_spec(key, name, addr, Void, 8, vec![A::I(g_bool_word)]).rust(move |c| {
        let a = c.i(1);
        f(tp(c, 0), a);
        ExpRet::Void
    })
}

/// A `this`-only void function on a `words`-word object.
fn void_this(
    key: &'static str,
    name: &'static str,
    addr: u32,
    words: usize,
    callees: &'static [u32],
    f: fn(base::MiPtr),
) -> Spec {
    this_spec(key, name, addr, Void, words, vec![])
        .callees(callees)
        .rust(move |c| {
            f(tp(c, 0));
            ExpRet::Void
        })
}

fn host_prefix(name: &[u8], text: &[u8], len: u32) -> bool {
    (0..len as usize).all(|k| name.get(k).copied().unwrap_or(0) == text[k])
}

/// Prefixes of `SetupVehicleUpgradeFlags`' tables (plus near misses).
const PREFIXES: [&[u8]; 24] = [
    b"bnt_",
    b"bntl_",
    b"bntr_",
    b"spl_",
    b"wg_l_",
    b"wg_r_",
    b"fbb_",
    b"bbb_",
    b"lgt_",
    b"rf_",
    b"nto_",
    b"hydralics",
    b"stereo",
    b"chss_",
    b"wheel_",
    b"exh_",
    b"fbmp_",
    b"rbmp_",
    b"misc_a_",
    b"misc_b_",
    b"misc_c_",
    b"bnt",
    b"BNT_",
    b"misc_",
];

/// Every spec of this file.
pub fn all() -> Vec<Spec> {
    vec![
        // ------------------------------------------------------------------------- CTimeInfo
        this_spec(
            "CTimeInfoGetTimeOn",
            "CTimeInfo::GetTimeOn",
            0x407320,
            Eax,
            1,
            vec![],
        )
        .rust(|c| ExpRet::Eax(time::get_time_on(tp(c, 0)))),
        this_spec(
            "CTimeInfoGetTimeOff",
            "CTimeInfo::GetTimeOff",
            0x407330,
            Eax,
            1,
            vec![],
        )
        .rust(|c| ExpRet::Eax(time::get_time_off(tp(c, 0)))),
        this_spec(
            "CTimeInfoGetOtherTimeModel",
            "CTimeInfo::GetOtherTimeModel",
            0x4C4A30,
            Eax,
            1,
            vec![],
        )
        .rust(|c| ExpRet::Eax(time::get_other_time_model(tp(c, 0)) as u32)),
        this_spec(
            "CTimeInfoSetTimes",
            "CTimeInfo::SetTimes",
            0x5B3430,
            Void,
            1,
            vec![A::I(g_rand), A::I(g_rand)],
        )
        .rust(|c| {
            let (a, b) = (c.i(1), c.i(2));
            time::set_times(tp(c, 0), a, b);
            ExpRet::Void
        }),
        this_spec(
            "CTimeInfoSetOtherTimeModel",
            "CTimeInfo::SetOtherTimeModel",
            0x5B3440,
            Void,
            1,
            vec![A::I(g_rand)],
        )
        .rust(|c| {
            let a = c.i(1);
            time::set_other_time_model(tp(c, 0), a);
            ExpRet::Void
        }),
        // -------------------------------------------------------------------- CBaseModelInfo
        eax_get(
            "CBaseModelInfoSwaysInWind",
            "CBaseModelInfo::SwaysInWind",
            0x4212C0,
            base::sways_in_wind,
        ),
        eax_get(
            "CBaseModelInfoIsTagModel",
            "CBaseModelInfo::IsTagModel",
            0x49CC20,
            base::is_tag_model,
        ),
        eax_get(
            "CBaseModelInfoIsBreakableStatuePart",
            "CBaseModelInfo::IsBreakableStatuePart",
            0x59F090,
            base::is_breakable_statue_part,
        ),
        eax_get(
            "CBaseModelInfoIsLod",
            "CBaseModelInfo::IsLod",
            0x4C4A00,
            base::is_lod,
        ),
        eax_get(
            "CBaseModelInfoIsRoad",
            "CBaseModelInfo::IsRoad",
            0x4C4DF0,
            base::is_road,
        ),
        eax_get(
            "CBaseModelInfoHasComplexHierarchy",
            "CBaseModelInfo::HasComplexHierarchy",
            0x4C4E00,
            base::has_complex_hierarchy,
        ),
        eax_get(
            "CBaseModelInfoHasBeenPreRendered",
            "CBaseModelInfo::HasBeenPreRendered",
            0x5328B0,
            base::has_been_pre_rendered,
        ),
        eax_get(
            "CBaseModelInfoGetIsDrawLast",
            "CBaseModelInfo::GetIsDrawLast",
            0x5328C0,
            base::get_is_draw_last,
        ),
        eax_get(
            "CBaseModelInfoIsBackfaceCulled",
            "CBaseModelInfo::IsBackfaceCulled",
            0x5328F0,
            base::is_backface_culled,
        ),
        set1(
            "CBaseModelInfoSetHasBeenPreRendered",
            "CBaseModelInfo::SetHasBeenPreRendered",
            0x4C42F0,
            base::set_has_been_pre_rendered,
        ),
        set1(
            "CBaseModelInfoSetIsLod",
            "CBaseModelInfo::SetIsLod",
            0x4C4A10,
            base::set_is_lod,
        ),
        set1(
            "CBaseModelInfoSetOwnsColModel",
            "CBaseModelInfo::SetOwnsColModel",
            0x5328D0,
            base::set_owns_col_model,
        ),
        void_this(
            "CBaseModelInfoInit",
            "CBaseModelInfo::Init",
            0x4C4B10,
            8,
            &[],
            base::init,
        ),
        void_this(
            "CBaseModelInfoClearTexDictionary",
            "CBaseModelInfo::ClearTexDictionary",
            0x4C4B70,
            8,
            &[],
            base::clear_tex_dictionary,
        ),
        void_this(
            "CBaseModelInfoInit2dEffects",
            "CBaseModelInfo::Init2dEffects",
            0x4C4C20,
            8,
            &[],
            base::init_2d_effects,
        ),
        void_this(
            "CBaseModelInfoIncreaseAlpha",
            "CBaseModelInfo::IncreaseAlpha",
            0x532900,
            8,
            &[],
            base::increase_alpha,
        ),
        ptr_of_this(
            "CBaseModelInfoAsAtomicModelInfoPtr",
            "CBaseModelInfo::AsAtomicModelInfoPtr",
            0x4C4A80,
            base::as_atomic_model_info_ptr,
        ),
        ptr_of_this(
            "CBaseModelInfoAsDamageAtomicModelInfoPtr",
            "CBaseModelInfo::AsDamageAtomicModelInfoPtr",
            0x4C4A90,
            base::as_damage_atomic_model_info_ptr,
        ),
        ptr_of_this(
            "CBaseModelInfoAsLodAtomicModelInfoPtr",
            "CBaseModelInfo::AsLodAtomicModelInfoPtr",
            0x4C4AA0,
            base::as_lod_atomic_model_info_ptr,
        ),
        time_ptr(
            "CBaseModelInfoGetTimeInfo",
            "CBaseModelInfo::GetTimeInfo",
            0x4C4AB0,
            base::get_time_info,
        ),
        this_spec(
            "CBaseModelInfoSetAnimFile",
            "CBaseModelInfo::SetAnimFile",
            0x4C4AC0,
            Void,
            1,
            vec![A::I(g_rand)],
        )
        .rust(|c| {
            base::set_anim_file(ga(c, 0), GPtr::from_addr(c.i(1) as usize));
            ExpRet::Void
        }),
        void_this(
            "CBaseModelInfoConvertAnimFileIndex",
            "CBaseModelInfo::ConvertAnimFileIndex",
            0x4C4AD0,
            1,
            &[],
            base::convert_anim_file_index,
        ),
        this_spec(
            "CBaseModelInfoGetAnimFileIndex",
            "CBaseModelInfo::GetAnimFileIndex",
            0x4C4AE0,
            Eax,
            1,
            vec![],
        )
        .rust(|c| ExpRet::Eax(base::get_anim_file_index(ga(c, 0)) as u32)),
        // EDX: callers 0x5B4179 (LoadAnimatedClumpObject), 0x5B3B2B and 0x5B3C3B keep a value in
        // EDX across the call (verifier V-9, VER-2)
        cdecl_spec(
            "CBaseModelInfoSetBaseModelInfoFlags",
            "CBaseModelInfo::SetBaseModelInfoFlags",
            0x5B3AD0,
            8,
            g_ide_flags,
        )
        .preserve(sa_core::abi::PRESERVE_EDX)
        .rust(|c| {
            let fl = c.i(1);
            base::set_base_model_info_flags(tp(c, 0), fl);
            ExpRet::Void
        }),
        this_spec(
            "CBaseModelInfoAdd2dEffect",
            "CBaseModelInfo::Add2dEffect",
            0x4C4D20,
            Void,
            8,
            vec![A::I(|r| match r.below(3) {
                0 => r.next_u32(),
                _ => 0xB4C2DC + 0x40 * r.below(100) + if r.chance(20) { r.below(0x40) } else { 0 },
            })],
        )
        .callees(&[0x4C5A60])
        .rust(|c| {
            let e = c.i(1);
            base::add_2d_effect_with(tp(c, 0), e, || 0xB4C2D8);
            ExpRet::Void
        }),
        // ------------------------------------------------------------------ CAtomicModelInfo
        ptr_of_this(
            "CAtomicModelInfoAsAtomicModelInfoPtr",
            "CAtomicModelInfo::AsAtomicModelInfoPtr",
            0x4C5560,
            atomic::as_atomic_model_info_ptr,
        ),
        al_const(
            "CAtomicModelInfoGetModelType",
            "CAtomicModelInfo::GetModelType",
            0x4C5570,
            atomic::get_model_type,
        ),
        this_spec(
            "CAtomicModelInfoGetRwModelType",
            "CAtomicModelInfo::GetRwModelType",
            0x4C5580,
            Eax,
            1,
            vec![],
        )
        .rust(|c| ExpRet::Eax(atomic::get_rw_model_type(ga(c, 0)))),
        void_this(
            "CAtomicModelInfoDtor",
            "FUN_004c5590",
            0x4C5590,
            1,
            &[],
            atomic::destructor,
        ),
        void_this(
            "CAtomicModelInfoInit",
            "CAtomicModelInfo::Init",
            0x4C4430,
            8,
            &[0x4C4B10],
            atomic::init,
        ),
        cdecl_spec(
            "CAtomicModelInfoSetAtomicModelInfoFlags",
            "CAtomicModelInfo::SetAtomicModelInfoFlags",
            0x5B3B20,
            8,
            g_ide_flags,
        )
        .callees(&[0x5B3AD0])
        .rust(|c| {
            let fl = c.i(1);
            atomic::set_atomic_model_info_flags(tp(c, 0), fl);
            ExpRet::Void
        }),
        b(
            "CAtomicModelInfoGetAtomicFromDistance",
            "CAtomicModelInfo::GetAtomicFromDistance",
            0x4C44B0,
            Thiscall,
            Eax,
        )
        .regions(vec![
            floats(1),
            Region {
                words: vec![W::R, W::R, W::R, W::R, W::R, W::R, W::F, W::R],
                kind: Kind::Any,
            },
        ])
        .args(vec![A::P(1), A::F])
        .patch(vec![(0x4C44B2, (SCRATCH + 0x10).to_le_bytes().to_vec())])
        .rust(|c| {
            // the hooked entry: `TheCamera.m_fLODDistMultiplier` (patched to region 0 in the
            // original) from sa-stream's host global, `this` = region 1. The returned pointer is
            // the 32-bit field +0x1C (the host reads it as the low half of a 64-bit GPtr).
            let mult = c.fl(0, 0);
            let (this, dist) = (tp(c, 1), c.f(1));
            let _g = crate::host_globals();
            atomic::LOD_DIST_MULTIPLIER.set(mult);
            ExpRet::Eax(atomic::get_atomic_from_distance(this, dist).addr() as u32)
        }),
        b(
            "CAtomicModelInfoSetupVehicleUpgradeFlags",
            "CAtomicModelInfo::SetupVehicleUpgradeFlags",
            0x4C4570,
            Thiscall,
            Void,
        )
        .regions(vec![raw(8), raw(4)])
        .args(vec![A::P(0), A::P(1)])
        .callees(&[0x8214D0])
        .fix(|v, lay| {
            // region 1 = a NUL-terminated name: a table prefix (or a near miss) plus a suffix
            let mut r = rng(v[lay.word_slot[1][0].unwrap()] ^ v[lay.word_slot[0][4].unwrap()]);
            let mut s = [0u8; 16];
            let p = PREFIXES[r.below(PREFIXES.len() as u32) as usize];
            let n = p.len().min(12);
            s[..n].copy_from_slice(&p[..n]);
            let extra = r.below(4) as usize;
            for k in 0..extra.min(15 - n) {
                s[n + k] = b'a' + r.below(26) as u8;
            }
            if r.chance(10) {
                s[r.below(6) as usize] = 0;
            }
            for w in 0..4 {
                v[lay.word_slot[1][w].unwrap()] = u32::from_le_bytes(s[4 * w..4 * w + 4].try_into().unwrap());
            }
            // the flags word: bit 15 clear most of the time
            let fw = lay.word_slot[0][4].unwrap();
            if r.chance(80) {
                v[fw] &= !0x8000;
            }
        })
        .rust(|c| {
            let name: Vec<u8> = (0..16).map(|k| c.byte(1, k)).collect();
            let end = name.iter().position(|b| *b == 0).unwrap_or(16);
            let name = name[..end].to_vec();
            atomic::setup_vehicle_upgrade_flags_with(tp(c, 0), |_, text, len| host_prefix(&name, text, len));
            ExpRet::Void
        }),
        // ------------------------------------------------------- CDamageAtomicModelInfo / Lod
        ptr_of_this(
            "CDamageAtomicModelInfoAsDamageAtomicModelInfoPtr",
            "CDamageAtomicModelInfo::AsDamageAtomicModelInfoPtr",
            0x4C55C0,
            atomic::as_damage_atomic_model_info_ptr,
        ),
        void_this(
            "CDamageAtomicModelInfoDtor",
            "FUN_004c55d0",
            0x4C55D0,
            1,
            &[],
            atomic::damage_destructor,
        ),
        void_this(
            "CDamageAtomicModelInfoInit",
            "CDamageAtomicModelInfo::Init",
            0x4C48B0,
            9,
            &[0x4C4B10],
            atomic::damage_init,
        ),
        al_const(
            "CLodAtomicModelInfoGetModelType",
            "CLodAtomicModelInfo::GetModelType",
            0x4C5600,
            atomic::lod_get_model_type,
        ),
        ptr_of_this(
            "CLodAtomicModelInfoAsLodAtomicModelInfoPtr",
            "CLodAtomicModelInfo::AsLodAtomicModelInfoPtr",
            0x4C5610,
            atomic::as_lod_atomic_model_info_ptr,
        ),
        void_this(
            "CLodAtomicModelInfoDtor",
            "FUN_004c5620",
            0x4C5620,
            1,
            &[],
            atomic::lod_destructor,
        ),
        void_this(
            "CLodAtomicModelInfoInit",
            "CLodAtomicModelInfo::Init",
            0x4C54D0,
            9,
            &[0x4C4430, 0x4C4B10],
            atomic::lod_init,
        ),
        // ---------------------------------------------------------- CTimeModelInfo / LodTime
        al_const(
            "CTimeModelInfoGetModelType",
            "CTimeModelInfo::GetModelType",
            0x4C5660,
            time::time_get_model_type,
        ),
        time_ptr(
            "CTimeModelInfoGetTimeInfo",
            "CTimeModelInfo::GetTimeInfo",
            0x4C5670,
            time::time_get_time_info,
        ),
        void_this(
            "CTimeModelInfoDtor",
            "FUN_004c5680",
            0x4C5680,
            1,
            &[],
            time::time_destructor,
        ),
        al_const(
            "CLodTimeModelInfoGetModelType",
            "CLodTimeModelInfo::vfunc_4",
            0x4C56C0,
            time::lod_time_get_model_type,
        ),
        time_ptr(
            "CLodTimeModelInfoGetTimeInfo",
            "CLodTimeModelInfo::vfunc_5",
            0x4C56D0,
            time::lod_time_get_time_info,
        ),
        void_this(
            "CLodTimeModelInfoDtor",
            "FUN_004c56e0",
            0x4C56E0,
            1,
            &[],
            time::lod_time_destructor,
        ),
        // ------------------------------------------------------------------ CClumpModelInfo
        this_spec(
            "CClumpModelInfoGetBoundingBox",
            "CClumpModelInfo::GetBoundingBox",
            0x4C5710,
            Eax,
            8,
            vec![],
        )
        .rust(|c| ExpRet::Eax(c.u(0, 5))),
        al_const(
            "CClumpModelInfoGetModelType",
            "CClumpModelInfo::GetModelType",
            0x4C5720,
            clump::get_model_type,
        ),
        this_spec(
            "CClumpModelInfoGetRwModelType",
            "CClumpModelInfo::GetRwModelType",
            0x4C5730,
            Eax,
            1,
            vec![],
        )
        .rust(|c| ExpRet::Eax(clump::get_rw_model_type(ga(c, 0)))),
        this_spec(
            "CClumpModelInfoGetAnimFileIndex",
            "CClumpModelInfo::GetAnimFileIndex",
            0x4C5740,
            Eax,
            9,
            vec![],
        )
        .rust(|c| ExpRet::Eax(clump::get_anim_file_index(tp(c, 0)) as u32)),
        void_this(
            "CClumpModelInfoInit",
            "CClumpModelInfo::Init",
            0x4C4E40,
            9,
            &[0x4C4B10],
            clump::init,
        ),
        cdecl_spec(
            "CClumpModelInfoSetClumpModelInfoFlags",
            "CClumpModelInfo::SetClumpModelInfoFlags",
            0x5B3C30,
            8,
            g_ide_flags,
        )
        .callees(&[0x5B3AD0])
        .rust(|c| {
            let fl = c.i(1);
            clump::set_clump_model_info_flags(tp(c, 0), fl);
            ExpRet::Void
        }),
        // ----------------------------------------------------------------- CWeaponModelInfo
        al_const(
            "CWeaponModelInfoGetModelType",
            "CWeaponModelInfo::GetModelType",
            0x4C5780,
            weapon::get_model_type,
        ),
        void_this(
            "CWeaponModelInfoDtor",
            "FUN_004c5790",
            0x4C5790,
            1,
            &[],
            weapon::destructor,
        ),
        void_this(
            "CWeaponModelInfoInit",
            "CWeaponModelInfo::Init",
            0x4C98F0,
            10,
            &[0x4C4E40, 0x4C4B10],
            weapon::init,
        ),
        // -------------------------------------------------------------------- CPedModelInfo
        al_const(
            "CPedModelInfoGetModelType",
            "CPedModelInfo::GetModelType",
            0x4C57C0,
            ped::get_model_type,
        ),
        this_spec(
            "CPedModelInfoAddXtraAtomics",
            "CPedModelInfo::AddXtraAtomics",
            0x4C6D40,
            Void,
            1,
            vec![A::I(g_rand)],
        )
        .rust(|c| {
            ped::add_xtra_atomics(ga(c, 0), GPtr::from_addr(c.i(1) as usize));
            ExpRet::Void
        }),
        this_spec(
            "CPedModelInfoSetFaceTexture",
            "CPedModelInfo::SetFaceTexture",
            0x4C6D50,
            Void,
            1,
            vec![A::I(g_rand)],
        )
        .rust(|c| {
            ped::set_face_texture(ga(c, 0), GPtr::from_addr(c.i(1) as usize));
            ExpRet::Void
        }),
        this_spec(
            "CPedModelInfoIncrementVoice",
            "CPedModelInfo::IncrementVoice",
            0x4C7300,
            Void,
            0x11,
            vec![],
        )
        .fix(|v, lay| {
            // voice min/max/current (+0x3E, +0x40, +0x42): small signed values most of the time
            let w15 = lay.word_slot[0][15].unwrap();
            let w16 = lay.word_slot[0][16].unwrap();
            let mut r = rng(v[w15] ^ v[w16].rotate_left(7));
            let (a, bb, cc) = (g_small_i16(&mut r), g_small_i16(&mut r), g_small_i16(&mut r));
            v[w15] = (v[w15] & 0xFFFF) | (a << 16);
            v[w16] = bb | (cc << 16);
        })
        .rust(|c| {
            ped::increment_voice(tp(c, 0));
            ExpRet::Void
        }),
        // ----------------------------------------------------------------------- CModelInfo
        b(
            "CModelInfoGet2dEffectStore",
            "CModelInfo::Get2dEffectStore",
            0x4C5A60,
            Cdecl,
            Eax,
        )
        .args(vec![A::I(g_rand)])
        .rust(|_| ExpRet::Eax(model_info::TWO_D_EFFECT_STORE.addr())),
    ]
}
