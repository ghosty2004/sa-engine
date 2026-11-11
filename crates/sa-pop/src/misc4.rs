//! `CDarkel`, car generators, `CWaterLevel::CreateBeachToy` (PORT-POP-3, lead). Ported from the 1.0 US bytes;
//! gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::mem::GlobalDesc;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `CDarkel::ReadStatus()` (0x43D1E0): `mov ax, [Status]`, the upper half of EAX is the caller's.
#[hook(addr = 0x43D1E0, conv = cdecl, name = "CDarkel::ReadStatus", expect = "66 A1 04 A7 96 00", entry_eax)]
pub fn read_status() -> u32 {
    sa_core::abi::with_entry_hi(g::<u16>(0x96A704).read())
}

/// `CDarkel::QueryModelsKilledByPlayer(int32 model, int32 player)` (stub 0x43D6C0 → 0x1564C20):
/// `RegisteredKills[model][player]` (int16 at 0x969A50), loaded with `mov ax` over EAX = `player`.
#[hook(addr = 0x43D6C0, conv = cdecl, name = "CDarkel::QueryModelsKilledByPlayer", expect = "E9 5B 75 12 01")]
pub fn query_models_killed_by_player(model: i32, player: i32) -> u32 {
    let k = g::<u16>(0x969A50 + (player.wrapping_add(model.wrapping_mul(2)) as u32).wrapping_mul(2) as usize)
        .read();
    (player as u32 & 0xFFFF_0000) | u32::from(k)
}

orig! {
    /// `CObject::operator new(uint32)` (0x5A1EE0)
    fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
    /// `CObject::CObject(int32 model, bool create)` (0x5A1D70)
    fn object_ctor(o: GPtr<u8>, model: u32, create: u32) -> GPtr<u8> = thiscall 0x5A1D70 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// virtual `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn entity_set_is_static(e: GPtr<u8>, s: u32) = thiscall vslot 0x10 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
}

/// `CWaterLevel::CreateBeachToy(const CVector& pos, eBeachToy type)` (0x6EABA0): no toy with 150 or more
/// temporary objects; type 5 picks a lounger by `rand() & 7` (site 0x6EAC40: 1, 7 → 2; 3, 5 → 4; else 3), type
/// 11 a towel (site 0x6EABF4: 1, 7 → 8; 2, 6 → 9; 3, 5 → 10; else 7) and then needs fewer than 145 temporary
/// objects; models by kind (1 ball, 2 / 4 wooden loungers, 3 towel lounger, 6 lotion, 7..10 towels static,
/// anything else the ball), a new `CObject(model, true)` at `pos` (RW matrix updated), zero speeds, temporary
/// type, static flag, one more temporary object, removal in 12 h, `CWorld::Add`.
#[hook(addr = 0x6EABA0, conv = cdecl, name = "CWaterLevel::CreateBeachToy", expect = "64 A1 00 00 00 00")]
pub fn create_beach_toy(pos: GPtr<f32>, toy: i32) -> GPtr<u8> {
    let temp = g::<u16>(0xBB4A70);
    if temp.read() >= 0x96 {
        return GPtr::NULL;
    }
    let mut k = toy;
    if toy == 5 {
        k = match crate::common::rand_at(0x6EAC40) & 7 {
            1 | 7 => 2,
            3 | 5 => 4,
            _ => 3,
        };
    } else if toy == 11 {
        k = match crate::common::rand_at(0x6EABF4) & 7 {
            1 | 7 => 8,
            2 | 6 => 9,
            3 | 5 => 10,
            _ => 7,
        };
        if temp.read() >= 0x91 {
            return GPtr::NULL;
        }
    }
    let mi = |a: usize| g::<u16>(a).read();
    let (model, stat) = match k {
        1 => (mi(0x8CD638), 0),
        2 => (mi(0x8CD67C), 0),
        3 => (mi(0x8CD680), 0),
        4 => (mi(0x8CD684), 0),
        6 => (mi(0x8CD688), 1),
        7 => (mi(0x8CD68C), 1),
        8 => (mi(0x8CD690), 1),
        9 => (mi(0x8CD694), 1),
        10 => (mi(0x8CD698), 1),
        _ => (mi(0x8CD638), 0),
    };
    let mem = object_new(0x17C);
    if mem.is_null() {
        return GPtr::NULL;
    }
    let obj = object_ctor(mem, u32::from(model), 1);
    if obj.is_null() {
        return GPtr::NULL;
    }
    let (px, py, pz) = (
        pos.read(),
        pos.wrapping_index(1).read(),
        pos.wrapping_index(2).read(),
    );
    let m = rd::<GPtr<u8>>(obj, 0x14);
    if m.is_null() {
        wr(obj, 4, px);
        wr(obj, 8, py);
        wr(obj, 0xC, pz);
    } else {
        wr(m, 0x30, px);
        wr(rd::<GPtr<u8>>(obj, 0x14), 0x34, py);
        wr(rd::<GPtr<u8>>(obj, 0x14), 0x38, pz);
    }
    let rw = rd::<GPtr<u8>>(obj, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(obj, 0x14);
        if m.is_null() {
            simple_transform_update_rw_matrix(obj.byte_add(4), rwm);
        } else {
            matrix_update_rw_matrix(m, rwm);
        }
    }
    for off in [0x44, 0x48, 0x4C, 0x50, 0x54, 0x58] {
        wr(obj, off, 0f32);
    }
    wr::<u8>(obj, 0x13C, 3);
    entity_set_is_static(obj, stat);
    temp.write(temp.read().wrapping_add(1));
    wr(obj, 0x150, g::<u32>(0xB7CB84).read().wrapping_add(0x2932E00));
    world_add(obj);
    obj
}

