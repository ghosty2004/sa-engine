//! `RxPipelineExecute` and the Rx heap / packet helpers it needs (batch `std9-rw-1`), ported from the 1.0 US bytes (RW
//! has no gta-reversed source). The heap layout is in `rxpipeline.rs`. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, wr};

/// The global `RxHeap*` (`RxHeapGetGlobalHeap`)
const RX_HEAP_GLOBAL: usize = 0xC9A6B0;
/// The execution context of `RxPipelineExecute`: exit code (0xC9BC48), pipeline (0xC9BC40), the node parameters
/// `{data, heap}` (0xC9BC50 / 0xC9BC54).
const EXEC_EXIT_CODE: usize = 0xC9BC48;
const EXEC_PIPELINE: usize = 0xC9BC40;
const EXEC_PARAMS: usize = 0xC9BC50;
const EXEC_PARAMS_HEAP: usize = 0xC9BC54;

orig! {
    /// `_rxHeapAddBlock(block, prevBlock, heap)` (0x809BF0)
    fn heap_add_block(block: GPtr<u8>, prev: GPtr<u8>, heap: GPtr<u8>) -> i32 = cdecl 0x809BF0 => crate::rxpipeline::rx_heap_add_block;
    /// `_rxHeapGetEntry(heap)` (0x8099F0)
    fn heap_new_entry(heap: GPtr<u8>) -> GPtr<u8> = cdecl 0x8099F0 => crate::rxpipeline::rx_heap_new_entry;
    /// `_rxHeapReset(heap)` (0x809EC0)
    fn heap_reset_call(heap: GPtr<u8>) -> i32 = cdecl 0x809EC0 => rx_heap_reset;
    /// `RxHeapFree(heap, block)` (0x8098B0)
    fn heap_free_call(heap: GPtr<u8>, block: GPtr<u8>) = cdecl 0x8098B0 => rx_heap_free;
    /// `_rxPacketDestroy(packet)` (0x80F070)
    fn packet_destroy_call(packet: GPtr<u8>) = cdecl 0x80F070 => rx_packet_destroy;
    /// A pipeline node's body (`nodeDef->nodeMethods.nodeBody`, `RwBool (*)(RxPipelineNode*, const RxPipelineNodeParam*)`)
    fn node_body(f: u32, node: GPtr<u8>, params: GPtr<u8>) -> i32 = cdecl dyn => unported;
}

/// `_rxHeapReset(heap)` (0x809EC0, cdecl): entries used (+0x14) = 0; every block after the head (+4, linked by +8) and
/// then the head block are re-added (`_rxHeapAddBlock(block, previous, heap)`), the first one's memory becoming the
/// first free header (+8); a failing add returns FALSE at once. Dirty (+0x18) = 0; TRUE.
#[hook(addr = 0x809EC0, conv = cdecl, name = "_rxHeapReset", expect = "53 8B 5C 24 08 56 57",
       unverified = "std9-rw-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn rx_heap_reset(heap: GPtr<u8>) -> i32 {
    wr::<u32>(heap, 0x14, 0);
    let mut prev = GPtr::<u8>::NULL;
    let mut b = p8(rd::<u32>(p8(rd::<u32>(heap, 4)), 8));
    while !b.is_null() {
        if heap_add_block(b, prev, heap) == 0 {
            return 0;
        }
        if prev.is_null() {
            wr::<u32>(heap, 8, rd::<u32>(b, 0));
        }
        prev = b;
        b = p8(rd::<u32>(b, 8));
    }
    let head = p8(rd::<u32>(heap, 4));
    if heap_add_block(head, prev, heap) == 0 {
        return 0;
    }
    if prev.is_null() {
        wr::<u32>(heap, 8, rd::<u32>(head, 0));
    }
    wr::<u32>(heap, 0x18, 0);
    1
}

