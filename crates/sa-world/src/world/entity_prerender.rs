//! `CEntity::PreRender` (WRITER-MVP-F batch `mvpf-ent-1`): the per-frame pre-render pass of a visible
//! entity (model-info first-use set-up, tree sway, cranes, pickup / missile / flare / streak effects,
//! traffic-light poles). Ported from the 1.0 US bytes (the SecuROM fragments at 0x53604B and 0x40A0BF
//! inlined; gta-reversed `Entity.cpp` a reading aid only). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{cam_pos, ftol32, g, rd, wr, x, xi};
use crate::collision::mvpc_coll1::local;
use crate::physical::shared::pos_ptr;

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CObject::fDistToNearestTree` (0x8D0A20, f32)
const DIST_TO_NEAREST_TREE: usize = 0x8D0A20;
/// `TheCamera` + 0x974 (0xB6F99C): the camera vector the motion-blur streaks are offset by
const CAM_VEC_B6F99C: u32 = 0xB6F99C;
/// byte 0xA43088: missile effects suppressed when 1
const NO_MISSILE_FX: usize = 0xA43088;
/// int16 at 0xB7C4E8: the shadow intensity of the 0x8CD638 model's shadow (low byte used as r/g/b)
const SHADOW_STRENGTH: usize = 0xB7C4E8;
/// shadow textures (`RwTexture*`) at 0xC403F4 (spot light) / 0xC403E4 (car-like) and the corona
/// texture at 0xC3E000
const TEX_SHADOW_LIGHT: usize = 0xC403F4;
const TEX_SHADOW_CAR: usize = 0xC403E4;
const TEX_CORONA: usize = 0xC3E000;

// `.rdata` constants (bits)
const K_1_16: u32 = 0x3D800000; // 0x858620
const K_120: u32 = 0x42F00000; // 0x858BB0
const K_160: u32 = 0x43200000; // 0x858970
const K_200: u32 = 0x43480000; // 0x858A48
const K_0_6: u32 = 0x3F19999A; // 0x858CC8
const K_0_8: u32 = 0x3F4CCCCD; // 0x858C98
const K_6: u32 = 0x40C00000; // 0x858B44
const K_190: u32 = 0x433E0000; // 0x858A20
const K_220: u32 = 0x435C0000; // 0x863994
const K_255: u32 = 0x437F0000; // 0x859AAC
const K_0_03: u32 = 0x3CF5C28F; // 0x858B10
const K_50: u32 = 0x42480000; // 0x858B40
const K_110: u32 = 0x42DC0000; // 0x858BA8
/// immediates
const F_0_07: u32 = 0x3D8F5C29;
const F_1: u32 = 0x3F800000;
const F_20: u32 = 0x41A00000;
const F_8: u32 = 0x41000000;
const F_M8: u32 = 0xC1000000;
const F_32: u32 = 0x42000000;
const F_15: u32 = 0x41700000;
const F_1_5: u32 = 0x3FC00000;
const F_300: u32 = 0x43960000;
const F_0_5: u32 = 0x3F000000;
const F_0_4: u32 = 0x3ECCCCCD;
const F_M0_4: u32 = 0xBECCCCCD;
const F_2: u32 = 0x40000000;
const F_M2: u32 = 0xC0000000;
const F_16: u32 = 0x41800000;

use crate::bindings::get_water_level_no_waves;
use crate::bindings::update_rw_matrix;
use sa_core::bindings::rp_geometry_for_all_materials;
use sa_rwcore::bindings::rw_frame_update_objects;
use sa_core::bindings::vector_magnitude;
use sa_stream::bindings::is_object_glass;
use crate::bindings::dn_pre_render_update as dn_pre_render_update_atomic;
use crate::bindings::entity_process_lights_for_entity as process_lights_for_entity;
use crate::bindings::vector_add;
use crate::bindings::vector_mul_scalar as vector_mul;
use crate::bindings::entity_update_rw_frame as update_rw_frame;
use crate::bindings::vector_sub;
use crate::bindings::add_light;
use crate::bindings::register_corona_6fc180 as register_corona;
use crate::bindings::store_shadow_to_be_rendered as store_shadow;
use crate::bindings::max_f;
orig! {
    /// `CBaseModelInfo::AsAtomicModelInfoPtr()` (vtable slot 4)
    fn mi_as_atomic(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CBaseModelInfo::GetModelType()` (vtable slot 0x10, AL)
    fn mi_get_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `RpMatFXAtomicQueryEffects(RpAtomic*)` (0x811c30)
    fn rp_matfx_atomic_query_effects(a: GPtr<u8>) -> u32 = cdecl 0x811c30 => unported;
    /// `CCustomBuildingDNPipeline::PreRenderUpdate(RpClump*, bool)` (0x5d72c0)
    fn dn_pre_render_update_clump(c: GPtr<u8>, b: u32) = cdecl 0x5d72c0 => unported;
    /// `CEntity::ModifyMatrixForTreeInWind()` (0x534e90)
    fn modify_matrix_for_tree_in_wind(e: GPtr<u8>) = thiscall 0x534e90 => unported;
    /// `CEntity::ModifyMatrixForCrane()` (0x533170)
    fn modify_matrix_for_crane(e: GPtr<u8>) = thiscall 0x533170 => super::entity::modify_matrix_for_crane;
    /// `CPickups::DoCollectableEffects(CEntity*)` (0x455e20)
    fn do_collectable_effects(e: GPtr<u8>) = cdecl 0x455e20 => unported;
    /// `CPickups::DoMoneyEffects(CEntity*)` (0x454e80)
    fn do_money_effects(e: GPtr<u8>) = cdecl 0x454e80 => unported;
    /// `CPickups::DoMineEffects(CEntity*)` (0x4560e0)
    fn do_mine_effects(e: GPtr<u8>) = cdecl 0x4560e0 => unported;
    /// `CPickups::DoPickUpEffects(CEntity*)` (0x455720)
    fn do_pick_up_effects(e: GPtr<u8>) = cdecl 0x455720 => unported;
    /// `CEntity::PreRenderForGlassWindow()` (0x533240)
    fn pre_render_for_glass_window(e: GPtr<u8>) = thiscall 0x533240 => super::entity::pre_render_for_glass_window;
    /// `CEntity::BuildWindSockMatrix()` (0x532db0)
    fn build_wind_sock_matrix(e: GPtr<u8>) = thiscall 0x532db0 => unported;
    /// `CTrafficLights::DisplayActualLight(CEntity*)` (0x49dab0)
    fn display_actual_light(e: GPtr<u8>) = cdecl 0x49dab0 => unported;
    /// `CShadows::StoreShadowForPole(CEntity*, float x, float y, float z, float height, float width,
    /// uint32 localId)` (0x70c750)
    #[allow(clippy::too_many_arguments)]
    fn store_shadow_for_pole(e: GPtr<u8>, ox: u32, oy: u32, oz: u32, h: u32, w: u32, id: u32) = cdecl 0x70c750 => unported;
    /// `CMotionBlurStreaks::RegisterStreak(uint32 id, uint8 r, g, b, a, CVector left, CVector right)`
    /// (0x721dc0; both vectors by value)
    #[allow(clippy::too_many_arguments)]
    fn register_streak(id: GPtr<u8>, r: u32, gr: u32, b: u32, a: u32, lx: u32, ly: u32, lz: u32, rx: u32, ry: u32,
        rz: u32) = cdecl 0x721dc0 => unported;
}

