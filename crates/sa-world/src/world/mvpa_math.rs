//! WRITER-MVP-A batch `mvpa-math-1`: vector/matrix leaves and small constructors on the native
//! loading path (catalogue subsystems MATH / UNK-game). Ported from the 1.0 US bytes with the x87
//! operation order of the original (the game runs with PC = 24 bits). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{rd, wr, x, xi};

orig! {
    /// `RwMatrixDestroy(RwMatrix*)` (0x7F2A20)
    fn rw_matrix_destroy(m: GPtr<u8>) -> i32 = cdecl 0x7F2A20 => unported;
    /// `operator delete` of the 8-byte list nodes (0x552390)
    fn single_node_delete(n: GPtr<u8>) = cdecl 0x552390 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    fn rw_matrix_update(m: GPtr<u8>) -> u32 = cdecl 0x7F18A0 => unported;
    /// `CCollision::RemoveTrianglePlanes(CCollisionData*)` (0x416400)
    fn remove_triangle_planes(d: GPtr<u8>) = cdecl 0x416400 => unported;
    /// `CMemoryMgr::Free(void*)` (0x72F430)
    fn memory_mgr_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// `CCollisionData::RemoveCollisionVolumes()` (0x40F070)
    fn remove_collision_volumes(d: GPtr<u8>) = thiscall 0x40F070 => unported;
    /// CRT `free` (0x8214BD)
    fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `eh vector constructor iterator(void*, size, count, ctor, dtor)` (0x821D30, stdcall)
    fn eh_vector_constructor_iterator(p: GPtr<u8>, size: u32, count: i32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
}

/// The three components of the vector at `p` (x, y, z).
fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

fn store3(out: GPtr<u8>, v: [X87; 3]) {
    wr::<f32>(out, 0, v[0].to_f32());
    wr::<f32>(out, 4, v[1].to_f32());
    wr::<f32>(out, 8, v[2].to_f32());
}

/// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60, cdecl): `out = a - b`, all
/// three differences computed before the stores (aliasing-safe as in the original); returns `out`.
#[hook(addr = 0x40FE60, conv = cdecl, name = "VectorSub", expect = "8B 44 24 08 D9 40 08",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let (va, vb) = (v3(a), v3(b));
    let z = x(va[2]).fsub_m(vb[2]);
    let y = x(va[1]).fsub_m(vb[1]);
    let xx = x(va[0]).fsub_m(vb[0]);
    store3(out, [xx, y, z]);
    out
}

/// `operator+(const CVector&, const CVector&)` (0x40FE30, cdecl, returns via `out`): `out = a + b`,
/// loads before stores; returns `out`.
#[hook(addr = 0x40FE30, conv = cdecl, name = "FUN_0040fe30", expect = "8B 44 24 08 D9 40 08",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "CVector operator+ (no catalogue name)")]
pub fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let (va, vb) = (v3(a), v3(b));
    let z = x(va[2]).fadd_m(vb[2]);
    let y = x(va[1]).fadd_m(vb[1]);
    let xx = x(va[0]).fadd_m(vb[0]);
    store3(out, [xx, y, z]);
    out
}

/// `operator*(const CVector&, float)` (0x40FEC0, cdecl): `out = v * s` (`s * v.c` per component, loads
/// before stores); returns `out`.
#[hook(addr = 0x40FEC0, conv = cdecl, name = "FUN_0040fec0", expect = "8B 44 24 08 D9 44 24 0C",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "CVector operator*(CVector, float) (no catalogue name)")]
pub fn vector_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> {
    let vv = v3(v);
    let z = x(s).fmul_m(vv[2]);
    let y = x(s).fmul_m(vv[1]);
    let xx = x(s).fmul_m(vv[0]);
    store3(out, [xx, y, z]);
    out
}

/// `operator*(float, const CVector&)` (0x40FE90, cdecl): `out = s * v`; returns `out`.
#[hook(addr = 0x40FE90, conv = cdecl, name = "FUN_0040fe90", expect = "8B 44 24 0C D9 44 24 08",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "CVector operator*(float, CVector) (no catalogue name)")]
pub fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> {
    let vv = v3(v);
    let z = x(s).fmul_m(vv[2]);
    let y = x(s).fmul_m(vv[1]);
    let xx = x(s).fmul_m(vv[0]);
    store3(out, [xx, y, z]);
    out
}

