//! ANIM: `BoneNode_c` and `BoneNodeManager_c` (PORT-ANIM-INPUT, fork D). Ported from the 1.0 US
//! bytes; gta-reversed `Ragdoll/BoneNode.cpp` / `BoneNodeManager.cpp` as reading aids,
//! `docs/re/specs/ANIM/BoneNode_c.deviations.md` for the kept original behaviour. See
//! `docs/arch/anim-input-port.md`.
//!
//! `BoneNode_c` (0x98): +0x00 `ListItem_c` (next +0, prev +4), +0x08 `m_BoneTag` (int32), +0x0C
//! `m_InterpFrame` (the bone's `RpHAnimBlendInterpFrame`: quaternion +0, translation +0x10), +0x10
//! `m_Orientation` (RtQuat x, y, z, w), +0x20 `m_Pos`, +0x2C `m_Parent`, +0x30 `m_Children`
//! (`TList_c`: head, tail, count), +0x3C `m_WorldMat` (RwMatrix), +0x7C `m_LimitMin[3]`, +0x88
//! `m_LimitMax[3]`, +0x94 `m_Speed`. `BoneNodeManager_c` (0x4C0C): 128 nodes, then the free list
//! `TList_c` at +0x4C00. `BoneNodeManager_c::ms_boneInfos` = 0x8D26D0 (32 × 0x28: int16 tag, int16
//! parent tag, 9 floats: pose rotation x/y/z, then min/max ranges per axis).
//!
//! x87 code runs through [`X87`] exactly as the bytes compute (PC = 24, f32 temporaries where the
//! original stores them). FPATAN / FSIN / FCOS use the model's CPU-checked implementations.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::{Cmp, X87};

use crate::common::{at, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// The 0x28 bytes before `BoneNodeManager_c::ms_boneInfos` (the original reads them as entry -1
    /// for an unknown tag) followed by `ms_boneInfos[32]` (0x8D26D0).
    pub static BONE_INFOS_AREA @ 0x8D26A8: [u8; 0x528] = BONE_INFOS_INIT;
    /// `s_TestSkipClampDefault`, `s_TestSkipClampCurrent`, the `MoveBonesToTarget` limit flag
    /// (1, 1, 1 in .data; never written).
    pub static BONE_TEST_FLAGS @ 0x8D2BD0: [u8; 3] = [1, 1, 1];
    /// `g_boneNodeMan`
    pub static G_BONE_NODE_MAN @ 0xC10820: [u8; 0x4C0C] = [0; 0x4C0C];
}

/// `ms_boneInfos` entry `i` (wrapping: `i = -1` is the 0x28 bytes before the table).
pub fn bone_info(i: i32) -> GPtr<u8> {
    BONE_INFOS_AREA.ptr().cast::<u8>().byte_add(0x28 + (i as isize) * 0x28)
}

/// `s_TestSkipClampDefault` (0x8D2BD0).
fn skip_clamp_default() -> u8 {
    BONE_TEST_FLAGS.get()[0]
}
/// `s_TestSkipClampCurrent` (0x8D2BD1).
fn skip_clamp_current() -> u8 {
    BONE_TEST_FLAGS.get()[1]
}
/// The flag `IKChain_c::MoveBonesToTarget` tests before `Limit` (0x8D2BD2).
pub fn limit_flag() -> u8 {
    BONE_TEST_FLAGS.get()[2]
}

/// `g_boneNodeMan` (0xC10820).
pub fn bone_node_man() -> GPtr<u8> {
    G_BONE_NODE_MAN.ptr().cast()
}

// .rdata constants
const ONE: f32 = 1.0; // 0x858624
const MINUS_ONE: f32 = -1.0; // 0x858C1C
const DEG180: f32 = 180.0; // 0x85A994
const INV_PI: f32 = f32::from_bits(0x3EA2_F983); // 0x86D2B4
const PI: f32 = f32::from_bits(0x4049_0FDB); // 0x858CB8
const INV180: f32 = f32::from_bits(0x3BB6_0B61); // 0x85F0AC
const HALF: f32 = 0.5; // 0x858B8C
const TWO: f32 = 2.0; // 0x858CA0
const ZERO: f32 = 0.0; // 0x858B50
const HEAD_FADE: f32 = f32::from_bits(0x3CB6_0B61); // 0x86D2B8 (1/45)
/// The sine polynomial of the inlined `RtQuatSlerp` (0x86D2B0, 0x86D2AC, 0x86D2A8, 0x86D2A4,
/// 0x864E0C, 0x85F0A0).
const SIN_C: [u32; 6] = [0x2F2E_C9D3, 0x32D7_2F34, 0x3638_EF1B, 0x3950_0D01, 0x3C08_8889, 0x3E2A_AAAB];

