//! MVP-C AI ports, batch `mvpc-ai-2`: the `CEventScanner` sub-scanners that the catalogue leaves unnamed: the
//! pedestrian-acquaintance scan over the nearby peds (0x607A90), its per-ped acquaintance-type probe (0x607560) and
//! event raiser (0x606BA0), and the attractor scanner's entity walk (0x6034B0) and best-attractor pick (0x600180).
//! Ported from the 1.0 US bytes (the exe wins over gta-reversed, which has not reversed them). Every call to
//! another catalogued function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::IsAlive` (0x5E0170, AL)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CAcquaintance::GetAcquaintances(int32 type)` (0x608970, `ret 4`)
    fn acquaintance_get(a: u32, ty: u32) -> u32 = thiscall 0x608970 => crate::gen_leaves::g_cacquaintance_get_acquaintances_608970;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830, cdecl)
    fn ped_type_get_ped_flag(ty: i32) -> u32 = cdecl 0x608830 => unported;
    /// `CGameLogic::LaRiotsActiveHere` (0x441C10, AL)
    fn la_riots_active_here() -> u8 = cdecl 0x441C10 => sa_world::world::misc2::la_riots_active_here;
    /// `FUN_00603af0` (0x603AF0, cdecl, AL): the riot gang-member test of two peds
    fn riot_peds_test(ped: GPtr<u8>, other: GPtr<u8>) -> u8 = cdecl 0x603AF0 => unported;
    /// `CDecisionMakerTypes::GetInstance` (0x4684F0)
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => crate::misc::decision_maker_types_get_instance_4684f0;
    /// `FUN_006042b0` (0x6042B0, `ret 0xC`, AL): does the ped's decision maker respond to one of the event types
    fn decision_maker_has_response(d: GPtr<u8>, ped: GPtr<u8>, evs: GPtr<u8>, n: i32) -> u8 = thiscall 0x6042B0 => crate::mvpc_ai1::decision_maker_types_has_response;
    /// `CPedGeometryAnalyser::CanPedTargetPed(const CPed&, const CPed&, bool)` (0x5F1C40, cdecl; full EAX)
    fn can_ped_target_ped(ped: GPtr<u8>, target: GPtr<u8>, check: u32) -> u32 = cdecl 0x5F1C40 => crate::mvpa_ai1::can_ped_target_ped;
    /// `FUN_00607560` (0x607560, `ret 0x18`)
    #[allow(clippy::too_many_arguments)]
    fn acq_probe_o(s: GPtr<u8>, ped: GPtr<u8>, ty: i32, best: i32, other: GPtr<u8>, out_ped: GPtr<u8>, out_ty: GPtr<u8>) -> i32 = thiscall 0x607560 => crate::mvpc_ai2::acq_scanner_probe;
    /// `FUN_00606ba0` (0x606BA0, `ret 0xC`, AL)
    fn acq_raise_o(s: GPtr<u8>, ped: GPtr<u8>, ty: i32, other: GPtr<u8>) -> u8 = thiscall 0x606BA0 => crate::mvpc_ai2::acq_scanner_raise_event;
    /// `CPedIntelligence::CanSeeEntityWithLights(const CEntity*, bool)` (0x605550, `ret 8`, ST0)
    fn intel_can_see_entity_with_lights(i: GPtr<u8>, e: GPtr<u8>, b: u32) -> f64 = thiscall 0x605550 => unported;
    /// `CEventAcquaintancePed::CEventAcquaintancePed(CPed*)` (0x4AF820, `ret 4`)
    fn event_acquaintance_ped_ctor(e: u32, ped: u32) -> u32 = thiscall 0x4AF820 => crate::gen_ctors::g_event_acquaintance_ped_constructor_4af820;
    /// `CEventAcquaintancePedHate::CEventAcquaintancePedHate(CPed*)` (0x420E70, `ret 4`)
    fn event_acquaintance_ped_hate_ctor(e: u32, ped: u32) -> u32 = thiscall 0x420E70 => crate::gen_ctors::g_event_acquaintance_ped_hate_constructor2_420e70;
    /// `CEventAcquaintancePed::~CEventAcquaintancePed` (0x4AF890)
    fn event_acquaintance_ped_dtor(e: GPtr<u8>) = thiscall 0x4AF890 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u8) -> GPtr<u8> = thiscall 0x4AB420 => crate::group::event_group_add;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn get_peds_group(ped: u32) -> u32 = cdecl 0x5F7E80 => crate::misc::ped_groups_get_peds_group_5f7e80;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470, `ret 4`, AL)
    fn pgi_add_event(pgi: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x5F7470 => unported;
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70, `ret 4`)
    fn model_get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4C4C70 => unported;
    /// `FUN_005fffd0` (0x5FFFD0, `ret 0xC`): offers one ped-attractor effect of an entity to the attractor scanner
    fn attractor_scanner_offer(s: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5FFFD0 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The entity's position: `m_matrix->pos` (+0x14 -> +0x30) when it has a matrix, else the placement (+4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `FUN_00607a90` (0x607A90, `ret 0x18`): the acquaintance scan of `ped` over `count` nearby entities, called as
/// `(ped, type, entities, count, &outPed, &outType)` (type -1 = every acquaintance type). `*outPed` = 0 first.
/// Pass 1 collects (up to the entity count, at most 16) living peds that are in front of `ped` (normalised
/// direction dotted with the ped's forward vector > [0xC0B034]) or where either ped has flag 0x100 at +0x46C, and
/// that are acquainted: for type -1 a ped of type 6 (cop) always; else for acquaintance types 4..0 (only `type`
/// when given) `GetAcquaintances(ped +0x4E0, t) & GetPedFlag(other type)`; else, with LA riots active here,
/// `FUN_00603af0(ped, other)`. Non-cops further need, unless the riot test passes, a decision-maker response to one
/// of the event types {0x24, 0x25, 0x27, 0x28, <uninitialised fifth local>} (the original passes a count of 5 for
/// a 4-entry array). Pass 2: with `best` = -1, for each collected ped while `best` != 4, if `CanPedTargetPed(ped,
/// other, 0x100 | check)` (check = neither ped has flag 0x100; the original pushes the whole EAX) then `best =
/// FUN_00607560(ped, type, best, other, outPed, outType)`.
#[hook(addr = 0x607A90, conv = thiscall, name = "FUN_00607a90", expect = "83 EC 6C 8B 84 24 80 00 00 00",
       unverified = "mvpc-ai-2", route = "L2", hazards = "uninit-locals,heap-writes", recs = "grove-walk",
       note = "the fifth event type passed to FUN_006042b0 is an uninitialised stack local (modelled as 0)")]