/// `CVector::operator+=(const CVector&)` (0x411A00, thiscall, `ret 4`): per component `this.c = o.c +
/// this.c` (x, then y, then z, each stored before the next load); returns `this`.
#[hook(addr = 0x411A00, conv = thiscall, name = "FUN_00411a00", expect = "8B C1 8B 4C 24 04",
       unverified = "mvpa-math-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk",
       note = "CVector::operator+= (no catalogue name); the original leaves ECX = the argument")]
pub fn vector_add_assign(this: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> {
    for off in [0, 4, 8] {
        wr::<f32>(this, off, x(rd::<f32>(o, off)).fadd_m(rd::<f32>(this, off)).to_f32());
    }
    this
}

/// `CVector::operator-=(const CVector&)` (0x406D70, thiscall, `ret 4`): per component `this.c -= o.c`
/// (x, y, z in turn); returns `this`.
#[hook(addr = 0x406D70, conv = thiscall, name = "FUN_00406d70", expect = "8B C1 8B 4C 24 04",
       unverified = "mvpa-math-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk",
       note = "CVector::operator-= (no catalogue name); the original leaves ECX = the argument")]
pub fn vector_sub_assign(this: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> {
    for off in [0, 4, 8] {
        wr::<f32>(this, off, x(rd::<f32>(this, off)).fsub_m(rd::<f32>(o, off)).to_f32());
    }
    this
}

/// `operator/(const CVector&, float)` (0x4119D0, cdecl): `r = 1.0f / s` (stored back over the `s`
/// argument slot by `fst`), `out.z = r(ST0) * v.z`, `out.y = r(f32) * v.y`, `out.x = r(f32) * v.x`;
/// returns `out`. The argument-slot store is not reproduced (caller-owned slot no caller reads).
#[hook(addr = 0x4119D0, conv = cdecl, name = "FUN_004119d0", expect = "D9 05 24 86 85 00",
       unverified = "mvpa-math-1", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "CVector operator/(CVector, float) (no catalogue name); fst over its own argument slot")]
pub fn vector_div_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> {
    let r = X87::one().fdiv_m(s);
    let rf = r.to_f32();
    let vv = v3(v);
    let z = r.fmul_m(vv[2]);
    let y = x(rf).fmul_m(vv[1]);
    let xx = x(rf).fmul_m(vv[0]);
    store3(out, [xx, y, z]);
    out
}

/// `CVector::SquaredMagnitude()` (0x406DA0, thiscall, ST0): `(x*x + y*y) + z*z` unrounded in ST0.
#[hook(addr = 0x406DA0, conv = thiscall, name = "FUN_00406da0", expect = "D9 41 08 D9 41 04",
       unverified = "mvpa-math-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk",
       note = "CVector::SquaredMagnitude (no catalogue name); ECX/EDX untouched by the original")]
pub fn vector_squared_magnitude(this: GPtr<u8>) -> X87 {
    let v = v3(this);
    let (xx, y, z) = (x(v[0]), x(v[1]), x(v[2]));
    xx.fmul(xx).fadd(y.fmul(y)).fadd(z.fmul(z))
}

/// `min(float a, float b)` (0x404330, cdecl, ST0): `a` when `a < b`, else `b` (equal and unordered
/// give `b`).
#[hook(addr = 0x404330, conv = cdecl, name = "FUN_00404330", expect = "D9 44 24 04 D8 5C 24 08",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "std::min<float> (no catalogue name); pure, L1 spec not written")]
pub fn min_f(a: f32, b: f32) -> X87 {
    if x(a).fcom(x(b)).test_parity(5) { x(b) } else { x(a) }
}