orig! {
    /// `ListItem_c::ListItem_c` (0x4A8DB0): next = prev = 0, returns this
    pub fn list_item_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8DB0 => unported;
    /// `ListItem_c::~ListItem_c` (0x4A8DC0): `ret`
    pub fn list_item_dtor(this: GPtr<u8>) = thiscall 0x4A8DC0 => unported;
    /// `TList_c::TList_c` (0x4A8DD0): head = tail = count = 0, returns this
    pub fn tlist_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8DD0 => unported;
    /// `TList_c::~TList_c` (0x4A8DE0): `ret`
    pub fn tlist_dtor(this: GPtr<u8>) = thiscall 0x4A8DE0 => unported;
    /// `TList_c::AddItem(item)` (0x4A8DF0): push front
    pub fn tlist_add_item(this: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `TList_c::RemoveItem(item)` (0x4A8E30)
    pub fn tlist_remove_item(this: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8E30 => unported;
    /// `TList_c::RemoveHead()` (0x4A8E70): the head or 0
    pub fn tlist_remove_head(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8E70 => unported;
    /// `TList_c::RemoveAll()` (0x4A8EB0)
    pub fn tlist_remove_all(this: GPtr<u8>) = thiscall 0x4A8EB0 => unported;
    /// `TList_c::GetNumItems()` (0x4A8EC0)
    pub fn tlist_get_num_items(this: GPtr<u8>) -> i32 = thiscall 0x4A8EC0 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `RwMatrixMultiply(dst, a, b)` (0x7F18B0)
    pub fn rw_matrix_multiply(dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => unported;
    /// `RtQuatSetupSlerpCache(from, to, cache)` (0x7EC220)
    pub fn rt_quat_setup_slerp_cache(from: GPtr<f32>, to: GPtr<f32>, cache: GPtr<f32>) = cdecl 0x7EC220 => unported;
    /// `BoneNode_c::CalcWldMat(parentMatrix)` (0x616CD0)
    pub fn call_calc_wld_mat(this: GPtr<u8>, parent: GPtr<u8>) = thiscall 0x616CD0 => calc_wld_mat;
    /// `BoneNode_c::QuatToEuler(quat, out)` (0x617080)
    pub fn call_quat_to_euler(q: GPtr<f32>, out: GPtr<f32>) = cdecl 0x617080 => quat_to_euler;
    /// `BoneNode_c::EulerToQuat(in, quat)` (0x6171F0)
    pub fn call_euler_to_quat(e: GPtr<f32>, q: GPtr<f32>) = cdecl 0x6171F0 => euler_to_quat;
    /// `BoneNode_c::InitLimits()` (0x617490)
    pub fn call_init_limits(this: GPtr<u8>) = thiscall 0x617490 => init_limits;
}

/// `List_c::AddItem(ListItem_c*)` (0x4A8DF0, thiscall; the doubly linked list of FX, bones and IK
/// chains): push front. `item->prev = NULL`, `item->next = head`, `head = item`; the old head's
/// `prev` = item, or `tail = item` when the list was empty; `count += 1`.
#[hook(addr = 0x4A8DF0, conv = thiscall, name = "FUN_004a8df0", expect = "8B 11 85 D2 8B 44 24 04",
       unverified = "std4-core-1", route = "L2", hazards = "none", recs = "boot")]
pub fn list_add_item(this: GPtr<u8>, item: GPtr<u8>) {
    let head = this.cast::<u32>().read();
    this.cast::<u32>().write(item.addr() as u32);
    item.cast::<u32>().write(0);
    item.byte_add(4).cast::<u32>().write(head);
    if head != 0 {
        GPtr::<u32>::from_addr(head as usize).write(item.addr() as u32);
    } else {
        this.byte_add(4).cast::<u32>().write(item.addr() as u32);
    }
    let c = this.byte_add(8).cast::<u32>();
    c.write(c.read().wrapping_add(1));
}

/// `BoneNode_c::BoneNode_c` (0x616B30): `ListItem_c` ctor, `m_Children` (`TList_c`) ctor; returns
/// this. The SEH frame around the member constructors is not built (stack only).
#[hook(addr = 0x616B30, conv = thiscall, name = "BoneNode_c::Constructor", expect = "6A FF 68 A8 EF 83 00", this = "BoneNode_c")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    tlist_ctor(this.byte_add(0x30));
    this
}

/// `BoneNode_c::~BoneNode_c` (0x616B80): `TList_c` dtor on `m_Children`, `ListItem_c` dtor (both
/// `ret`). EAX is left over from the SEH prologue (`fs:[0]`): not reproduced (void).
#[hook(addr = 0x616B80, conv = thiscall, name = "BoneNode_c::Destructor", expect = "6A FF 68 C8 EF 83 00", this = "BoneNode_c")]
pub fn destructor(this: GPtr<u8>) {
    tlist_dtor(this.byte_add(0x30));
    list_item_dtor(this);
}

/// `BoneNode_c::AddChild(child)` (0x616BD0): `child->m_Parent = this`, tail-jump
/// `m_Children.AddItem(child)`.
#[hook(addr = 0x616BD0, conv = thiscall, name = "BoneNode_c::AddChild", expect = "8B 44 24 04 89 48 2C", this = "BoneNode_c")]
pub fn add_child(this: GPtr<u8>, child: GPtr<u8>) {
    wr::<u32>(child, 0x2C, this.addr() as u32);
    tlist_add_item(this.byte_add(0x30), child);
}

/// `BoneNode_c::GetLimits(axis, &min, &max)` (0x616BF0): axis 0/1/2 copy `m_LimitMin[axis]` /
/// `m_LimitMax[axis]` (raw dwords); other axes: nothing.
#[hook(addr = 0x616BF0, conv = thiscall, name = "BoneNode_c::GetLimits", expect = "8B 44 24 04 85 C0", this = "BoneNode_c")]
pub fn get_limits(this: GPtr<u8>, axis: i32, min: GPtr<u32>, max: GPtr<u32>) {
    if (0..3).contains(&axis) {
        let o = axis as isize * 4;
        min.write(rd::<u32>(this, 0x7C + o));
        max.write(rd::<u32>(this, 0x88 + o));
    }
}

/// `BoneNode_c::SetLimits(axis, min, max)` (0x616C50): the mirror of [`get_limits`].
#[hook(addr = 0x616C50, conv = thiscall, name = "BoneNode_c::SetLimits", expect = "8B 44 24 04 85 C0", this = "BoneNode_c")]
pub fn set_limits(this: GPtr<u8>, axis: i32, min: u32, max: u32) {
    if (0..3).contains(&axis) {
        let o = axis as isize * 4;
        wr::<u32>(this, 0x7C + o, min);
        wr::<u32>(this, 0x88 + o, max);
    }
}

/// `BoneNode_c::GetSpeed()` (0x616CB0): `fld m_Speed`.
#[hook(addr = 0x616CB0, conv = thiscall, name = "BoneNode_c::GetSpeed", expect = "D9 81 94 00 00 00", this = "BoneNode_c")]
pub fn get_speed(this: GPtr<u8>) -> X87 {
    x(rd::<f32>(this, 0x94))
}

/// `BoneNode_c::SetSpeed(speed)` (0x616CC0): raw dword store.
#[hook(addr = 0x616CC0, conv = thiscall, name = "BoneNode_c::SetSpeed", expect = "8B 44 24 04 89 81 94 00 00 00", this = "BoneNode_c")]
pub fn set_speed(this: GPtr<u8>, speed: u32) {
    wr::<u32>(this, 0x94, speed);
}

/// The index of the first `ms_boneInfos` entry whose int16 tag (sign-extended) equals `tag`, else -1.
pub fn find_bone_info(tag: i32) -> i32 {
    for i in 0..32 {
        if i32::from(rd::<i16>(bone_info(i), 0)) == tag {
            return i;
        }
    }
    -1
}

/// `BoneNode_c::GetIdFromBoneTag(tag)` (0x617050, cdecl): index of the tag in `ms_boneInfos`, -1 if
/// none.
#[hook(addr = 0x617050, conv = cdecl, name = "BoneNode_c::GetIdFromBoneTag", expect = "8B 54 24 04 33 C0")]
pub fn get_id_from_bone_tag(tag: i32) -> i32 {
    find_bone_info(tag)
}

/// `atan2(y, x) * 180 * (1/pi)` as two f32 multiplies after FPATAN.
fn deg(y: X87, xx: X87) -> f32 {
    y.fpatan(xx).fmul_m(DEG180).fmul_m(INV_PI).to_f32()
}

/// `BoneNode_c::QuatToEuler(const RtQuat* q, CVector* out)` (0x617080, cdecl). See the deviation
/// notes: exact `n == +1` / `n == -1` gimbal tests on the unrounded value, f32 temporaries, `1/c`
/// scaling inside the atan2 arguments.
#[hook(addr = 0x617080, conv = cdecl, name = "BoneNode_c::QuatToEuler", expect = "83 EC 24 8B 4C 24 28")]
pub fn quat_to_euler(q: GPtr<f32>, out: GPtr<f32>) {
    let qx = || q.read();
    let qy = || q.wrapping_index(1).read();
    let qz = || q.wrapping_index(2).read();
    let qw = || q.wrapping_index(3).read();
    let y2 = x(qy()).fmul(x(qy())).fdouble();
    let z = x(qz());
    let l0 = x(ONE).fsub(y2).fsub(z.fmul(z).fdouble()).to_f32();
    let lc = x(qz()).fmul_m(qw()).fdouble().fadd(x(qx()).fmul_m(qy()).fdouble()).to_f32();
    let v = x(qz()).fmul_m(qx()).fdouble().fsub(x(qw()).fmul_m(qy()).fdouble());
    let l1c = x(qz()).fmul_m(qy()).fdouble().fadd(x(qw()).fmul_m(qx()).fdouble()).to_f32();
    let xx = x(qx());
    let l20 = xx.fmul(xx).fdouble().fsubr_m(ONE).fsub(y2).to_f32();
    let n = v.fchs();
    let c = n.fmul(n).fsubr_m(ONE).fsqrt().to_f32();
    out.wrapping_index(1).write(deg(n, x(c)));
    if n.fcom(x(ONE)) == Cmp::Equal || n.fcom(x(MINUS_ONE)) == Cmp::Equal {
        // gimbal lock
        let gx = x(qx());
        let g = gx.fmul(gx).fdouble().fsubr_m(ONE);
        let gz = x(qz());
        let g = g.fsub(gz.fmul(gz).fdouble());
        let h = x(qz()).fmul_m(qy()).fdouble().fsub(x(qw()).fmul_m(qx()).fdouble()).fchs();
        out.write(deg(h, g));
        out.wrapping_index(2).write(deg(X87::from_f64_exact(0.0), X87::from_f64_exact(1.0)));
        return;
    }
    let r = x(ONE).fdiv_m(c);
    let a = x(l1c).fmul(r);
    let b = r.fmul_m(l20);
    out.write(deg(a, b));
    let a = x(lc).fmul(r);
    let b = r.fmul_m(l0);
    out.wrapping_index(2).write(deg(a, b));
}

/// `BoneNode_c::EulerToQuat(const CVector* in, RtQuat* out)` (0x6171F0, cdecl): degrees → half
/// radians (`* pi * (1/180) * 0.5`), FSIN/FCOS, f32 temporaries as the original stores them, and the
/// result scaled by `1 / |q|²` (no square root: the original's quirk).
#[hook(addr = 0x6171F0, conv = cdecl, name = "BoneNode_c::EulerToQuat", expect = "83 EC 10 8B 44 24 14")]
pub fn euler_to_quat(e: GPtr<f32>, out: GPtr<f32>) {
    let half = |v: f32| x(v).fmul_m(PI).fmul_m(INV180).fmul_m(HALF);
    let ax = half(e.read());
    let ay = half(e.wrapping_index(1).read());
    let t8 = half(e.wrapping_index(2).read()).to_f32();
    let t4 = ax.fcos().to_f32(); // cos x
    let t14 = ay.fcos().to_f32(); // cos y
    let t0 = x(t8).fcos().to_f32(); // cos z
    let tc = ax.fsin().to_f32(); // sin x
    let sy = ay.fsin();
    let sz = x(t8).fsin();
    let czcx = x(t0).fmul_m(t4).to_f32();
    let p = x(t4).fmul(sz); // cx * sz
    let qq = x(tc).fmul_m(t0); // sx * cz
    let szsx = sz.fmul_m(tc).to_f32();
    let r = x(t14).fmul(qq).fsub(p.fmul(sy)).to_f32();
    let t4b = x(czcx).fmul(sy).fadd(x(szsx).fmul_m(t14)).to_f32();
    let s = p.fmul_m(t14).fsub(qq.fmul(sy));
    let w = x(szsx).fmul(sy).fadd(x(czcx).fmul_m(t14));
    let wf = w.to_f32();
    let sum = w
        .fmul_m(wf)
        .fadd(s.fmul(s))
        .fadd(x(t4b).fmul_m(t4b))
        .fadd(x(r).fmul_m(r));
    let inv = x(ONE).fdiv(sum).to_f32();
    out.write(x(inv).fmul_m(r).to_f32());
    out.wrapping_index(1).write(x(inv).fmul_m(t4b).to_f32());
    out.wrapping_index(2).write(x(inv).fmul(s).to_f32());
    out.wrapping_index(3).write(x(inv).fmul_m(wf).to_f32());
}

/// `BoneNode_c::CalcWldMat(const RwMatrix* parent)` (0x616CD0): the rotation matrix of
/// `m_Orientation` (scaled by `2 / |q|²`) with `m_Pos`, flags 3, into a local `RwMatrix`;
/// `RwMatrixMultiply(&m_WorldMat, &local, parent)`; then every child (`m_Children`, next at +4)
/// recursively with `&m_WorldMat`. The local's padding words are 0 here (uninitialised stack in the
/// original; `RwMatrixMultiply` does not read them).
#[hook(addr = 0x616CD0, conv = thiscall, name = "BoneNode_c::CalcWldMat", expect = "83 EC 70 56 8B F1", this = "BoneNode_c")]
pub fn calc_wld_mat(this: GPtr<u8>, parent: GPtr<u8>) {
    let f = |o: isize| rd::<f32>(this, o);
    let (qx, qy, qz, qw) = (x(f(0x10)), x(f(0x14)), x(f(0x18)), x(f(0x1C)));
    let n = qx.fmul(qx).fadd(qy.fmul(qy)).fadd(qz.fmul(qz)).fadd(qw.fmul(qw));
    let k = n.fdivr_m(TWO);
    let kx = k.fmul_m(f(0x10));
    let a24 = k.fmul_m(f(0x14)).to_f32(); // ky
    let kz = k.fmul_m(f(0x18));
    let a8 = kx.fmul_m(f(0x1C)).to_f32(); // kx*w
    let ac = x(a24).fmul_m(f(0x1C)).to_f32(); // ky*w
    let a10 = kz.fmul_m(f(0x1C)).to_f32(); // kz*w
    let a14 = kx.fmul_m(f(0x10)).to_f32(); // kx*x
    let a18 = x(a24).fmul_m(f(0x14)).to_f32(); // ky*y
    let a1c = kz.fmul_m(f(0x18)).to_f32(); // kz*z
    let a2c = kz.fmul_m(f(0x14)).to_f32(); // kz*y
    let b = kx.fmul_m(f(0x18)); // kx*z
    let c = x(a24).fmul_m(f(0x10)); // ky*x
    let mut m = [0u32; 16];
    let set = |m: &mut [u32; 16], i: usize, v: f32| m[i] = v.to_bits();
    set(&mut m, 0, x(a1c).fadd_m(a18).fsubr_m(ONE).to_f32());
    set(&mut m, 1, x(a10).fadd(c).to_f32());
    set(&mut m, 2, b.fsub_m(ac).to_f32());
    m[3] = 3;
    set(&mut m, 4, c.fsub_m(a10).to_f32());
    set(&mut m, 5, x(a1c).fadd_m(a14).fsubr_m(ONE).to_f32());
    set(&mut m, 6, x(a2c).fadd_m(a8).to_f32());
    set(&mut m, 8, b.fadd_m(ac).to_f32());
    set(&mut m, 9, x(a2c).fsub_m(a8).to_f32());
    set(&mut m, 10, x(a18).fadd_m(a14).fsubr_m(ONE).to_f32());
    m[12] = rd::<u32>(this, 0x20);
    m[13] = rd::<u32>(this, 0x24);
    m[14] = rd::<u32>(this, 0x28);
    let world = this.byte_add(0x3C);
    rw_matrix_multiply(world, GPtr::from_raw(m.as_mut_ptr()).cast(), parent);
    let mut child = rd::<GPtr<u8>>(this, 0x30);
    while !child.is_null() {
        call_calc_wld_mat(child, world);
        child = rd::<GPtr<u8>>(child, 4);
    }
}

/// `BoneNode_c::BlendKeyframe(float blend)` (0x616E30): blend >= 1.0 (or NaN): the interp frame's
/// quaternion = `m_Orientation`; blend <= 0: nothing; otherwise the inlined `RtQuatSlerp` from the
/// frame's quaternion to `m_Orientation` (`RtQuatSetupSlerpCache`, then the 6-term sine polynomial
/// unless the cache's `nearlyZeroOm`).
#[hook(addr = 0x616E30, conv = thiscall, name = "BoneNode_c::BlendKeyframe", expect = "D9 44 24 04 83 EC 48", this = "BoneNode_c")]
pub fn blend_keyframe(this: GPtr<u8>, blend: f32) {
    let frame = || rd::<GPtr<u32>>(this, 0xC);
    let orient = at::<u32>(this, 0x10);
    let b = x(blend);
    // fcomp qword 1.0; test ah, 1: C0 (less or unordered) jumps to the blend path
    if !matches!(b.fcom(X87::from_f64_exact(1.0)), Cmp::Less | Cmp::Unordered) {
        let fr = frame();
        for k in 0..4 {
            fr.wrapping_index(k).write(orient.wrapping_index(k).read());
        }
        return;
    }
    // test ah, 0x41 / jne: less, equal (or unordered) return
    if b.fcom(x(ZERO)) != Cmp::Greater {
        return;
    }
    let mut from = [0u32; 4];
    let mut to = [0u32; 4];
    let fr = frame();
    for k in 0..4 {
        from[k] = fr.wrapping_index(k as i32).read();
        to[k] = orient.wrapping_index(k as i32).read();
    }
    // cache: raFrom (4), raTo (4), omega, nearlyZeroOm
    let mut cache = [0u32; 10];
    rt_quat_setup_slerp_cache(
        GPtr::from_raw(from.as_mut_ptr()).cast(),
        GPtr::from_raw(to.as_mut_ptr()).cast(),
        GPtr::from_raw(cache.as_mut_ptr()).cast(),
    );
    // test ah, 0x41 / jp: blend <= 0 → from (unreachable here, kept)
    if matches!(b.fcom(x(ZERO)), Cmp::Less | Cmp::Equal) {
        let fr = frame();
        for (k, v) in from.iter().enumerate() {
            fr.wrapping_index(k as i32).write(*v);
        }
        return;
    }
    // 1.0 <= blend → to (unreachable here, kept)
    if matches!(x(ONE).fcom(b), Cmp::Less | Cmp::Equal) {
        let fr = frame();
        for (k, v) in to.iter().enumerate() {
            fr.wrapping_index(k as i32).write(*v);
        }
        return;
    }
    let cf = |i: usize| f32::from_bits(cache[i]);
    let one_minus = x(ONE).fsub_m(blend);
    let (s_to, s_from) = if cache[9] != 0 {
        (b, one_minus)
    } else {
        let sin = |t: X87| {
            let t2 = t.fmul(t);
            let p = x(f32::from_bits(SIN_C[0]))
                .fmul(t2)
                .fsub_m(f32::from_bits(SIN_C[1]))
                .fmul(t2)
                .fadd_m(f32::from_bits(SIN_C[2]))
                .fmul(t2)
                .fsub_m(f32::from_bits(SIN_C[3]))
                .fmul(t2)
                .fadd_m(f32::from_bits(SIN_C[4]))
                .fmul(t2)
                .fsub_m(f32::from_bits(SIN_C[5]));
            t.fadd(t2.fmul(t).fmul(p))
        };
        let sa = sin(one_minus.fmul_m(cf(8)));
        let sb = sin(x(cf(8)).fmul_m(blend));
        (sb, sa)
    };
    let fr = || frame().cast::<f32>();
    for k in 0..3 {
        fr().wrapping_index(k).write(x(cf(k as usize)).fmul(s_from).to_f32());
    }
    for k in 0..3 {
        let p = fr().wrapping_index(k);
        p.write(x(cf(4 + k as usize)).fmul(s_to).fadd_m(p.read()).to_f32());
    }
    let w = x(cf(7)).fmul(s_to).fadd(x(cf(3)).fmul(s_from)).to_f32();
    fr().wrapping_index(3).write(w);
}

/// `BoneNode_c::InitLimits()` (0x617490): the bone's `ms_boneInfos` entry (index -1 for an unknown
/// tag: the 0x28 bytes before the table): `min = pose - minRange`, `max = maxRange + pose` per axis.
#[hook(addr = 0x617490, conv = thiscall, name = "BoneNode_c::InitLimits", expect = "83 EC 28 56 8B D1", this = "BoneNode_c")]
pub fn init_limits(this: GPtr<u8>) {
    let e = bone_info(find_bone_info(rd::<i32>(this, 8)));
    let ef = |o: isize| rd::<f32>(e, o);
    wr::<f32>(this, 0x7C, x(ef(4)).fsub_m(ef(0x10)).to_f32());
    wr::<f32>(this, 0x80, x(ef(8)).fsub_m(ef(0x18)).to_f32());
    wr::<f32>(this, 0x84, x(ef(0xC)).fsub_m(ef(0x20)).to_f32());
    wr::<f32>(this, 0x88, x(ef(0x14)).fadd_m(ef(4)).to_f32());
    wr::<f32>(this, 0x8C, x(ef(0x1C)).fadd_m(ef(8)).to_f32());
    wr::<f32>(this, 0x90, x(ef(0x24)).fadd_m(ef(0xC)).to_f32());
}

/// `BoneNode_c::ClampLimitsDefault(bool x, bool y, bool z)` (0x617530): only when
/// `s_TestSkipClampDefault` is 0 (it is 1 in the shipped .data): per selected axis `min = max =`
/// the pose rotation of the bone's entry (entry -1 for an unknown tag; raw dwords).
#[hook(addr = 0x617530, conv = thiscall, name = "BoneNode_c::ClampLimitsDefault", expect = "A0 D0 2B 8D 00", this = "BoneNode_c")]
pub fn clamp_limits_default(this: GPtr<u8>, lx: u8, ly: u8, lz: u8) {
    if skip_clamp_default() != 0 {
        return;
    }
    let e = bone_info(find_bone_info(rd::<i32>(this, 8)));
    for (k, f) in [lx, ly, lz].into_iter().enumerate() {
        if f != 0 {
            let v = rd::<u32>(e, 4 + 4 * k as isize);
            wr::<u32>(this, 0x88 + 4 * k as isize, v);
            wr::<u32>(this, 0x7C + 4 * k as isize, v);
        }
    }
}

/// `BoneNode_c::ClampLimitsCurrent(bool x, bool y, bool z)` (0x6175D0): only when
/// `s_TestSkipClampCurrent` is 0 (1 in .data): per selected axis `min = max =` the current Euler
/// angle (`QuatToEuler(&m_Orientation)`).
#[hook(addr = 0x6175D0, conv = thiscall, name = "BoneNode_c::ClampLimitsCurrent", expect = "A0 D1 2B 8D 00", this = "BoneNode_c")]
pub fn clamp_limits_current(this: GPtr<u8>, lx: u8, ly: u8, lz: u8) {
    if skip_clamp_current() != 0 {
        return;
    }
    let mut a = [0u32; 3];
    call_quat_to_euler(at(this, 0x10), GPtr::from_raw(a.as_mut_ptr()).cast());
    for (k, f) in [lx, ly, lz].into_iter().enumerate() {
        if f != 0 {
            wr::<u32>(this, 0x88 + 4 * k as isize, a[k]);
            wr::<u32>(this, 0x7C + 4 * k as isize, a[k]);
        }
    }
}

/// `if (a < lo) a = lo` with `test ah, 5 / jp` (only "less" stores).
fn lt(a: f32, lo: X87) -> bool {
    x(a).fcom(lo) == Cmp::Less
}
/// `if (a > hi) a = hi` with `test ah, 0x41 / jne` (only "greater" stores).
fn gt(a: f32, hi: f32) -> bool {
    x(a).fcom(x(hi)) == Cmp::Greater
}

/// `BoneNode_c::Limit(float blend)` (0x617650; `blend` unused): clamp the Euler angles of
/// `m_Orientation` to the limits, min first then max (min > max gives max); x and z of the
/// head (tag 5) fade toward the pose with `1 - |x| * (1/45)` (not below 0); back to the quaternion.
#[hook(addr = 0x617650, conv = thiscall, name = "BoneNode_c::Limit", expect = "83 EC 38 53 56", this = "BoneNode_c")]
pub fn limit(this: GPtr<u8>, _blend: f32) {
    let mut a = [0u32; 3];
    call_quat_to_euler(at(this, 0x10), GPtr::<u32>::from_raw(a.as_mut_ptr()).cast());
    let af = |a: &[u32; 3], k: usize| f32::from_bits(a[k]);
    // x: raw dword copies
    if lt(af(&a, 0), x(rd::<f32>(this, 0x7C))) {
        a[0] = rd::<u32>(this, 0x7C);
    }
    if gt(af(&a, 0), rd::<f32>(this, 0x88)) {
        a[0] = rd::<u32>(this, 0x88);
    }
    // y: the minimum goes through the x87 (fld / fstp)
    let lo_y = x(rd::<f32>(this, 0x80));
    let hi_y = rd::<f32>(this, 0x8C);
    if lt(af(&a, 1), lo_y) {
        a[1] = lo_y.to_f32().to_bits();
    }
    if gt(af(&a, 1), hi_y) {
        a[1] = hi_y.to_bits();
    }
    // z
    let mut lo = x(rd::<f32>(this, 0x84));
    let mut hi = rd::<f32>(this, 0x90);
    if rd::<i32>(this, 8) == 5 {
        let mut idx = -1;
        for i in 0..32 {
            if rd::<i16>(bone_info(i), 0) == 5 {
                idx = i;
                break;
            }
        }
        let pz = rd::<f32>(bone_info(idx), 0xC);
        let ax = X87::from_f64_exact(x(af(&a, 0)).to_f64().abs());
        let mut f = ax.fmul_m(HEAD_FADE).fsubr_m(ONE);
        if x(ZERO).fcom(f) == Cmp::Greater {
            f = x(ZERO);
        }
        lo = lo.fsub_m(pz).fmul(f).fadd_m(pz);
        hi = x(hi).fsub_m(pz).fmul(f).fadd_m(pz).to_f32();
    }
    if lt(af(&a, 2), lo) {
        a[2] = lo.to_f32().to_bits();
    }
    if gt(af(&a, 2), hi) {
        a[2] = hi.to_bits();
    }
    call_euler_to_quat(GPtr::<u32>::from_raw(a.as_mut_ptr()).cast(), at(this, 0x10));
}

/// `BoneNode_c::Init(tag, interpFrame)` (0x6177B0): tag, frame pointer, the frame's quaternion and
/// translation into `m_Orientation` / `m_Pos`, no parent, `m_Children.RemoveAll()`, `InitLimits()`,
/// speed 1.0; returns true (AL).
#[hook(addr = 0x6177B0, conv = thiscall, name = "BoneNode_c::Init", expect = "8B 44 24 04 56", this = "BoneNode_c")]
pub fn init(this: GPtr<u8>, tag: i32, frame: GPtr<u8>) -> u8 {
    wr::<i32>(this, 8, tag);
    wr::<u32>(this, 0xC, frame.addr() as u32);
    for k in 0..4 {
        wr::<u32>(this, 0x10 + 4 * k, rd::<u32>(frame, 4 * k));
    }
    for k in 0..3 {
        wr::<u32>(this, 0x20 + 4 * k, rd::<u32>(frame, 0x10 + 4 * k));
    }
    wr::<u32>(this, 0x2C, 0);
    tlist_remove_all(this.byte_add(0x30));
    call_init_limits(this);
    wr::<u32>(this, 0x94, 0x3F80_0000);
    1
}

/// `BoneNodeManager_c::BoneNodeManager_c` (0x617330): `eh vector constructor iterator` over the 128
/// nodes (0x98 each, ctor 0x616B30, dtor 0x616B80), the free list's `TList_c` ctor; returns this.
#[hook(addr = 0x617330, conv = thiscall, name = "BoneNodeManager_c::Constructor", expect = "6A FF 68 F9 EF 83 00", this = "BoneNodeManager_c")]
pub fn manager_constructor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 0x98, 0x80, 0x616B30, 0x616B80);
    tlist_ctor(this.byte_add(0x4C00));
    this
}