fn mi_of(e: GPtr<u8>) -> GPtr<u8> {
    g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(e, 0x22) as isize as usize).read()
}

/// `movzx` of a model-index global compared with the `movsx` of the entity's model index.
fn is_mi(m: i32, addr: usize) -> bool {
    m == i32::from(g::<u16>(addr).read())
}

use sa_math::fpu::fstp_bits as bits;
use sa_core::field::rd3u as v3;

/// The camera position the original reads: `[0xB6F03C] + 0x30`, else 0xB6F02C.
fn cam() -> GPtr<u8> {
    cam_pos().cast()
}

/// `CEntity::PreRender()` (0x535FA0, thiscall). `mi = ms_modelInfoPtrs[+0x22]`, `ami =
/// mi->AsAtomicModelInfoPtr()`; `mi+0xD` (2dfx count) != 0 → `ProcessLightsForEntity`. First use of the
/// model info (`mi+0x12` bit 0 clear): set it; `ami` with an atomic (+0x1C) that has MatFX effects and a
/// geometry (+0x18) → `RpGeometryForAllMaterials(geo, 0x532D70, 0)`; alpha `mi+0xC` += 0x10 (0xFF from
/// 0xEF up); `DNPipeline::PreRenderUpdate(atomic, 0)` for `ami`, else for clump models (type 5) of the
/// clump (+0x1C). Stops unless entity flag 0x200000 (+0x1C). Swaying trees (`ami+0x12 & 0x7800` 0x800 /
/// 0x1000; not an exploded object, +0x140 bit 6): `fDistToNearestTree` (0x8D0A20) = the 2D camera
/// distance (x87 `fsqrt`) when it is not larger (`!(G < d)`, so NaN is stored), then
/// `ModifyMatrixForTreeInWind`. Then by type (+0x36 & 7): building → `ModifyMatrixForCrane` for cranes
/// (0x4800); object → the model's effect (collectable, money, mines, missile 0x159, flare, glass,
/// pickups, streaks 0x156/0x158, shadows, wind sock), dummies skip that; objects and dummies then the
/// traffic-light models (`DisplayActualLight` and/or `StoreShadowForPole`). `rand()` at 0x536230 /
/// 0x5363EC (return sites 0x536235 / 0x5363F1).
#[hook(addr = 0x535FA0, conv = thiscall, name = "CEntity::PreRender", this = "CEntity", expect = "81 EC 9C 00 00 00 53 55",
       unverified = "mvpf-ent-1", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk,driving",
       note = "fsqrt of the tree distance; writes the model info (+0x12 flag, +0xC alpha; model infos may be heap) and fDistToNearestTree; rand at_site 0x536235 / 0x5363F1")]
