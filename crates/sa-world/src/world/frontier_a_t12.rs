//! FRONTIER-2 turn 12, lane A (batches `frontier2-a19`..): small WORLD ports from the 1.0 US bytes (gta-reversed read
//! as an aid; where it differs the bytes win).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use sa_core::bindings::{cross_product, entity_update_rw_frame, matrix_mul_vec, vector_normalise};
use sa_core::field::ptr_of as p;
use sa_math::fpu::{le_or_nan, lt};

use super::common::{entity_pos, f32_of, g, rd, wr, x};
use super::entity::get_matrix;
use crate::bindings::{update_rw_matrix, world_process_vertical_line};
use sa_stream::bindings::model_info_sways_in_wind;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;
/// `CWeather::WindDir`
const WIND_DIR: usize = 0xC8_13E0;

/// `CEntity::BuildWindSockMatrix()` (0x532DB0, thiscall): forward = normalised `(-(wind.x + 0.01), -(wind.y + 0.01),
/// 0.1)` (the bytes negate both sums, gta-reversed does not), right = normalised `(0, 0, 1) x forward`, up =
/// `forward x right`; the nine axis floats are written through their own inlined `GetMatrix()`, then the inlined
/// `UpdateRwMatrix` / `UpdateRwFrame`.
#[hook(addr = 0x532DB0, conv = thiscall, name = "CEntity::BuildWindSockMatrix", this = "CEntity",
       expect = "83 EC 30 A1 E0 13 C8 00", unverified = "frontier2-a19", route = "L2", hazards = "heap-writes",
       note = "wind sock objects (CEntity::PreRender)")]
pub fn entity_build_wind_sock_matrix(this: GPtr<u8>) {
    let w = g::<[f32; 3]>(WIND_DIR).read();
    let mut fwd = [f32_of(x(w[0]).fadd_m(0.01).fchs()), f32_of(x(w[1]).fadd_m(0.01).fchs()), 0.1f32];
    vector_normalise(GPtr::from_raw(&mut fwd));
    let mut up0 = [0.0f32, 0.0, 1.0];
    let mut out = [0.0f32; 3];
    let mut right = cross_product(p(&mut out), p(&mut up0), p(&mut fwd)).cast::<[f32; 3]>().read();
    vector_normalise(GPtr::from_raw(&mut right));
    let up = cross_product(p(&mut out), p(&mut fwd), p(&mut right)).cast::<[f32; 3]>().read();
    for (o, v) in [(0, right), (0x10, fwd), (0x20, up)] {
        for (i, f) in v.into_iter().enumerate() {
            wr::<f32>(get_matrix(this), o + 4 * i as isize, f);
        }
    }
    super::mvpd_w1::update_rw_matrix_and_frame(this);
}

/// The model index cells (`ModelIndices::MI_*`, u16) of `RepositionOneObject`'s first list (parking meters, phone
/// booths, bins, post boxes, news stands, traffic cones, road work barriers, bus / no-parking / phone signs, fire
/// hydrants, bollard lights, park tables and benches, telegraph poles, barriers), in the bytes' compare order.
const SET_ON_GROUND: [usize; 21] = [
    0x8CD548, 0x8CD560, 0x8CD564, 0x8CD568, 0x8CD56C, 0x8CD570, 0x8CD574, 0x8CD578, 0x8CD57C, 0x8CD580, 0x8CD584,
    0x8CD588, 0x8CD58C, 0x8CD590, 0x8CD594, 0x8CD5FC, 0x8CD60C, 0x8CD54C, 0x8CD620, 0x8CD628, 0x8CD618,
];
/// The second list (street lights, traffic lights, lamp posts): grounded at their collision's lowest point.
const SET_ON_GROUND_AT_COL: [usize; 12] = [
    0x8CD518, 0x8CD51C, 0x8CD520, 0x8CD4FC, 0x8CD500, 0x8CD504, 0x8CD508, 0x8CD50C, 0x8CD510, 0x8CD614, 0x8CD528,
    0x8CD52C,
];
/// `MI_BUOY`
const MI_BUOY: usize = 0x8CD608;

/// `movsx` of the entity's model index against `movzx` of a model index cell.
fn model_is(model: i32, cell: usize) -> bool {
    model == i32::from(g::<u16>(cell).read())
}

/// The vertical line `RepositionOneObject` casts three times: `ProcessVerticalLine(point, -1000, colPoint, entity,
/// buildings only)`; returns the hit point's z when it hits. The entity output lands in the original's reused
/// argument slot, never read.
fn ground_hit(point: [f32; 3]) -> Option<f32> {
    let mut pt = point;
    let mut cp = [0u32; 0x2C / 4];
    let mut ent = GPtr::<u8>::NULL;
    (world_process_vertical_line(p(&mut pt), -1000.0, p(&mut cp), p(&mut ent), 1, 0, 0, 0, 0, 0, GPtr::NULL) != 0)
        .then(|| f32::from_bits(cp[2]))
}