/// `BoneNodeManager_c::~BoneNodeManager_c` (0x617390): the free list's dtor, then `eh vector
/// destructor iterator` over the nodes.
#[hook(addr = 0x617390, conv = thiscall, name = "BoneNodeManager_c::Destructor", expect = "6A FF 68 29 F0 83 00", this = "BoneNodeManager_c")]
pub fn manager_destructor(this: GPtr<u8>) {
    tlist_dtor(this.byte_add(0x4C00));
    eh_vector_dtor(this, 0x98, 0x80, 0x616B80);
}

/// `BoneNodeManager_c::Init()` (0x6173F0): every node onto the free list (in index order, each at the
/// head); returns true (AL).
#[hook(addr = 0x6173F0, conv = thiscall, name = "BoneNodeManager_c::Init", expect = "53 56 57 8D 99 00 4C 00 00", this = "BoneNodeManager_c")]
pub fn manager_init(this: GPtr<u8>) -> u8 {
    let list = this.byte_add(0x4C00);
    for i in 0..0x80 {
        tlist_add_item(list, this.byte_add(i * 0x98));
    }
    1
}

/// `BoneNodeManager_c::Exit()` (0x617420): tail-jump `RemoveAll` of the free list.
#[hook(addr = 0x617420, conv = thiscall, name = "BoneNodeManager_c::Exit", expect = "81 C1 00 4C 00 00", this = "BoneNodeManager_c")]
pub fn manager_exit(this: GPtr<u8>) {
    tlist_remove_all(this.byte_add(0x4C00));
}