pub fn pre_render(this: GPtr<u8>) {
    let mi = mi_of(this);
    let ami = mi_as_atomic(mi);
    if rd::<u8>(mi, 0xD) != 0 {
        process_lights_for_entity(this);
    }
    let fl = rd::<u16>(mi, 0x12);
    if fl & 1 == 0 {
        wr::<u16>(mi, 0x12, fl | 1);
        if !ami.is_null() {
            let atomic = rd::<GPtr<u8>>(ami, 0x1C);
            if !atomic.is_null() && rp_matfx_atomic_query_effects(atomic) != 0 {
                let geo = rd::<GPtr<u8>>(atomic, 0x18);
                if !geo.is_null() {
                    rp_geometry_for_all_materials(geo, 0x532D70, GPtr::NULL);
                }
            }
        }
        let a = rd::<u8>(mi, 0xC);
        wr::<u8>(mi, 0xC, if a < 0xEF { a + 0x10 } else { 0xFF });
        if !ami.is_null() {
            dn_pre_render_update_atomic(rd::<GPtr<u8>>(ami, 0x1C), 0);
        } else if mi_get_model_type(mi) == 5 {
            dn_pre_render_update_clump(rd::<GPtr<u8>>(mi, 0x1C), 0);
        }
    }
    if rd::<u32>(this, 0x1C) & 0x200000 == 0 {
        return;
    }
    if !ami.is_null() {
        let k = rd::<u16>(ami, 0x12) & 0x7800;
        if (k == 0x800 || k == 0x1000) && !(rd::<u8>(this, 0x36) & 7 == 4 && rd::<u8>(this, 0x140) & 0x40 != 0) {
            let (c, p) = (cam(), pos_ptr(this));
            let dy = x(rd::<f32>(c, 4)).fsub_m(rd::<f32>(p, 4));
            let dx = x(rd::<f32>(c, 0)).fsub_m(rd::<f32>(p, 0));
            let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
            let gp = g::<f32>(DIST_TO_NEAREST_TREE);
            if x(gp.read()).fcom(d).test_parity(5) {
                gp.write(d.to_f32());
            }
            modify_matrix_for_tree_in_wind(this);
        }
    }
    match rd::<u8>(this, 0x36) & 7 {
        1 => {
            if !ami.is_null() && rd::<u16>(ami, 0x12) & 0x7800 == 0x4800 {
                modify_matrix_for_crane(this);
            }
            return;
        }
        4 => object_effects(this),
        5 => {}
        _ => return,
    }
    traffic_light_poles(this);
}