/// `CWorld::RepositionOneObject(CEntity*)` (0x569850, cdecl). The model info's collision model `cm` (+0x14), its box
/// height `h = max(f32(max.z - min.z), 2)`:
/// - a model of the first list, or one that sways in the wind (`SwaysInWind` after the list): z on the ground below
///   `(pos.x, pos.y, pos.z + h)` minus `cm.min.z` (0 for the hit z without a hit), the RW matrix / frame updated;
/// - else a model of the second list with collision data: the point is the single box's `((min.x + max.x) / 2,
///   (min.y + max.y) / 2, min.z)` (the bytes take the box's min z, not its centre as gta-reversed), or the lowest
///   sphere centre below 1000, transformed by the matrix; without a box or a sphere below 1000 the position. z as
///   above from `(point.x, point.y, pos.z + h)` (through `CEntity::UpdateRwMatrix`). Without collision data nothing
///   (gta-reversed grounds it at its position);
/// - then for the buoy: a vertical line from `pos + (0, 0, 2)` whose result is unused, and `z = (6 - H / 2) + H / 5`
///   with the unrounded box height H.
#[hook(addr = 0x569850, conv = cdecl, name = "CWorld::RepositionOneObject", expect = "0F B7 05 48 D5 8C 00 83",
       unverified = "frontier2-a19", route = "L2", hazards = "heap-writes",
       note = "IPL object placement (CWorld::RepositionCertainDynamicObjects, CObject::Init of map objects)")]
pub fn world_reposition_one_object(obj: GPtr<u8>) {
    let model = i32::from(rd::<i16>(obj, 0x22));
    let mi_of = |m: i32| g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * m as usize).read();
    let cm_of = || rd::<GPtr<u8>>(mi_of(i32::from(rd::<i16>(obj, 0x22))), 0x14);
    let height = |cm: GPtr<u8>| {
        let h = f32_of(x(rd::<f32>(cm, 0x14)).fsub_m(rd::<f32>(cm, 8)));
        if le_or_nan(x(2.0), x(h)) { h } else { 2.0 }
    };
    let mi = mi_of(model);
    if SET_ON_GROUND.iter().any(|&c| model_is(model, c)) || model_info_sways_in_wind(mi) != 0 {
        let h = height(rd::<GPtr<u8>>(mi, 0x14));
        let m = get_matrix(obj);
        let z = ground_hit([rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), f32_of(x(h).fadd_m(rd::<f32>(m, 0x38)))]);
        let z = f32_of(x(z.unwrap_or(0.0)).fsub_m(rd::<f32>(cm_of(), 8)));
        wr::<f32>(get_matrix(obj), 0x38, z);
        super::mvpd_w1::update_rw_matrix_and_frame(obj);
    } else if SET_ON_GROUND_AT_COL.iter().any(|&c| model_is(model, c)) {
        let cm = rd::<GPtr<u8>>(mi, 0x14);
        let h = height(cm);
        let pos = entity_pos(obj);
        let mut pt = [rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)];
        let cd = rd::<GPtr<u8>>(cm, 0x2C);
        if !cd.is_null() {
            let mut transform = false;
            if rd::<i16>(cd, 2) == 1 {
                let b = rd::<GPtr<u8>>(cd, 0xC);
                pt[0] = f32_of(x(rd::<f32>(b, 0xC)).fadd_m(rd::<f32>(b, 0)).fmul_m(0.5)).to_bits();
                pt[1] = f32_of(x(rd::<f32>(b, 0x10)).fadd_m(rd::<f32>(b, 4)).fmul_m(0.5)).to_bits();
                pt[2] = rd::<u32>(b, 8);
                transform = true;
            } else if rd::<i16>(cd, 0) > 0 {
                pt[2] = 1000.0f32.to_bits();
                let mut i = 0;
                while i < i32::from(rd::<i16>(cd, 0)) {
                    let sp = rd::<GPtr<u8>>(cd, 8).byte_add(i as isize * 0x14);
                    if lt(x(rd::<f32>(sp, 8)), x(f32::from_bits(pt[2]))) {
                        pt = [rd::<u32>(sp, 0), rd::<u32>(sp, 4), rd::<u32>(sp, 8)];
                    }
                    i += 1;
                }
                transform = lt(x(f32::from_bits(pt[2])), x(1000.0));
            }
            if transform {
                let mut out = [0u32; 3];
                pt = matrix_mul_vec(p(&mut out), get_matrix(obj), p(&mut pt)).cast::<[u32; 3]>().read();
            }
            let top = f32_of(x(h).fadd_m(rd::<f32>(get_matrix(obj), 0x38)));
            let z = ground_hit([f32::from_bits(pt[0]), f32::from_bits(pt[1]), top]);
            let z = f32_of(x(z.unwrap_or(0.0)).fsub_m(rd::<f32>(cm_of(), 8)));
            wr::<f32>(get_matrix(obj), 0x38, z);
            update_rw_matrix(obj);
            entity_update_rw_frame(obj);
        }
    }
    if model_is(i32::from(rd::<i16>(obj, 0x22)), MI_BUOY) {
        let m = get_matrix(obj);
        ground_hit([rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), f32_of(x(rd::<f32>(m, 0x38)).fadd_m(2.0))]);
        let cm = cm_of();
        let hh = x(rd::<f32>(cm, 0x14)).fsub_m(rd::<f32>(cm, 8));
        let z = x(0.5).fmul(hh).fsubr_m(6.0).fadd(hh.fmul_m(0.2));
        wr::<f32>(get_matrix(obj), 0x38, f32_of(z));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ENTITY_BUILD_WIND_SOCK_MATRIX, HOOK_WORLD_REPOSITION_ONE_OBJECT];
