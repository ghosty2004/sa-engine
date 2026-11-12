//! `FUN_00606060` (INTRO-2: the decision of the default decision makers, reached from
//! `CDecisionMakerTypes::MakeDecision` when the ped's decision maker index is -1). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CDecision::MakeDecision(int32, bool, int32, int32, int32, int32, int16*, int16*)` (0x6040D0, `ret 0x20`)
    #[allow(clippy::too_many_arguments)]
    fn decision_make_decision(this: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) = thiscall 0x6040D0 => unported;
}

/// `FUN_00606060(int32 index, 8 dwords)` (0x606060, thiscall on a `CDecisionMaker`, `ret 0x24`): the decision
/// `this + index * 0x3C` makes the decision with the other eight arguments (`CDecision::MakeDecision`, tail of the
/// argument copy).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x606060, conv = thiscall, name = "FUN_00606060", this = "CDecisionMaker", expect = "8B 54 24 20 8B C1",
       unverified = "intro3-8", route = "L2", hazards = "none", recs = "intro")]
pub fn decision_maker_make_decision_at(this: GPtr<u8>, index: i32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) {
    decision_make_decision(this.byte_add(index.wrapping_mul(0x3C) as isize), a1, a2, a3, a4, a5, a6, a7, a8);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DECISION_MAKER_MAKE_DECISION_AT];
