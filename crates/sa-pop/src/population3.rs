//! CPopulation ped creation (PORT-POP-3, helper B). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, jne_mask, jp5, model_info, rand_at, rd, wr, x};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

use sa_core::bindings::general_random_int_in_range as random_int_in_range;
use sa_core::bindings::get_weapon_info;
use sa_core::bindings::stricmp as crt_stricmp;
use sa_core::bindings::task_new;
use sa_world::bindings::world_add;
use sa_world::bindings::civilian_ped_ctor;
use sa_world::bindings::cop_ped_ctor;
use sa_world::bindings::emergency_ped_ctor;
use sa_world::bindings::event_attractor_ctor;
use sa_world::bindings::event_attractor_dtor;
use sa_world::bindings::event_group_add;
use sa_world::bindings::get_ped_attractor_manager;
use sa_world::bindings::give_delayed_weapon as ped_give_delayed_weapon;
use sa_world::bindings::intel_process;
use sa_world::bindings::ped_new;
use sa_world::bindings::set_char_created_by as ped_set_char_created_by;
use sa_world::bindings::set_ped_decision_maker_type as intel_set_ped_decision_maker_type;
use sa_world::bindings::task_wander_cop_ctor;
use sa_world::bindings::wander_get_wander_task_by_ped_type as get_wander_task_by_ped_type;
use crate::bindings::add_ped as add_ped_call;
use sa_world::bindings::ped_set_current_weapon_slot;
use sa_world::bindings::place_ped_at_effect;
use sa_world::bindings::set_orientation as placeable_set_orientation;
use sa_world::bindings::stand_still_ctor as task_stand_still_ctor;
use sa_world::bindings::tm_set_task as task_manager_set_task;
orig! {
    /// `CPedAttractorManager::HasQueueTailArrivedAtSlot(const C2dEffect*, const CEntity*)` (0x5EBBA0, `ret 8`)
    fn has_queue_tail_arrived_at_slot(mgr: GPtr<u8>, fx: GPtr<u8>, ent: GPtr<u8>) -> u8 = thiscall 0x5EBBA0 => unported;
    /// `CPedIntelligence::ProcessEventHandler()` (0x601BB0)
    fn intel_process_event_handler(intel: GPtr<u8>) = thiscall 0x601BB0 => unported;
}

/// `GiveDelayedWeapon(w, 25001)` + `SetCurrentWeapon(GetWeaponInfo(w, 1)->m_nSlot)` (AddPed's weapon tail).
fn give_weapon(ped: GPtr<u8>, w: u32) {
    ped_give_delayed_weapon(ped, w as i32, 0x61A9);
    let slot = rd::<u32>(get_weapon_info(w as i32, 1), 0x14);
    ped_set_current_weapon_slot(ped, slot as i32);
}

/// `CPopulation::AddPed(ePedType type, uint32 model, const CVector& pos, bool makeWander)` (0x612710, entry
/// `nop; jmp` into its EH-frame prologue at 0x4018C3, which jumps back; no protection VM involved), partial port
/// (ADR-002 fallback). PORTED: types 4..20 and 22..31 (jump tables 0x612C98 / 0x612C7C): `CPed::operator new`
/// (0x79C, cops 0x7C4) and `CCivilianPed(type, model)` (4, 5, 7..17, 20, 22..31), `CCopPed(model)` (6: the
/// model argument is passed as the cop type), `CEmergencyPed(0x12 / 0x13, model)` (18, 19); a failed
/// allocation continues with a null ped as the original. Gang types 7..16: `GetRandomNumberInRange(0, 100)
/// < 33` arms the gang member with one of the gang's three weapons (0xC091F4 + (type-7)*16, picked by a second
/// draw: < 33 / < 50 / < 66 depending on which slots are set). Then position (matrix or placement),
/// `SetOrientation(0, 0, 0)`, `CWorld::Add`, and with `makeWander` the wander task (cops: `CTaskComplexWanderCop
/// (4, rand 0..8)` primary + `CTaskSimpleStandStill(0, true, false, 8.0)` secondary; others
/// `GetWanderTaskByPedType`). Cheat paths (WRITER-POP-4, new and unverified): 0x969140 / 0x96913E arm types 4 / 5
/// with `rand() % 5` (rand 0x61279C) → 0x16, 5, 0x19, 0x1F, 0x23; 0x96915C gives type 0xD weapon 8 instead of the
/// gang draws; with `makeWander`, 0x96915D gives type 0x16 an event about the player (0x4AEDF0, type word 0x4BC)
/// and `GetRandomNumberInRange(10, 14)` without ammo, and 0x96913E sends a hate-acquaintance event (vtable
/// 0x858E68, type word 0x3E8) about the nearest other ped of the pool (its own `type` argument slot holds the
/// best distance in the original; callers only pop it, not reproduced). Types < 4, 21 and > 31 (the switch
/// default 0x6129D1) take the whole `makeWander` argument slot as the ped pointer and go on with it (a caller's
/// pushed 0 / 1 there faults in the original too), so the slot is read as a dword; `makeWander` itself is its low
/// byte (`mov al, [esp+0x5C]`).
#[hook(addr = 0x612710, conv = cdecl, name = "CPopulation::AddPed", expect = "90 E9 AD F1 DE FF",
       unverified = "frontier2-a16", route = "split", hazards = "heap-writes",
       note = "was a partial port (popw4-partial-1, cheat paths new there): the unhandled types no longer run the original")]
