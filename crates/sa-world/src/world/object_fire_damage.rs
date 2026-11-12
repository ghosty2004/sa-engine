//! PORT-VEH-3 (batch `world3-3`, SA:MP-executed rows): `CObject::ObjectFireDamage`. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr, x};

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CAudioEngine::ReportObjectDestruction(CEntity*)` (0x506ED0) on `AudioEngine` (0xB6BC90)
    fn audio_report_object_destruction(ae: GPtr<u8>, e: GPtr<u8>) = thiscall 0x506ED0 => unported;
    /// `BreakManager_c::Add(CObject*, RwV3d*, float, bool)` (0x59E9B0) on `g_breakMan` (0xBB4240)
    fn break_manager_add(m: GPtr<u8>, o: GPtr<u8>, v: GPtr<u8>, f: u32, b: u32) = thiscall 0x59E9B0 => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => unported;
    /// virtual slot +0x20 (`CEntity::DeleteRwObject`)
    fn v_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CShadows::AddPermanentShadow(uint8, RwTexture*, CVector*, float, float, float, float, int16, uint8, uint8, uint8, float, uint32, float)` (0x706F60)
    fn add_permanent_shadow(ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32, i: u32, r: u32, gg: u32, b: u32, dist: u32, time: u32, up: u32) = cdecl 0x706F60 => unported;
}

/// `CObject::ObjectFireDamage(float damage, CEntity* damager)` (0x5A1580): only for the two burnable
/// models (0x8CD6F8 / 0x8CD6F4): health (+0x154) loses `damage` (not below 0). With a fire-damage
/// effect (+0x144): invulnerable-to-others objects (+0x40 bit 22) only take it from the player;
/// model 0x8CD6F8 starts burning (+0x174 = now + 3000, +0x178 = 1 - health * 0.0005); at zero
/// health: effect 1 hides the object (+0x1C bit 9, DeleteRwObject) and leaves a permanent scorch
/// shadow, effects 200/202 break it (`BreakManager_c::Add` with the object info's break velocity,
/// the destruction sound unless already reported, static, no speed).
#[hook(addr = 0x5A1580, conv = thiscall, name = "CObject::ObjectFireDamage", this = "CObject",
       expect = "83 EC 0C 56 8B F1",
       unverified = "world3-3", route = "L2", hazards = "heap-writes",
       note = "breaks objects into the break manager (heap), adds shadows")]
pub fn object_fire_damage(this: GPtr<u8>, damage: f32, damager: GPtr<u8>) {
    let mi = i32::from(rd::<i16>(this, 0x22));
    let a = i32::from(g::<u16>(0x8C_D6F8).read());
    if mi != a && mi != i32::from(g::<u16>(0x8C_D6F4).read()) {
        return;
    }
    let h = x(rd::<f32>(this, 0x154)).fsub_m(damage);
    wr::<f32>(this, 0x154, h.to_f32());
    if !h.fcom(x(0.0)).test_parity(0x05) {
        wr::<u32>(this, 0x154, 0);
    }
    if rd::<u8>(this, 0x144) == 0 {
        return;
    }
    if rd::<u32>(this, 0x40) & 0x40_0000 != 0 && damager != find_player_ped(-1) && damager != find_player_vehicle(-1, 0) {
        return;
    }
    if i32::from(rd::<i16>(this, 0x22)) == i32::from(g::<u16>(0x8C_D6F8).read()) {
        wr::<u32>(this, 0x174, g::<u32>(0xB7_CB84).read().wrapping_add(3000));
        let v = x(rd::<f32>(this, 0x154)).fmul_m(f32::from_bits(0x3A03_126F)).fsubr_m(1.0);
        wr::<f32>(this, 0x178, v.to_f32());
    }
    if x(rd::<f32>(this, 0x154)).fcom(x(0.0)).test_parity(0x44) {
        return;
    }
    let effect = rd::<u8>(this, 0x144);
    wr::<u32>(this, 0x174, 0);
    match effect {
        1 => {
            let f = rd::<u32>(this, 0x1C);
            if f & 0x200 != 0 {
                return;
            }
            wr::<u32>(this, 0x1C, f | 0x200);
            v_delete_rw_object(this);
            let m = rd::<GPtr<u8>>(this, 0x14);
            let p = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
            let mut pos = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
            add_permanent_shadow(
                1,
                g::<u32>(0xC4_03E8).read(),
                GPtr::from_raw(&mut pos as *mut [u32; 3]).cast(),
                0x4040_0000,
                0,
                0,
                0xC040_0000,
                200,
                0,
                0,
                0,
                0x4120_0000,
                30000,
                0x3F80_0000,
            );
        }
        200 | 202 => {
            if rd::<u32>(this, 0x140) & 0x400 == 0 {
                audio_report_object_destruction(GPtr::from_addr(0xB6_BC90), this);
            }
            let info = rd::<GPtr<u8>>(this, 0x160);
            let mut v = [rd::<u32>(info, 0x38), rd::<u32>(info, 0x3C), rd::<u32>(info, 0x40)];
            break_manager_add(
                GPtr::from_addr(0xBB_4240),
                this,
                GPtr::from_raw(&mut v as *mut [u32; 3]).cast(),
                rd::<u32>(info, 0x44),
                1,
            );
            let f = rd::<u32>(this, 0x1C) & 0xFFFF_FF7E;
            wr::<u32>(this, 0x1C, f);
            if f & 4 == 0 && f & 0x4_0000 == 0 {
                remove_from_moving_list(this);
            }
            wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 4);
            wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x80_0000);
            for o in [0x44, 0x48, 0x4C, 0x50, 0x54, 0x58] {
                wr::<u32>(this, o, 0);
            }
            wr::<u32>(this, 0x140, rd::<u32>(this, 0x140) | 0x400);
            v_delete_rw_object(this);
        }
        _ => {}
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_OBJECT_FIRE_DAMAGE];