#[allow(clippy::too_many_arguments)]
pub fn acq_scanner_scan_entities(this: GPtr<u8>, ped: GPtr<u8>, ty: i32, ents: GPtr<u8>, n: i32, out_ped: GPtr<u8>, out_ty: GPtr<u8>) {
    wr::<u32>(out_ped, 0, 0);
    let mut list = [GPtr::<u8>::NULL; 16];
    let mut count = 0usize;
    let mut i = 0i32;
    while i < n {
        let other = rd::<GPtr<u8>>(ents, 4 * i as isize);
        i += 1;
        if other.is_null() || ped_is_alive(other) == 0 {
            continue;
        }
        let a = pos(ped);
        let b = pos(other);
        let mut d = [0u32; 3];
        let dp = local(&mut d);
        let dz = f32_of(x(rd::<f32>(b, 8)).fsub_m(rd::<f32>(a, 8)));
        let dy = f32_of(x(rd::<f32>(b, 4)).fsub_m(rd::<f32>(a, 4)));
        let dx = f32_of(x(rd::<f32>(b, 0)).fsub_m(rd::<f32>(a, 0)));
        wr::<f32>(dp, 0, dx);
        wr::<f32>(dp, 4, dy);
        wr::<f32>(dp, 8, dz);
        vector_normalise(dp);
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let dot = x(rd::<f32>(dp, 8))
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(x(rd::<f32>(dp, 4)).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x(rd::<f32>(dp, 0)).fmul_m(rd::<f32>(m, 0x10)));
        if dot.fcom(x(g::<f32>(0xC0B034).read())).test_any(0x41)
            && rd::<u32>(ped, 0x46C) & 0x100 == 0
            && rd::<u32>(other, 0x46C) & 0x100 == 0
        {
            continue;
        }
        let mut found = ty == -1 && rd::<i32>(other, 0x598) == 6;
        if !found {
            let mut t = 4i32;
            while t >= 0 && !found {
                if ty == -1 || ty == t {
                    let acq = acquaintance_get(ped.byte_add(0x4E0).addr() as u32, t as u32);
                    if acq & ped_type_get_ped_flag(rd::<i32>(other, 0x598)) != 0 {
                        found = true;
                    }
                }
                t -= 1;
            }
            if !found && (la_riots_active_here() == 0 || riot_peds_test(ped, other) == 0) {
                continue;
            }
        }
        if rd::<i32>(other, 0x598) != 6 {
            let riot = la_riots_active_here() != 0 && riot_peds_test(ped, other) != 0;
            if !riot {
                let mut evs = [0x24u32, 0x25, 0x27, 0x28, 0];
                let e = GPtr::from_raw(&mut evs as *mut [u32; 5]).cast::<u8>();
                if decision_maker_has_response(decision_maker_types_get_instance(), ped, e, 5) == 0 {
                    continue;
                }
            }
        }
        list[count] = other;
        count += 1;
    }
    let mut best = -1i32;
    for &other in list.iter().take(count) {
        if best == 4 {
            continue;
        }
        let check = (rd::<u32>(ped, 0x46C) & 0x100 == 0 && rd::<u32>(other, 0x46C) & 0x100 == 0) as u32;
        if can_ped_target_ped(ped, other, 0x100 | check) as u8 != 0 {
            best = acq_probe_o(this, ped, ty, best, other, out_ped, out_ty);
        }
    }
}

