//! FRONTIER-2 turn 10, lane A (batch `frontier2-a14`): small WORLD ports from the 1.0 US bytes (gta-reversed read as
//! an aid; where it differs the bytes win).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use sa_core::bindings::{eh_vector_dtor, list_dtor, matrix_mul_vec};
use sa_core::crt::rand_at;
use sa_core::field::ptr_of as p;
use sa_math::fpu::ftol32;

use super::common::{g, rd, x, xi};
use crate::bindings::{fx_add_particle, fx_prt_mult_ctor};

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;
/// `g_fx.m_SmokeII3expand` (0xA9AE24)
const FX_SMOKE_II3_EXPAND: usize = 0xA9_AE24;
/// 0x858C28 1/20, 0x858C7C 1/RAND_MAX
const F_20TH: f32 = f32::from_bits(0x3D4C_CCCD);
const F_RAND: f32 = f32::from_bits(0x3800_0100);

/// `CObject::DoBurnEffect()` (0x59FB50, thiscall). The model's bounding box (col model +0 min, +0xC max): `n =
/// ftol(sx * sy * sz * burn damage (+0x178) / 20)` (x87, sx unrounded); n times a point `min + rand / RAND_MAX *
/// size` per axis (`rand` x, y, z, f32 each), through the object matrix (`CMatrix * CVector`), gets a
/// `g_fx.m_SmokeII3expand` particle with velocity (0, 0, 0.02), time 0, `FxPrtMult_c(0, 0, 0, 1, 1, 1, 0.4)`, -1,
/// 1.2, 0.6, 0. The original first default-constructs a stack `FxPrtMult_c` (0x4AB270) that the copy of the full
/// one overwrites: not reproduced (a dead stack temporary).
#[hook(addr = 0x59FB50, conv = thiscall, name = "CObject::DoBurnEffect", this = "CObject",
       expect = "83 EC 78 53 8B D9",
       unverified = "frontier2-a14", route = "L2", hazards = "heap-writes",
       note = "particles; rand sites 0x59FBE5 / 0x59FC04 / 0x59FC23")]
pub fn object_do_burn_effect(this: GPtr<u8>) {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(this, 0x22) as usize).read();
    let bb = rd::<GPtr<u8>>(mi, 0x14);
    let mn = [rd::<f32>(bb, 0), rd::<f32>(bb, 4), rd::<f32>(bb, 8)];
    let mx = [rd::<f32>(bb, 0xC), rd::<f32>(bb, 0x10), rd::<f32>(bb, 0x14)];
    let sz = x(mx[2]).fsub_m(mn[2]).to_f32();
    let sy = x(mx[1]).fsub_m(mn[1]).to_f32();
    let sx80 = x(mx[0]).fsub_m(mn[0]);
    let sx = sx80.to_f32();
    let n = ftol32(sx80.fmul_m(sy).fmul_m(sz).fmul_m(rd::<f32>(this, 0x178)).fmul_m(F_20TH));
    for _ in 0..n.max(0) {
        let px = xi(rand_at(0x59FBE5)).fmul_m(F_RAND).fmul_m(sx).fadd_m(mn[0]).to_f32();
        let py = xi(rand_at(0x59FC04)).fmul_m(F_RAND).fmul_m(sy).fadd_m(mn[1]).to_f32();
        let pz = xi(rand_at(0x59FC23)).fmul_m(F_RAND).fmul_m(sz).fadd_m(mn[2]).to_f32();
        let mut pt = [px, py, pz];
        let mut pos = [0f32; 3];
        matrix_mul_vec(p(&mut pos), rd::<GPtr<u8>>(this, 0x14), p(&mut pt));
        let mut mult = [0u32; 7];
        fx_prt_mult_ctor(p(&mut mult), 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, f32::from_bits(0x3ECC_CCCD));
        let mut vel = [0.0f32, 0.0, f32::from_bits(0x3CA3_D70A)];
        fx_add_particle(
            g::<GPtr<u8>>(FX_SMOKE_II3_EXPAND).read(),
            p(&mut pos),
            p(&mut vel),
            0,
            p(&mut mult),
            0xBF80_0000,
            f32::from_bits(0x3F99_999A),
            f32::from_bits(0x3F19_999A),
            0,
        );
    }
}

/// `InteriorManager_c::~InteriorManager_c()` (0x598580, thiscall): the member lists and arrays in reverse order: the
/// lists at +0x4298 and +0x428C, the 8 interior groups at +0x3CAC (0xBC each, `InteriorGroup_c::~InteriorGroup_c`
/// 0x597FF0), the list at +0x3CA0, the 8 interiors at +0 (0x794 each, `Interior_c::~Interior_c` 0x591360).
#[hook(addr = 0x598580, conv = thiscall, name = "InteriorManager_c::~InteriorManager_c", this = "InteriorManager_c",
       expect = "6A FF 68 8D D1 83 00",
       unverified = "frontier2-a14", route = "L2", hazards = "heap-writes",
       note = "only at exit (the global g_interiorMan's atexit destructor)")]
pub fn interior_manager_dtor(this: GPtr<u8>) {
    list_dtor(this.byte_add(0x4298));
    list_dtor(this.byte_add(0x428C));
    eh_vector_dtor(this.byte_add(0x3CAC), 0xBC, 8, 0x597FF0);
    list_dtor(this.byte_add(0x3CA0));
    eh_vector_dtor(this, 0x794, 8, 0x591360);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INTERIOR_MANAGER_DTOR, HOOK_OBJECT_DO_BURN_EFFECT];
