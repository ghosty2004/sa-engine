//! `CMenuSystem::Initialise` (PORT-HUD, newgame-1): reached from `CGame::ReInitGameObjectVariables`
//! on a New Game from the pause menu. Ported from the 1.0 US bytes; gta-reversed `MenuSystem.cpp` as
//! reading aid.

use crate::common::g;
use crate::menu::{switch_off_menu, CURRENT_MENU_IN_USE};

/// `CMenuSystem::Initialise()` (0x5822D0): for both menus, `SwitchOffMenu(i)` when `MenuInUse[i]` is
/// set; then `CurrentMenuInUse = 0`. `num_menus_in_use` is only changed by the `SwitchOffMenu` calls.
/// Not hookable (phase B native only): the loop branches back to 0x5822D3, inside the 5-byte patch.
/// Unverified (batch newgame-1, route L2, hazards heap-writes, recs pause-restart).
#[sa_core::native(addr = 0x5822D0, conv = cdecl, name = "CMenuSystem::Initialise")]
pub fn menu_system_initialise() {
    for i in 0u8..2 {
        if g::<u8>(0xBA82E0 + usize::from(i)).read() != 0 {
            switch_off_menu(i);
        }
    }
    CURRENT_MENU_IN_USE.set(0);
}

/// This module's phase B natives.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_MENU_SYSTEM_INITIALISE];
