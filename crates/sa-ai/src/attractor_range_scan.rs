//! The ped attractor scan `FUN_006060a0` (batch `mvpl-ai-1`): `CAttractorScanner`'s periodic scan of the world
//! sectors and the scripted 2D effects around a ped, raising `CEventAttractor` / `CEventScriptedAttractor` for the
//! closest attractor found. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CAttractorScanner::Clear()` (0x5FFF90)
    fn attractor_scanner_clear(s: GPtr<u8>) = thiscall 0x5FFF90 => crate::attractor_scanner::attractor_scanner_clear;
    /// `FUN_006034b0(CPtrList& list, CPed* ped)` (0x6034B0, `ret 8`): the attractor effects of one sector list's
    /// entities
    fn scan_entity_list(s: GPtr<u8>, list: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6034B0 => unported;
    /// `FUN_005fe960(int32)` (0x5FE960, `ret 4`) on a scripted effect's user list, with the ped's +0x598
    fn scripted_user_list_check(list: GPtr<u8>, v: u32) -> u8 = thiscall 0x5FE960 => unported;
    /// `FUN_005fffd0(C2dEffect*, CEntity*, CPed*)` (0x5FFFD0, `ret 0xC`): offers one attractor effect
    fn add_effect(s: GPtr<u8>, effect: GPtr<u8>, entity: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5FFFD0 => attractor_scanner_add_effect;
    /// `FUN_00600180(C2dEffect** effect, CEntity** entity)` (0x600180, `ret 8`): the chosen attractor
    fn get_closest_attractor(s: GPtr<u8>, effect: GPtr<u8>, entity: GPtr<u8>) = thiscall 0x600180 => unported;
    /// `FUN_00603570(C2dEffect*)` (0x603570): the ped a scripted attractor (type 7) is meant for
    fn scripted_attractor_ped(effect: GPtr<u8>) -> GPtr<u8> = cdecl 0x603570 => unported;
    /// `CEventAttractor::CEventAttractor(C2dEffect*, CEntity*, bool)` (0x4AF350, `ret 0xC`)
    fn event_attractor_ctor(e: GPtr<u8>, effect: GPtr<u8>, entity: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AF350 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventAttractor::~CEventAttractor()` (`FUN_004af400`)
    fn event_attractor_dtor(e: GPtr<u8>) = thiscall 0x4AF400 => unported;
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIMER_TIME_MS: usize = 0xB7CB84;
/// `CWorld::ms_aSectors` (0xB7D0B8, 120 x 120, 8 bytes: the building list first)
const SECTORS: u32 = 0xB7D0B8;
/// `CWorld::ms_aRepeatSectors` (0xB992B8, 16 x 16, 12 bytes: the object list at +8)
const REPEAT_SECTORS: u32 = 0xB992B8;
/// The scripted 2D effects: active flags (0xC3A1A0, 64 bytes), user lists (0xC3A200, 0x24 bytes: four model ids,
/// +0x20 restricted flag), effects (0xC3AB00, 0x40 bytes)
const SCRIPTED_ACTIVE: u32 = 0xC3A1A0;
const SCRIPTED_USERS: u32 = 0xC3A200;
const SCRIPTED_EFFECTS: u32 = 0xC3AB00;
/// `CEventScriptedAttractor` vtable (0x86C9F0)
const SCRIPTED_ATTRACTOR_VTABLE: u32 = 0x86C9F0;
/// The scan radius (0x86C52C), 1/50 (0x858B38), 60 (0x858B34): `GetSectorX/Y`
const K_RADIUS: usize = 0x86C52C;
const K_SECTOR_SCALE: usize = 0x858B38;
const K_SECTOR_OFFSET: usize = 0x858B34;

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `_ftol(floor(v))` of the register value stored as a double (CRT `floor` 0x8219F0, `_ftol` 0x821B40).
fn floor_ftol(v: X87) -> i32 {
    ftol32(X87::from_f64_exact(sa_world::physical::base::floor_f64(v.to_f64())))
}

/// `FUN_006060a0(CPed* ped)` (thiscall on a `CAttractorScanner`, `ret 4`). Layout: +0 enabled, +4 task timer (+4
/// start, +8 period, +0xC set, +0xD reset), +0x10 / +0x14 the attractor effect and entity in use.
///
/// Disabled, or ped flags +0x46C bit 8 set → nothing. An active task of type 0xE9 → timer (now, 3000), set, nothing
/// more. Timer: unset → (now, 1500), set (re-read: 0 → nothing); reset → start = now, cleared; not `now >= start +
/// period` (unsigned) → nothing; else (now, 1500), set, `CAttractorScanner::Clear`.
///
/// Sector box around the ped position `p` with radius `r` (x87, `fstp qword` to `floor`, `_ftol`): `x0 = max(⌊(p.x -
/// r) / 50 + 60⌋, 0)`, `y0` likewise, `x1 = min(⌊(r + p.x) / 50 + 60⌋, 119)`, `y1` likewise (`1/50` as a multiply).
/// For `y` in `y0..=y1`, `x` in `x0..=x1`: `FUN_006034b0` on the building list of sector `(min(max(y, 0), 119),
/// min(max(x, 0), 119))` then on the object list of repeat sector `(y & 15, x & 15)`. Then each of the 64 active
/// scripted effects whose user list is unrestricted (+0x20 clear), names the ped's model (four dwords against the
/// sign-extended +0x22), or passes `FUN_005fe960(ped +0x598)` → `FUN_005fffd0(effect, NULL, ped)`.
///
/// `FUN_00600180` picks `(effect, entity)`; no effect, or the pair already in use → nothing. Attractor type
/// (+0x34) 7: the ped must be `FUN_00603570(effect)`; a stack `CEventAttractor(effect, entity, false)` turned into
/// a `CEventScriptedAttractor` (vtable 0x86C9F0); other types: a plain `CEventAttractor`. Added to the ped's event
/// group (+0x47C → +0x68): a non-NULL result stores the pair in use (+0x14 entity, then +0x10 effect). Then the
/// event's destructor (the scripted one re-sets its vtable first). The original keeps `x1` and the chosen effect
/// in its own ped argument slot.
#[hook(addr = 0x6060a0, conv = thiscall, name = "FUN_006060a0", expect = "64 A1 00 00 00 00",
       unverified = "mvpl-ai-1", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the events live on the Rust stack; the ped argument slot holds the sector bound and the chosen effect")]
pub fn attractor_scanner_scan_in_range(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0) == 0 {
        return;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return;
    }
    let now = || g::<u32>(TIMER_TIME_MS).read();
    let task = tm_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if !task.is_null() && v_task_type(task) == 0xE9 {
        wr::<u32>(this, 4, now());
        wr::<u32>(this, 8, 3000);
        wr::<u8>(this, 0xC, 1);
        return;
    }
    if rd::<u8>(this, 0xC) == 0 {
        wr::<u32>(this, 4, now());
        wr::<u32>(this, 8, 1500);
        wr::<u8>(this, 0xC, 1);
        if rd::<u8>(this, 0xC) == 0 {
            return;
        }
    }
    if rd::<u8>(this, 0xD) != 0 {
        wr::<u32>(this, 4, now());
        wr::<u8>(this, 0xD, 0);
    }
    let start = rd::<u32>(this, 4);
    let t = now();
    if t < rd::<u32>(this, 8).wrapping_add(start) {
        return;
    }
    wr::<u32>(this, 4, t);
    wr::<u32>(this, 8, 1500);
    wr::<u8>(this, 0xC, 1);
    attractor_scanner_clear(this);

    let (r, s, o) = (k(K_RADIUS), k(K_SECTOR_SCALE), k(K_SECTOR_OFFSET));
    let lo = |c: isize| floor_ftol(x(rd::<f32>(entity_pos(ped), c)).fsub_m(r).fmul_m(s).fadd_m(o));
    let hi = |c: isize| floor_ftol(x(r).fadd_m(rd::<f32>(entity_pos(ped), c)).fmul_m(s).fadd_m(o));
    let x0 = lo(0).max(0);
    let y0 = lo(4).max(0);
    let x1 = hi(0).min(0x77);
    let y1 = hi(4).min(0x77);
    let mut y = y0;
    while y <= y1 {
        let cy = y.clamp(0, 0x77);
        let row = (y & 0xF) << 4;
        let mut xx = x0;
        while xx <= x1 {
            let cx = xx.clamp(0, 0x77);
            let sector = SECTORS.wrapping_add((cy.wrapping_mul(0x78).wrapping_add(cx) as u32).wrapping_mul(8));
            scan_entity_list(this, GPtr::from_addr(sector as usize), ped);
            let rep = REPEAT_SECTORS.wrapping_add((((xx & 0xF) + row) as u32).wrapping_mul(12)).wrapping_add(8);
            scan_entity_list(this, GPtr::from_addr(rep as usize), ped);
            xx += 1;
        }
        y += 1;
    }

    for i in 0..64u32 {
        if g::<u8>((SCRIPTED_ACTIVE + i) as usize).read() == 0 {
            continue;
        }
        let users: GPtr<u8> = GPtr::from_addr((SCRIPTED_USERS + i * 0x24) as usize);
        let offer = rd::<u8>(users, 0x20) == 0 || {
            let model = i32::from(rd::<i16>(ped, 0x22));
            (0..4).any(|j| rd::<i32>(users, 4 * j) == model)
                || scripted_user_list_check(users, rd::<u32>(ped, 0x598)) != 0
        };
        if offer {
            add_effect(this, GPtr::from_addr((SCRIPTED_EFFECTS + i * 0x40) as usize), GPtr::NULL, ped);
        }
    }

    let mut effect = GPtr::<u8>::NULL;
    let mut entity = GPtr::<u8>::NULL;
    get_closest_attractor(
        this,
        GPtr::from_raw(&mut effect as *mut GPtr<u8>).cast(),
        GPtr::from_raw(&mut entity as *mut GPtr<u8>).cast(),
    );
    if effect.is_null() {
        return;
    }
    if effect.addr() == rd::<GPtr<u8>>(this, 0x10).addr() && rd::<GPtr<u8>>(this, 0x14).addr() == entity.addr() {
        return;
    }
    let scripted = rd::<u8>(effect, 0x34) == 7;
    if scripted && scripted_attractor_ped(effect).addr() != ped.addr() {
        return;
    }
    let mut ev = [0u32; 8];
    let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_attractor_ctor(ev_p, effect, entity, 0);
    if scripted {
        wr::<u32>(ev_p, 0, SCRIPTED_ATTRACTOR_VTABLE);
    }
    if !event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ev_p, 0).is_null() {
        wr::<GPtr<u8>>(this, 0x14, entity);
        wr::<GPtr<u8>>(this, 0x10, effect);
    }
    if scripted {
        wr::<u32>(ev_p, 0, SCRIPTED_ATTRACTOR_VTABLE);
    }
    event_attractor_dtor(ev_p);
}

orig! {
    /// `CEntity::TransformFromObjectSpace(CVector& out, const CVector& offset)` (0x5334F0, `ret 8`)
    fn entity_transform_from_object_space(e: GPtr<u8>, out: GPtr<u8>, offset: GPtr<u8>) -> GPtr<u8> = thiscall 0x5334F0 => unported;
    /// `CScripted2dEffects::GetIndex(const C2dEffect*)` (0x6F9F60)
    fn scripted_2d_effects_get_index(effect: GPtr<u8>) -> i32 = cdecl 0x6F9F60 => unported;
    /// `GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::HasEmptySlot(const C2dEffect*, const CEntity*)` (0x5EBB00, `ret 8`)
    fn attractor_manager_has_empty_slot(m: GPtr<u8>, effect: GPtr<u8>, entity: GPtr<u8>) -> u8 = thiscall 0x5EBB00 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, `ret 4`)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CMatrix::SetScale(float)` (0x59AED0, `ret 4`)
    fn matrix_set_scale(m: GPtr<u8>, s: u32) = thiscall 0x59AED0 => unported;
    /// `CPedAttractorManager::IsApproachable(C2dEffect*, const CMatrix&, int32, CPed*)` (0x5EA220)
    fn attractor_manager_is_approachable(effect: GPtr<u8>, m: GPtr<u8>, unused: u32, ped: GPtr<u8>) -> u8 = cdecl 0x5EA220 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// `CWeather::Rain` (0xC81324) and the rain level (0x858CC4) from which only shelter attractors (type 4) count
const WEATHER_RAIN: usize = 0xC81324;
const K_RAIN_LEVEL: usize = 0x858CC4;
/// `CScripted2dEffects` radius per scripted effect (0xC39EE0)
const SCRIPTED_RADII: u32 = 0xC39EE0;

/// `FUN_005fffd0(C2dEffect* effect, CEntity* entity, CPed* ped)` (0x5FFFD0, thiscall on a `CAttractorScanner`, `ret
/// 0xC`; batch `mvpl-ai-1`): offers one attractor effect. `t` = the attractor type (+0x34). `Rain` below the level
/// (x87 C0, unordered too): type 4 is refused; otherwise only type 4 passes (the rain test is repeated for the other
/// types). Position: `entity.TransformFromObjectSpace(effect position)` with an entity, else the effect position.
/// `d2 = (dz² + dy²) + dx²` of `ped position - that` (x87, `fst` into the original's own `effect` argument slot) must
/// be below the type's best distance (+0x68 + 4t; `jp` on `test ah, 5`). Type 7: with the effect's scripted radius `r`
/// (0xC39EE0 + 4 × `CScripted2dEffects::GetIndex`) not below 0 (`jnp` skips the test when it is), `f32(d2)` must be
/// below `r * r`. Then `GetPedAttractorManager()->HasEmptySlot(effect, entity)`; a stack `CMatrix` (attach / owner
/// fields cleared) = the entity's matrix (`GetMatrix`, `operator=`) or `SetScale(1.0)`; `IsApproachable(effect,
/// matrix, 0, ped)` → +0x18 + 4t = entity, +0x68 + 4t = `f32(d2)`, +0x40 + 4t = effect. The matrix destructor runs
/// after `HasEmptySlot` passed.
#[hook(addr = 0x5fffd0, conv = thiscall, name = "FUN_005fffd0", expect = "64 A1 00 00 00 00",
       unverified = "mvpl-ai-1", route = "L2", hazards = "stack-residue,heap-writes", recs = "grove-walk",
       note = "writes the ped's attractor scanner (ped object); the matrix lives on the Rust stack; the effect argument slot holds f32(d2)")]
