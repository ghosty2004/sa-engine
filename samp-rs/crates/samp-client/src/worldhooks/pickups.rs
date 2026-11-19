//! Pickup collision reports.

use super::*;

// ---- pickup collision (patch table row 44) ---------------------------------------------------------------------

/// The pickup's index in the game's array, for the next tick (which reports it when it is one of the server's and
/// its cooldown allows). Mirrors samp.dll 0x100A3F80.
extern "C" fn pickup_touched(pickup: u32) {
    let index = pickup.wrapping_sub(world::GTA_PICKUPS) / world::GTA_PICKUP_SIZE;
    try_shared(|s| {
        if s.active {
            s.events.push(Event::PickupTouched(index));
        }
    });
}

#[unsafe(naked)]
pub(super) unsafe extern "C" fn pickup_update_stub() {
    core::arch::naked_asm!(
        "pushad",
        "push esi",
        "call {f}",
        "add esp, 4",
        "popad",
        "mov al, [esi+0x1C]",
        "cmp al, 6",
        "push {cont}",
        "ret",
        f = sym pickup_touched,
        cont = const 0x4579CBu32,
    )
}
