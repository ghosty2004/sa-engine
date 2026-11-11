//! `CPlantLocTri::Add` (batch `mvpm-render-3`): fills a plant location triangle from a collision triangle and puts it
//! on the plant or the object list. Ported from the 1.0 US bytes (gta-reversed `PlantLocTri.cpp` as a reading aid; the
//! bytes win). Unverified (docs/verify/HOWTO.md).
//!
//! `CPlantLocTri` (from the bytes): +0 / +0xC / +0x18 the three vertices, +0x24 centre, +0x30 radius, +0x34 / +0x38 /
//! +0x3C random seeds, +0x40 / +0x42 / +0x44 u16 plant counts, +0x46 surface, +0x47 lighting, +0x48 flags (bit 0
//! creates plants, bit 1 creates objects).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `CPlantSurfPropMgr::GetSurfProperties(uint16 surface)` (0x6F9DE0, cdecl; reads the low word)
    fn plant_get_surf_properties(surface: u32) -> GPtr<u8> = cdecl 0x6F9DE0 => unported;
    /// `CPlantLocTri::GetPlantDensity()` (0x5DC210): register convention, `this` in EAX, ST0 result
    fn plant_loc_tri_get_plant_density(t: GPtr<u8>) -> X87 = reg("args = eax") 0x5DC210 => unported;
    /// `CPlantMgr::MoveLocTriToList(CPlantLocTri** from, CPlantLocTri** to, CPlantLocTri*)` (0x5DB590, cdecl)
    fn plant_move_loc_tri_to_list(from: u32, to: u32, t: GPtr<u8>) = cdecl 0x5DB590 => unported;
    /// CRT `rand()` (0x821B1E)
    fn rand() -> i32 = cdecl 0x821B1E => sa_core::crt::rand_owned;
}

/// 1/3 (0x859040)
const K_THIRD: usize = 0x859040;
/// The radius scale (0x862E2C)
const K_RADIUS: usize = 0x862E2C;
/// 1/32767 (0x858C7C)
const K_RAND_SCALE: usize = 0x858C7C;
/// `CPlantMgr::m_UnusedLocTriListHead` (0xC03984), `m_LocTrisPlantsList` (0xC03988, per plant set),
/// `m_LocTrisObjsList` (0xC03994)
const UNUSED_LIST: u32 = 0xC03984;
const PLANTS_LISTS: u32 = 0xC03988;
const OBJS_LIST: u32 = 0xC03994;

fn ftol(v: X87) -> i32 {
    v.ftol() as u32 as i32
}

/// `CPlantLocTri::Add(const CVector& v1, const CVector& v2, const CVector& v3, uint8 surface, uint8 lighting, bool
/// createsPlants, bool createsObjects)` (0x5DC290, thiscall, `ret 0x1C`), EAX = `this` or 0.
///
/// Copies the vertices (raw dwords), the surface (+0x46) and lighting (+0x47) bytes, and sets flags +0x48 = (old &
/// 0xF8) | (createsObjects & 1) << 1 | (createsPlants & 1). Centre (+0x24): `x = f32((v2.x + v1.x) + v3.x)`,
/// `y = f32(v3.y + (v2.y + v1.y))`, `z = f32(v3.z + f32(v2.z + v1.z))`, then each `*= 1/3` stored; radius (+0x30) =
/// `f32(sqrt((dz² + dy²) + dx²) * k)` with `d = centre - v1`. With the creates-objects bit and without the plants bit
/// it goes on the object list. Else `GetSurfProperties(surface)`, `density = GetPlantDensity()` and the three counts
/// `_ftol(density * props[+0x20 / +0x48 / +0x70])`: when their u16 sum is positive, three seeds `f32(rand() *
/// 1/32767)` (+0x34, +0x38, +0x3C), the counts as u16 and the triangle moves from the unused list to the plant list
/// `props[+0]`; otherwise with the objects bit the plants bit is cleared and it goes on the object list, else EAX = 0.
#[hook(addr = 0x5dc290, conv = thiscall, name = "CPlantLocTri::Add", expect = "83 EC 24 8B 44 24 28",
       unverified = "mvpm-render-3", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "rand() through at_site (0x5DC45D, 0x5DC473, 0x5DC489); GetPlantDensity is a register-convention callee (EAX = this)")]
