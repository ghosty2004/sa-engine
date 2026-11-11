//! `CAttractorScanner::Clear` and `CConversationNode::ClearRecursively` (in-world path; written by
//! WRITER-MVP-D, batch `mvpd-ai-1`). Ported from the 1.0 US bytes; gta-reversed `AttractorScanner.cpp`
//! and `ConversationNode.cpp` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CConversationNode::ClearRecursively()` (0x43A710)
    fn clear_recursively_o(this: GPtr<u8>) = thiscall 0x43A710 => crate::attractor_scanner::conversation_node_clear_recursively;
}

/// `CAttractorScanner::Clear()` (0x5FFF90, thiscall): for each of the 10 attractor types the entity
/// (+0x18) and attractor (+0x40) slots are cleared and the minimum distance (+0x68) set to `15 * 15`
/// (computed on the x87), then to 25.0 for every type but 4 and 7.
#[hook(addr = 0x5FFF90, conv = thiscall, name = "CAttractorScanner::Clear", this = "CAttractorScanner", expect = "56 33 F6 33 D2 8D 41 68", preserve = "ecx",
       unverified = "mvpd-ai-1", route = "L2", hazards = "heap-writes,preserved-regs",
       note = "writes the ped's attractor scanner (ped object); the caller at 0x6034A7 reads ECX after the call")]
pub fn attractor_scanner_clear(this: GPtr<u8>) {
    for i in 0..10isize {
        this.byte_add(0x18 + i * 4).cast::<u32>().write(0);
        this.byte_add(0x40 + i * 4).cast::<u32>().write(0);
        let d = this.byte_add(0x68 + i * 4).cast::<f32>();
        d.write(X87::from_f32(15.0).fmul_m(15.0).to_f32());
        if i != 4 && i != 7 {
            d.write(25.0);
        }
    }
}

/// `CConversationNode::ClearRecursively()` (0x43A710, thiscall): the yes (+8) and no (+0xA) children
/// (i16 indices into the node pool 0x969570, 0x18 bytes each; negative = none) are cleared first;
/// then the name byte (+0) = 0, both links = -1, +0x10, +0x14 and +0xC = 0.
#[hook(addr = 0x43A710, conv = thiscall, name = "CConversationNode::ClearRecursively", this = "CConversationNode", expect = "56 8B F1 66 8B 46 08", preserve = "edx",
       unverified = "mvpd-ai-1", route = "L2", hazards = "preserved-regs",
       note = "callers (0x43A786, 0x156ADE2) read EDX after the call")]
pub fn conversation_node_clear_recursively(this: GPtr<u8>) {
    for o in [8isize, 0xA] {
        let i = this.byte_add(o).cast::<i16>().read();
        if i >= 0 {
            clear_recursively_o(GPtr::<u8>::from_addr(0x969570).byte_add(isize::from(i) * 0x18));
        }
    }
    this.write(0);
    this.byte_add(0xA).cast::<i16>().write(-1);
    this.byte_add(8).cast::<i16>().write(-1);
    this.byte_add(0x10).cast::<u32>().write(0);
    this.byte_add(0x14).cast::<u32>().write(0);
    this.byte_add(0xC).cast::<u32>().write(0);
}

pub const HOOKS: &[HookReg] = &[HOOK_ATTRACTOR_SCANNER_CLEAR, HOOK_CONVERSATION_NODE_CLEAR_RECURSIVELY];