pub fn add_ped(ty: u32, model: u32, pos: GPtr<f32>, make_wander_slot: u32) -> GPtr<u8> {
    let make_wander = make_wander_slot as u8;
    if !sa_core::options::enabled(sa_core::options::Feature::AmbientPeds) {
        return GPtr::from_addr(0);
    }
    let cheat = |a: usize| g::<u8>(a).read() != 0;
    let ped = match ty {
        _ if !(4..=31).contains(&ty) || ty == 21 => GPtr::from_addr(make_wander_slot as usize),
        6 => {
            let m = ped_new(0x7C4);
            if m.is_null() { m } else { cop_ped_ctor(m, model) }
        }
        18 | 19 => {
            let m = ped_new(0x79C);
            if m.is_null() {
                m
            } else {
                emergency_ped_ctor(m, ty, model)
            }
        }
        _ => {
            let m = ped_new(0x79C);
            if m.is_null() {
                m
            } else {
                civilian_ped_ctor(m, ty, model)
            }
        }
    };
    if (ty == 4 || ty == 5) && (cheat(0x969140) || cheat(0x96913E)) {
        // armed civilians: `rand() % 5` (idiv) through the jump table 0x612CB4
        let weapon = match rand_at(0x61279C) % 5 {
            0 => 0x16,
            1 => 5,
            2 => 0x19,
            3 => 0x1F,
            4 => 0x23,
            _ => make_wander_slot,
        };
        if weapon != 0 {
            give_weapon(ped, weapon);
        }
    }
    if ty == 0xD && cheat(0x96915C) {
        give_weapon(ped, 8);
    } else if (7..=16).contains(&ty) && random_int_in_range(0, 100) < 0x21 {
        let r = random_int_in_range(0, 100);
        let off = ((ty - 7) * 16) as usize;
        let w = |a: usize| g::<u32>(a + off).read();
        let (w0, w1, w2) = (w(0xC091F4), w(0xC091F8), w(0xC091FC));
        let weapon = if w2 != 0 {
            if r < 0x21 {
                w0
            } else if r >= 0x42 {
                w2
            } else {
                w1
            }
        } else if w1 != 0 && r >= 0x32 {
            w1
        } else {
            w0
        };
        if weapon != 0 {
            give_weapon(ped, weapon);
        }
    }
    // 0x6129D5
    let (px, py, pz) = (
        pos.read(),
        pos.wrapping_index(1).read(),
        pos.wrapping_index(2).read(),
    );
    let m = rd::<GPtr<u8>>(ped, 0x14);
    if m.is_null() {
        ped.byte_add(4).cast::<f32>().write(px);
        ped.byte_add(8).cast::<f32>().write(py);
        ped.byte_add(0xC).cast::<f32>().write(pz);
    } else {
        m.byte_add(0x30).cast::<f32>().write(px);
        rd::<GPtr<u8>>(ped, 0x14).byte_add(0x34).cast::<f32>().write(py);
        rd::<GPtr<u8>>(ped, 0x14).byte_add(0x38).cast::<f32>().write(pz);
    }
    placeable_set_orientation(ped, 0.0, 0.0, 0.0);
    world_add(ped);
    if make_wander == 0 {
        return ped;
    }
    if rd::<u32>(ped, 0x598) == 6 {
        let t = task_new(0x50);
        let t = if t.is_null() {
            t
        } else {
            let dir = random_int_in_range(0, 8);
            task_wander_cop_ctor(t, 4, dir as u32)
        };
        task_manager_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 0);
        let t2 = task_new(0x20);
        let t2 = if t2.is_null() {
            t2
        } else {
            task_stand_still_ctor(t2, 0, 1, 0, 8.0)
        };
        task_manager_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t2, 4, 0);
    } else {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        let t = get_wander_task_by_ped_type(ped);
        task_manager_set_task(intel.byte_add(4), t, 4, 0);
    }
    if cheat(0x96915D) && ty == 0x16 {
        // 0x612AFA: an event about the player (type word 0x4BC) and a weapon 10..13 with no ammo
        let mut ev = [0u32; 16];
        let evp = GPtr::from_raw(ev.as_mut_ptr());
        event_player_ctor(evp.cast::<u8>(), find_player_ped(-1));
        wr::<u16>(evp.cast(), 0xE, 0x4BC);
        event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp.cast(), 0);
        let w = random_int_in_range(0xA, 0xE) as u32;
        ped_give_delayed_weapon(ped, w as i32, 0);
        ped_set_current_weapon_type(ped, w as i32);
        event_player_dtor(evp.cast::<u8>());
    }
    if cheat(0x96913E) {
        // 0x612B6B: the nearest other ped of the pool (descending slots, squared 3D distance, `jp` keeps the
        // first of equals) gets a hate-acquaintance event (vtable 0x858E68, type word 0x3E8). The original keeps
        // the best distance in its own `type` argument slot (callers only pop it; not reproduced).
        let pool = g::<GPtr<u8>>(0xB74490).read();
        let flags = rd::<GPtr<u8>>(pool, 4);
        let mut best = GPtr::<u8>::NULL;
        let mut best_d = 0f32;
        let mut i = rd::<i32>(pool, 8);
        while i != 0 {
            i -= 1;
            if flags.byte_add(i as isize).read() & 0x80 != 0 {
                continue;
            }
            let other = rd::<GPtr<u8>>(g::<GPtr<u8>>(0xB74490).read(), 0).byte_add(i as isize * 0x7C4);
            if other.is_null() || other == ped {
                continue;
            }
            let c = ped_pos(other);
            let a = ped_pos(ped);
            let dz = x(a.wrapping_index(2).read()).fsub_m(c.wrapping_index(2).read());
            let dy = x(a.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read());
            let dx = x(a.read()).fsub_m(c.read());
            let d = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz));
            if best.is_null() || !d.fcom(x(best_d)).test_parity(5) {
                best_d = d.to_f32();
                best = other;
            }
        }
        if !best.is_null() {
            let mut ev = [0u32; 8];
            let evp = GPtr::from_raw(ev.as_mut_ptr());
            event_acquaintance_ped_ctor(GPtr::<u8>::addr(evp.cast::<u8>()) as u32, best.addr() as u32);
            evp.write(0x0085_8E68);
            wr::<u16>(evp.cast(), 0xE, 0x3E8);
            event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp.cast(), 0);
            evp.write(0x0085_8E68);
            event_acquaintance_ped_dtor(evp);
        }
    }
    ped
}