/// `max(float a, float b)` (0x420800, cdecl, ST0): `a` when `a > b`, else `b` (equal and unordered
/// give `b`).
#[hook(addr = 0x420800, conv = cdecl, name = "FUN_00420800", expect = "D9 44 24 04 D8 5C 24 08",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "std::max<float> (no catalogue name); pure, L1 spec not written")]
pub fn max_f(a: f32, b: f32) -> X87 {
    if x(a).fcom(x(b)).test_any(0x41) { x(b) } else { x(a) }
}

/// `approxEqual(float a, float b, float eps)` (0x4EEA80, cdecl): 1 when `|a - b| < eps`, else 0 (EAX).
#[hook(addr = 0x4EEA80, conv = cdecl, name = "approxEqual", expect = "D9 44 24 04 D8 64 24 08",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "pure, L1 spec not written")]
pub fn approx_equal(a: f32, b: f32, eps: f32) -> u32 {
    if x(a).fsub_m(b).fabs().fcom(x(eps)).test_parity(5) { 0 } else { 1 }
}

/// Compressed-vector decompression (0x411590, cdecl `(CVector* out, const int16 in[3])`): `out.c =
/// in[c] * (1/128)` via `fild`; the original parks each int in its own second-argument slot. Returns
/// the sign-extended z (the EAX the original leaves).
#[hook(addr = 0x411590, conv = cdecl, name = "FUN_00411590", expect = "8B 44 24 08 0F BF 08",
       unverified = "mvpa-math-1", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "CompressedVector::Uncompress (no catalogue name); writes its own argument slot")]
pub fn uncompress_vector(out: GPtr<u8>, inp: GPtr<u8>) -> i32 {
    let mut last = 0;
    for k in 0..3isize {
        last = i32::from(rd::<i16>(inp, 2 * k));
        wr::<f32>(out, 4 * k, xi(last).fmul_m(0.0078125).to_f32());
    }
    last
}

/// `Multiply3x3(CVector* out, const CMatrix& m, const CVector& v)` (0x59C790, cdecl): `out = m * v`
/// (rotation part): x = `(m.at.x*v.z + m.up.x*v.y) + m.right.x*v.x`, y = `(m.at.y*v.z + m.right.y*v.x) +
/// m.up.y*v.y`, z = `(m.at.z*v.z + m.right.z*v.x) + m.up.z*v.y`; returns `out`.
#[hook(addr = 0x59C790, conv = cdecl, name = "Multiply3x3", expect = "83 EC 0C 8B 4C 24 14",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> {
    let f = |o: isize| rd::<f32>(m, o);
    let vv = v3(v);
    let rx = x(f(0x20)).fmul_m(vv[2]).fadd(x(f(0x10)).fmul_m(vv[1])).fadd(x(f(0)).fmul_m(vv[0]));
    let ry = x(f(0x24)).fmul_m(vv[2]).fadd(x(f(4)).fmul_m(vv[0])).fadd(x(f(0x14)).fmul_m(vv[1]));
    let rz = x(f(0x28)).fmul_m(vv[2]).fadd(x(f(8)).fmul_m(vv[0])).fadd(x(f(0x18)).fmul_m(vv[1]));
    store3(out, [rx, ry, rz]);
    out
}

/// `Multiply3x3(CVector* out, const CVector& v, const CMatrix& m)` (0x59C810, cdecl): `out = v * m`
/// (transposed rotation): x = `(m.right.y*v.y + m.right.z*v.z) + v.x*m.right.x`, y = `(m.up.y*v.y +
/// m.up.x*v.x) + m.up.z*v.z`, z = `(m.at.y*v.y + m.at.x*v.x) + m.at.z*v.z`; returns `out`.
#[hook(addr = 0x59C810, conv = cdecl, name = "FUN_0059c810", expect = "83 EC 0C 8B 4C 24 18",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "Multiply3x3(CVector, CMatrix) (no catalogue name)")]
pub fn multiply3x3_transposed(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    let f = |o: isize| rd::<f32>(m, o);
    let vv = v3(v);
    let rx = x(f(4)).fmul_m(vv[1]).fadd(x(f(8)).fmul_m(vv[2])).fadd(x(vv[0]).fmul_m(f(0)));
    let ry = x(f(0x14)).fmul_m(vv[1]).fadd(x(f(0x10)).fmul_m(vv[0])).fadd(x(f(0x18)).fmul_m(vv[2]));
    let rz = x(f(0x24)).fmul_m(vv[1]).fadd(x(f(0x20)).fmul_m(vv[0])).fadd(x(f(0x28)).fmul_m(vv[2]));
    store3(out, [rx, ry, rz]);
    out
}