/// `RxHeapFree(heap, block)` (0x8098B0, cdecl): the header `h` sits 0x20 bytes before `block` (+0 prev, +4 next, +8
/// size, +0xC free entry). Both neighbours free: the next one's entry is replaced by the last used entry (whose header
/// gets the new slot; entries used - 1), the previous header absorbs `h` and the next (`size += h.size + next.size +
/// 0x40`, its entry's size updated) and links past them. Only the previous free: it absorbs `h` (`+ h.size + 0x20`).
/// Only the next free: `h` absorbs it (`h.size += next.size + 0x20`) and takes over its entry (pointer and size).
/// Neither: a new entry (`_rxHeapGetEntry`) = `{h.size, h}`, `h.entry` = it (nothing when none is free).
#[hook(addr = 0x8098B0, conv = cdecl, name = "RxHeapFree", expect = "8B 44 24 08 56 57 8D 70 E0",
       unverified = "std9-rw-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn rx_heap_free(heap: GPtr<u8>, block: GPtr<u8>) {
    let h = block.byte_add(-0x20);
    let prev = p8(rd::<u32>(h, 0));
    let prev_free = !prev.is_null() && rd::<u32>(prev, 0xC) != 0;
    let next = p8(rd::<u32>(h, 4));
    let next_free = !next.is_null() && rd::<u32>(next, 0xC) != 0;
    let link_back = |n: u32, to: GPtr<u8>| {
        if n != 0 {
            wr::<u32>(p8(n), 0, to.addr() as u32);
        }
    };
    if prev_free && next_free {
        let e = rd::<u32>(next, 0xC);
        let used = rd::<u32>(heap, 0x14);
        let arr = rd::<u32>(heap, 0xC);
        let last = arr.wrapping_add(used.wrapping_mul(8)).wrapping_sub(8);
        if last != e {
            wr::<u32>(p8(e), 0, rd::<u32>(p8(last), 0));
            let lh = rd::<u32>(p8(last), 4);
            wr::<u32>(p8(e), 4, lh);
            wr::<u32>(p8(lh), 0xC, e);
        }
        wr::<u32>(heap, 0x14, used.wrapping_sub(1));
        let (next, prev) = (p8(rd::<u32>(h, 4)), p8(rd::<u32>(h, 0)));
        let add = rd::<u32>(h, 8).wrapping_add(rd::<u32>(next, 8)).wrapping_add(0x40);
        wr::<u32>(prev, 8, rd::<u32>(prev, 8).wrapping_add(add));
        wr::<u32>(p8(rd::<u32>(prev, 0xC)), 0, rd::<u32>(prev, 8));
        wr::<u32>(prev, 4, rd::<u32>(next, 4));
        link_back(rd::<u32>(next, 4), prev);
    } else if prev_free {
        let add = rd::<u32>(h, 8).wrapping_add(0x20);
        wr::<u32>(prev, 8, rd::<u32>(prev, 8).wrapping_add(add));
        wr::<u32>(p8(rd::<u32>(prev, 0xC)), 0, rd::<u32>(prev, 8));
        wr::<u32>(prev, 4, rd::<u32>(h, 4));
        link_back(rd::<u32>(h, 4), prev);
    } else if next_free {
        let size = rd::<u32>(next, 8).wrapping_add(0x20).wrapping_add(rd::<u32>(h, 8));
        wr::<u32>(h, 8, size);
        wr::<u32>(h, 0xC, rd::<u32>(next, 0xC));
        wr::<u32>(p8(rd::<u32>(next, 0xC)), 4, h.addr() as u32);
        wr::<u32>(p8(rd::<u32>(p8(rd::<u32>(h, 4)), 0xC)), 0, rd::<u32>(h, 8));
        let nn = rd::<u32>(p8(rd::<u32>(h, 4)), 4);
        wr::<u32>(h, 4, nn);
        link_back(nn, h);
    } else {
        let e = heap_new_entry(heap);
        if !e.is_null() {
            wr::<u32>(e, 4, h.addr() as u32);
            wr::<u32>(e, 0, rd::<u32>(h, 8));
            wr::<u32>(h, 0xC, e.addr() as u32);
        }
    }
}