use sa_world::world::common::entity_pos_f as ped_pos;

use sa_world::bindings::acquaintance_ped_ctor as event_acquaintance_ped_ctor;
use sa_world::bindings::ctor_4aedf0 as event_player_ctor;
use sa_world::bindings::dtor_4aeea0 as event_player_dtor;
use sa_world::bindings::event_acquaintance_ped_dtor;
use sa_world::bindings::ped_set_current_weapon as ped_set_current_weapon_type;
use sa_world::bindings::find_player_ped;

/// `_stricmp(name, s) == 0` against a string of the exe.
fn name_is(name: GPtr<u8>, s: usize) -> bool {
    crt_stricmp(name, g(s)) == 0
}

/// `CPopulation::PedMICanBeCreatedAtThisAttractor(int32 mi, const char* name)` (0x6110E0, AL): no name → true;
/// "COPSIT" / "COPLOOK" / "BROWSE" → the model's ped type (+0x28) is 6 (cop); otherwise cops → false;
/// "DANCER" → models 12, 20, 22, 40, 46, 56, 58..60, 91, 93, 98, 101 (jump table 0x61124C / 0x611254);
/// "BARGUY" / "PEDROUL" / "PEDCARD" / "PEDSLOT" → every model but 27, 35, 50, 69, 71..73, 77, 134..137, 142,
/// 153, 170, 218, 225, 226, 229, 230, 232, 233, 236, 239 (table 0x611364 / 0x61136C; outside 27..239 true);
/// "STRIPW" → 87, 244, 246, 256, 257 (table 0x6112B0 / 0x6112B8); "STRIPM" → ped type != 5; any other name
/// → true. Names compared with `_stricmp` in the original order.
#[hook(addr = 0x6110E0, conv = cdecl, name = "CPopulation::PedMICanBeCreatedAtThisAttractor", expect = "56 8B 74 24 0C")]
pub fn ped_mi_can_be_created_at_this_attractor(mi: i32, name: GPtr<u8>) -> u8 {
    if name.is_null() {
        return 1;
    }
    let ped_type = || rd::<u32>(model_info(mi), 0x28);
    let ty = ped_type();
    for s in [0x86D264, 0x86D25C, 0x86D254] {
        if name_is(name, s) {
            return u8::from(ty == 6);
        }
    }
    if ty == 6 {
        return 0;
    }
    if name_is(name, 0x86D24C) {
        const DANCER: [i32; 13] = [12, 20, 22, 40, 46, 56, 58, 59, 60, 91, 93, 98, 101];
        return u8::from(DANCER.contains(&mi));
    }
    if [0x86D244, 0x86D23C, 0x86D234, 0x86D22C]
        .into_iter()
        .any(|s| name_is(name, s))
    {
        const NOT: [i32; 24] = [
            27, 35, 50, 69, 71, 72, 73, 77, 134, 135, 136, 137, 142, 153, 170, 218, 225, 226, 229, 230, 232,
            233, 236, 239,
        ];
        return u8::from(!NOT.contains(&mi));
    }
    if name_is(name, 0x86D224) {
        const STRIPW: [i32; 5] = [87, 244, 246, 256, 257];
        return u8::from(STRIPW.contains(&mi));
    }
    if !name_is(name, 0x86D21C) {
        return 1;
    }
    u8::from(ped_type() != 5)
}