/// `BoneNodeManager_c::Reset()` (0x617430): `RemoveAll`, then every node back onto the free list.
#[hook(addr = 0x617430, conv = thiscall, name = "BoneNodeManager_c::Reset", expect = "53 56 8B F1 57", this = "BoneNodeManager_c")]
pub fn manager_reset(this: GPtr<u8>) {
    let list = this.byte_add(0x4C00);
    tlist_remove_all(list);
    for i in 0..0x80 {
        tlist_add_item(list, this.byte_add(i * 0x98));
    }
}

/// `BoneNodeManager_c::GetBoneNode()` (0x617460): tail-jump `RemoveHead` of the free list. (The
/// dword 0x00617461 found in .data/.rdata is the string "ata", not a read of the entry.)
#[hook(addr = 0x617460, conv = thiscall, name = "BoneNodeManager_c::GetBoneNode", expect = "81 C1 00 4C 00 00", this = "BoneNodeManager_c")]
pub fn manager_get_bone_node(this: GPtr<u8>) -> GPtr<u8> {
    tlist_remove_head(this.byte_add(0x4C00))
}

/// `BoneNodeManager_c::ReturnBoneNode(node)` (0x617470): tail-jump `AddItem` on the free list.
#[hook(addr = 0x617470, conv = thiscall, name = "BoneNodeManager_c::ReturnBoneNode", expect = "81 C1 00 4C 00 00", this = "BoneNodeManager_c")]
pub fn manager_return_bone_node(this: GPtr<u8>, node: GPtr<u8>) {
    tlist_add_item(this.byte_add(0x4C00), node);
}

