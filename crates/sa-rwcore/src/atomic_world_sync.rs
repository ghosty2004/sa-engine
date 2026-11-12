//! STD-5: `_rpWorldObjAtomicSync` (0x7507E0), the frame-sync callback the world-object plugin installs
//! on atomics: after the saved callback, an atomic in a world is re-tied to every world sector its
//! world bounding sphere's box overlaps. Ported from the 1.0 US bytes. Layouts in `worldplugin.rs`
//! (a tie: +0/+4 link in the sector's atomic list at sector +0x38, +8 atomic, +0xC/+0x10 link in the
//! atomic's tie list at atomic +0x64, +0x14 sector; plane sector: +0 axis byte offset, +8 left, +0xC
//! right, +0x10 left value, +0x14 right value; leaf: type < 0).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, module, p8, rd, rw_memory_alloc, rw_memory_free, wr};

/// World-object plugin offset of atomics (+0 world, +4 saved sync callback).
const ATOMIC_WORLD_OFFSET: usize = 0xC925A8;
/// WORLDOBJMODULE (+0 the atomic-tie freelist).
const WORLDOBJ_MODULE: usize = 0xC9259C;

orig! {
    /// The atomic's saved frame-sync callback `cb(atomic)`
    fn call_saved_sync(f: u32, atomic: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `RpAtomicGetWorldBoundingSphere(atomic)` (0x749330)
    fn world_bounding_sphere(atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x749330 => crate::atomic::get_world_bounding_sphere;
}

/// Unlinks a two-pointer link (`next` +0, `prev` +4).
fn unlink(l: GPtr<u8>) {
    let (next, prev) = (p8(rd::<u32>(l, 0)), p8(rd::<u32>(l, 4)));
    wr::<u32>(prev, 0, addr(next));
    wr::<u32>(next, 4, addr(prev));
}

fn addr(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `_rpWorldObjAtomicSync(atomic)` (0x7507E0): the saved sync callback first (NULL → NULL). With a
/// world: every tie of the atomic whose atomic and sector are set is unlinked from both lists and freed;
/// the world bounding sphere gives the box `centre ± radius` (x87 PC=24 `fsub`/`fadd` stored as f32);
/// then a descent from the world's root sector with an explicit stack: a leaf gets a new tie (hint
/// 0x30507) linked at the front of the sector's atomic list and of the atomic's tie list, then the
/// stack is popped; a plane goes left when `inf[axis] < leftValue` (and also pushes the right child
/// unless `sup[axis] <= rightValue`), else right unless `sup[axis] <= rightValue` (then popped).
/// Returns the atomic.
#[hook(addr = 0x7507E0, conv = cdecl, name = "_rpWorldObjAtomicSync", expect = "A1 A8 25 C9 00 81 EC 1C 01 00",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn world_obj_atomic_sync(atomic: GPtr<u8>) -> GPtr<u8> {
    let ext = atomic.byte_add(g::<u32>(ATOMIC_WORLD_OFFSET).read() as isize);
    if call_saved_sync(rd::<u32>(ext, 4), atomic).is_null() {
        return p8(0);
    }
    let world = p8(rd::<u32>(ext, 0));
    if world.is_null() {
        return atomic;
    }
    let head = atomic.byte_add(0x64);
    let mut l = p8(rd::<u32>(head, 0));
    while addr(l) != addr(head) {
        let tie = l.byte_add(-0xC);
        l = p8(rd::<u32>(l, 0));
        if rd::<u32>(tie, 8) != 0 && rd::<u32>(tie, 0x14) != 0 {
            unlink(tie.byte_add(0xC));
            unlink(tie);
            rw_memory_free(p8(rd::<u32>(module(WORLDOBJ_MODULE), 0)), tie);
        }
    }
    let s = world_bounding_sphere(atomic);
    let r = rd::<f32>(s, 0xC);
    let c = [rd::<f32>(s, 0), rd::<f32>(s, 4), rd::<f32>(s, 8)];
    let inf = c.map(|v| X87::from_f32(v).fsub_m(r).to_f32());
    let sup = c.map(|v| X87::from_f32(v).fadd_m(r).to_f32());
    let axis = |a: u32| (a / 4) as usize;
    let mut stack = [0u32; 0x40];
    let mut top: i32 = 0;
    let mut sector = p8(rd::<u32>(world, 0x1C));
    loop {
        let ty = rd::<i32>(sector, 0);
        if ty < 0 {
            let tie = rw_memory_alloc(p8(rd::<u32>(module(WORLDOBJ_MODULE), 0)), 0x30507);
            wr::<u32>(tie, 0x14, addr(sector));
            wr::<u32>(tie, 8, addr(atomic));
            let sh = sector.byte_add(0x38);
            wr::<u32>(tie, 4, addr(sh));
            wr::<u32>(tie, 0, rd::<u32>(sh, 0));
            wr::<u32>(p8(rd::<u32>(sh, 0)), 4, addr(tie));
            wr::<u32>(sh, 0, addr(tie));
            let al = tie.byte_add(0xC);
            wr::<u32>(tie, 0x10, addr(head));
            wr::<u32>(al, 0, rd::<u32>(head, 0));
            wr::<u32>(p8(rd::<u32>(head, 0)), 4, addr(al));
            wr::<u32>(head, 0, addr(al));
            sector = p8(stack[top.max(0) as usize]);
            top -= 1;
            if top < 0 {
                break;
            }
            continue;
        }
        let k = axis(ty as u32);
        let (lv, rv) = (rd::<f32>(sector, 0x10), rd::<f32>(sector, 0x14));
        if inf[k] < lv {
            let node = sector;
            sector = p8(rd::<u32>(node, 8));
            if sup[k] > rd::<f32>(node, 0x14) {
                top += 1;
                stack[top as usize] = rd::<u32>(node, 0xC);
            }
        } else if sup[k] > rv {
            sector = p8(rd::<u32>(sector, 0xC));
        } else {
            sector = p8(stack[top.max(0) as usize]);
            top -= 1;
            if top < 0 {
                break;
            }
        }
    }
    atomic
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_WORLD_OBJ_ATOMIC_SYNC];