/// `FUN_00607560` (0x607560, `ret 0x18`): the acquaintance type `ped` has with `other`, called as `(ped, type, best,
/// other, &outPed, &outType)`. light = `CanSeeEntityWithLights(other, false)` (stored as f32 into the original's
/// own `other` argument slot). For t = 4 down to 0, stopping (-1) when t reaches `best`: with `type` given only t
/// == `type` is tried; for type -1, t == 2 with a cop `other` skips the acquaintance test. Acquainted =
/// `GetAcquaintances(ped +0x4E0, t) & GetPedFlag(other type)` or, with LA riots, `FUN_00603af0(ped, other)`. An
/// acquainted t needs light > [0x858B50], or t == 4 with light < it (or unordered). Then `*outPed = other`,
/// `*outType = t`; a given `type` is returned at once. For type -1, a non-null `*outPed` raises the event
/// (`FUN_00606ba0(ped, t, other)`), the scanner timer restarts (interval [0x8D2360] for type 4 with light <
/// [0x859EF8] (f64), else [0x8D235C]) and, when the event was added, `*outType` is returned.
#[hook(addr = 0x607560, conv = thiscall, name = "FUN_00607560", expect = "8B 44 24 04 53 8B 5C 24 14",
       unverified = "mvpc-ai-2", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "overwrites its own fourth argument slot with the f32 light value")]
#[allow(clippy::too_many_arguments)]
pub fn acq_scanner_probe(this: GPtr<u8>, ped: GPtr<u8>, ty: i32, best: i32, other: GPtr<u8>, out_ped: GPtr<u8>, out_ty: GPtr<u8>) -> i32 {
    let light = f32_of(X87::from_f64_exact(intel_can_see_entity_with_lights(rd::<GPtr<u8>>(ped, 0x47C), other, 0)));
    let thr = x(g::<f32>(0x858B50).read());
    let mut t = 4i32;
    loop {
        if t == best {
            return -1;
        }
        let tried = if ty != -1 { ty == t } else { true };
        if tried {
            let skip_acq = ty == -1 && t == 2 && rd::<i32>(other, 0x598) == 6;
            let acquainted = skip_acq || {
                let acq = acquaintance_get(ped.byte_add(0x4E0).addr() as u32, t as u32);
                acq & ped_type_get_ped_flag(rd::<i32>(other, 0x598)) != 0
                    || (la_riots_active_here() != 0 && riot_peds_test(ped, other) != 0)
            };
            let lit = acquainted && {
                let c = x(light).fcom(thr);
                !c.test_any(0x41) || (t == 4 && c.test_parity(0x44))
            };
            if lit {
                wr::<GPtr<u8>>(out_ped, 0, other);
                wr::<i32>(out_ty, 0, t);
                if ty != -1 {
                    return ty;
                }
                let op = rd::<GPtr<u8>>(out_ped, 0);
                if !op.is_null() {
                    let added = acq_raise_o(this, ped, t, op);
                    let interval = if rd::<i32>(out_ty, 0) == 4
                        && !x(light).fcom(X87::from_f64_exact(g::<f64>(0x859EF8).read())).test_parity(5)
                    {
                        g::<u32>(0x8D2360).read()
                    } else {
                        g::<u32>(0x8D235C).read()
                    };
                    wr::<u32>(this, 0, g::<u32>(TIME_MS).read());
                    wr::<u32>(this, 4, interval);
                    wr::<u8>(this, 8, 1);
                    if added != 0 {
                        return rd::<i32>(out_ty, 0);
                    }
                }
            }
        }
        t -= 1;
        if t < 0 {
            return -1;
        }
    }
}