/// `BoneNodeManager_c::GetNumBoneNodesLeft()` (0x617480): tail-jump `GetNumItems` of the free list.
#[hook(addr = 0x617480, conv = thiscall, name = "BoneNodeManager_c::GetNumBoneNodesLeft", expect = "81 C1 00 4C 00 00", this = "BoneNodeManager_c")]
pub fn manager_get_num_bone_nodes_left(this: GPtr<u8>) -> i32 {
    tlist_get_num_items(this.byte_add(0x4C00))
}

pub const HOOKS: &[HookReg] = &[
    HOOK_LIST_ADD_ITEM,
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_ADD_CHILD,
    HOOK_GET_LIMITS,
    HOOK_SET_LIMITS,
    HOOK_GET_SPEED,
    HOOK_SET_SPEED,
    HOOK_GET_ID_FROM_BONE_TAG,
    HOOK_QUAT_TO_EULER,
    HOOK_EULER_TO_QUAT,
    HOOK_CALC_WLD_MAT,
    HOOK_BLEND_KEYFRAME,
    HOOK_INIT_LIMITS,
    HOOK_CLAMP_LIMITS_DEFAULT,
    HOOK_CLAMP_LIMITS_CURRENT,
    HOOK_LIMIT,
    HOOK_INIT,
    HOOK_MANAGER_CONSTRUCTOR,
    HOOK_MANAGER_DESTRUCTOR,
    HOOK_MANAGER_INIT,
    HOOK_MANAGER_EXIT,
    HOOK_MANAGER_RESET,
    HOOK_MANAGER_GET_BONE_NODE,
    HOOK_MANAGER_RETURN_BONE_NODE,
    HOOK_MANAGER_GET_NUM_BONE_NODES_LEFT,
];

