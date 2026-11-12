//! `CAutomobile::SetRandomDamage(bool)` (0x6A2530), reached when the car generator / population code creates a
//! damaged parked or ambient car (batch `std12-veh-1`). Ported from the 1.0 US bytes; gta-reversed `Automobile.cpp`
//! as a reading aid (its `FIX_BUGS` variant differs). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::automobile2::{
    dm_get_door_status, dm_get_panel_status, dm_set_door_status, node_from_door_call, node_from_panel_call,
    set_component_visibility,
};
use crate::common::{ftol32, g, model_info, rand_at, rand_frac, rd, xi};

/// `fmul dword [addr]` operand: a `.rdata` constant.
fn k(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

orig! {
    /// `CDamageManager::SetPanelStatus(ePanels, ePanelDamageState)` (0x6C2150)
    fn dm_set_panel_status(dm: GPtr<u8>, panel: u32, status: u32) = thiscall 0x6C2150 => crate::damage::set_panel_status;
}

/// `_ftol((rand() & 0xFFFF) * (1 / 32768) * m)` with `rand` called from `site` (the inlined integer
/// `GetRandomNumberInRange(0, m)`).
fn random_below(site: u32, m: Option<f32>) -> i32 {
    let f = rand_frac(rand_at(site));
    ftol32(match m {
        Some(m) => f.fmul_m(m),
        None => f,
    })
}

/// The `n`-th (0-based) entry of `picked[..limit]` that is still 0, as the original's search loop finds it:
/// the index starts at -1 and stops at `limit` when fewer entries are free (unreached: `n` is below the free
/// count). `n` below 0 gives -1.
fn nth_unpicked(picked: &[u8], limit: i32, n: i32) -> i32 {
    let (mut found, mut i) = (-1i32, -1i32);
    if n > -1 {
        while i < limit {
            let v = picked[(i + 1) as usize];
            i += 1;
            if v == 0 {
                found += 1;
            }
            if found >= n {
                break;
            }
        }
    }
    i
}

/// Whether the car's model allows damage on `node` (`vehicleStruct->m_nMaskComponentDamagable` +0x310 bit `node`).
fn damageable(car: GPtr<u8>, node: i32) -> bool {
    let vs = rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(car, 0x22))), 0x5C);
    rd::<u32>(vs, 0x310) & (1u32 << (node as u32 & 31)) != 0
}

/// `CAutomobile::SetRandomDamage(bool removeStuff)` (0x6A2530, thiscall, `ret 4`). Door count: removeStuff ?
/// `rand * 6` : `rand * 1` (i.e. 0); panel count: with no doors removeStuff ? `1 - _ftol(rand * -3.0)` : 1, else
/// `rand * 4` / `rand * 1` (each `rand` = `(rand() & 0xFFFF) / 32768`, truncated). Doors: pick `rand * (6 - i)`
/// (the first pick directly, later ones as the n-th unpicked door), and a door with status 0 whose node the model
/// marks damageable gets status 2 and its frame `SetComponentVisibility(.., 2)`; every pick is marked. Panels: pick
/// the n-th unpicked panel of 7 (`rand * (4 - j)`), panels 2, 3 and 4 pre-marked; an undamaged damageable panel
/// gets status 1 and visibility 2.
#[hook(addr = 0x6A2530, conv = thiscall, name = "CAutomobile::SetRandomDamage", expect = "83 EC 1C 53 55 33 C0",
       unverified = "std12-veh-1", route = "L2", hazards = "none", recs = "grove-walk,driving")]
pub fn set_random_damage(car: GPtr<u8>, remove_stuff: u32) {
    let mut doors = [0u8; 8];
    let mut panels = [0u8; 8];
    for (i, p) in panels.iter_mut().enumerate().take(7) {
        *p = u8::from(matches!(i, 2..=4));
    }
    let (n_doors, n_panels) = if remove_stuff as u8 != 0 {
        let d = random_below(0x6A2576, Some(k(0x858B44)));
        if d == 0 {
            (d, 1 - random_below(0x6A25A1, Some(k(0x859024))))
        } else {
            (d, random_below(0x6A25D1, Some(k(0x858B90))))
        }
    } else {
        let d = random_below(0x6A25F1, None);
        if d == 0 { (d, 1) } else { (d, random_below(0x6A2620, None)) }
    };
    let dm = car.byte_add(0x5A0);
    let mut i = 0;
    while i < n_doors {
        let r = rand_frac(rand_at(0x6A2655)).fmul(xi(6 - i));
        let n = ftol32(r);
        let door = if i == 0 { n } else { nth_unpicked(&doors, 6, n) };
        let node = node_from_door_call(door as u32);
        if dm_get_door_status(dm, door) == 0 && damageable(car, node) {
            dm_set_door_status(dm, door, 2);
            set_component_visibility(car, rd::<GPtr<u8>>(car, 0x648 + 4 * node as isize), 2);
        }
        i += 1;
        doors[door as usize & 7] = 1;
    }
    let mut left = n_panels;
    let mut mult = 4;
    while left > 0 {
        let r = rand_frac(rand_at(0x6A2735)).fmul(xi(mult));
        let panel = nth_unpicked(&panels, 7, ftol32(r));
        let node = node_from_panel_call(panel as u32);
        if dm_get_panel_status(dm, panel as u32) == 0 && damageable(car, node) {
            dm_set_panel_status(dm, panel as u32, 1);
            set_component_visibility(car, rd::<GPtr<u8>>(car, 0x648 + 4 * node as isize), 2);
        }
        panels[panel as usize & 7] = 1;
        mult -= 1;
        left -= 1;
    }
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_SET_RANDOM_DAMAGE];
