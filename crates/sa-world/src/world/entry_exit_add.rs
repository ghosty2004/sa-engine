//! STD-5: `CEntryExitManager::AddOne` (0x43FA00), ported from the 1.0 US bytes (gta-reversed
//! `EntryExitManager.cpp` / `EntryExit.cpp` as the reading aid; the bytes win: the time window is
//! rewritten in the argument slots, the rectangle comes from two rotated half extents, the linked exit's
//! window is forced to 0..24). The SEH frame and the local `CMatrix`'s destructor bookkeeping are not
//! reproduced (its destructor is called).
//!
//! `CEntryExit` (0x3C, pool `[0x96A7D8]`): +0 name (8), +8 left, +0xC top... (+8/+0x14 = centre -
//! half range, +0x10/+0xC = centre + half range), +0x18 z + 1, +0x1C angle (radians), +0x20 exit
//! position (z + 1 at +0x28), +0x2C exit angle, u16 +0x30 flags, bytes +0x32 area, +0x33 sky colour,
//! +0x34 time on, +0x35 time off, +0x36 peds, +0x38 the linked entry/exit.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CPool<CEntryExit>::New()` (0x43F7D0)
    fn pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x43F7D0 => unported;
    /// CRT `rand()` (0x821B1E)
    fn rand() -> i32 = cdecl 0x821B1E => unported;
    /// CRT `strncpy(dst, src, n)` (0x821F40)
    fn crt_strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0)
    fn set_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B0E0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CQuadTreeNode::AddItem(void*, const CRect&)` (0x552CD0)
    fn quad_tree_add_item(node: GPtr<u8>, item: GPtr<u8>, r: GPtr<u8>) = thiscall 0x552CD0 => unported;
    /// `CEntryExitManager::GetEntryExitIndex(const char*, uint16 enabled, uint16 disabled)` (0x43EFD0)
    fn get_entry_exit_index(name: GPtr<u8>, enabled: u32, disabled: u32) -> i32 = cdecl 0x43EFD0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

const POOL: usize = 0x96A7D8;
const QUAD_TREE: usize = 0x96A7D0;
/// The return address of the `rand()` call (0x43FA7D).
const RAND_SITE: u32 = 0x43FA82;

fn f(p: GPtr<u8>, o: isize) -> GPtr<f32> {
    p.byte_add(o).cast()
}
fn c(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}

/// `CEntryExitManager::AddOne(centre x, y, z, angle, range x, y, z, exit x, y, z, exit angle, area,
/// flags, sky colour, time on, time off, peds, name)` (0x43FA00): 0 without a free pool slot. Flag
/// 0x1000 forces the window 0..24; flag 0x400 does too when `rand() < 0x3FFF` (0..0); flag 0x4000 is
/// set when `[0x96A730]` is set or 0x1000 is clear. The entry's rectangle is the bounding box of the
/// two corners `R(angle) * (range * k, 0)` and `R(angle) * (range / 2, 0)` (k = [0x858F40]) moved to
/// the centre; it goes into the quad tree. With flag 4 the entry links to the named one (enabled 0,
/// disabled 4), which links back when unlinked and gets the window 0..24. Returns the pool index.
#[hook(addr = 0x43FA00, conv = cdecl, name = "CEntryExitManager::AddOne", expect = "64 A1 00 00 00 00 8B 0D D8 A7",
       unverified = "std5-world-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
#[allow(clippy::too_many_arguments)]
pub fn add_one(
    cx: f32,
    cy: f32,
    cz: f32,
    angle: f32,
    range_x: f32,
    range_y: f32,
    _range_z: f32,
    exit_x: u32,
    exit_y: u32,
    exit_z: f32,
    exit_angle: u32,
    area: u32,
    flags: u32,
    sky: u32,
    time_on: u32,
    time_off: u32,
    peds: u32,
    name: GPtr<u8>,
) -> u32 {
    let pool = GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(POOL).read() as usize);
    let e = pool_new(pool);
    if e.is_null() {
        return 0;
    }
    let mut flags = flags;
    let mut time_off = time_off;
    let forced = flags & 0x1000 != 0;
    let mut on = if forced {
        time_off = 0x18;
        0u8
    } else {
        time_on as u8
    };
    if flags & 0x400 != 0 && sa_core::orig::at_site(RAND_SITE, rand) < 0x3FFF {
        on = 0;
        time_off = 0;
    }
    if GPtr::<u8>::from_addr(0x96A730).read() != 0 || !forced {
        flags |= 0x4000;
    }
    let half = c(0x858B8C);
    let hx = X87::from_f32(range_x).fmul_m(half).to_f32();
    e.byte_add(0x20).cast::<u32>().write(exit_x);
    e.byte_add(0x32).write(area as u8);
    e.byte_add(0x36).write(peds as u8);
    f(e, 8).write(X87::from_f32(cx).fsub_m(hx).to_f32());
    e.byte_add(0x24).cast::<u32>().write(exit_y);
    let hy = X87::from_f32(range_y).fmul_m(half).to_f32();
    e.byte_add(0x2C).cast::<u32>().write(exit_angle);
    e.byte_add(0x33).write(sky as u8);
    e.byte_add(0x34).write(on);
    e.byte_add(0x35).write(time_off as u8);
    e.byte_add(0x30).cast::<u16>().write(flags as u16);
    e.byte_add(0x38).cast::<u32>().write(0);
    f(e, 0x14).write(X87::from_f32(cy).fsub_m(hy).to_f32());
    f(e, 0x10).write(X87::from_f32(hx).fadd_m(cx).to_f32());
    f(e, 0xC).write(X87::from_f32(hy).fadd_m(cy).to_f32());
    let one = c(0x858624);
    f(e, 0x18).write(X87::from_f32(cz).fadd_m(one).to_f32());
    f(e, 0x1C).write(X87::from_f32(angle).fmul_m(c(0x8595EC)).to_f32());
    f(e, 0x28).write(X87::from_f32(exit_z).fadd_m(one).to_f32());
    if name.is_null() {
        e.write(0);
    } else {
        crt_strncpy(e, name, 8);
    }
    let k = c(0x858F40);
    // corners: v0 = (range x * k, range y * k, 0), v1 = (hx, hy, 0)
    let mut v = [
        X87::from_f32(range_x).fmul_m(k).to_f32(),
        X87::from_f32(range_y).fmul_m(k).to_f32(),
        0.0,
        hx,
        hy,
        0.0,
    ];
    let mut m = [0u32; 0x12];
    let mp: GPtr<u8> = GPtr::from_raw(m.as_mut_ptr()).cast();
    set_rotate_z(mp, angle);
    for i in 0..2 {
        let mut out = [0f32; 3];
        let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr().wrapping_add(3 * i)).cast();
        let r = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()).cast(), mp, vp);
        for k in 0..3 {
            v[3 * i + k] = f(r, 4 * k as isize).read();
        }
        v[3 * i] = X87::from_f32(cx).fadd_m(v[3 * i]).to_f32();
        v[3 * i + 1] = X87::from_f32(cy).fadd_m(v[3 * i + 1]).to_f32();
    }
    // rect {left, bottom, right, top}
    let mut rect = [0f32; 4];
    if v[0] < v[3] {
        rect[0] = v[0];
        rect[2] = v[3];
    } else {
        rect[2] = v[0];
        rect[0] = v[3];
    }
    if v[1] < v[4] {
        rect[3] = v[1];
        rect[1] = v[4];
    } else {
        rect[1] = v[1];
        rect[3] = v[4];
    }
    quad_tree_add_item(
        GPtr::from_addr(GPtr::<u32>::from_addr(QUAD_TREE).read() as usize),
        e,
        GPtr::from_raw(rect.as_mut_ptr()).cast(),
    );
    if e.byte_add(0x30).read() & 4 != 0 {
        let idx = get_entry_exit_index(e, 0, 4);
        if idx != -1 {
            let pool = GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(POOL).read() as usize);
            let map = GPtr::<i8>::from_addr(pool.byte_add(4).cast::<u32>().read() as usize);
            let linked = if map.byte_add(idx as isize).read() < 0 {
                GPtr::<u8>::NULL
            } else {
                GPtr::from_addr((pool.cast::<u32>().read() as usize).wrapping_add(0x3C * idx as usize))
            };
            e.byte_add(0x38).cast::<u32>().write(linked.addr() as u32);
            if linked.byte_add(0x38).cast::<u32>().read() == 0 {
                linked.byte_add(0x38).cast::<u32>().write(e.addr() as u32);
            }
            linked.byte_add(0x34).write(0);
            linked.byte_add(0x35).write(0x18);
        }
    }
    let base = GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(POOL).read() as usize).cast::<u32>().read();
    let index = ((e.addr() as u32).wrapping_sub(base) as i32) / 0x3C;
    matrix_dtor(mp);
    index as u32
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ADD_ONE];
