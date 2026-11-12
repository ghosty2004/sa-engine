//! `CPickups::CreateSomeMoney` (INTRO-2: the money of the peds the intro's drive-by kills). Ported from the 1.0 US
//! bytes; gta-reversed `Pickups.cpp` as a reading aid.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, x, xi};

orig! {
    /// `CWorld::FindGroundZFor3DCoord(float x, float y, float z, bool* found, CEntity** entity)` (0x5696C0, ST0)
    fn find_ground_z_for_3d_coord(px: f32, py: f32, pz: f32, found: GPtr<u8>, entity: GPtr<u8>) -> f64 = cdecl 0x5696C0 => unported;
    /// `CPickups::GenerateNewOne(CVector, uint32 model, uint8 type, uint32 ammo, uint32 money, bool, const char*)`
    /// (0x456F20)
    #[allow(clippy::too_many_arguments)]
    fn generate_new_one(px: f32, py: f32, pz: f32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> i32 = cdecl 0x456F20 => unported;
}

/// `ModelIndices::MI_MONEY` (int16, set at run time)
const MI_MONEY: usize = 0x8CD59C;
/// 2 pi / 256 (0x859BBC), 1.5 (0x858CE8), 0.5 (0x858B8C)
const ANGLE_STEP: u32 = 0x3CC9_0FDB;
const SCATTER: u32 = 0x3FC0_0000;
const LIFT: u32 = 0x3F00_0000;

/// `rand()` called from the original at `site` (the return address).
fn rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// `CPickups::CreateSomeMoney(CVector pos, int32 amount)` (0x458970, cdecl).
///
/// `n = min(amount / 20 + 1, 7)` pickups of `amount / n` each. Each one moves the position (its own argument
/// slots, so the offsets accumulate): `x += sin((rand() & 0xFF) * 2pi/256) * 1.5`, `y += cos(...) * 1.5` (second
/// rand), each stored as f32; `z = FindGroundZFor3DCoord(x, y, z, &found, null) + 0.5` (stored as f32; `found` is
/// written over the low byte of the amount slot, already read); when found, `GenerateNewOne((x, y, z), MI_MONEY,
/// 8, amount / n + (rand() & 3), 0, false, null)`.
#[hook(addr = 0x458970, conv = cdecl, name = "CPickups::CreateSomeMoney", expect = "83 EC 08 53 55 56 8B 74 24 24",
       unverified = "intro3-11", route = "L2", hazards = "heap-writes", recs = "intro",
       note = "the position offsets accumulate in the argument slots as in the original")]
pub fn create_some_money(px: f32, py: f32, pz: f32, amount: i32) {
    let mut n = amount / 20 + 1;
    if n >= 7 {
        n = 7;
    }
    let each = amount.wrapping_div(n);
    let (mut px, mut py, mut pz) = (px, py, pz);
    let mut left = n;
    while left > 0 {
        let r = rand_at(0x4589B5) & 0xFF;
        px = xi(r).fmul_m(f32::from_bits(ANGLE_STEP)).fsin().fmul_m(f32::from_bits(SCATTER)).fadd_m(px).to_f32();
        let r = rand_at(0x4589DD) & 0xFF;
        py = xi(r).fmul_m(f32::from_bits(ANGLE_STEP)).fcos().fmul_m(f32::from_bits(SCATTER)).fadd_m(py).to_f32();
        let mut found = 0u8;
        let z = find_ground_z_for_3d_coord(px, py, pz, GPtr::from_raw(&raw mut found), GPtr::NULL);
        pz = X87::from_f64_exact(z).fadd(x(f32::from_bits(LIFT))).to_f32();
        if found != 0 {
            let r = rand_at(0x458A3B) & 3;
            let model = u32::from(g::<u16>(MI_MONEY).read());
            generate_new_one(px, py, pz, model, 8, r.wrapping_add(each) as u32, 0, 0, 0);
        }
        left -= 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CREATE_SOME_MONEY];
