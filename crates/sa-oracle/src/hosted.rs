//! Running hooked entries of the ports on the host for L1 specs (VER-2, verifier finding V-10).
//!
//! An L1 spec's Rust side should be the function `#[hook]` installs, not a helper or a model. On
//! the host, three things stand in the way, and this module bridges them:
//!
//! - **Globals.** sa-core / sa-stream keep game globals in Rust-owned statics on the host (phase B
//!   storage). A spec copies its scratch inputs into them and reads results back, holding
//!   [`crate::host_globals`] so specs on other test threads do not interleave.
//! - **Calls into unported code.** An entry that calls an allocator or an import (`orig!` with
//!   `=> unported`) gets a stand-in through `sa_core::orig::host`, doing what the spec's patched
//!   original does (e.g. the allocator stub that records its argument and returns a queued value).
//! - **The 64-bit header layout of `CPool<T>`.** On the host its two pointers are 8 bytes, so the
//!   32-bit header in scratch memory (`objects, byte_map, size, first_free, owns, locked`) cannot be
//!   handed to the port as it is. [`run_pool`] runs the entry twice on two host views of the same
//!   scratch image and splices the results:
//!   1. *aliased*: the host header is placed 8 bytes before the scratch copy, so `size`,
//!      `first_free`, `owns` and `locked` are the scratch bytes themselves (host offsets
//!      0x10/0x14/0x18/0x19 = scratch 0x08/0x0C/0x10/0x11). Byte-map writes that land in the header
//!      (negative slot indices) and the original's re-reads of `first_free` after them behave as in
//!      the original. Only scratch bytes 0..8 (the 32-bit `objects` and `byte_map` words) hold the
//!      host `byte_map` pointer instead;
//!   2. *separate*: a host header elsewhere, initialised from the 32-bit one, and an untouched copy
//!      of the scratch image. Its bytes 0..8 are exact (no pointer lives there); header fields the
//!      entry wrote are translated back to 32-bit addresses.
//!
//!   The expected image is run 1's with bytes 0..8 from run 2. Guest addresses inside scratch are
//!   turned into host pointers into the current view and back ([`to_host`], [`to_guest`]); other
//!   addresses are carried as integers (never dereferenced: the generators keep every byte the
//!   original touches inside scratch, as before).
//!
//! What stays untested on the host: types whose host size differs from the 32-bit one (`TxdDef`
//! holds a pointer: `CPool<TxdDef>` specs use a 0x0C-byte slot type, the size the i686 build asserts).

use crate::call::{SCRATCH, SCRATCH_LEN};
use sa_core::mem::GPtr;
use sa_core::pool::CPool;
use std::cell::Cell;

thread_local! {
    /// (host address of the current scratch view, host address of the current pool header, guest
    /// address of that header)
    static VIEW: Cell<(usize, usize, u32)> = const { Cell::new((0, 0, 0)) };
    /// Guest address of the allocator the current spec's original calls (its stub is patched there).
    static ALLOC: Cell<u32> = const { Cell::new(0) };
}

/// Host pointer for a guest address: inside scratch → into the current view, else the integer.
pub fn to_host<T>(v: u32) -> GPtr<T> {
    let (base, _, _) = VIEW.with(Cell::get);
    if base != 0 && (SCRATCH..SCRATCH + SCRATCH_LEN as u32).contains(&v) {
        GPtr::from_raw((base as *mut u8).wrapping_add((v - SCRATCH) as usize).cast())
    } else {
        GPtr::from_addr(v as usize)
    }
}

/// Guest address for a host address (inverse of [`to_host`]; the header maps to its guest address,
/// other values are truncated to 32 bits).
pub fn to_guest(p: usize) -> u32 {
    let (base, hdr, ghdr) = VIEW.with(Cell::get);
    if hdr != 0 && p == hdr {
        ghdr
    } else if base != 0 && (base..base + SCRATCH_LEN).contains(&p) {
        SCRATCH + (p - base) as u32
    } else {
        // a guest address computed on the host (e.g. `objects + i * slot`): the 32-bit value, as
        // the original's wrapping arithmetic gives it
        p as u32
    }
}

