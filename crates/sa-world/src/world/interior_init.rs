//! PORT-VEH-3 (batch `world3-3`, SA:MP-executed rows): `Interior_c::Init`. Ported from the 1.0 US
//! bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, wr, x, xi};

orig! {
    /// `Interior_c::CalcMatrix(const CVector*)` (0x5914D0)
    fn interior_calc_matrix(i: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x5914D0 => super::interior_info::interior_calc_matrix;
    /// `Interior_c::ResetTiles()` (0x593910)
    fn interior_reset_tiles(i: GPtr<u8>) = thiscall 0x593910 => unported;
    /// `InteriorGroup_c::GetEntity()` (0x594BD0)
    fn ig_get_entity(ig: GPtr<u8>) -> GPtr<u8> = thiscall 0x594BD0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(p: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `Interior_c::FurnishShop(int32)` (0x59A790)
    fn interior_furnish_shop(i: GPtr<u8>, kind: u32) = thiscall 0x59A790 => unported;
    /// `Interior_c::FurnishOffice()` (0x599AF0)
    fn interior_furnish_office(i: GPtr<u8>) = thiscall 0x599AF0 => unported;
    /// `Interior_c::FurnishLounge()` (0x597740)
    fn interior_furnish_lounge(i: GPtr<u8>) = thiscall 0x597740 => unported;
    /// `Interior_c::FurnishBedroom()` (0x593FC0)
    fn interior_furnish_bedroom(i: GPtr<u8>) = thiscall 0x593FC0 => unported;
    /// `Interior_c::FurnishKitchen()` (0x5970B0)
    fn interior_furnish_kitchen(i: GPtr<u8>) = thiscall 0x5970B0 => unported;
    /// `Interior_c::CalcExitPts()` (0x5924A0)
    fn interior_calc_exit_pts(i: GPtr<u8>) = thiscall 0x5924A0 => unported;
    /// `InteriorManager_c::HasInteriorHadStealDataSetup(Interior_c*)` (0x5982B0) on `g_interiorMan` (0xBAF670)
    fn interior_manager_has_steal_data(m: GPtr<u8>, i: GPtr<u8>) -> u8 = thiscall 0x5982B0 => unported;
    /// `Interior_c::AddPickups()` (0x591F90)
    fn interior_add_pickups(i: GPtr<u8>) = thiscall 0x591F90 => unported;
}

/// `Interior_c::Init(const CVector* pos)` (0x593BF0): the interior matrix and tiles; the group
/// entity gets a matrix if it has none and a copy of it is taken; unless the info type (+0x14 -> +0)
/// is 0x63 the random seed becomes `ftol(a.z) * ftol(a.y) * ftol(a.x) + ftol(p.z) * ftol(p.y) *
/// ftol(p.x) + seed byte (+0x1E)` with `a` the vector at 0xBB3DAC (+8/+0x14/+0x18), or without it
/// `ftol(p.z * p.y * p.x + seed byte)` (`p` = the entity matrix position); the goto/info counts are
/// cleared and the interior furnished by type (0 shop, 1 office, 2 lounge, 3 bedroom, 4 kitchen);
/// exit points; interiors without steal data are listed (0xBB3918, up to 64); lounges and bedrooms
/// get pickups. Returns true.
#[hook(addr = 0x593BF0, conv = thiscall, name = "Interior_c::Init", this = "Interior_c",
       expect = "64 A1 00 00 00 00",
       unverified = "world3-3", route = "L2", hazards = "heap-writes",
       note = "direct srand at 0x593CDB (return 0x593CE0); furnishing creates objects; the SEH frame is not reproduced")]
pub fn interior_init(this: GPtr<u8>, pos: GPtr<u8>) -> u32 {
    interior_calc_matrix(this, pos);
    interior_reset_tiles(this);
    let ent = ig_get_entity(rd::<GPtr<u8>>(this, 0xC));
    if rd::<u32>(ent, 0x14) == 0 {
        placeable_allocate_matrix(ent);
        simple_transform_update_matrix(ent.byte_add(4), rd::<GPtr<u8>>(ent, 0x14));
    }
    let mut buf = [0u32; 0x12];
    let m: GPtr<u8> = GPtr::from_raw(&mut buf as *mut [u32; 0x12]).cast();
    matrix_copy_ctor(m, rd::<GPtr<u8>>(ent, 0x14));
    let info = rd::<GPtr<u8>>(this, 0x14);
    if rd::<u8>(info, 0) != 0x63 {
        let p = [rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)];
        let seed_byte = i32::from(rd::<u8>(info, 0x1E));
        let a = g::<GPtr<u8>>(0xBB_3DAC).read();
        let seed = if a.is_null() {
            ftol32(x(p[2]).fmul_m(p[1]).fmul_m(p[0]).fadd(xi(seed_byte)))
        } else {
            let s1 = ftol32(x(rd::<f32>(a, 0x18)))
                .wrapping_mul(ftol32(x(rd::<f32>(a, 0x14))))
                .wrapping_mul(ftol32(x(rd::<f32>(a, 0x8))));
            let s2 = ftol32(x(p[2])).wrapping_mul(ftol32(x(p[1]))).wrapping_mul(ftol32(x(p[0])));
            s1.wrapping_add(s2).wrapping_add(seed_byte)
        };
        sa_core::orig::at_site(0x59_3CE0, || sa_core::crt::srand(seed as u32));
    }
    wr::<u8>(this, 0x40C, 0);
    wr::<u8>(this, 0x40D, 0);
    match rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0) {
        0 => interior_furnish_shop(this, 0),
        1 => interior_furnish_office(this),
        2 => interior_furnish_lounge(this),
        3 => interior_furnish_bedroom(this),
        4 => interior_furnish_kitchen(this),
        _ => {}
    }
    interior_calc_exit_pts(this);
    if interior_manager_has_steal_data(GPtr::from_addr(0xBA_F670), this) == 0 {
        let n = g::<i32>(0xBB_3914).read();
        if n < 0x40 {
            g::<u32>(0xBB_3918 + 4 * n as usize).write(rd::<u32>(this, 8));
            g::<i32>(0xBB_3914).write(n + 1);
        }
    }
    let t = rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0);
    if t == 2 || t == 3 {
        interior_add_pickups(this);
    }
    matrix_dtor(m);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INTERIOR_INIT];