/// `CPopulation::AddPedAtAttractor(int32 mi, C2dEffect* attractor, CVector pos, CEntity* entity, int32
/// decisionMaker)` (0x614210, SEH frame for a stack `CEventAttractor`, AL): fails when any
/// ped of the pool (walked downwards) is within `sqrt(0.1225)` (squared 2D distance, running minimum kept as
/// f32, start FLT_MAX) of `pos` (an empty pool skips the test), or the attractor's queue tail has not arrived
/// (`HasQueueTailArrivedAtSlot`), or `AddPed(model type, mi, pos, false)` fails. Then created-by 1, decision
/// maker (`-1` → 2), wander task, `PlacePedAtEffect(attractor, entity, ped, 0.02)`, ped +0x474 bit 21, and a
/// `CEventAttractor(attractor, entity, true)` (word +0xE = 0xE9) added to the ped's event group and processed
/// at once (`ProcessEventHandler`, `Process`). The original passes `&pos` (its own argument slot) to AddPed;
/// the port passes a local copy.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x614210, conv = cdecl, name = "CPopulation::AddPedAtAttractor", expect = "64 A1 00 00 00 00")]
pub fn add_ped_at_attractor(
    mi: i32,
    attractor: GPtr<u8>,
    px: f32,
    py: f32,
    pz: f32,
    entity: GPtr<u8>,
    decision_maker: i32,
) -> u8 {
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let n = rd::<i32>(pool, 8);
    if n != 0 {
        let mut best = x(f32::from_bits(0x7F7F_FFFF));
        let flags = rd::<GPtr<u8>>(pool, 4);
        for i in (0..n).rev() {
            if flags.wrapping_index(i).read() & 0x80 != 0 {
                continue;
            }
            let ped = rd::<GPtr<u8>>(pool, 0).byte_add((i as isize) * 0x7C4);
            if ped.is_null() {
                continue;
            }
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let p: GPtr<f32> = if m.is_null() {
                ped.byte_add(4).cast()
            } else {
                m.byte_add(0x30).cast()
            };
            let dx = x(px).fsub_m(p.read());
            let dy = x(py).fsub_m(p.wrapping_index(1).read());
            let d2 = dy.fmul(dy).fadd(dx.fmul(dx));
            if !jp5(d2, best) {
                best = x(d2.to_f32());
            }
        }
        if jne_mask(best, x(f32::from_bits(0x3DFA_E147)), 0x41) {
            return 0;
        }
    }
    let mgr = get_ped_attractor_manager();
    if has_queue_tail_arrived_at_slot(mgr, attractor, entity) == 0 {
        return 0;
    }
    let mut pos = [px, py, pz];
    let ped = add_ped_call(rd::<u32>(model_info(mi), 0x28), mi as u32, GPtr::from_raw(pos.as_mut_ptr()), 0);
    if ped.is_null() {
        return 0;
    }
    ped_set_char_created_by(ped, 1);
    let dm = if decision_maker == -1 { 2 } else { decision_maker };
    intel_set_ped_decision_maker_type(rd::<GPtr<u8>>(ped, 0x47C), dm);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let task = get_wander_task_by_ped_type(ped);
    task_manager_set_task(intel.byte_add(4), task, 4, 0);
    place_ped_at_effect(attractor, entity, ped, f32::from_bits(0x3CA3_D70A));
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x20_0000);
    let mut ev = [0u32; 8];
    let evp = GPtr::<u8>::from_raw(ev.as_mut_ptr().cast());
    event_attractor_ctor(evp, attractor, entity, 1);
    wr::<u16>(evp, 0xE, 0xE9);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
    intel_process_event_handler(rd::<GPtr<u8>>(ped, 0x47C));
    intel_process(rd::<GPtr<u8>>(ped, 0x47C));
    event_attractor_dtor(evp);
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_ADD_PED,
    HOOK_PED_MI_CAN_BE_CREATED_AT_THIS_ATTRACTOR,
    HOOK_ADD_PED_AT_ATTRACTOR,
];