/// Bytes before and after a scratch view, filled with [`CANARY`]: an access the generators should
/// have kept inside scratch stays inside the buffer, and a write there is reported.
const MARGIN: usize = 0x1000;
const CANARY: u8 = 0xCD;

/// A scratch view: the scratch image inside an 8-aligned buffer, with margins.
struct View {
    buf: Vec<u64>,
    /// bytes right before the scratch image that belong to a host header (not canary-checked)
    header: usize,
}

impl View {
    fn new(init: &[u8; SCRATCH_LEN], header: usize) -> View {
        let mut v = View {
            buf: vec![u64::from_ne_bytes([CANARY; 8]); (2 * MARGIN + SCRATCH_LEN) / 8],
            header,
        };
        // SAFETY: the buffer holds MARGIN + SCRATCH_LEN + MARGIN bytes.
        unsafe { core::ptr::copy_nonoverlapping(init.as_ptr(), v.scratch(), SCRATCH_LEN) };
        v
    }
    fn scratch(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr().cast::<u8>().wrapping_add(MARGIN)
    }
    fn image(&mut self) -> [u8; SCRATCH_LEN] {
        let mut out = [0u8; SCRATCH_LEN];
        // SAFETY: as in `new`.
        unsafe { core::ptr::copy_nonoverlapping(self.scratch(), out.as_mut_ptr(), SCRATCH_LEN) };
        let all: &[u8] = bytemuck_bytes(&self.buf);
        let bad = all[..MARGIN - self.header]
            .iter()
            .chain(&all[MARGIN + SCRATCH_LEN..])
            .any(|&b| b != CANARY);
        assert!(!bad, "the port wrote outside the scratch image");
        out
    }
}

fn bytemuck_bytes(v: &[u64]) -> &[u8] {
    // SAFETY: u64 has no padding; the slice covers the same memory.
    unsafe { core::slice::from_raw_parts(v.as_ptr().cast(), v.len() * 8) }
}

fn word(s: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(s[o..o + 4].try_into().unwrap())
}

/// The host `CPool<T>` header offsets this module relies on (64-bit host).
fn check_layout<T: 'static>() {
    use core::mem::offset_of;
    assert_eq!(
        (
            offset_of!(CPool<T>, objects),
            offset_of!(CPool<T>, byte_map),
            offset_of!(CPool<T>, size),
            offset_of!(CPool<T>, first_free),
            offset_of!(CPool<T>, owns_arrays),
            offset_of!(CPool<T>, locked),
        ),
        (0, 8, 0x10, 0x14, 0x18, 0x19),
        "host CPool layout"
    );
}