/// Constructs a `CEventAcquaintancePed` of `other` in `ev`, sets `vtable`, adds it to `ped`'s event group
/// (`Add(event, false)`), restores the vtable and destroys it; true when the group took it.
fn raise_acquaintance(ev: GPtr<u8>, ped: GPtr<u8>, other: GPtr<u8>, vtable: u32) -> u8 {
    event_acquaintance_ped_ctor(ev.addr() as u32, other.addr() as u32);
    wr::<u32>(ev, 0, vtable);
    let r = event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ev, 0);
    wr::<u32>(ev, 0, vtable);
    event_acquaintance_ped_dtor(ev);
    !r.is_null() as u8
}

/// `FUN_00606ba0` (0x606BA0, `ret 0xC`, AL): raises the acquaintance event of type `t` for `ped` about `other`
/// (stack events, added with `CEventGroup::Add(event, false)`; true when added): 0 respect (vtable 0x86CA38), 1 like
/// (0x86CA80), 2 dislike (0x86CB58, only for a cop `other`), 3 hate-like (0x86CAC8). Type 4 (hate) by light =
/// `CanSeeEntityWithLights(other, false)`: light > [0x858B50]: unless LA riots ([0x96913F] set, or the riot test
/// `FUN_00603af0(ped, other)` passes) a `CEventAcquaintancePedHate` goes to the ped's events; in a riot it goes, with
/// response task 0x5DE, to the ped's group (`CPedGroupIntelligence::AddEvent`) or, without a group, with response
/// 0x3E8 to the ped's events. Light below: a `CEventAcquaintancePedHateBadlyLit` (vtable 0x86CB10, time
/// now, position of `other`) goes to the ped's events; equal or unordered: nothing. Other types: false.
#[hook(addr = 0x606BA0, conv = thiscall, name = "FUN_00606ba0", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "mvpc-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn acq_scanner_raise_event(_this: GPtr<u8>, ped: GPtr<u8>, t: i32, other: GPtr<u8>) -> u8 {
    let mut a = [0u32; 6];
    let ea = local(&mut a);
    match t {
        0 => raise_acquaintance(ea, ped, other, 0x86CA38),
        1 => raise_acquaintance(ea, ped, other, 0x86CA80),
        2 => {
            if rd::<i32>(other, 0x598) != 6 {
                return 0;
            }
            raise_acquaintance(ea, ped, other, 0x86CB58)
        }
        3 => raise_acquaintance(ea, ped, other, 0x86CAC8),
        4 => {
            let light = X87::from_f64_exact(intel_can_see_entity_with_lights(rd::<GPtr<u8>>(ped, 0x47C), other, 0));
            let c = light.fcom(x(g::<f32>(0x858B50).read()));
            if !c.test_any(0x41) {
                let riot = g::<u8>(0x96913F).read() != 0
                    || (la_riots_active_here() != 0 && riot_peds_test(ped, other) != 0);
                event_acquaintance_ped_hate_ctor(ea.addr() as u32, other.addr() as u32);
                let r = if !riot {
                    let r = event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ea, 0);
                    !r.is_null() as u8
                } else {
                    let grp = get_peds_group(ped.addr() as u32);
                    if grp != 0 {
                        wr::<u16>(ea, 0xE, 0x5DE);
                        pgi_add_event(GPtr::from_addr(grp as usize + 0x30), ea)
                    } else {
                        wr::<u16>(ea, 0xE, 0x3E8);
                        let r = event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ea, 0);
                        !r.is_null() as u8
                    }
                };
                wr::<u32>(ea, 0, 0x858E68);
                event_acquaintance_ped_dtor(ea);
                return r;
            }
            if c.test_parity(5) {
                return 0;
            }
            let mut b = [0u32; 10];
            let eb = local(&mut b);
            event_acquaintance_ped_ctor(eb.addr() as u32, other.addr() as u32);
            wr::<u32>(eb, 0x18, g::<u32>(TIME_MS).read());
            wr::<u32>(eb, 0, 0x86CB10);
            let p = pos(other);
            wr::<u32>(eb, 0x1C, rd::<u32>(p, 0));
            wr::<u32>(eb, 0x20, rd::<u32>(p, 4));
            wr::<u32>(eb, 0x24, rd::<u32>(p, 8));
            let r = event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), eb, 0);
            wr::<u32>(eb, 0, 0x86CB10);
            event_acquaintance_ped_dtor(eb);
            !r.is_null() as u8
        }
        _ => 0,
    }
}