#[allow(clippy::too_many_arguments)]
pub fn plant_loc_tri_add(
    this: GPtr<u8>,
    v1: GPtr<u8>,
    v2: GPtr<u8>,
    v3: GPtr<u8>,
    surface: u32,
    lighting: u32,
    creates_plants: u32,
    creates_objects: u32,
) -> GPtr<u8> {
    for k in 0..3 {
        wr::<u32>(this, 4 * k, rd::<u32>(v1, 4 * k));
    }
    for k in 0..3 {
        wr::<u32>(this, 0xC + 4 * k, rd::<u32>(v2, 4 * k));
    }
    let v3c = [rd::<f32>(v3, 0), rd::<f32>(v3, 4), rd::<f32>(v3, 8)];
    for (k, v) in v3c.iter().enumerate() {
        wr::<f32>(this, 0x18 + 4 * k as isize, *v);
    }
    let surface = surface as u8;
    wr::<u8>(this, 0x47, lighting as u8);
    let fl = (((creates_objects as u8) & 1) << 1) | (rd::<u8>(this, 0x48) & 0xF8) | ((creates_plants as u8) & 1);
    wr::<u8>(this, 0x46, surface);
    wr::<u8>(this, 0x48, fl);
    let a = [rd::<f32>(v2, 0), rd::<f32>(v2, 4), rd::<f32>(v2, 8)];
    let b = [rd::<f32>(v1, 0), rd::<f32>(v1, 4), rd::<f32>(v1, 8)];
    let sx = x(a[0]).fadd_m(b[0]);
    let sy = x(a[1]).fadd_m(b[1]);
    let sz = x(a[2]).fadd_m(b[2]).to_f32();
    let cx = sx.fadd_m(v3c[0]).to_f32();
    let cy = x(v3c[1]).fadd(sy).to_f32();
    let cz = x(v3c[2]).fadd_m(sz).to_f32();
    let third = g::<f32>(K_THIRD).read();
    wr::<f32>(this, 0x24, x(cx).fmul_m(third).to_f32());
    wr::<f32>(this, 0x28, x(cy).fmul_m(third).to_f32());
    wr::<f32>(this, 0x2C, x(cz).fmul_m(third).to_f32());
    let dx = x(rd::<f32>(this, 0x24)).fsub_m(rd::<f32>(this, 0));
    let dy = x(rd::<f32>(this, 0x28)).fsub_m(rd::<f32>(this, 4));
    let dz = x(rd::<f32>(this, 0x2C)).fsub_m(rd::<f32>(this, 8));
    let r = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    wr::<f32>(this, 0x30, r.fmul_m(g::<f32>(K_RADIUS).read()).to_f32());

    if fl & 2 != 0 && fl & 1 == 0 {
        plant_move_loc_tri_to_list(UNUSED_LIST, OBJS_LIST, this);
        return this;
    }
    let props = plant_get_surf_properties(u32::from(surface));
    let d = plant_loc_tri_get_plant_density(this);
    let n1 = ftol(d.fmul_m(rd::<f32>(props, 0x20)));
    let n2 = ftol(d.fmul_m(rd::<f32>(props, 0x48)));
    let n3 = ftol(d.fmul_m(rd::<f32>(props, 0x70)));
    let sum = u32::from(n3 as u16) + u32::from(n2 as u16) + u32::from(n1 as u16);
    if sum as i32 > 0 {
        let ks = g::<f32>(K_RAND_SCALE).read();
        wr::<f32>(this, 0x34, X87::from_i32(at_site(0x5DC45D, rand)).fmul_m(ks).to_f32());
        wr::<f32>(this, 0x38, X87::from_i32(at_site(0x5DC473, rand)).fmul_m(ks).to_f32());
        let r3 = at_site(0x5DC489, rand);
        wr::<u16>(this, 0x40, n1 as u16);
        wr::<u16>(this, 0x42, n2 as u16);
        wr::<u16>(this, 0x44, n3 as u16);
        wr::<f32>(this, 0x3C, X87::from_i32(r3).fmul_m(ks).to_f32());
        let set = u32::from(rd::<u16>(props, 0));
        plant_move_loc_tri_to_list(UNUSED_LIST, PLANTS_LISTS + set * 4, this);
        return this;
    }
    let f = rd::<u8>(this, 0x48);
    if f & 2 == 0 {
        return GPtr::NULL;
    }
    wr::<u8>(this, 0x48, f & 0xFE);
    plant_move_loc_tri_to_list(UNUSED_LIST, OBJS_LIST, this);
    this
}

pub const HOOKS: &[HookReg] = &[HOOK_PLANT_LOC_TRI_ADD];