/// `CMatrixLinkList` sentinel setup (0x54F440, thiscall): zeroes the prev/next pointers (+0x40/+0x44
/// of each 0x54-byte link) of the five sentinel links at +0, +0x54, +0xA8, +0xFC, +0x150 and the dword
/// +0x1F8; returns `this`.
#[hook(addr = 0x54F440, conv = thiscall, name = "FUN_0054f440", expect = "8B C1 33 C9 89 48 40",
       unverified = "mvpa-math-1", route = "split", hazards = "preserved-regs", recs = "grove-walk",
       note = "CMatrixLinkList::CMatrixLinkList (static initialiser); the original leaves ECX = 0")]
pub fn matrix_link_list_ctor(this: GPtr<u8>) -> GPtr<u8> {
    for o in [0x40, 0x44, 0x94, 0x98, 0xE8, 0xEC, 0x13C, 0x140, 0x190, 0x194, 0x1E4, 0x1E8, 0x1F8] {
        wr::<u32>(this, o, 0);
    }
    this
}

/// `CMatrix::~CMatrix()` (0x59ACD0, thiscall): when it owns (+0x44) an attached RW matrix (+0x40),
/// `RwMatrixDestroy` of it.
#[hook(addr = 0x59ACD0, conv = thiscall, name = "CMatrix", expect = "8B 41 44 85 C0",
       unverified = "mvpa-math-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn matrix_dtor(this: GPtr<u8>) {
    if rd::<u32>(this, 0x44) != 0 {
        let m = rd::<GPtr<u8>>(this, 0x40);
        if !m.is_null() {
            rw_matrix_destroy(m);
        }
    }
}

/// `CColLine::CColLine(const CVector& start, const CVector& end)` (0x40EF50, `.HOODLUM` stub, `ret 8`):
/// copies start to +0 and end to +0x10 (dword copies); returns `this`.
#[hook(addr = 0x40EF50, conv = thiscall, name = "CColLine", expect = "E9 5B 22 15 01",
       unverified = "mvpa-math-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk",
       note = "the original leaves ECX = end.z bits, EDX = this + 0x10")]
pub fn col_line_ctor(this: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> GPtr<u8> {
    for o in [0, 4, 8] {
        wr::<u32>(this, o, rd::<u32>(start, o));
    }
    for o in [0, 4, 8] {
        wr::<u32>(this, 0x10 + o, rd::<u32>(end, o));
    }
    this
}

/// `CCollisionData::CCollisionData()` (0x40F030, thiscall): the four counts (+0, +2, +4: u16; +6: u8)
/// = 0, the low three flag bits of +7 cleared, the ten pointers +8..+0x2C = null; returns `this`.
#[hook(addr = 0x40F030, conv = thiscall, name = "CCollisionData", expect = "8B C1 8A 50 07",
       unverified = "mvpa-math-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk",
       note = "the original leaves ECX = 0, DL = the new flags")]
pub fn collision_data_ctor(this: GPtr<u8>) -> GPtr<u8> {
    let f = rd::<u8>(this, 7) & 0xF8;
    wr::<u16>(this, 0, 0);
    wr::<u16>(this, 2, 0);
    wr::<u16>(this, 4, 0);
    wr::<u8>(this, 6, 0);
    wr::<u8>(this, 7, f);
    for o in (8..=0x2C).step_by(4) {
        wr::<u32>(this, o, 0);
    }
    this
}

/// `CMatrixLink::CMatrixLink()` (0x54F0C0, thiscall): prev (+0x40) and next (+0x44) = null; returns
/// `this`.
#[hook(addr = 0x54F0C0, conv = thiscall, name = "CMatrixLink", expect = "8B C1 33 C9 89 48 40",
       unverified = "mvpa-math-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk",
       note = "the original leaves ECX = 0")]