/// `UpdateRwMatrix` + `UpdateRwFrame` (0x536966).
fn update_rw(this: GPtr<u8>) {
    update_rw_matrix(this);
    update_rw_frame(this);
}

/// The object-only part of `PreRender` (0x536174..0x536977).
fn object_effects(this: GPtr<u8>) {
    let m = i32::from(rd::<i16>(this, 0x22));
    if is_mi(m, 0x8CD598) {
        do_collectable_effects(this);
        update_rw_matrix(this);
        let rw = rd::<GPtr<u8>>(this, 0x18);
        if !rw.is_null() {
            rw_frame_update_objects(rd::<GPtr<u8>>(rw, 4));
        }
        return;
    }
    if is_mi(m, 0x8CD59C) {
        do_money_effects(this);
        update_rw(this);
        return;
    }
    if is_mi(m, 0x8CD5A0) || is_mi(m, 0x8CD5A4) || is_mi(m, 0x8CD5AC) {
        if rd::<u8>(this, 0x140) & 1 != 0 {
            do_mine_effects(this);
            update_rw(this);
        }
        return;
    }
    if m as u16 == 0x159 {
        if g::<u8>(NO_MISSILE_FX).read() != 1 {
            missile_effects(this);
        }
        return;
    }
    if is_mi(m, 0x8CD75C) {
        flare_effects(this);
        return;
    }
    if is_object_glass(this) != 0 {
        pre_render_for_glass_window(this);
        return;
    }
    if rd::<u8>(this, 0x140) & 1 != 0 {
        do_pick_up_effects(this);
        update_rw(this);
        return;
    }
    let m16 = rd::<u16>(this, 0x22);
    if m16 == 0x156 || m16 == 0x158 {
        streak(this, m16 == 0x158);
        return;
    }
    let m = i32::from(m16 as i16);
    if is_mi(m, 0x8CD638) {
        let mut d = [0u32; 3];
        let r = vector_sub(local(&mut d), cam(), pos_ptr(this));
        // `jp` past unless `|d| < 50`
        if X87::from_f64_exact(vector_magnitude(r.cast::<u32>())).fcom(x(f32::from_bits(K_50))).test_parity(5) {
            return;
        }
        let mut pos = v3(pos_ptr(this));
        let w = g::<u16>(SHADOW_STRENGTH).read();
        let c = u32::from(w & 0xFF);
        // EAX = the position's z dword with its low word replaced by the strength
        let intensity = (pos[2] & 0xFFFF_0000) | u32::from(w);
        store_shadow(1, GPtr::from_addr((g::<u32>(TEX_SHADOW_CAR).read()) as usize), local(&mut pos), F_0_4, 0, 0, F_M0_4, intensity, c, c, c, F_20, 0,
            F_1, GPtr::NULL, 0);
        return;
    }
    if is_mi(m, 0x8CD6E0) || is_mi(m, 0x8CD71C) || is_mi(m, 0x8CD700) || is_mi(m, 0x8CD740) || is_mi(m, 0x8CD6FC) {
        let mut d = [0u32; 3];
        let r = vector_sub(local(&mut d), cam(), pos_ptr(this));
        if X87::from_f64_exact(vector_magnitude(r.cast::<u32>())).fcom(x(f32::from_bits(K_110))).test_parity(5) {
            return;
        }
        let mut pos = v3(pos_ptr(this));
        store_shadow(1, GPtr::from_addr((g::<u32>(TEX_SHADOW_CAR).read()) as usize), local(&mut pos), F_2, 0, 0, F_M2, 0x80, 0x80, 0x80, 0x80,
            K_50, 0, F_1, GPtr::NULL, 0);
        return;
    }
    if is_mi(m, 0x8CD718) {
        build_wind_sock_matrix(this);
    }
}