/// Runs `entry` (called with the host pool header) on the 32-bit pool header at scratch offset 0
/// of `init` (see the module docs). Returns the expected scratch image and `entry`'s result (a host
/// address or value) translated to a guest address.
pub fn run_pool<T: 'static>(init: &[u8; SCRATCH_LEN], entry: &dyn Fn(GPtr<CPool<T>>) -> usize) -> ([u8; SCRATCH_LEN], u32) {
    check_layout::<T>();
    let objects0 = word(init, 0);
    let map0 = word(init, 4);
    // 1. aliased
    let mut v1 = View::new(init, 8);
    let hdr1 = v1.scratch().wrapping_sub(8);
    VIEW.with(|c| c.set((v1.scratch() as usize, hdr1 as usize, SCRATCH)));
    let map1: GPtr<u8> = to_host(map0);
    // SAFETY: the header occupies 0x1C bytes from 8 bytes before the scratch image (host layout
    // checked above), inside the buffer.
    unsafe {
        hdr1.cast::<usize>().write(objects0 as usize);
        hdr1.add(8).cast::<usize>().write(map1.addr());
    }
    let r1 = entry(GPtr::from_raw(hdr1.cast()));
    let ret1 = to_guest(r1);
    let out1 = v1.image();
    // 2. separate
    let mut v2 = View::new(init, 0);
    let mut h2 = vec![0u64; 4];
    let hdr2: *mut u8 = h2.as_mut_ptr().cast();
    VIEW.with(|c| c.set((v2.scratch() as usize, hdr2 as usize, SCRATCH)));
    let map2: GPtr<u8> = to_host(map0);
    let (obj_init, map_init) = (objects0 as usize, map2.addr());
    // SAFETY: a 32-byte host header.
    unsafe {
        hdr2.cast::<usize>().write(obj_init);
        hdr2.add(8).cast::<usize>().write(map_init);
        core::ptr::copy_nonoverlapping(init.as_ptr().add(8), hdr2.add(0x10), 0x0C);
    }
    let r2 = entry(GPtr::from_raw(hdr2.cast()));
    let ret2 = to_guest(r2);
    let mut out2 = v2.image();
    // SAFETY: as above.
    let (obj_now, map_now) = unsafe { (hdr2.cast::<usize>().read(), hdr2.add(8).cast::<usize>().read()) };
    if obj_now != obj_init {
        out2[0..4].copy_from_slice(&to_guest(obj_now).to_le_bytes());
    }
    if map_now != map_init {
        out2[4..8].copy_from_slice(&to_guest(map_now).to_le_bytes());
    }
    VIEW.with(|c| c.set((0, 0, 0)));
    assert_eq!(ret1, ret2, "the two host views disagree on the result");
    let mut out = out1;
    out[0..8].copy_from_slice(&out2[0..8]);
    (out, ret1)
}

// ------------------------------------------------------------------------ allocator stand-in ----

/// The CRT allocator entry points the pool constructors call.
pub const NEW_ARRAY: u32 = 0x821195;
pub const NEW: u32 = 0x82119A;

/// What the spec's allocator stub (`specs_core::alloc_stub`) does, on the current view: the last
/// two size arguments go to scratch 0x4E8 (last) / 0x4EC (previous), the result is the queue head at
/// 0x4F0, refilled from 0x4F4. Checks that the port calls the allocator the original calls.
fn alloc_at(which: u32, size: u32) -> GPtr<u8> {
    let want = ALLOC.with(Cell::get);
    assert_eq!(
        which, want,
        "the port calls the allocator at {which:#x}, the original calls {want:#x}"
    );
    let (base, _, _) = VIEW.with(Cell::get);
    let p = base as *mut u8;
    // SAFETY: the current view holds SCRATCH_LEN bytes; offsets < SCRATCH_LEN.
    unsafe {
        let rd = |o: usize| p.add(o).cast::<u32>().read_unaligned();
        let wr = |o: usize, v: u32| p.add(o).cast::<u32>().write_unaligned(v);
        wr(0x4EC, rd(0x4E8));
        wr(0x4E8, size);
        let head = rd(0x4F0);
        wr(0x4F0, rd(0x4F4));
        to_host(head)
    }
}

fn new_array(size: u32) -> GPtr<u8> {
    alloc_at(NEW_ARRAY, size)
}

fn new_scalar(size: u32) -> GPtr<u8> {
    alloc_at(NEW, size)
}

/// Registers the allocator stand-ins (every `orig!` declaration of `operator new` / `new[]` the
/// pool constructors use) and records which one the current spec's original calls.
pub fn expect_allocator(addr: u32) {
    use sa_core::orig::host::set;
    type Alloc = fn(u32) -> GPtr<u8>;
    set::<Alloc>("sa_core::pools::call::operator_new_array", new_array);
    set::<Alloc>("sa_core::pools::call::operator_new", new_scalar);
    set::<Alloc>("sa_stream::misc::operator_new_array", new_array);
    set::<Alloc>("sa_stream::misc::operator_new", new_scalar);
    ALLOC.with(|c| c.set(addr));
}