pub fn matrix_link_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x40, 0);
    wr::<u32>(this, 0x44, 0);
    this
}

/// Frees a pool node inline as the list flushes do: index = signed `(node - objects) / size`, byte-map
/// bit 7 set, first-free index lowered.
fn pool_free_inline(pool: GPtr<u8>, node: GPtr<u8>, size: i32) {
    let idx = (node.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32 / size;
    let m = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize);
    m.write(m.read() | 0x80);
    if idx < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, idx);
    }
}

/// `CPtrListDoubleLink::Flush()` (0x552470, thiscall): unlinks every node (head +0; node: item +0, next
/// +4, prev +8) and frees it into `CPools::ms_pPtrNodeDoubleLinkPool` (0xB74488, 0xC-byte nodes) inline.
#[hook(addr = 0x552470, conv = thiscall, name = "CPtrListDoubleLink::Flush", expect = "57 8B 39 85 FF",
       unverified = "mvpa-math-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn double_link_flush(this: GPtr<u8>) {
    let mut n = rd::<GPtr<u8>>(this, 0);
    while !n.is_null() {
        let node = n;
        n = rd::<GPtr<u8>>(node, 4);
        if rd::<GPtr<u8>>(this, 0) == node {
            wr::<GPtr<u8>>(this, 0, n);
        }
        let prev = rd::<GPtr<u8>>(node, 8);
        if !prev.is_null() {
            wr::<GPtr<u8>>(prev, 4, rd::<GPtr<u8>>(node, 4));
        }
        let next = rd::<GPtr<u8>>(node, 4);
        if !next.is_null() {
            wr::<GPtr<u8>>(next, 8, rd::<GPtr<u8>>(node, 8));
        }
        pool_free_inline(GPtr::<GPtr<u8>>::from_addr(0xB74488).read(), node, 0xC);
    }
}

/// `CPtrListSingleLink::Flush()` (0x552400, thiscall): removes every node (head +0; node: item +0,
/// next +4): the head node is unlinked directly, any other is searched from the head and unlinked; each
/// is freed into `CPools::ms_pPtrNodeSingleLinkPool` (0xB74484, 8-byte nodes, index `>> 3`) inline.
#[hook(addr = 0x552400, conv = thiscall, name = "CPtrListSingleLink::Flush", expect = "53 8B 19 85 DB",
       unverified = "mvpa-math-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn single_link_flush(this: GPtr<u8>) {
    let mut n = rd::<GPtr<u8>>(this, 0);
    while !n.is_null() {
        let node = n;
        n = rd::<GPtr<u8>>(node, 4);
        let head = rd::<GPtr<u8>>(this, 0);
        if head == node {
            wr::<GPtr<u8>>(this, 0, n);
        } else {
            let mut p = head;
            while !p.is_null() {
                let nx = rd::<GPtr<u8>>(p, 4);
                if nx == node {
                    wr::<GPtr<u8>>(p, 4, rd::<GPtr<u8>>(node, 4));
                    break;
                }
                p = nx;
            }
        }
        let pool = GPtr::<GPtr<u8>>::from_addr(0xB74484).read();
        let idx = ((node.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32) >> 3;
        let m = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize);
        m.write(m.read() | 0x80);
        if idx < rd::<i32>(pool, 0xC) {
            wr::<i32>(pool, 0xC, idx);
        }
    }
}

/// `CPtrListSingleLink::DeleteItem(void* item)` (0x533610, thiscall, `ret 4`): finds the first node whose
/// item is `item`, unlinks it (head or after its predecessor) and frees it with the node `operator
/// delete` (0x552390). Nothing when absent.
#[hook(addr = 0x533610, conv = thiscall, name = "CPtrListSingleLink::DeleteItem", expect = "56 8B 31 8B C6",
       unverified = "mvpa-math-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn single_link_delete_item(this: GPtr<u8>, item: GPtr<u8>) {
    let head = rd::<GPtr<u8>>(this, 0);
    let mut prev = GPtr::<u8>::NULL;
    let mut n = head;
    while !n.is_null() {
        if rd::<GPtr<u8>>(n, 0) == item {
            if head == n {
                wr::<GPtr<u8>>(this, 0, rd::<GPtr<u8>>(n, 4));
            } else if !prev.is_null() {
                wr::<GPtr<u8>>(prev, 4, rd::<GPtr<u8>>(n, 4));
            }
            single_node_delete(n);
            return;
        }
        prev = n;
        n = rd::<GPtr<u8>>(n, 4);
    }
}