pub fn attractor_scanner_add_effect(this: GPtr<u8>, effect: GPtr<u8>, entity: GPtr<u8>, ped: GPtr<u8>) {
    let rain_low = || x(g::<f32>(WEATHER_RAIN).read()).fcom(x(g::<f32>(K_RAIN_LEVEL).read())).c0();
    let first = rain_low();
    let t = rd::<u8>(effect, 0x34) as isize;
    if first {
        if t == 4 {
            return;
        }
    } else if t != 4 && !rain_low() {
        return;
    }
    let mut pos = [0f32; 3];
    let pos_p: GPtr<u8> = GPtr::from_raw(pos.as_mut_ptr()).cast();
    if !entity.is_null() {
        entity_transform_from_object_space(entity, pos_p, effect);
    } else {
        pos = [rd::<f32>(effect, 0), rd::<f32>(effect, 4), rd::<f32>(effect, 8)];
    }
    let pp = entity_pos(ped);
    let dx = x(rd::<f32>(pp, 0)).fsub_m(pos[0]);
    let dy = x(rd::<f32>(pp, 4)).fsub_m(pos[1]);
    let dz = x(rd::<f32>(pp, 8)).fsub_m(pos[2]);
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    let d2f = d2.to_f32();
    if d2.fcom(x(rd::<f32>(this, 0x68 + 4 * t))).test_parity(5) {
        return;
    }
    if t == 7 {
        let idx = scripted_2d_effects_get_index(effect);
        let r = x(g::<f32>(SCRIPTED_RADII.wrapping_add((idx as u32).wrapping_mul(4)) as usize).read());
        if r.fcom(x(0.0)).test_parity(5) && x(d2f).fcom(r.fmul(r)).test_parity(5) {
            return;
        }
    }
    if attractor_manager_has_empty_slot(get_ped_attractor_manager(), effect, entity) == 0 {
        return;
    }
    let mut m = [0u32; 0x12];
    let m_p: GPtr<u8> = GPtr::from_raw(m.as_mut_ptr()).cast();
    if !entity.is_null() {
        matrix_assign(m_p, placeable_get_matrix(entity));
    } else {
        matrix_set_scale(m_p, 0x3F80_0000);
    }
    if attractor_manager_is_approachable(effect, m_p, 0, ped) != 0 {
        wr::<GPtr<u8>>(this, 0x18 + 4 * t, entity);
        wr::<f32>(this, 0x68 + 4 * t, d2f);
        wr::<GPtr<u8>>(this, 0x40 + 4 * t, effect);
    }
    matrix_dtor(m_p);
}

pub const HOOKS: &[HookReg] = &[HOOK_ATTRACTOR_SCANNER_SCAN_IN_RANGE, HOOK_ATTRACTOR_SCANNER_ADD_EFFECT];
