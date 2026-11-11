//! L1 specs for PORT-WEAPON-PED `sa_weapon` (see `docs/arch/weapon-ped-port.md`): the pure WEAPON
//! functions whose memory fits the hosted scratch image (the 60-fire `CFireManager`, the explosion and
//! shot arrays do not; they are L2-verified), and every hook that preserves registers.

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::abi::PRESERVE_ECX;
use sa_core::mem::GPtr;
use sa_weapon::{fire, water_cannon, weapon, weapon_info};

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

/// A weapon type: mostly 0..60, sometimes any word.
fn g_type(r: &mut Rng) -> u32 {
    if r.chance(90) { r.below(60) } else { r.next_u32() }
}

/// Region word 0 is a weapon type (small most of the time).
fn fix_type(v: &mut [u32], l: &Layout, region: usize) {
    let s = l.word_slot[region][0].unwrap();
    if !v[s].is_multiple_of(10) {
        v[s] %= 60;
    }
}

const FIRE_NAMES: [&[u8]; 8] = [
    b"MELEE\0",
    b"INSTANT_HIT\0",
    b"PROJECTILE\0",
    b"AREA_EFFECT\0",
    b"CAMERA\0",
    b"USE\0",
    b"USEX\0",
    b"MELE\0",
];

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "WeaponIsType2Handed",
            "CWeapon::IsType2Handed",
            0x73B1E0,
            Thiscall,
            Al,
        )
        .regions(vec![raw(1)])
        .args(vec![A::P(0)])
        .fix(|v, l| fix_type(v, l, 0))
        .rust(|c| ExpRet::Al(weapon::is_type_2_handed(tp(c, 0)))),
        b(
            "WeaponIsTypeProjectile",
            "CWeapon::IsTypeProjectile",
            0x73B210,
            Thiscall,
            Al,
        )
        .regions(vec![raw(1)])
        .args(vec![A::P(0)])
        .fix(|v, l| fix_type(v, l, 0))
        .rust(|c| ExpRet::Al(weapon::is_type_projectile(tp(c, 0)))),
        b(
            "WeaponHasWeaponAmmoToBeUsed",
            "CWeapon::HasWeaponAmmoToBeUsed",
            0x73B2A0,
            Thiscall,
            Al,
        )
        .regions(vec![raw(7)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            fix_type(v, l, 0);
            let s = l.word_slot[0][3].unwrap();
            if v[s].is_multiple_of(3) {
                v[s] = 0;
            }
        })
        .rust(|c| ExpRet::Al(weapon::has_weapon_ammo_to_be_used(tp(c, 0)))),
        b(
            "WeaponCanBeUsedFor2Player",
            "CWeapon::CanBeUsedFor2Player-Static",
            0x73B240,
            Cdecl,
            Al,
        )
        .args(vec![A::I(g_type)])
        .rust(|c| ExpRet::Al(weapon::can_be_used_for_2_player(c.i(0) as i32))),
        b(
            "WeaponInfoGetSkillStatIndex",
            "CWeaponInfo::GetSkillStatIndex",
            0x743CD0,
            Cdecl,
            Eax,
        )
        .args(vec![A::I(g_type)])
        .rust(|c| ExpRet::Eax(weapon_info::get_skill_stat_index(c.i(0) as i32) as u32)),
        // the string: one of the names (or a near miss) in region 0
        b(
            "WeaponInfoFindWeaponFireType",
            "CWeaponInfo::FindWeaponFireType",
            0x5BCF30,
            Cdecl,
            Eax,
        )
        .regions(vec![raw(4)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            let s0 = l.word_slot[0][0].unwrap();
            let pick = v[s0] % 10;
            if (pick as usize) < FIRE_NAMES.len() {
                let n = FIRE_NAMES[pick as usize];
                let mut bytes = [0u8; 16];
                for (k, b) in n.iter().enumerate() {
                    bytes[k] = *b;
                }
                for w in 0..4 {
                    let s = l.word_slot[0][w].unwrap();
                    v[s] = u32::from_le_bytes(bytes[4 * w..4 * w + 4].try_into().unwrap());
                }
            }
        })
        .rust(|c| ExpRet::Eax(weapon_info::find_weapon_fire_type(tp(c, 0)))),
        b(
            "WeaponInfoGetCrouchReloadAnimationID",
            "CWeaponInfo::GetCrouchReloadAnimationID",
            0x685700,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(7)])
        .args(vec![A::P(0)])
        .rust(|c| ExpRet::Eax(weapon_info::get_crouch_reload_animation_id(tp(c, 0)))),
        b(
            "WeaponInfoGetTargetHeadRange",
            "CWeaponInfo::GetTargetHeadRange",
            0x743D50,
            Thiscall,
            St0,
        )
        .regions(vec![Region {
            words: (0..13).map(|w| if w == 2 { W::F } else { W::R }).collect(),
            kind: Kind::Any,
        }])
        .args(vec![A::P(0)])
        .rust(|c| st0(weapon_info::get_target_head_range(tp(c, 0)))),
        b("FireInitialise", "CFire::Initialise", 0x538B30, Thiscall, Void)
            .regions(vec![raw(10)])
            .args(vec![A::P(0)])
            .preserve(PRESERVE_ECX)
            .rust(|c| {
                fire::initialise(tp(c, 0));
                ExpRet::Void
            }),
        b(
            "WaterCannonCtor",
            "CWaterCannon::CWaterCannon",
            0x728B10,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(0xEB)])
        .args(vec![A::P(0)])
        .rust(|c| {
            water_cannon::constructor(tp(c, 0));
            ExpRet::Eax(c.addr(0))
        }),
        // section index (int16 at +4) 0..31; the point/velocity arrays and flags lie inside region 0
        b(
            "WaterCannonUpdateNewInput",
            "CWaterCannon::Update_NewInput",
            0x728C20,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0xCB), vec3(), vec3()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .fix(|v, l| {
            let s = l.word_slot[0][1].unwrap();
            v[s] = (v[s] & 0xFFFF_0000) | (v[s] & 0x1F);
        })
        .rust(|c| {
            water_cannon::update_new_input(tp(c, 0), tp(c, 1), tp(c, 2));
            ExpRet::Void
        }),
    ]
}
