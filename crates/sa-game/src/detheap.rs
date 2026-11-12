//! The deterministic mode's game heap (`--fixed-step`, [`crate::det`]): `malloc` / `free` / `calloc` / `realloc` and
//! `CMemoryMgr::MallocAlign` / `FreeAlign` are served from one region mapped at a fixed address, with size-class free
//! lists (LIFO) and a bump pointer, so two runs of one scenario hand out the same addresses (the game seeds `rand`
//! with entity addresses, e.g. the timed enex lights at 0x5339F5; glibc's addresses depend on what the other threads
//! allocated first). Not a general-purpose allocator: freed blocks are reused only by requests of the same class.

#![allow(unsafe_code)]

use std::sync::Mutex;

use sa_core::mem::GPtr;

/// Where the region is mapped (below the no-ASLR PIE image at 0x56555000) and its size.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const BASE: usize = 0x1800_0000;
const SIZE: usize = 0x3800_0000;
/// Block header: the class size (the pointer handed out is 16-aligned after it).
const HDR: usize = 16;

struct Heap {
    top: usize,
    end: usize,
    /// free blocks per class size (header addresses)
    free: std::collections::BTreeMap<usize, Vec<usize>>,
}

static HEAP: Mutex<Option<Heap>> = Mutex::new(None);

/// The class (total block size, header included) of a request of `n` bytes: multiples of 16 up to 256 bytes, then
/// four steps per power of two up to 1 MiB, then multiples of 64 KiB.
fn class(n: usize) -> usize {
    let t = n.max(1) + HDR;
    if t <= 256 {
        return t.div_ceil(16) * 16;
    }
    if t > 1 << 20 {
        return t.div_ceil(0x1_0000) * 0x1_0000;
    }
    let p = t.next_power_of_two() >> 1;
    let q = p / 4;
    t.div_ceil(q) * q
}

fn with<R>(f: impl FnOnce(&mut Heap) -> R) -> R {
    let mut h = HEAP.lock().unwrap_or_else(|e| e.into_inner());
    let h = h.get_or_insert_with(|| {
        #[cfg(target_arch = "wasm32")]
        let p = Box::leak(vec![0u8; SIZE].into_boxed_slice()).as_mut_ptr();
        #[cfg(not(target_arch = "wasm32"))]
        let p = {
            // SAFETY: an anonymous private mapping; MAP_FIXED_NOREPLACE never replaces an existing mapping.
            let mut p = unsafe {
                libc::mmap(
                    BASE as *mut libc::c_void,
                    SIZE,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE | libc::MAP_FIXED_NOREPLACE,
                    -1,
                    0,
                )
            };
            if p == libc::MAP_FAILED {
                eprintln!("sa-game: --fixed-step heap: {BASE:#x} is taken; the kernel picks the address (runs may differ)");
                // SAFETY: as above, without a fixed address.
                p = unsafe {
                    libc::mmap(
                        std::ptr::null_mut(),
                        SIZE,
                        libc::PROT_READ | libc::PROT_WRITE,
                        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                        -1,
                        0,
                    )
                };
                assert!(p != libc::MAP_FAILED, "sa-game: --fixed-step heap: mmap failed");
            }
            p
        };
        let base = p as usize;
        Heap { top: base, end: base + SIZE, free: Default::default() }
    });
    f(h)
}

/// A block of at least `n` bytes, 16-aligned (contents unspecified).
pub fn alloc(n: usize) -> GPtr<u8> {
    let c = class(n);
    let hdr = with(|h| {
        if let Some(b) = h.free.get_mut(&c).and_then(Vec::pop) {
            return b;
        }
        assert!(
            h.top + c <= h.end,
            "sa-game: --fixed-step heap exhausted ({} MiB)",
            SIZE >> 20
        );
        let b = h.top;
        h.top += c;
        b
    });
    GPtr::<u32>::from_addr(hdr).write(c as u32);
    GPtr::from_addr(hdr + HDR)
}

fn class_of(p: GPtr<u8>) -> usize {
    GPtr::<u32>::from_addr(p.addr() - HDR).read() as usize
}

pub fn free(p: GPtr<u8>) {
    if p.is_null() {
        return;
    }
    let c = class_of(p);
    with(|h| h.free.entry(c).or_default().push(p.addr() - HDR));
}

pub fn calloc(n: usize) -> GPtr<u8> {
    let p = alloc(n);
    // SAFETY: `p` is a block of at least `n` bytes of this heap.
    unsafe { std::ptr::write_bytes(p.raw(), 0, n) };
    p
}

pub fn realloc(p: GPtr<u8>, n: usize) -> GPtr<u8> {
    if p.is_null() {
        return alloc(n);
    }
    let c = class_of(p);
    // a shrink stays in place, as MSVC's realloc does: `FxMemoryPool_c::Optimise` shrinks the effects pool after
    // `LoadFxProject` and keeps using the blueprints (and their list links) already allocated inside it
    if class(n) <= c {
        return p;
    }
    let q = alloc(n);
    // SAFETY: both blocks belong to this heap; the old one holds c - HDR bytes, the new one at least n.
    unsafe { std::ptr::copy_nonoverlapping(p.raw(), q.raw(), (c - HDR).min(n)) };
    free(p);
    q
}

#[cfg(test)]
mod tests {
    use super::class;

    #[test]
    fn classes_cover_their_requests() {
        for n in [
            0,
            1,
            15,
            16,
            240,
            241,
            1000,
            4096,
            100_000,
            1 << 20,
            (1 << 20) + 1,
            5_000_000,
        ] {
            let c = class(n);
            assert!(c >= n + 16 && c % 16 == 0, "{n} -> {c}");
        }
        assert_eq!(class(1000), class(1001));
    }
}