/// `FUN_006034b0` (0x6034B0, `ret 8`): the attractor scanner's walk over an entity list (`CPtrList`, nodes {entity,
/// next}) for `ped`. Objects (type 4) need flag 4 at +0x1C or flag 0x40000, and not object flag 0x40 at +0x140;
/// for them `scripted` = bit 24 of +0x140 (the flag persists into later non-objects). Every 2D effect of the
/// entity's model info (count u8 +0xD) of type 3 (ped attractor) that is not script-only (bit 0 of +0x37), or any
/// when `scripted`, is offered with `FUN_005fffd0(effect, entity, ped)`. The next-node pointer is kept in the
/// original's own first argument slot.
#[hook(addr = 0x6034B0, conv = thiscall, name = "FUN_006034b0", expect = "51 8B 44 24 08 8B 00",
       unverified = "mvpc-ai-2", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "keeps the list cursor in its own first argument slot")]
pub fn attractor_scanner_scan_list(this: GPtr<u8>, list: GPtr<u8>, ped: GPtr<u8>) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    let mut scripted = false;
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        let ty = rd::<u8>(e, 0x36) & 7;
        if ty == 4 {
            let of = rd::<u32>(e, 0x140);
            let f = rd::<u32>(e, 0x1C);
            scripted = (of >> 24) & 1 != 0;
            if (ty as u32 & f) == 0 && f & 0x40000 == 0 {
                continue;
            }
            if of & 0x40 != 0 {
                continue;
            }
        }
        let mi = g::<GPtr<u8>>(0xA9B0C8 + 4 * rd::<i16>(e, 0x22) as isize as usize).read();
        let mut j = 0i32;
        while j < rd::<u8>(mi, 0xD) as i32 {
            let fx = model_get_2d_effect(mi, j);
            if rd::<u8>(fx, 0xC) == 3 && (rd::<u8>(fx, 0x37) & 1 == 0 || scripted) {
                attractor_scanner_offer(this, fx, e, ped);
            }
            j += 1;
        }
    }
}

/// `FUN_00600180` (0x600180, `ret 8`): the attractor scanner's choice, written to `*outEffect` and `*outEntity`
/// (both 0 first). With slot 4's entity (+0x28) set: effect +0x50 and entity +0x28 directly. Else, best =
/// [0x863A3C]: slot 0 (distance +0x68, effect +0x40, entity +0x18) wins when its distance is below best and its
/// effect is set; then each slot 1..9 (distance +0x68+4k, effect +0x40+4k, entity +0x18+4k) wins when best is
/// greater than its distance and its effect is set. All values compared are f32 loads, compared exactly.
#[hook(addr = 0x600180, conv = thiscall, name = "FUN_00600180", expect = "8B 54 24 04 56 8B 74 24 0C",
       unverified = "mvpc-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn attractor_scanner_pick(this: GPtr<u8>, out_fx: GPtr<u8>, out_e: GPtr<u8>) {
    wr::<u32>(out_fx, 0, 0);
    wr::<u32>(out_e, 0, 0);
    if rd::<u32>(this, 0x28) != 0 {
        wr::<u32>(out_fx, 0, rd::<u32>(this, 0x50));
        wr::<u32>(out_e, 0, rd::<u32>(this, 0x28));
        return;
    }
    let mut best = g::<f32>(0x863A3C).read();
    let d0 = rd::<f32>(this, 0x68);
    if d0 < g::<f32>(0x863A3C).read() && rd::<u32>(this, 0x40) != 0 {
        best = rd::<f32>(this, 0x68);
        wr::<u32>(out_fx, 0, rd::<u32>(this, 0x40));
        wr::<u32>(out_e, 0, rd::<u32>(this, 0x18));
    }
    for k in 1..10isize {
        if best > rd::<f32>(this, 0x68 + 4 * k) && rd::<u32>(this, 0x40 + 4 * k) != 0 {
            best = rd::<f32>(this, 0x68 + 4 * k);
            wr::<u32>(out_fx, 0, rd::<u32>(this, 0x40 + 4 * k));
            wr::<u32>(out_e, 0, rd::<u32>(this, 0x18 + 4 * k));
        }
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ACQ_SCANNER_SCAN_ENTITIES,
    HOOK_ACQ_SCANNER_PROBE,
    HOOK_ACQ_SCANNER_RAISE_EVENT,
    HOOK_ATTRACTOR_SCANNER_SCAN_LIST,
    HOOK_ATTRACTOR_SCANNER_PICK,
];