orig! {
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    fn save_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => unported;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    fn load_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => unported;
    /// `CTheCarGenerators::Init()` (0x6F3270)
    fn car_generators_init() = cdecl 0x6F3270 => crate::cargen::init;
}

/// `CTheCarGenerators::CarGeneratorArray[i]` (0xC27AD0, 500 x 0x20).
fn generator(i: u32) -> GPtr<u8> {
    g::<u8>(0xC27AD0 + i as usize * 0x20)
}

/// A generator that is saved: +0x1C clear and +0x1D set.
fn saved_generator(i: u32) -> bool {
    rd::<u8>(generator(i), 0x1C) == 0 && rd::<u8>(generator(i), 0x1D) != 0
}

/// `CTheCarGenerators::Save()` (0x5D38C0): the number of saved generators (+0x1C clear, +0x1D set), the bytes
/// 0xC279D1 and 0xC279D0, each saved generator as its index (2 bytes) and its 0x20 bytes, the dword 0xC27AC8,
/// the 15 0x10-byte entries at 0xC279D8; returns 1.
#[hook(addr = 0x5D38C0, conv = cdecl, name = "CTheCarGenerators::Save", expect = "83 EC 08 33 D2")]
pub fn car_generators_save() -> u8 {
    let mut n = [(0..500).filter(|&i| saved_generator(i)).count() as u32];
    save_data(GPtr::from_raw(n.as_mut_ptr()).cast(), 4);
    save_data(g::<u8>(0xC279D1), 1);
    save_data(g::<u8>(0xC279D0), 1);
    for i in 0..500u32 {
        if saved_generator(i) {
            let mut idx = [i];
            save_data(GPtr::from_raw(idx.as_mut_ptr()).cast(), 2);
            save_data(generator(i), 0x20);
        }
    }
    save_data(g::<u8>(0xC27AC8), 4);
    for k in 0..15 {
        save_data(g::<u8>(0xC279D8 + k * 0x10), 0x10);
    }
    1
}

/// `CTheCarGenerators::Load()` (0x5D39B0): `Init`, the count (0xC279D4) and the two bytes, then per entry an
/// index word and, when below 500, the generator's 0x20 bytes (a larger index consumes only its 2 bytes, as
/// the bytes do), the dword 0xC27AC8 and the 15 0x10-byte entries; returns 1.
#[hook(addr = 0x5D39B0, conv = cdecl, name = "CTheCarGenerators::Load", expect = "51 56 57 E8 B8 F8 11 00")]
pub fn car_generators_load() -> u8 {
    car_generators_init();
    load_data(g::<u8>(0xC279D4), 4);
    load_data(g::<u8>(0xC279D1), 1);
    load_data(g::<u8>(0xC279D0), 1);
    let mut i: u16 = 0;
    while i32::from(i) < g::<i32>(0xC279D4).read() {
        let mut idx = [0u16];
        load_data(GPtr::from_raw(idx.as_mut_ptr()).cast(), 2);
        if idx[0] < 500 {
            load_data(generator(u32::from(idx[0])), 0x20);
        }
        i = i.wrapping_add(1);
    }
    load_data(g::<u8>(0xC27AC8), 4);
    for k in 0..15 {
        load_data(g::<u8>(0xC279D8 + k * 0x10), 0x10);
    }
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_READ_STATUS,
    HOOK_QUERY_MODELS_KILLED_BY_PLAYER,
    HOOK_CREATE_BEACH_TOY,
    HOOK_CAR_GENERATORS_SAVE,
    HOOK_CAR_GENERATORS_LOAD,
];
