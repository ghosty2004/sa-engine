//! STD-5: `CPickups::GenerateNewOne` (0x456F20), ported from the 1.0 US bytes (gta-reversed only has a
//! `plugin::Call` stub). `CPickup` (0x20, `CPickups::aPickUps` 0x9788C0, 620 slots): +0 revenue, +4
//! object, +8 ammo, +0xC regeneration time, +0x10 position (int16 x, y, z in 1/8 units), u16 +0x16
//! money per day, u16 +0x18 model, u16 +0x1A reference, byte +0x1C type, byte +0x1D flags (bit 1 empty,
//! bit 3 visible, bits 4..6 text index).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::X87;

use crate::bindings::world_add;
use crate::bindings::world_remove;
use crate::bindings::find_text_index_for_string;
use crate::bindings::is_visible;
use crate::bindings::give_us_a_pick_up_object as give_us_a_pickup_object_call;
orig! {
    /// virtual scalar deleting destructor (+0)
    fn delete_entity(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

const PICKUPS: usize = 0x9788C0;
const COUNT: u32 = 0x26C;
/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7CB84;

fn slot(i: u32) -> GPtr<u8> {
    GPtr::from_addr(PICKUPS + 0x20 * i as usize)
}
fn ty(i: u32) -> u8 {
    slot(i).byte_add(0x1C).read()
}

/// `CPickups::GenerateNewOne(CVector pos, uint32 model, uint8 type, uint32 ammo, uint32 money, bool
/// empty, const char* message)` (0x456F20): a free slot (searched from the end for types 0xB / 0xD or
/// an empty pickup, else from the start), else the first type-8 slot, else the first type-4/5 slot
/// (their object is removed and deleted), else -1. The slot gets the arguments, the regeneration time
/// (now, + 20 s for type 4, + 120 s for 5, + 30 s for 8, + 1.5 s for 9 / 10 (made 9) and 0xB / 0xC (made
/// 0xB)), the text index of the message, the position in 1/8 units (`_ftol`), the visibility and, when
/// visible, its object (added to the world). Returns `reference << 16 | index` with the reference
/// incremented (wrapping from 0xFFFE to 1).
///
/// Not registered: 0x456F20 is hooked by sa-world pickups.rs (batch mvpi-world-1) (duplicate port left unregistered when STD-6 merged both).
#[allow(clippy::too_many_arguments)]
pub fn generate_new_one(x: f32, y: f32, z: f32, model: u32, kind: u32, ammo: u32, money: u32, empty: u32, message: GPtr<u8>) -> i32 {
    let kind = kind as u8;
    let empty = empty as u8;
    let mut found: Option<u32> = None;
    if kind == 0xD || kind == 0xB || empty != 0 {
        found = (0..COUNT).rev().find(|&i| ty(i) == 0);
    }
    if found.is_none() {
        found = (0..COUNT).find(|&i| ty(i) == 0);
    }
    if found.is_none() {
        let reuse = (0..COUNT).find(|&i| ty(i) == 8).or_else(|| (0..COUNT).find(|&i| ty(i) == 4 || ty(i) == 5));
        let Some(i) = reuse else {
            return -1;
        };
        let obj = slot(i).byte_add(4).cast::<u32>();
        if obj.read() != 0 {
            world_remove(GPtr::from_addr(obj.read() as usize));
            if obj.read() != 0 {
                delete_entity(GPtr::from_addr(obj.read() as usize), 1);
            }
            obj.write(0);
        }
        found = Some(i);
    }
    let i = found.unwrap_or(0);
    let p = slot(i);
    p.byte_add(8).cast::<u32>().write(ammo);
    p.byte_add(0x16).cast::<u16>().write(money as u16);
    let flags = (p.byte_add(0x1D).read() & 0xF8) | ((empty & 1) << 1);
    let now = GPtr::<u32>::from_addr(TIME_MS).read();
    p.byte_add(0x1C).write(kind);
    p.cast::<u32>().write(0);
    p.byte_add(0xC).cast::<u32>().write(now);
    p.byte_add(0x1D).write(flags);
    let regen = p.byte_add(0xC).cast::<u32>();
    match kind {
        4 => regen.write(now.wrapping_add(20_000)),
        5 => regen.write(now.wrapping_add(120_000)),
        8 => regen.write(now.wrapping_add(30_000)),
        _ => {
            if kind == 9 || kind == 10 {
                p.byte_add(0x1C).write(9);
                regen.write(now.wrapping_add(1500));
            }
            if kind == 0xB || kind == 0xC {
                p.byte_add(0x1C).write(0xB);
                regen.write(now.wrapping_add(1500));
            }
        }
    }
    p.byte_add(0x18).cast::<u16>().write(model as u16);
    let text = find_text_index_for_string(message) as u8;
    let f = p.byte_add(0x1D).read();
    p.byte_add(0x1D).write(f ^ ((text << 4 ^ f) & 0x70));
    let eight = GPtr::<f32>::from_addr(0x859000).read();
    for (k, v) in [x, y, z].into_iter().enumerate() {
        p.byte_add(0x10 + 2 * k as isize).cast::<u16>().write(X87::from_f32(v).fmul_m(eight).ftol() as u16);
    }
    let vis = is_visible(p) as u8;
    let f = p.byte_add(0x1D).read();
    let f = f ^ ((vis << 3 ^ f) & 8);
    p.byte_add(0x1D).write(f);
    let obj = p.byte_add(4);
    obj.cast::<u32>().write(0);
    if f & 8 != 0 {
        give_us_a_pickup_object_call(p, obj.cast::<GPtr<u8>>(), -1);
        let o = obj.cast::<u32>().read();
        if o != 0 {
            world_add(GPtr::from_addr(o as usize));
        }
    }
    let r = p.byte_add(0x1A).cast::<u16>();
    let n = if r.read() < 0xFFFE { r.read() + 1 } else { 1 };
    r.write(n);
    ((u32::from(n) << 16) | i) as i32
}

orig! {
    /// virtual `CBaseModelInfo::GetModelType()` (+0x10, AL)
    fn model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[];