/// Model 0x159 (0x53620D): a flickering light under the missile: `r = (rand() & 15) * 0.0625` (stored
/// f32), a light-texture shadow (8 x 8, colour `ftol(r*200), ftol(r*160), ftol(r*120)`), a point light
/// (radius 8, colour `r, r*0.8, r*0.6`) and a corona (size `r*6`, colour `ftol(r*255), ftol(r*220),
/// ftol(r*190)`).
fn missile_effects(this: GPtr<u8>) {
    let mut pos = v3(pos_ptr(this));
    let ri = sa_core::orig::at_site(0x536235, sa_core::crt::rand) & 0xF;
    let r = xi(ri).fmul_m(f32::from_bits(K_1_16)).to_f32();
    let b = ftol32(x(r).fmul_m(f32::from_bits(K_120))) as u32;
    let gr = ftol32(x(r).fmul_m(f32::from_bits(K_160))) as u32;
    let rr = ftol32(x(r).fmul_m(f32::from_bits(K_200))) as u32;
    let tex = g::<u32>(TEX_SHADOW_LIGHT).read();
    store_shadow(2, GPtr::from_addr(tex as usize), local(&mut pos), F_8, 0, 0, F_M8, 0xFF, rr, gr, b, F_20, 0, F_1, GPtr::NULL, 0);
    let lb = bits(x(r).fmul_m(f32::from_bits(K_0_6)));
    let lg = bits(x(r).fmul_m(f32::from_bits(K_0_8)));
    add_light(0, f32::from_bits(pos[0]), f32::from_bits(pos[1]), f32::from_bits(pos[2]), f32::from_bits(0), f32::from_bits(0), f32::from_bits(0), f32::from_bits(F_8), f32::from_bits(r.to_bits()), f32::from_bits(lg), f32::from_bits(lb), 0, 1, GPtr::NULL);
    let ctex = g::<u32>(TEX_CORONA).read();
    let size = bits(x(r).fmul_m(f32::from_bits(K_6)));
    let cb = ftol32(x(r).fmul_m(f32::from_bits(K_190))) as u32;
    let cg = ftol32(x(r).fmul_m(f32::from_bits(K_220))) as u32;
    let cr = ftol32(x(r).fmul_m(f32::from_bits(K_255))) as u32;
    register_corona(this.addr() as u32, GPtr::NULL, cr, cg, cb, 0xFF, local(&mut pos), f32::from_bits(size).to_bits(), f32::from_bits(F_300).to_bits(), ctex, 0, 1, 0, 0, f32::from_bits(0).to_bits(), 0, f32::from_bits(F_1_5).to_bits(), 0, f32::from_bits(F_15).to_bits(), 0, 0);
}

/// The flare model (0x8CD75C, 0x5363C4): `m = max((rand() & 15) * 0.0625, 0.5)` (the ST0 of `max`
/// scaled directly for the shadow colour, `fst` to f32 for the rest): a light-texture shadow (8 x 8,
/// grey `ftol(m*200)`), a point light (radius 32, colour m, m, m) and a corona (size `m*6`, grey
/// `ftol(m*255)`).
fn flare_effects(this: GPtr<u8>) {
    let mut pos = v3(pos_ptr(this));
    let ri = sa_core::orig::at_site(0x5363F1, sa_core::crt::rand) & 0xF;
    let a = xi(ri).fmul_m(f32::from_bits(K_1_16)).to_f32();
    let st = X87::from_f64_exact(max_f(a, f32::from_bits(F_0_5)).to_f64());
    let m = st.to_f32();
    let s = ftol32(st.fmul_m(f32::from_bits(K_200))) as u32;
    store_shadow(2, GPtr::from_addr((g::<u32>(TEX_SHADOW_LIGHT).read()) as usize), local(&mut pos), F_8, 0, 0, F_M8, 0xFF, s, s, s, F_20, 0, F_1,
        GPtr::NULL, 0);
    let mb = m.to_bits();
    add_light(0, f32::from_bits(pos[0]), f32::from_bits(pos[1]), f32::from_bits(pos[2]), f32::from_bits(0), f32::from_bits(0), f32::from_bits(0), f32::from_bits(F_32), f32::from_bits(mb), f32::from_bits(mb), f32::from_bits(mb), 0, 1, GPtr::NULL);
    let c = ftol32(x(m).fmul_m(f32::from_bits(K_255))) as u32;
    let ctex = g::<u32>(TEX_CORONA).read();
    let size = bits(x(m).fmul_m(f32::from_bits(K_6)));
    register_corona(this.addr() as u32, GPtr::NULL, c, c, c, 0xFF, local(&mut pos), f32::from_bits(size).to_bits(), f32::from_bits(F_300).to_bits(), ctex, 0, 1, 0, 0, f32::from_bits(0).to_bits(), 0, f32::from_bits(F_1_5).to_bits(), 0, f32::from_bits(F_15).to_bits(), 0, 0);
}