/// `ms_boneInfos` with the 0x28 bytes before it, as in the exe's .data (0x8D26A8..0x8D2BD0).
const BONE_INFOS_INIT: [u8; 0x528] = [
    0x36, 0x00, 0x00, 0x00, 0x36, 0x00, 0x00, 0x00, 0x36, 0x00, 0x00, 0x00, 0x37, 0x00, 0x00, 0x00, 0x37, 0x00, 0x00, 0x00,
    0x37, 0x00, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xFF, 0xFF, 0xCD, 0xCC, 0x4C, 0xBE, 0xCD, 0xCC, 0xCC, 0x3E, 0x9A, 0x99, 0xAE, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34, 0xC3, 0x00, 0x00, 0xB1, 0xC2, 0x00, 0x00, 0xB4, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x01, 0x00, 0x66, 0x66, 0x06, 0x40, 0x00, 0x00, 0x00, 0x00, 0xCD, 0xCC, 0x4C, 0xBF, 0x00, 0x00, 0xF0, 0x41,
    0x00, 0x00, 0xF0, 0x41, 0x00, 0x00, 0xA0, 0x41, 0x00, 0x00, 0xA0, 0x41, 0x00, 0x00, 0xA0, 0x40, 0x00, 0x00, 0x70, 0x42,
    0x03, 0x00, 0x02, 0x00, 0x66, 0x66, 0x26, 0x40, 0xCD, 0xCC, 0xCC, 0x3E, 0x33, 0x33, 0xC3, 0x40, 0x00, 0x00, 0x20, 0x41,
    0x00, 0x00, 0x20, 0x41, 0x00, 0x00, 0x20, 0x41, 0x00, 0x00, 0x20, 0x41, 0x00, 0x00, 0x70, 0x41, 0x00, 0x00, 0x20, 0x41,
    0x04, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x04, 0x00, 0x66, 0x66, 0xE6, 0xBF, 0x00, 0x00, 0x00, 0xBF, 0x9A, 0x99, 0xE9, 0xC0, 0x00, 0x00, 0x82, 0x42,
    0x00, 0x00, 0x82, 0x42, 0x00, 0x00, 0x70, 0x41, 0x00, 0x00, 0x70, 0x41, 0x00, 0x00, 0x34, 0x42, 0x00, 0x00, 0x34, 0x42,
    0x06, 0x00, 0x05, 0x00, 0x33, 0x33, 0xB3, 0xBF, 0x00, 0x00, 0x10, 0xC1, 0x00, 0x00, 0xA3, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x07, 0x00, 0x05, 0x00, 0xCD, 0xCC, 0x4C, 0x40, 0x66, 0x66, 0xD6, 0x40, 0x66, 0x66, 0xA3, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDE, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x15, 0x00, 0x04, 0x00, 0x00, 0x00, 0xB0, 0x41, 0x33, 0x33, 0xA1, 0x42, 0x33, 0xB3, 0x26, 0xC3, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x16, 0x00, 0x15, 0x00, 0xCD, 0xCC, 0xAC, 0xC0, 0x00, 0x00, 0x34, 0xC2, 0x00, 0x00, 0x50, 0xC1, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE6, 0x42, 0x00, 0x00, 0xB4, 0x42, 0x00, 0x00, 0x34, 0x42,
    0x17, 0x00, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x9A, 0x99, 0x59, 0xC1, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x43, 0x00, 0x00, 0x50, 0x41,
    0x18, 0x00, 0x17, 0x00, 0xCD, 0xCC, 0xE2, 0x42, 0x00, 0x00, 0x80, 0xC0, 0x00, 0x00, 0xD0, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x19, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x9C, 0x41, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x1A, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xCD, 0xCC, 0x54, 0x41, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x1F, 0x00, 0x04, 0x00, 0x00, 0x00, 0xB0, 0xC1, 0x33, 0x33, 0xA1, 0xC2, 0x33, 0xB3, 0x26, 0xC3, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x20, 0x00, 0x1F, 0x00, 0xCD, 0xCC, 0xAC, 0x40, 0x66, 0x66, 0x36, 0x42, 0x00, 0x00, 0x50, 0xC1, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE6, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xB4, 0x42, 0x00, 0x00, 0x34, 0x42,
    0x21, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x9A, 0x99, 0x59, 0xC1, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x43, 0x00, 0x00, 0x50, 0x41,
    0x22, 0x00, 0x21, 0x00, 0xCD, 0xCC, 0xE2, 0xC2, 0x00, 0x00, 0x80, 0x40, 0x00, 0x00, 0xD0, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x23, 0x00, 0x22, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x9C, 0x41, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x24, 0x00, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xCD, 0xCC, 0x54, 0x41, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x29, 0x00, 0x01, 0x00, 0x00, 0x00, 0x30, 0x43, 0x66, 0x66, 0xF6, 0xC0, 0x66, 0xE6, 0x25, 0x43, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA0, 0x41, 0x00, 0x00, 0xA0, 0x40, 0x00, 0x00, 0xF0, 0x41, 0x00, 0x00, 0x00, 0x00,
    0x2A, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x33, 0x33, 0xC3, 0xC1, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDC, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x2B, 0x00, 0x2A, 0x00, 0x33, 0x33, 0xB3, 0xBF, 0x00, 0x00, 0xA0, 0xC0, 0x00, 0x00, 0x20, 0x41, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x2C, 0x00, 0x2B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xB4, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x33, 0x00, 0x01, 0x00, 0x00, 0x00, 0x30, 0xC3, 0x66, 0x66, 0xF6, 0x40, 0x66, 0xE6, 0x25, 0x43, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA0, 0x40, 0x00, 0x00, 0xA0, 0x41, 0x00, 0x00, 0xF0, 0x41, 0x00, 0x00, 0x00, 0x00,
    0x34, 0x00, 0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x33, 0x33, 0xC3, 0xC1, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDC, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x35, 0x00, 0x34, 0x00, 0x33, 0x33, 0xB3, 0x3F, 0x00, 0x00, 0xA0, 0x40, 0x00, 0x00, 0x20, 0x41, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x36, 0x00, 0x35, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xB4, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x2E, 0x01, 0x03, 0x00, 0x9A, 0x99, 0x71, 0x41, 0x33, 0x33, 0xF3, 0xBF, 0x00, 0x00, 0xA2, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x2D, 0x01, 0x03, 0x00, 0x00, 0x00, 0xC0, 0xC0, 0x00, 0x00, 0x00, 0x40, 0x9A, 0x99, 0xA8, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xC9, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x33, 0x33, 0xB0, 0x42, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