/// `CMaths::InitMathsTables()` (0x59AC90): `ms_SinTable[i]` (0xBB3E00, 256 floats) = `fsin(fild(i) *
/// 0.024543693f)` (2*pi/256 as stored at 0x859BBC).
#[hook(addr = 0x59AC90, conv = cdecl, name = "CMaths::InitMathsTables", expect = "51 33 C0 89 44 24 00",
       unverified = "mvpa-math-1", route = "L2", hazards = "x87-transcendental", recs = "grove-walk")]
pub fn init_maths_tables() {
    let k = GPtr::<f32>::from_addr(0x859BBC).read();
    for i in 0..256i32 {
        let v = xi(i).fmul_m(k).fsin();
        GPtr::<f32>::from_addr(0xBB3E00 + 4 * i as usize).write(v.to_f32());
    }
}

/// `CSimpleTransform::UpdateRwMatrix(RwMatrix* m)` (0x54EF40, thiscall, `ret 4`): `c = fcos(heading
/// (+0xC))`, `s = fsin(heading)`; right = (c, s, 0), up = (-s, c, 0), at = (0, 0, 1), pos = the
/// transform's position (dword copies); then `RwMatrixUpdate(m)`, whose EAX is returned.
#[hook(addr = 0x54EF40, conv = thiscall, name = "CSimpleTransform::UpdateRwMatrix", expect = "D9 41 0C 8B 44 24 04",
       unverified = "mvpa-math-1", route = "L2", hazards = "x87-transcendental,preserved-regs", recs = "grove-walk",
       note = "the original leaves ECX = m (pop ecx)")]
pub fn simple_transform_update_rw_matrix(this: GPtr<u8>, m: GPtr<u8>) -> u32 {
    let h = rd::<f32>(this, 0xC);
    let c = x(h).fcos();
    let s = x(h).fsin();
    for o in [8, 0x18, 0x20, 0x24] {
        wr::<u32>(m, o, 0);
    }
    wr::<u32>(m, 0x28, 0x3F80_0000);
    wr::<f32>(m, 0, c.to_f32());
    wr::<f32>(m, 4, s.to_f32());
    wr::<f32>(m, 0x10, s.fchs().to_f32());
    wr::<f32>(m, 0x14, c.to_f32());
    wr::<u32>(m, 0x30, rd::<u32>(this, 0));
    wr::<u32>(m, 0x34, rd::<u32>(this, 4));
    wr::<u32>(m, 0x38, rd::<u32>(this, 8));
    rw_matrix_update(m)
}

/// `CColModel::~CColModel()` (0x40F700, thiscall): when it owns collision data (bit 2 of +0x29) and the
/// pointer +0x2C is set: single-allocation data (bit 1) → `CCollision::RemoveTrianglePlanes(data)` and
/// `CMemoryMgr::Free(data)`; else `data->RemoveCollisionVolumes()` and CRT `free(data)`.
#[hook(addr = 0x40F700, conv = thiscall, name = "CColModel", expect = "56 8B F1 8A 46 29",
       unverified = "mvpa-math-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn col_model_dtor(this: GPtr<u8>) {
    let f = rd::<u8>(this, 0x29);
    if f & 4 == 0 {
        return;
    }
    let d = rd::<GPtr<u8>>(this, 0x2C);
    if d.is_null() {
        return;
    }
    if f & 2 != 0 {
        remove_triangle_planes(d);
        memory_mgr_free(rd::<GPtr<u8>>(this, 0x2C));
    } else {
        remove_collision_volumes(d);
        crt_free(rd::<GPtr<u8>>(this, 0x2C));
    }
}

