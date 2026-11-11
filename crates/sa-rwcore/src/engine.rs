//! PORT-RW-1 module `engine` (see docs/arch/rw-port.md).

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
