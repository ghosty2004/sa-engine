//! `CTaskComplexPartnerDeal::~CTaskComplexPartnerDeal` (0x684200, batch `attr-2`): reached by the deal partner tasks
//! ambient peds start next to attractors. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplexPartner::~CTaskComplexPartner()` (0x683A40)
    fn partner_dtor(t: GPtr<u8>) = thiscall 0x683A40 => unported;
}

/// `CTaskComplexPartnerDeal::~CTaskComplexPartnerDeal()` (0x684200, catalogue `FUN_00684200`, thiscall): vtable 0x870754, then a tail jump to
/// `CTaskComplexPartner::~CTaskComplexPartner`.
#[hook(addr = 0x684200, conv = thiscall, name = "FUN_00684200", expect = "C7 01 54 07 87 00",
       unverified = "attr-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn partner_deal_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870754);
    partner_dtor(this);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PARTNER_DEAL_DTOR];
