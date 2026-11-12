//! `CInterestingEvents::Add` (0x602590, batch `writerq-ai-3`): records an event a ped may look at. Not reversed in
//! gta-reversed; ported from the 1.0 US bytes (the first test sits in the protection fragment 0x401F31). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970, ST0)
    fn vector_normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, cam: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `TheCamera.ActiveCam` (byte)
const ACTIVE_CAM: usize = 0xB6F081;
/// `TheCamera.Cams[0]` (0x238 bytes each; the source position first)
const CAMS: usize = 0xB6F338;
/// `CTimer::m_FrameCounter`
const FRAME_COUNTER: usize = 0xB7CB4C;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CInterestingEvents::Add(EType type, CEntity* entity)` (0x602590, thiscall, `ret 8`). x87. Nothing when disabled
/// (+0x12C bit 0 clear) or without an entity. cam = the active camera's source. Once per frame (+0x130 vs
/// `m_FrameCounter`): dir (+0x148) = (f32(p.x - cam.x), f32(p.y - cam.y), 0) (z computed and overwritten) with p the player's position,
/// `NormaliseAndMag`; a zero length takes the player's forward vector instead; centre (+0x13C) = (f32(f32(r*dir.x) +
/// p.x), f32(f32(r*dir.y) + p.y), f32(r*dir.z + p.z)) with r = +0x138. The entity must lie within r of the centre in
/// 2D ((dy² + dx²) not above r², register values), and with +0x12C bit 1 in front of the camera plane (dir·e -
/// ((cam.y*dir.y + cam.z*dir.z) + cam.x*dir.x) not below 0), and be visible from the camera (`GetIsLineOfSightClear`,
/// buildings and see-through). Slot choice over the 8 slots (+0 type, +4 time, +8 entity, stride 0xC): an empty
/// entity (type and entity zeroed) or type 0 takes it; else a slot whose priority (+0x60[type]) is not above the
/// event's, or whose time + duration (+0x7E[type], word) has passed, is taken when the event type's next time
/// (+0xB8[type]) has passed and the slot is not the one being looked at (+0x154). The old entity reference is cleaned,
/// the slot = (type, now, entity) with a reference registered, and +0xB8[type] = now (+ half the duration with +0x12C
/// bit 3).
#[hook(addr = 0x602590, conv = thiscall, name = "CInterestingEvents::Add", expect = "83 EC 2C 56 8B F1",
       unverified = "writerq-ai-3", route = "L2", hazards = "none",
       recs = "grove-walk", note = "the first test (+0x12C bit 0) is in the protection fragment 0x401F31")]
pub fn interesting_events_add(this: GPtr<u8>, ty: i32, entity: GPtr<u8>) {
    if rd::<u8>(this, 0x12C) & 1 == 0 || entity.is_null() {
        return;
    }
    let cam = GPtr::<u8>::from_addr(CAMS + usize::from(g::<u8>(ACTIVE_CAM).read()) * 0x238);
    let mut c = [rd::<f32>(cam, 0), rd::<f32>(cam, 4), rd::<f32>(cam, 8)];
    let frame = g::<u32>(FRAME_COUNTER).read();
    if rd::<u32>(this, 0x130) != frame {
        wr::<u32>(this, 0x130, frame);
        let player = find_player_ped(-1);
        let pp = pos_of(player);
        let dir = this.byte_add(0x148);
        wr::<f32>(dir, 0, x(rd::<f32>(pp, 0)).fsub_m(c[0]).to_f32());
        wr::<f32>(dir, 4, x(rd::<f32>(pp, 4)).fsub_m(c[1]).to_f32());
        wr::<u32>(dir, 8, 0);
        let mag = X87::from_f64_exact(vector_normalise_and_mag(dir));
        if !mag.fcom(X87::zero()).test_parity(0x44) {
            let f = rd::<GPtr<u8>>(player, 0x14).byte_add(0x10);
            for k in 0..3 {
                wr::<u32>(dir, 4 * k, rd::<u32>(f, 4 * k));
            }
        }
        let r = x(rd::<f32>(this, 0x138));
        let rx = r.fmul_m(rd::<f32>(dir, 0)).to_f32();
        let ry = r.fmul_m(rd::<f32>(dir, 4)).to_f32();
        let rz = r.fmul_m(rd::<f32>(dir, 8));
        let pp = pos_of(player);
        wr::<f32>(this, 0x13C, x(rx).fadd_m(rd::<f32>(pp, 0)).to_f32());
        wr::<f32>(this, 0x140, x(ry).fadd_m(rd::<f32>(pp, 4)).to_f32());
        wr::<f32>(this, 0x144, rz.fadd_m(rd::<f32>(pp, 8)).to_f32());
    }
    let r = x(rd::<f32>(this, 0x138));
    let r2 = r.fmul(r);
    let ep = pos_of(entity);
    let a = x(rd::<f32>(this, 0x13C)).fsub_m(rd::<f32>(ep, 0));
    let b = x(rd::<f32>(this, 0x140)).fsub_m(rd::<f32>(ep, 4));
    if !b.fmul(b).fadd(a.fmul(a)).fcom(r2).test_any(0x41) {
        return;
    }
    if rd::<u8>(this, 0x12C) & 2 != 0 {
        let d = |o: isize| rd::<f32>(this, 0x148 + o);
        let dd = x(c[1])
            .fmul_m(d(4))
            .fadd(x(c[2]).fmul_m(d(8)))
            .fadd(x(c[0]).fmul_m(d(0)))
            .fchs();
        let s = x(d(8))
            .fmul_m(rd::<f32>(ep, 8))
            .fadd(x(d(4)).fmul_m(rd::<f32>(ep, 4)))
            .fadd(x(d(0)).fmul_m(rd::<f32>(ep, 0)))
            .fadd(dd);
        if !s.fcom(X87::zero()).test_parity(0x05) {
            return;
        }
    }
    if world_get_is_line_of_sight_clear(
        GPtr::from_raw(c.as_mut_ptr()).cast(),
        pos_of(entity),
        1,
        0,
        0,
        0,
        0,
        1,
        0,
    ) == 0
    {
        return;
    }
    let prio = i32::from(rd::<u8>(this, ty as isize + 0x60));
    let now = g::<u32>(TIME_MS).read();
    let type_ready = now > rd::<u32>(this, 0xB8 + 4 * ty as isize);
    let watched = i32::from(rd::<i8>(this, 0x154));
    for i in 0..8 {
        let slot = this.byte_add(0xC * i as isize);
        let st = rd::<i32>(slot, 0);
        let expired =
            now > u32::from(rd::<u16>(this, 0x7E + 2 * st as isize)).wrapping_add(rd::<u32>(slot, 4));
        let take = if rd::<u32>(slot, 8) == 0 {
            wr::<u32>(slot, 0, 0);
            wr::<u32>(slot, 8, 0);
            true
        } else if st == 0 {
            true
        } else {
            (prio >= i32::from(rd::<u8>(this, st as isize + 0x60)) || expired) && type_ready && watched != i
        };
        if !take {
            continue;
        }
        let old = rd::<GPtr<u8>>(slot, 8);
        if !old.is_null() {
            entity_clean_up_old_reference(old, slot.byte_add(8));
        }
        wr::<i32>(slot, 0, ty);
        wr::<GPtr<u8>>(slot, 8, entity);
        wr::<u32>(slot, 4, now);
        entity_register_reference(entity, slot.byte_add(8));
        let next = if rd::<u8>(this, 0x12C) & 8 != 0 {
            (u32::from(rd::<u16>(this, 0x7E + 2 * ty as isize)) >> 1).wrapping_add(now)
        } else {
            now
        };
        wr::<u32>(this, 0xB8 + 4 * ty as isize, next);
        return;
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_INTERESTING_EVENTS_ADD];