/// `CMatrixLinkList::Init(int32 count)` (0x54F0D0, thiscall, `ret 4`): `new CMatrixLink[count]` (count
/// header, `eh vector constructor iterator` with `CMatrixLink::CMatrixLink` / `~CMatrixLink`; null array
/// when the allocation fails), the four sentinel pairs linked (+0x50 = +0x54, +0x1A0 = +0x1A4, +0x1F8 =
/// array, +0xA0 = this, +0xF8 = +0xFC, +0x148 = +0xA8, +0x1F0 = +0x150) and every link, from the last to
/// the first, pushed after the free-list head (+0x150): `l.next (+0x50) = head.next`, `head.next.prev
/// (+0x4C) = l`, `l.prev = head`, `head.next = l`.
#[hook(addr = 0x54F0D0, conv = thiscall, name = "CMatrixLinkList::Init", expect = "64 A1 00 00 00 00",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "one-time init: allocates the matrix array")]
pub fn matrix_link_list_init(this: GPtr<u8>, count: i32) {
    let size = count.wrapping_mul(0x54);
    let mem = operator_new(size.wrapping_add(4) as u32);
    let mut arr = GPtr::<u8>::NULL;
    if !mem.is_null() {
        arr = mem.byte_add(4);
        wr::<i32>(mem, 0, count);
        eh_vector_constructor_iterator(arr, 0x54, count, 0x54F0C0, 0x54E8E0);
    }
    wr::<GPtr<u8>>(this, 0x50, this.byte_add(0x54));
    wr::<GPtr<u8>>(this, 0x1A0, this.byte_add(0x1A4));
    wr::<GPtr<u8>>(this, 0x1F8, arr);
    wr::<GPtr<u8>>(this, 0xA0, this);
    wr::<GPtr<u8>>(this, 0xF8, this.byte_add(0xFC));
    wr::<GPtr<u8>>(this, 0x148, this.byte_add(0xA8));
    wr::<GPtr<u8>>(this, 0x1F0, this.byte_add(0x150));
    let head = this.byte_add(0x150);
    let mut off = size;
    let mut n = count;
    while n != 0 {
        off = off.wrapping_sub(0x54);
        let l = rd::<GPtr<u8>>(this, 0x1F8).byte_add(off as isize);
        n -= 1;
        wr::<GPtr<u8>>(l, 0x50, rd::<GPtr<u8>>(head, 0x50));
        wr::<GPtr<u8>>(rd::<GPtr<u8>>(head, 0x50), 0x4C, l);
        wr::<GPtr<u8>>(l, 0x4C, head);
        wr::<GPtr<u8>>(head, 0x50, l);
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_VECTOR_SUB,
    HOOK_VECTOR_ADD,
    HOOK_VECTOR_MUL_SCALAR,
    HOOK_SCALAR_MUL_VECTOR,
    HOOK_VECTOR_ADD_ASSIGN,
    HOOK_VECTOR_SUB_ASSIGN,
    HOOK_VECTOR_DIV_SCALAR,
    HOOK_VECTOR_SQUARED_MAGNITUDE,
    HOOK_MIN_F,
    HOOK_MAX_F,
    HOOK_APPROX_EQUAL,
    HOOK_UNCOMPRESS_VECTOR,
    HOOK_MULTIPLY3X3,
    HOOK_MULTIPLY3X3_TRANSPOSED,
    HOOK_MATRIX_LINK_LIST_CTOR,
    HOOK_MATRIX_DTOR,
    HOOK_COL_LINE_CTOR,
    HOOK_COLLISION_DATA_CTOR,
    HOOK_MATRIX_LINK_CTOR,
    HOOK_DOUBLE_LINK_FLUSH,
    HOOK_SINGLE_LINK_FLUSH,
    HOOK_SINGLE_LINK_DELETE_ITEM,
    HOOK_INIT_MATHS_TABLES,
    HOOK_SIMPLE_TRANSFORM_UPDATE_RW_MATRIX,
    HOOK_COL_MODEL_DTOR,
    HOOK_MATRIX_LINK_LIST_INIT,
];