/// Models 0x156 / 0x158 (0x536559 / 0x536639): when the 2D move speed (+0x44/+0x48, x87 `fsqrt`) is
/// above 0.03, a motion-blur streak from `pos - cam*0.07` to `pos + cam*0.07` (`cam` = the vector at
/// 0xB6F99C): colour (100, 100, 100, 255) for 0x156; (255, 160, 100, 255) for 0x158, which also needs
/// the position above the still water level when `GetWaterLevelNoWaves` finds water (`z > level` or
/// unordered).
fn streak(this: GPtr<u8>, wet: bool) {
    let (mut t1, mut t2, mut t3, mut t4) = ([0u32; 3], [0u32; 3], [0u32; 3], [0u32; 3]);
    let s1 = vector_mul(local(&mut t1), GPtr::from_addr(CAM_VEC_B6F99C as usize), f32::from_bits(F_0_07));
    let a = v3(vector_sub(local(&mut t2), pos_ptr(this), s1));
    let s2 = vector_mul(local(&mut t3), GPtr::from_addr(CAM_VEC_B6F99C as usize), f32::from_bits(F_0_07));
    let b = v3(vector_add(local(&mut t4), pos_ptr(this), s2));
    let (vx, vy) = (x(rd::<f32>(this, 0x44)), x(rd::<f32>(this, 0x48)));
    let sp = vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt();
    if sp.fcom(x(f32::from_bits(K_0_03))).test_any(0x41) {
        return;
    }
    if !wet {
        register_streak(this, 0x64, 0x64, 0x64, 0xFF, a[0], a[1], a[2], b[0], b[1], b[2]);
        return;
    }
    let p = pos_ptr(this);
    let mut level = [0u32; 1];
    let lv = local(&mut level);
    if get_water_level_no_waves(f32::from_bits(rd::<u32>(p, 0)).to_bits(), f32::from_bits(rd::<u32>(p, 4)).to_bits(), f32::from_bits(rd::<u32>(p, 8)).to_bits(), lv.cast::<f32>(), GPtr::NULL, GPtr::NULL) != 0
        && !x(rd::<f32>(pos_ptr(this), 8)).fcom(x(rd::<f32>(lv, 0))).test_parity(0x41)
    {
        return;
    }
    register_streak(this, 0xFF, 0xA0, 0x64, 0xFF, a[0], a[1], a[2], b[0], b[1], b[2]);
}

/// The traffic-light models (0x536977, objects and dummies): `DisplayActualLight` and / or a pole
/// shadow (`StoreShadowForPole(this, x, y, 0, 16, 0.4, 0)`) per model index global 0x8CD4F4..0x8CD524.
fn traffic_light_poles(this: GPtr<u8>) {
    let m = i32::from(rd::<i16>(this, 0x22));
    let pole = |ox: u32, oy: u32| store_shadow_for_pole(this, ox, oy, 0, F_16, F_0_4, 0);
    if is_mi(m, 0x8CD4F4) {
        display_actual_light(this);
        pole(0x403D3F7D, 0x3E16872B);
    } else if is_mi(m, 0x8CD4F8) {
        display_actual_light(this);
    } else if is_mi(m, 0x8CD4FC) {
        display_actual_light(this);
        pole(0x4099EB85, 0);
    } else if is_mi(m, 0x8CD504) {
        display_actual_light(this);
        pole(0x40F01893, 0);
    } else if is_mi(m, 0x8CD508) || is_mi(m, 0x8CD50C) || is_mi(m, 0x8CD514) || is_mi(m, 0x8CD510) {
        display_actual_light(this);
    } else if is_mi(m, 0x8CD518) {
        pole(0x3F3E76C9, 0);
    } else if is_mi(m, 0x8CD51C) {
        pole(0x3D3020C5, 0);
    } else if is_mi(m, 0x8CD520) {
        pole(0x3F924DD3, 0x3E147AE1);
    } else if is_mi(m, 0x8CD524) {
        pole(0, 0xBD449BA6);
    } else if is_mi(m, 0x8CD500) {
        display_actual_light(this);
        pole(0x40F00000, 0x3E4CCCCD);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PRE_RENDER];
