//! FRONTIER-2 turn 10, lane A (batch `frontier2-a14`): small WORLD ports from the 1.0 US bytes (gta-reversed read as
//! an aid; where it differs the bytes win).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_core::mem::GPtr;

use sa_core::bindings::{automobile_ctor, eh_vector_dtor, list_dtor, matrix_mul_vec, vector_normalise};
use sa_core::crt::rand_at;
use sa_core::field::ptr_of as p;
use sa_math::fpu::ftol32;

use super::common::{g, rd, x, xi};
use crate::bindings::{
    find_objects_in_range, fx_add_particle, fx_prt_mult_ctor, placeable_allocate_matrix, tag_manager_get_alpha,
    tag_manager_is_tag, tag_manager_set_alpha,
};
use crate::bindings::{
    find_objects_kinda_colliding, vehicle_change_law_enforcer_state, vehicle_new, world_add, world_process_vertical_line,
};
use sa_stream::bindings::{get_default_cop_car_model, simple_transform_update_matrix};

orig! {
    /// `CVehicle::GetHeightAboveRoad()` (vtable +0xD4, ST0)
    fn v_get_height_above_road(v: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

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

/// `CWorld::SprayPaintWorld(CVector& pos, CVector& outDir, float radius, bool processTagAlpha)` (0x565B70, cdecl):
/// up to 15 buildings within `radius` (3D, `FindObjectsInRange`); for every tag among them `outDir` = its matrix's
/// forward row (the matrix allocated from the placement first when missing) and, with `processTagAlpha`, its alpha
/// raised by the short at 0x8CDEF0 (at most 255); the new alpha (a register the bytes never set without
/// `processTagAlpha`: the found count, then the previous tag's value) is written back. 2 when a tag reached 255 now,
/// else 1 when a tag was found, else 0.
#[hook(addr = 0x565B70, conv = cdecl, name = "CWorld::SprayPaintWorld", expect = "83 EC 4C 8B 54 24 58",
       unverified = "frontier2-a14", route = "L2", hazards = "heap-writes",
       note = "the spray can on gang tags; without processTagAlpha the alpha written is EDI's leftover (modelled)")]
pub fn world_spray_paint_world(pos: GPtr<u8>, out_dir: GPtr<u32>, radius: f32, process: u8) -> u32 {
    let mut objs = [GPtr::<u8>::NULL; 15];
    let mut count: i16 = 0;
    find_objects_in_range(pos, radius, 0, p(&mut count), 15, p(&mut objs), 1, 0, 0, 0, 0);
    if count <= 0 {
        return 0;
    }
    let (mut found, mut reached) = (false, false);
    let mut edi = u32::from(count as u16);
    for &e in objs.iter().take(count as u16 as usize) {
        if tag_manager_is_tag(e) == 0 {
            continue;
        }
        found = true;
        if rd::<GPtr<u8>>(e, 0x14).is_null() {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let f = rd::<GPtr<u8>>(e, 0x14).byte_add(0x10);
        for k in 0..3 {
            out_dir.wrapping_index(k).write(rd::<u32>(f, 4 * k as isize));
        }
        let a = u16::from(tag_manager_get_alpha(e));
        if process != 0 {
            let v = i32::from(g::<i16>(0x8C_DEF0).read()) + i32::from(a as i16);
            edi = if v > 0xFF { 0xFF } else { v as u32 };
        }
        if (a as i16) < 0xFF && edi as u16 == 0xFF {
            reached = true;
        }
        tag_manager_set_alpha(e, edi as u8);
    }
    if reached {
        2
    } else {
        u32::from(found)
    }
}

/// `CSetPiece::TryToGenerateCopCar(CVector2D posn, CVector2D target)` (0x4998A0, thiscall, `ret 0x10`; `this` unused).
/// No car without a default cop car model (`GetDefaultCopCarModel(1)` < 0). A new `CAutomobile(model, 1, true)` (a
/// failed allocation leaves a null car that the next steps dereference, as the bytes) placed at `(posn, 1000)`, or on
/// the ground of a vertical line down to -1000 (buildings and dummies: `GetHeightAboveRoad() + hit z`), facing the
/// normalised `(target - posn, 0)` (right `(d.y, -d.x, 0)`, up z). With nothing (vehicles, peds) within the model's
/// bounding radius it turns law enforcer, joins the world and is returned; otherwise it is deleted and 0 returned.
/// The vertical line's entity output slot is not initialised by the original (written by the callee only).
#[hook(addr = 0x4998A0, conv = thiscall, name = "CSetPiece::TryToGenerateCopCar", this = "CSetPiece",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a14", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "set-piece cop cars (CSetPiece::Update, wanted level road blocks); GetHeightAboveRoad is ST0")]
pub fn set_piece_try_to_generate_cop_car(_this: GPtr<u8>, sx: f32, sy: f32, tx: f32, ty: f32) -> GPtr<u8> {
    let model = get_default_cop_car_model(1);
    if model < 0 {
        return GPtr::NULL;
    }
    let mem = vehicle_new(0x988);
    let car = if mem.is_null() { GPtr::NULL } else { automobile_ctor(mem, model as u32, 1, 1) };
    let mut point = [sx, sy, 1000.0f32];
    let mut cp = [0u32; 0x2C / 4];
    let mut ent = GPtr::<u8>::NULL;
    if world_process_vertical_line(p(&mut point), -1000.0, p(&mut cp), p(&mut ent), 1, 0, 0, 0, 1, 0, GPtr::NULL) != 0 {
        point[2] = sa_math::fpu::st0(v_get_height_above_road(car)).fadd_m(f32::from_bits(cp[2])).to_f32();
    }
    let mut d = [x(tx).fsub_m(sx).to_f32(), x(ty).fsub_m(sy).to_f32(), 0.0f32];
    vector_normalise(GPtr::from_raw(&mut d));
    let m = rd::<GPtr<u8>>(car, 0x14);
    let w = |o: isize, v: f32| sa_core::field::wr::<f32>(m, o, v);
    w(0x10, d[0]);
    w(0x14, d[1]);
    w(0x18, 0.0);
    w(0, d[1]);
    w(4, x(d[0]).fchs().to_f32());
    w(8, 0.0);
    w(0x20, 0.0);
    w(0x24, 0.0);
    w(0x28, 1.0);
    w(0x30, point[0]);
    w(0x34, point[1]);
    w(0x38, point[2]);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(car, 0x22) as usize).read();
    let radius = rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24);
    let mut count: i16 = 0;
    find_objects_kinda_colliding(GPtr::from_raw(point.as_mut_ptr()), radius, 0, GPtr::from_raw(&mut count), 16, GPtr::NULL, 0, 1, 1, 0, 0);
    if count != 0 {
        v_delete(car, 1);
        return GPtr::NULL;
    }
    vehicle_change_law_enforcer_state(car, 1);
    world_add(car);
    car
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTERIOR_MANAGER_DTOR,
    HOOK_OBJECT_DO_BURN_EFFECT,
    HOOK_WORLD_SPRAY_PAINT_WORLD,
    HOOK_SET_PIECE_TRY_TO_GENERATE_COP_CAR,
];
