//! CPopulation ped creation (PORT-POP-3, helper B). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, jne_mask, jp5, model_info, rd, wr, x};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CPed::operator new(uint32)` (0x5E4720)
    fn ped_new(size: u32) -> GPtr<u8> = cdecl 0x5E4720 => unported;
    /// `CCivilianPed::CCivilianPed(ePedType, uint32 model)` (0x5DDB70, `ret 8`)
    fn civilian_ped_ctor(this: GPtr<u8>, ty: u32, model: u32) -> GPtr<u8> = thiscall 0x5DDB70 => unported;
    /// `CCopPed::CCopPed(eCopType)` (0x5DDC60, `ret 4`)
    fn cop_ped_ctor(this: GPtr<u8>, cop_type: u32) -> GPtr<u8> = thiscall 0x5DDC60 => unported;
    /// `CEmergencyPed::CEmergencyPed(ePedType, uint32 model)` (0x5DE340, `ret 8`)
    fn emergency_ped_ctor(this: GPtr<u8>, ty: u32, model: u32) -> GPtr<u8> = thiscall 0x5DE340 => unported;
    /// `CPed::GiveDelayedWeapon(eWeaponType, uint32 ammo)` (0x5E89B0, `ret 8`)
    fn ped_give_delayed_weapon(ped: GPtr<u8>, w: u32, ammo: u32) = thiscall 0x5E89B0 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(w: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0, `ret 4`)
    fn ped_set_current_weapon_slot(ped: GPtr<u8>, slot: u32) = thiscall 0x5E61F0 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80, `ret 0xC`)
    fn placeable_set_orientation(e: GPtr<u8>, xr: f32, yr: f32, zr: f32) = thiscall 0x439A80 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexWanderCop::CTaskComplexWanderCop(int32 moveState, uint8 dir)` (0x460C80, `ret 8`)
    fn task_wander_cop_ctor(this: GPtr<u8>, move_state: u32, dir: u32) -> GPtr<u8> = thiscall 0x460C80 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn task_stand_still_ctor(this: GPtr<u8>, time: u32, looped: u32, b: u32, f: f32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn task_manager_set_task(tm: GPtr<u8>, task: GPtr<u8>, slot: u32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00)
    fn get_wander_task_by_ped_type(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::HasQueueTailArrivedAtSlot(const C2dEffect*, const CEntity*)` (0x5EBBA0, `ret 8`)
    fn has_queue_tail_arrived_at_slot(mgr: GPtr<u8>, fx: GPtr<u8>, ent: GPtr<u8>) -> u8 = thiscall 0x5EBBA0 => unported;
    /// `CPopulation::AddPed(ePedType, uint32, const CVector&, bool)` (0x612710)
    fn add_ped_call(ty: u32, model: u32, pos: GPtr<f32>, make_wander: u8) -> GPtr<u8> = cdecl 0x612710 => add_ped;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0, `ret 4`)
    fn ped_set_char_created_by(ped: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50, `ret 4`)
    fn intel_set_ped_decision_maker_type(intel: GPtr<u8>, t: i32) = thiscall 0x600B50 => unported;
    /// `CPedAttractorPedPlacer::PlacePedAtEffect(const C2dEffect&, CEntity*, CPed*, float)` (0x5EA390)
    fn place_ped_at_effect(fx: GPtr<u8>, ent: GPtr<u8>, ped: GPtr<u8>, f: f32) = cdecl 0x5EA390 => unported;
    /// `CEventAttractor::CEventAttractor(C2dEffect*, CEntity*, bool)` (0x4AF350, `ret 0xC`)
    fn event_attractor_ctor(this: GPtr<u8>, fx: GPtr<u8>, ent: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AF350 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPedIntelligence::ProcessEventHandler()` (0x601BB0)
    fn intel_process_event_handler(intel: GPtr<u8>) = thiscall 0x601BB0 => unported;
    /// `CPedIntelligence::Process()` (0x608260)
    fn intel_process(intel: GPtr<u8>) = thiscall 0x608260 => unported;
    /// `CEventAttractor::~CEventAttractor` (0x4AF400)
    fn event_attractor_dtor(this: GPtr<u8>) = thiscall 0x4AF400 => unported;
}

/// `GiveDelayedWeapon(w, 25001)` + `SetCurrentWeapon(GetWeaponInfo(w, 1)->m_nSlot)` (AddPed's weapon tail).
fn give_weapon(ped: GPtr<u8>, w: u32) {
    ped_give_delayed_weapon(ped, w, 0x61A9);
    let slot = rd::<u32>(get_weapon_info(w, 1), 0x14);
    ped_set_current_weapon_slot(ped, slot);
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
/// `GetWanderTaskByPedType`). NOT PORTED (fallback to the whole original, decided at entry): any of the cheat
/// bytes 0x969140, 0x96913E, 0x96915C, 0x96915D set (armed civilians, gang weapons, angry peds, ped riot event
/// paths) and types < 4, 21, > 31 (the original uses its `makeWander` argument as the ped pointer there).
#[hook(addr = 0x612710, conv = cdecl, name = "CPopulation::AddPed", expect = "90 E9 AD F1 DE FF", fallback)]
pub fn add_ped(ty: u32, model: u32, pos: GPtr<f32>, make_wander: u8) -> GPtr<u8> {
    let cheat = |a: usize| g::<u8>(a).read() != 0;
    if cheat(0x969140)
        || cheat(0x96913E)
        || cheat(0x96915C)
        || cheat(0x96915D)
        || !(4..=31).contains(&ty)
        || ty == 21
    {
        return add_ped_original(ty, model, pos, make_wander);
    }
    let ped = match ty {
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
    if (7..=16).contains(&ty) && random_int_in_range(0, 100) < 0x21 {
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
    ped
}

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
    let ped = add_ped_call(
        rd::<u32>(model_info(mi), 0x28),
        mi as u32,
        GPtr::from_raw(pos.as_mut_ptr()),
        0,
    );
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
