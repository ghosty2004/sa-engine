//! STD-5: `CPickups::GenerateNewOne` (0x456F20), ported from the 1.0 US bytes (gta-reversed only has a
//! `plugin::Call` stub). `CPickup` (0x20, `CPickups::aPickUps` 0x9788C0, 620 slots): +0 revenue, +4
//! object, +8 ammo, +0xC regeneration time, +0x10 position (int16 x, y, z in 1/8 units), u16 +0x16
//! money per day, u16 +0x18 model, u16 +0x1A reference, byte +0x1C type, byte +0x1D flags (bit 1 empty,
//! bit 3 visible, bits 4..6 text index).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::X87;

orig! {
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// virtual scalar deleting destructor (+0)
    fn delete_entity(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CPickup::FindTextIndexForString(const char*)` (0x455500)
    fn find_text_index_for_string(s: GPtr<u8>) -> u8 = cdecl 0x455500 => unported;
    /// `CPickup::IsVisible()` (0x454C70)
    fn is_visible(p: GPtr<u8>) -> u8 = thiscall 0x454C70 => unported;
    /// `CPickup::GiveUsAPickUpObject(CObject**, int32 slot)` (0x4567E0)
    fn give_us_a_pickup_object_call(p: GPtr<u8>, out: GPtr<u8>, slot: i32) = thiscall 0x4567E0 => give_us_a_pickup_object;
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
    let text = find_text_index_for_string(message);
    let f = p.byte_add(0x1D).read();
    p.byte_add(0x1D).write(f ^ ((text << 4 ^ f) & 0x70));
    let eight = GPtr::<f32>::from_addr(0x859000).read();
    for (k, v) in [x, y, z].into_iter().enumerate() {
        p.byte_add(0x10 + 2 * k as isize).cast::<u16>().write(X87::from_f32(v).fmul_m(eight).ftol() as u16);
    }
    let vis = is_visible(p);
    let f = p.byte_add(0x1D).read();
    let f = f ^ ((vis << 3 ^ f) & 8);
    p.byte_add(0x1D).write(f);
    let obj = p.byte_add(4);
    obj.cast::<u32>().write(0);
    if f & 8 != 0 {
        give_us_a_pickup_object_call(p, obj, -1);
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
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, uint8 skill)` (0x743C60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPools::MakeSureSlotInObjectPoolIsEmpty(int32)` (0x550080)
    fn make_sure_slot_in_object_pool_is_empty(slot: i32) = cdecl 0x550080 => unported;
    /// `CObject::operator new(size, int32 poolRef)` (0x5A1EF0)
    fn object_new_at(size: u32, pool_ref: i32) -> GPtr<u8> = cdecl 0x5A1EF0 => unported;
    /// `CObject::CObject(int32 model, bool create)` (0x5A1D70)
    fn object_ctor(this: GPtr<u8>, model: u32, create: u32) -> GPtr<u8> = thiscall 0x5A1D70 => unported;
    /// `CObject::Create(int32 model, bool)` (0x5A1F60)
    fn object_create(model: u32, b: u32) -> GPtr<u8> = cdecl 0x5A1F60 => unported;
    /// `CPickup::GetPosn()` (0x4549A0, the CVector through the hidden pointer)
    fn pickup_get_posn(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x4549A0 => unported;
    /// `CPlaceable::SetOrientation(float x, float y, float z)` (0x439A80)
    fn set_orientation(this: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x439A80 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) -> GPtr<u8> = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    fn transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CPickup::PickUpShouldBeInvisible()` (0x454D20)
    fn pick_up_should_be_invisible(p: GPtr<u8>) -> u8 = thiscall 0x454D20 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

fn rd32(p: GPtr<u8>, o: isize) -> u32 {
    p.byte_add(o).cast::<u32>().read()
}
fn or32(p: GPtr<u8>, o: isize, v: u32) {
    p.byte_add(o).cast::<u32>().write(rd32(p, o) | v)
}
fn and32(p: GPtr<u8>, o: isize, v: u32) {
    p.byte_add(o).cast::<u32>().write(rd32(p, o) & v)
}

/// `CPickup::GiveUsAPickUpObject(CObject** out, int32 poolSlot)` (0x4567E0): `*out = NULL`; nothing in
/// area 2 (`[0xB5F84C]`); a weapon model looks its weapon info up first. With a pool slot the slot is
/// emptied and (unless `*out` got set meanwhile) a `CObject` made in it, else `CObject::Create`. The
/// object (type 5) is placed at the pickup, oriented (0, 0, -pi/2), its RW frame updated, flagged
/// (no collision, pickup flags, invisibility from `PickUpShouldBeInvisible`), referenced, given the
/// ammo for the two money / ammo models, and marked for types 7 and 0x12 (ammo / 5).
///
/// Not registered: 0x4567E0 is hooked by sa-world pickups.rs (batch mvpi-world-1) (duplicate port left unregistered when STD-6 merged both).
pub fn give_us_a_pickup_object(this: GPtr<u8>, out: GPtr<u8>, pool_slot: i32) {
    out.cast::<u32>().write(0);
    if GPtr::<u32>::from_addr(0xB5F84C).read() == 2 {
        return;
    }
    let model = u32::from(this.byte_add(0x18).cast::<u16>().read());
    let mi = |m: u32| GPtr::<u8>::from_addr(GPtr::<u32>::from_addr(0xA9B0C8 + 4 * m as usize).read() as usize);
    if model_type(mi(model)) == 4 {
        get_weapon_info(rd32(mi(model), 0x24), 1);
    }
    let obj = if pool_slot >= 0 {
        make_sure_slot_in_object_pool_is_empty(pool_slot);
        let cur = rd32(out, 0);
        if cur != 0 {
            GPtr::from_addr(cur as usize)
        } else {
            let p = object_new_at(0x17C, pool_slot << 8);
            if p.is_null() { p } else { object_ctor(p, model, 0) }
        }
    } else {
        object_create(model, 0)
    };
    if rd32(out, 0) == 0 {
        out.cast::<u32>().write(obj.addr() as u32);
        if obj.is_null() {
            return;
        }
    }
    let obj = GPtr::<u8>::from_addr(rd32(out, 0) as usize);
    obj.byte_add(0x13C).write(5);
    let mut pos = [0f32; 3];
    let pp = pickup_get_posn(this, GPtr::from_raw(pos.as_mut_ptr()).cast());
    let dst = if rd32(obj, 0x14) != 0 { GPtr::<u8>::from_addr(rd32(obj, 0x14) as usize).byte_add(0x30) } else { obj.byte_add(4) };
    for k in 0..3isize {
        dst.byte_add(4 * k).cast::<f32>().write(X87::from_f32(pp.byte_add(4 * k).cast::<f32>().read()).to_f32());
    }
    if i32::from(obj.byte_add(0x22).cast::<i16>().read()) == i32::from(GPtr::<u16>::from_addr(0x8CD750).read()) {
        or32(obj, 0x1C, 0x10_0000);
    }
    set_orientation(obj, 0, 0, 0xBFC9_0FDB);
    let rw = rd32(obj, 0x18);
    if rw != 0 {
        let frame_matrix = GPtr::<u8>::from_addr(rd32(GPtr::from_addr(rw as usize), 4) as usize + 0x10);
        if rd32(obj, 0x14) != 0 {
            matrix_update_rw_matrix(GPtr::from_addr(rd32(obj, 0x14) as usize), frame_matrix);
        } else {
            transform_update_rw_matrix(obj.byte_add(4), frame_matrix);
        }
    }
    update_rw_frame(obj);
    and32(obj, 0x40, !2);
    or32(obj, 0x40, 0x80_0000);
    and32(obj, 0x1C, !1);
    or32(obj, 0x140, 1);
    let f = rd32(obj, 0x140);
    obj.byte_add(0x140).cast::<u32>().write(f ^ ((u32::from(this.byte_add(0x1D).read()) ^ f) & 2));
    or32(obj, 0x1C, 0x20_0000);
    let inv = u32::from(pick_up_should_be_invisible(this));
    let f = rd32(obj, 0x140);
    obj.byte_add(0x140).cast::<u32>().write(f ^ (((inv << 25) ^ f) & 0x200_0000));
    or32(obj, 0x1C, 0x8000_0000);
    register_reference(obj, out);
    let m = this.byte_add(0x18).cast::<u16>().read();
    let ammo_byte = if m == GPtr::<u16>::from_addr(0x8CD5CC).read() || m == GPtr::<u16>::from_addr(0x8CD5F0).read() {
        this.byte_add(8).read()
    } else {
        0
    };
    obj.byte_add(0x13D).write(ammo_byte);
    match this.byte_add(0x1C).read() {
        7 => {
            or32(obj, 0x140, 8);
            or32(obj, 0x40, 0x2000_0000);
        }
        0x12 => {
            or32(obj, 0x140, 4);
            obj.byte_add(0x13E).cast::<u16>().write((rd32(this, 8) as i32 / 5) as u16);
        }
        _ => {}
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[];