/// `_rxPacketDestroy(packet)` (0x80F070, cdecl): the packet's pipeline (+4) gets +0x10 = 1; for each of the
/// `numClusters` (u16 +2) clusters (0x1C bytes from +0x14) with a cluster reference (+0x14): its data (+4), unless
/// external (flags byte +0 bit 1), goes back to the global heap (`RxHeapFree`), and +0, +4, +0xC, +0x10, +0x14 are
/// cleared. The flags word (+0) = 0.
#[hook(addr = 0x80F070, conv = cdecl, name = "_rxPacketDestroy", expect = "53 55 8B 6C 24 0C 56 57",
       unverified = "std9-rw-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "a zero cluster count wraps the 32-bit loop counter as the original does")]
pub fn rx_packet_destroy(packet: GPtr<u8>) {
    wr::<u32>(p8(rd::<u32>(packet, 4)), 0x10, 1);
    let mut n = u32::from(rd::<u16>(packet, 2));
    let mut c = packet.byte_add(0x14);
    loop {
        if rd::<u32>(c, 0x14) != 0 {
            let data = rd::<u32>(c, 4);
            if data != 0 && rd::<u8>(c, 0) & 2 == 0 {
                heap_free_call(p8(g::<u32>(RX_HEAP_GLOBAL).read()), p8(data));
            }
            for o in [0isize, 4, 0xC, 0x10, 0x14] {
                wr::<u32>(c, o, 0);
            }
        }
        c = c.byte_add(0x1C);
        n = n.wrapping_sub(1);
        if n == 0 {
            break;
        }
    }
    wr::<u16>(packet, 0, 0);
}

/// `RxPipelineExecute(pipeline, data, heapReset)` (0x805710, cdecl): with `heapReset` and a dirty global heap,
/// `_rxHeapReset`. The execution context: exit code 1, the pipeline, `{data, global heap}`; pipeline +0x10 = 0; the
/// first node's body (`nodes[0].nodeDef->body(node, &params)`); FALSE makes the exit code 0. A pipeline +0x10 above 1
/// becomes 2 and its packet (+0x14) is destroyed. The context is cleared; returns the pipeline, or NULL on failure.
#[hook(addr = 0x805710, conv = cdecl, name = "RxPipelineExecute", expect = "8B 44 24 0C 56 57 33 FF",
       unverified = "std9-rw-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn rx_pipeline_execute(pipeline: GPtr<u8>, data: GPtr<u8>, heap_reset: i32) -> GPtr<u8> {
    if heap_reset != 0 {
        let heap = p8(g::<u32>(RX_HEAP_GLOBAL).read());
        if rd::<u32>(heap, 0x18) != 0 {
            heap_reset_call(heap);
        }
    }
    let heap = g::<u32>(RX_HEAP_GLOBAL).read();
    g::<u32>(EXEC_EXIT_CODE).write(1);
    g::<u32>(EXEC_PIPELINE).write(pipeline.addr() as u32);
    g::<u32>(EXEC_PARAMS).write(data.addr() as u32);
    g::<u32>(EXEC_PARAMS_HEAP).write(heap);
    let node = p8(rd::<u32>(pipeline, 8));
    wr::<u32>(pipeline, 0x10, 0);
    let body = rd::<u32>(p8(rd::<u32>(node, 0)), 4);
    if node_body(body, node, g(EXEC_PARAMS)) == 0 {
        g::<u32>(EXEC_EXIT_CODE).write(0);
    }
    if rd::<i32>(pipeline, 0x10) > 1 {
        wr::<u32>(pipeline, 0x10, 2);
        packet_destroy_call(p8(rd::<u32>(pipeline, 0x14)));
    }
    let ok = g::<u32>(EXEC_EXIT_CODE).read() != 0;
    g::<u32>(EXEC_PIPELINE).write(0);
    g::<u32>(EXEC_PARAMS).write(0);
    g::<u32>(EXEC_PARAMS_HEAP).write(0);
    if ok { pipeline } else { GPtr::NULL }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RX_HEAP_RESET, HOOK_RX_HEAP_FREE, HOOK_RX_PACKET_DESTROY, HOOK_RX_PIPELINE_EXECUTE];
