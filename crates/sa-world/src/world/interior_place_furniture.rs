//! WRITER-WORLD-2 (batch `world6-2`, SA:MP-executed): `Interior_c::PlaceFurniture`, which creates
//! one furniture building on the interior's tile grid. Ported from the 1.0 US bytes; gta-reversed
//! has it unreversed. The `CMatrix` temporaries live in Rust buffers of 0x48 bytes and go through the
//! original matrix functions in the bytes' order (constructions, products, assignments, destructors).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{rd, wr, x, xi};

orig! {
    /// `List_c::GetNumItems()` (0x4A8EC0) on the free furniture item list (0xBAD3EC)
    fn list_num_items(l: GPtr<u8>) -> i32 = thiscall 0x4A8EC0 => unported;
    /// `List_c::RemoveHead()` (0x4A8E70)
    fn list_remove_head(l: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8E70 => unported;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0)
    fn list_add_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `Interior_c::CheckTilesEmpty(int32 x, int32 y, int32 w, int32 d, bool)` (0x591680)
    fn check_tiles_empty(this: GPtr<u8>, tx: i32, ty: i32, w: i32, d: i32, b: u32) -> u8 = thiscall 0x591680 => unported;
    /// `CMatrix::SetUnity()` (0x59AE70)
    fn m_set_unity(m: GPtr<u8>) = thiscall 0x59AE70 => unported;
    /// `CMatrix::SetTranslate(float, float, float)` (0x59AF40)
    fn m_set_translate(m: GPtr<u8>, tx: u32, ty: u32, tz: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B390)
    fn m_set_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B390 => unported;
    /// `operator*(const CMatrix&, const CMatrix&)` (0x59BE30): the product constructed at `out`
    fn m_mul(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59BE30 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn m_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    fn m_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn m_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CBuilding::operator new(uint32)` (0x404090)
    fn building_new(size: u32) -> GPtr<u8> = cdecl 0x404090 => unported;
    /// `CBuilding::CBuilding()` (0x403E00)
    fn building_ctor(b: GPtr<u8>) -> GPtr<u8> = thiscall 0x403E00 => unported;
    /// `CEntity::SetModelIndex(uint32)` (vtable +0x18)
    fn v_set_model_index(e: GPtr<u8>, mi: u32) = thiscall vslot 0x18 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610)
    fn set_matrix(e: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `Interior_c::SetTilesStatus(int32 x, int32 y, int32 w, int32 d, int32 status, bool)` (0x591700)
    fn set_tiles_status(this: GPtr<u8>, x: i32, y: i32, w: i32, d: i32, status: i32, force: u32) = thiscall 0x591700 => unported;
}

/// The free furniture item list (0xBAD3EC).
const FREE_ITEMS: usize = 0xBA_D3EC;
/// `0.5` (0x858B8C), degrees to radians (0x8595EC), `pi / 2` (0x858FE4).
const F_0_5: f32 = 0.5;
const DEG2RAD: f32 = f32::from_bits(0x3C8E_FA35);
const HALF_PI: f32 = f32::from_bits(0x3FC9_0FDB);

type Mat = [u32; 0x12];

fn m(v: &mut Mat) -> GPtr<u8> {
    GPtr::from_raw(v as *mut Mat).cast()
}

/// `Interior_c::PlaceFurniture(Furniture_c* f, int32 x, int32 y, float height, int32 mode, int32 rot,
/// int32* outW, int32* outD, uint8 force)` (0x592AA0). Fails (both outputs 0, null) without a free
/// list item or, in mode 1 unless `force`, when the footprint (`f` +0xC by +0xD, swapped for rot 1 /
/// 3) is not empty (`CheckTilesEmpty` with `f` +0x10). The placement matrix is the interior's
/// (`this` +0x18) times `RotateZ(rot * pi/2)` translated to the footprint's centre (modes 0 / 1 / 2;
/// the z is `info +4 * 0.5 - height` for mode 0 and `-info +4 * 0.5 + height` for 1 / 2, the x / y
/// `-W/2 + offset + x + 0.5` with the rotation's corner offset) times, for furniture with a random
/// rotation range (+0x13), a jitter matrix (`translate(-c) * RotateZ(random(-n, n) degrees) *
/// translate(c)` about the piece's centre). A new `CBuilding` (model +8, the matrix, area = this +0x10,
/// flags +0x1C |= 0x410000) is added to the world and to the interior's furniture list (+0x5C) with
/// its tile; mode 1 marks the footprint status 6 (`f` +0x11 set) or 5. Returns the building with the
/// footprint size in the outputs.
#[hook(addr = 0x592AA0, conv = thiscall, name = "Interior_c::PlaceFurniture", this = "Interior_c",
       expect = "6A FF 64 A1 00 00 00 00",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       note = "creates a CBuilding (world, building pool); eight stack CMatrix objects handled through the original matrix functions")]
#[allow(clippy::too_many_arguments)]
pub fn place_furniture(this: GPtr<u8>, f: GPtr<u8>, tx: i32, ty: i32, height: f32, mode: i32, rot: i32,
                       out_w: GPtr<i32>, out_d: GPtr<i32>, force: u8) -> GPtr<u8> {
    let fail = || {
        out_w.write(0);
        out_d.write(0);
        GPtr::NULL
    };
    if list_num_items(GPtr::from_addr(FREE_ITEMS)) <= 0 {
        return fail();
    }
    let fc = i32::from(rd::<u8>(f, 0xC));
    let fd = i32::from(rd::<u8>(f, 0xD));
    let (w, d) = if rot == 1 || rot == 3 { (fd, fc) } else { (fc, fd) };
    if mode == 1 && force == 0 && check_tiles_empty(this, tx, ty, w, d, u32::from(rd::<u8>(f, 0x10))) == 0 {
        return fail();
    }
    let mut m0: Mat = [0; 0x12]; // [esp+0x78]
    let n = rd::<u8>(f, 0x13);
    if n != 0 {
        let cy = xi(fd).fmul_m(F_0_5).fsubr_m(F_0_5).to_f32();
        let cx = xi(fc).fmul_m(F_0_5).fsubr_m(F_0_5).to_f32();
        let mut m1: Mat = [0; 0x12];
        m_set_unity(m(&mut m1));
        m_set_translate(m(&mut m1), cx.to_bits(), cy.to_bits(), 0);
        let mut m2: Mat = [0; 0x12];
        m_set_unity(m(&mut m2));
        let r = random_in_range(-i32::from(n), i32::from(n));
        m_set_rotate_z(m(&mut m2), xi(r).fmul_m(DEG2RAD).to_f32());
        let mut m3: Mat = [0; 0x12];
        m_set_unity(m(&mut m3));
        let ny = x(cy).fchs().to_f32();
        let nx = x(cx).fchs().to_f32();
        m_set_translate(m(&mut m3), nx.to_bits(), ny.to_bits(), 0);
        let mut t1: Mat = [0; 0x12];
        let mut t2: Mat = [0; 0x12];
        let a = m_mul(m(&mut t1), m(&mut m3), m(&mut m2));
        let b = m_mul(m(&mut t2), a, m(&mut m1));
        m_assign(m(&mut m0), b);
        m_dtor(m(&mut t2));
        m_dtor(m(&mut t1));
        m_dtor(m(&mut m3));
        m_dtor(m(&mut m2));
        m_dtor(m(&mut m1));
    } else {
        m_set_unity(m(&mut m0));
    }
    let mut m5: Mat = [0; 0x12]; // [esp+0x1E0]
    m_ctor_rw(m(&mut m5), this.byte_add(0x18), 0);
    let mut m6: Mat = [0; 0x12]; // [esp+0x30]
    let mut m7: Mat = [0; 0x12]; // [esp+0xC0]
    m_set_unity(m(&mut m6));
    m_set_rotate_z(m(&mut m6), xi(rot).fmul_m(HALF_PI).to_f32());
    let (mut ox, mut oy) = (0i32, 0i32);
    match rot {
        1 => ox = w - 1,
        2 => {
            ox = w - 1;
            oy = d - 1;
        }
        3 => oy = d - 1,
        _ => {}
    }
    if mode == 0 || mode == 1 || mode == 2 {
        let info = rd::<GPtr<u8>>(this, 0x14);
        let hh = i32::from(rd::<u8>(info, 4));
        let dd = i32::from(rd::<u8>(info, 3));
        let ww = i32::from(rd::<u8>(info, 2));
        let z = if mode == 0 {
            xi(hh).fmul_m(F_0_5).fsub_m(height)
        } else {
            xi(-hh).fmul_m(F_0_5).fadd_m(height)
        };
        let y = xi(-dd).fmul_m(F_0_5).fadd(xi(oy)).fadd(xi(ty)).fadd_m(F_0_5);
        let xx = xi(-ww).fmul_m(F_0_5).fadd(xi(ox)).fadd(xi(tx)).fadd_m(F_0_5);
        let mb = m(&mut m6);
        for (k, v) in [xx, y, z].into_iter().enumerate() {
            let o = 0x30 + 4 * k as isize;
            wr::<f32>(mb, o, x(rd::<f32>(mb, o)).fadd(v).to_f32());
        }
    }
    let mut t1: Mat = [0; 0x12];
    let mut t2: Mat = [0; 0x12];
    let a = m_mul(m(&mut t1), m(&mut m5), m(&mut m6));
    let b = m_mul(m(&mut t2), a, m(&mut m0));
    m_assign(m(&mut m7), b);
    m_dtor(m(&mut t2));
    m_dtor(m(&mut t1));
    let item = list_remove_head(GPtr::from_addr(FREE_ITEMS));
    if item.is_null() {
        m_dtor(m(&mut m7));
        m_dtor(m(&mut m6));
        m_dtor(m(&mut m5));
        m_dtor(m(&mut m0));
        return fail();
    }
    let mem = building_new(0x38);
    let obj = if mem.is_null() { GPtr::NULL } else { building_ctor(mem) };
    wr::<GPtr<u8>>(item, 8, obj);
    v_set_model_index(obj, u32::from(rd::<u16>(f, 8)));
    set_matrix(rd::<GPtr<u8>>(item, 8), m(&mut m7));
    wr::<u8>(rd::<GPtr<u8>>(item, 8), 0x2F, rd::<u8>(this, 0x10));
    let o = rd::<GPtr<u8>>(item, 8);
    wr::<u32>(o, 0x1C, rd::<u32>(o, 0x1C) | 0x1_0000);
    let o = rd::<GPtr<u8>>(item, 8);
    wr::<u32>(o, 0x1C, rd::<u32>(o, 0x1C) | 0x40_0000);
    world_add(rd::<GPtr<u8>>(item, 8));
    wr::<u16>(item, 0xC, tx as u16);
    wr::<u16>(item, 0xE, ty as u16);
    list_add_item(this.byte_add(0x5C), item);
    if mode == 1 {
        let status = if rd::<u8>(f, 0x11) != 0 { 6 } else { 5 };
        set_tiles_status(this, tx, ty, w, d, status, 1);
    }
    out_w.write(w);
    out_d.write(d);
    let ret = rd::<GPtr<u8>>(item, 8);
    m_dtor(m(&mut m7));
    m_dtor(m(&mut m6));
    m_dtor(m(&mut m5));
    m_dtor(m(&mut m0));
    ret
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PLACE_FURNITURE];
