//! `CGarage` / `CGarages` leaves (PORT-WORLD). Ported from the 1.0 US bytes (gta-reversed `Garage.cpp`,
//! `Garages.cpp`, `docs/re/specs/WORLD/CGarage*.md` as reading aids).
//!
//! `CGarage` (0xD8), `aGarages[50]` at 0x96C048: +0 base (x, y, z), +0xC dir A (x, y), +0x14 dir B (x, y),
//! +0x1C top z, +0x20 width (along A), +0x24 depth (along B), +0x38 door position (f32), +0x3C, +0x44 name
//! (char[8]), +0x4C type, +0x4D door state, +0x4E flags (bit 1 inactive), +0x50 `CAEDoorAudioEntity`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr, x};

pub const MAX_GARAGES: usize = 50;
pub const GARAGE_SIZE: usize = 0xD8;

const _: () = assert!(GARAGE_SIZE == sa_core::layouts::size::CGarage);

sa_core::globals! {
    list GLOBALS;
    /// `CGarages::NumGarages`
    pub static NUM_GARAGES @ 0x96C024: u32 = 0;
    /// `CGarages::bCamShouldBeOutside`
    pub static CAM_SHOULD_BE_OUTSIDE @ 0x96BFE4: u8 = 0;
}

/// `CGarages::aGarages` (constructed by the static initialiser 0x849E90, owned by this module)
pub const GARAGES: usize = 0x96C048;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `mod_HandlingManager.m_aVehicleHandling` (0xE0 each), flags at +0xD0
const HANDLING: usize = 0xC2B9DC;

orig! {
    /// `CAEDoorAudioEntity::Reset` (0x4DC6B0)
    pub fn door_audio_reset(e: GPtr<u8>) = thiscall 0x4DC6B0 => unported;
    /// `CAESound::~CAESound` (0x4EF660)
    pub fn ae_sound_dtor(s: GPtr<u8>) = thiscall 0x4EF660 => unported;
    /// `CGarage::Update(int32 index)` (0x44AA50)
    pub fn garage_update(gp: GPtr<u8>, i: i32) = thiscall 0x44AA50 => unported;
    /// `CGarage::TidyUpGarage` (0x449C50)
    pub fn tidy_up_garage(gp: GPtr<u8>) = thiscall 0x449C50 => unported;
    /// `CGarage::TidyUpGarageClose` (0x449D10)
    pub fn tidy_up_garage_close(gp: GPtr<u8>) = thiscall 0x449D10 => unported;
    /// `CGameLogic::IsCoopGameGoingOn` (0x441390)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CBaseModelInfo::AsAtomicModelInfoPtr` (vtable slot 4)
    pub fn as_atomic_model_info_ptr(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
}

pub fn garage(i: i32) -> GPtr<u8> {
    g::<u8>(GARAGES).byte_add(i as isize * GARAGE_SIZE as isize)
}

/// `CGarage::CGarage` (0x4470E0, thiscall): +0x60 = 0, audio vtable 0x85998C, +0xCC..+0xD4 = 0; EAX = this.
#[hook(addr = 0x4470E0, conv = thiscall, name = "CGarage::CGarage", expect = "8B C1 33 C9 89 48 60", this = "CGarage")]
pub fn ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x60, 0);
    wr::<u32>(this, 0x50, 0x0085_998C);
    wr::<u32>(this, 0xCC, 0);
    wr::<u32>(this, 0xD0, 0);
    wr::<u32>(this, 0xD4, 0);
    this
}

/// `CGarage::~CGarage` (0x447110, thiscall): audio entity vtable 0x859988, then `~CAESound` of +0x58.
#[hook(addr = 0x447110, conv = thiscall, name = "CGarage::~CGarage", expect = "C7 41 50 88 99 85 00", this = "CGarage")]
pub fn dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0x50, 0x0085_9988);
    ae_sound_dtor(this.byte_add(0x58));
}

/// `CGarages::Init` (0x447120): the globals of the bytes, then model index 0 of every stored car
/// (`aCarsInSafeHouse`, 20 per hideout, 4 rows of 0x40 bytes).
#[hook(addr = 0x447120, conv = cdecl, name = "CGarages::Init", expect = "53 33 DB 83 C8 FF")]
pub fn init() {
    NUM_GARAGES.set(0);
    for a in [0x96C01C, 0x96C020, 0x96C004, 0x96C000, 0x96BFFC, 0x96BFEC, 0x96BFF0, 0x96BFF4, 0x96BFF8, 0x96BFE0] {
        g::<u32>(a).write(0);
    }
    g::<u8>(0x96C00B).write(0);
    g::<u8>(0x96C00A).write(0);
    g::<i32>(0x96BFE8).write(-1);
    g::<i32>(0x96BFDC).write(-1);
    g::<u8>(0x96C008).write(0);
    let mut e = 0x96ABEA;
    while e < 0x96ACEA {
        for k in 0..20 {
            g::<u16>(e + 0x100 * k).write(0);
        }
        e += 0x40;
    }
}

/// `CGarages::Shutdown` (stub 0x4471B0 → 0x1566E00): `NumGarages` = 0, `Reset` of all 50 door audio
/// entities.
#[hook(addr = 0x4471B0, conv = cdecl, name = "CGarages::Shutdown", expect = "E9 4B FC 11 01")]
pub fn shutdown() {
    NUM_GARAGES.set(0);
    for i in 0..MAX_GARAGES as i32 {
        door_audio_reset(garage(i).byte_add(0x50));
    }
}

fn door_closed(gp: GPtr<u8>) {
    wr::<u8>(gp, 0x4D, 0);
    wr::<u32>(gp, 0x38, 0);
}

fn door_open(gp: GPtr<u8>) {
    wr::<u8>(gp, 0x4D, 1);
    wr::<u32>(gp, 0x38, 0x3F80_0000);
}

/// `CGarage::InitDoorsAtStart` (0x447600, thiscall): flags (+0x4E & 0x39) | 0x40, door state 0, +0x3C = 0;
/// then by type: 1, 6..12, 14..45 closed; 2..5 open (1.0); others unchanged.
#[hook(addr = 0x447600, conv = thiscall, name = "CGarage::InitDoorsAtStart", expect = "8A 41 4E 24 39", this = "CGarage")]
pub fn init_doors_at_start(this: GPtr<u8>) {
    wr::<u8>(this, 0x4E, (rd::<u8>(this, 0x4E) & 0x39) | 0x40);
    let i = u32::from(rd::<u8>(this, 0x4C)).wrapping_sub(1);
    wr::<u8>(this, 0x4D, 0);
    wr::<u32>(this, 0x3C, 0);
    if i > 0x2C {
        return;
    }
    match i {
        1..=4 => door_open(this),
        12 => {}
        _ => door_closed(this),
    }
}

/// `CGarages::GetGarageNumberByName(const char*)` (stub 0x447680 → 0x156DCF0): `_stricmp` against the
/// names; -1 when none.
#[hook(addr = 0x447680, conv = cdecl, name = "CGarages::GetGarageNumberByName", expect = "E9 6B 66 12 01")]
pub fn get_garage_number_by_name(name: GPtr<u8>) -> i16 {
    if NUM_GARAGES.get() == 0 {
        return -1;
    }
    let mut i: i16 = 0;
    loop {
        if stricmp(name, garage(i32::from(i)).byte_add(0x44)) == 0 {
            return i;
        }
        i = i.wrapping_add(1);
        if (i32::from(i) as u32) >= NUM_GARAGES.get() {
            return -1;
        }
    }
}

/// `CGarages::ChangeGarageType(int16, uint8 type, uint32)` (stub 0x4476D0 → 0x1561690).
#[hook(addr = 0x4476D0, conv = cdecl, name = "CGarages::ChangeGarageType", expect = "E9 BB 9F 11 01")]
pub fn change_garage_type(idx: i16, ty: u8) {
    let gp = garage(i32::from(idx));
    wr::<u8>(gp, 0x4C, ty);
    if ty < 2 {
        door_closed(gp);
    } else if ty <= 5 {
        if rd::<u8>(gp, 0x4D) == 0 {
            door_open(gp);
        }
    } else if ty != 0x2B {
        door_closed(gp);
    }
}

/// `thunk_FUN_01565260` (stub 0x447720 → 0x1565260, thiscall `CGarage*`, `ret 4`, called from
/// `CGarage::Update`): whether the vehicle's handling flags (+0xD0) suit the garage type 0x24 (bit 25
/// set), 0x25 (bit 26 set), 0x26 (bits 25, 26 clear); false for other types or no vehicle.
#[hook(addr = 0x447720, conv = thiscall, name = "thunk_FUN_01565260", expect = "E9 3B DB 11 01", this = "CGarage")]
pub fn vehicle_suits_garage(this: GPtr<u8>, veh: GPtr<u8>) -> u8 {
    if veh.is_null() {
        return 0;
    }
    let model = i32::from(rd::<i16>(veh, 0x22));
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model).read();
    let h = g::<u8>(HANDLING).byte_add(isize::from(rd::<u8>(mi, 0x4A)) * 0xE0);
    let f = rd::<u32>(h, 0xD0);
    match rd::<u8>(this, 0x4C) {
        0x24 => (f & 0x0200_0000 != 0) as u8,
        0x25 => (f & 0x0400_0000 != 0) as u8,
        0x26 => (f & 0x0600_0000 == 0) as u8,
        _ => 0,
    }
}

/// `CGarages::DeActivateGarage(int16)` (stub 0x447CB0 → 0x1563E70): flags |= 2.
#[hook(addr = 0x447CB0, conv = cdecl, name = "CGarages::DeActivateGarage", expect = "E9 BB C1 11 01")]
pub fn deactivate_garage(idx: i16) {
    let gp = garage(i32::from(idx));
    wr::<u8>(gp, 0x4E, rd::<u8>(gp, 0x4E) | 2);
}

/// `CGarages::ActivateGarage(int16)` (stub 0x447CD0 → 0x15677F0): flags &= ~2; a type 0xB garage
/// with a closed door starts opening (3).
#[hook(addr = 0x447CD0, conv = cdecl, name = "CGarages::ActivateGarage", expect = "E9 1B FB 11 01")]
pub fn activate_garage(idx: i16) {
    let gp = garage(i32::from(idx));
    wr::<u8>(gp, 0x4E, rd::<u8>(gp, 0x4E) & 0xFD);
    if rd::<u8>(gp, 0x4C) == 0xB && rd::<u8>(gp, 0x4D) == 0 {
        wr::<u8>(gp, 0x4D, 3);
    }
}

/// `CGarages::IsGarageOpen(int16)` (stub 0x447D00 → 0x156D2D0): door state 1 or 4.
#[hook(addr = 0x447D00, conv = cdecl, name = "CGarages::IsGarageOpen", expect = "E9 CB 55 12 01")]
pub fn is_garage_open(idx: i16) -> u8 {
    matches!(rd::<u8>(garage(i32::from(idx)), 0x4D), 1 | 4) as u8
}

/// `CGarages::IsGarageClosed(int16)` (stub 0x447D30 → 0x1560F70): door state 0.
#[hook(addr = 0x447D30, conv = cdecl, name = "CGarages::IsGarageClosed", expect = "E9 3B 92 11 01")]
pub fn is_garage_closed(idx: i16) -> u8 {
    (rd::<u8>(garage(i32::from(idx)), 0x4D) == 0) as u8
}

/// `CGarage::OpenThisGarage` (stub 0x447D50 → 0x1567910, thiscall): door 0, 2, 5 → 3 (opening).
#[hook(addr = 0x447D50, conv = thiscall, name = "CGarage::OpenThisGarage", expect = "E9 BB FB 11 01", this = "CGarage")]
pub fn open_this_garage(this: GPtr<u8>) {
    if matches!(rd::<u8>(this, 0x4D), 0 | 2 | 5) {
        wr::<u8>(this, 0x4D, 3);
    }
}

/// `CGarage::CloseThisGarage` (stub 0x447D70 → 0x156D5C0, thiscall): door 1, 3 → 2 (closing).
#[hook(addr = 0x447D70, conv = thiscall, name = "CGarage::CloseThisGarage", expect = "E9 4B 58 12 01", this = "CGarage")]
pub fn close_this_garage(this: GPtr<u8>) {
    if matches!(rd::<u8>(this, 0x4D), 1 | 3) {
        wr::<u8>(this, 0x4D, 2);
    }
}

/// `CGarages::CameraShouldBeOutside` (stub 0x448650 → 0x1565080).
#[hook(addr = 0x448650, conv = cdecl, name = "CGarages::CameraShouldBeOutside", expect = "E9 2B CA 11 01")]
pub fn camera_should_be_outside() -> u8 {
    CAM_SHOULD_BE_OUTSIDE.get()
}

/// `CGarage::PlayerArrestedOrDied` (0x4486C0, thiscall): types 2..5, 13: door 0, 2, 3 → 3; types 1,
/// 6..12, 14..45: door 1..3 → 2; other types unchanged.
#[hook(addr = 0x4486C0, conv = thiscall, name = "CGarage::PlayerArrestedOrDied", preserve = "ecx,edx", expect = "0F B6 41 4C 48", this = "CGarage")]
pub fn player_arrested_or_died(this: GPtr<u8>) {
    let i = u32::from(rd::<u8>(this, 0x4C)).wrapping_sub(1);
    if i > 0x2C {
        return;
    }
    let d = rd::<u8>(this, 0x4D);
    if matches!(i, 1..=4 | 12) {
        if d == 0 || (2..=3).contains(&d) {
            wr::<u8>(this, 0x4D, 3);
        }
    } else if (1..=3).contains(&d) {
        wr::<u8>(this, 0x4D, 2);
    }
}

fn gf(gp: GPtr<u8>, o: isize) -> f32 {
    rd::<f32>(gp, o)
}

/// `CGarage::IsPointInsideGarage(CVector)` (0x448740, thiscall, by value, `ret 0xC`).
#[hook(addr = 0x448740, conv = thiscall, name = "CGarage::IsPointInsideGarage-CVector", preserve = "edx", expect = "51 D9 44 24 10", this = "CGarage")]
pub fn is_point_inside_garage(this: GPtr<u8>, px: f32, py: f32, pz: f32) -> u8 {
    // test ah,5; jnp: false on Less
    if !x(pz).fcom(x(gf(this, 8))).test_parity(0x05) {
        return 0;
    }
    // test ah,0x41; je: false on Greater
    if !x(pz).fcom(x(gf(this, 0x1C))).test_any(0x41) {
        return 0;
    }
    let dy = x(py).fsub_m(gf(this, 4)).to_f32();
    let dx = x(px).fsub_m(gf(this, 0));
    let dxf = dx.to_f32();
    let t1 = dx.fmul_m(gf(this, 0xC)).fadd(x(dy).fmul_m(gf(this, 0x10)));
    if !t1.fcom(X87::zero()).test_parity(0x05) || !t1.fcom(x(gf(this, 0x20))).test_any(0x41) {
        return 0;
    }
    let t2 = x(dxf).fmul_m(gf(this, 0x14)).fadd(x(dy).fmul_m(gf(this, 0x18)));
    if !t2.fcom(X87::zero()).test_parity(0x05) || !t2.fcom(x(gf(this, 0x24))).test_any(0x41) {
        return 0;
    }
    1
}

/// `CGarage::IsPointInsideGarage(CVector, float margin)` (0x4487D0, thiscall, by value, `ret 0x10`).
#[hook(addr = 0x4487D0, conv = thiscall, name = "CGarage::IsPointInsideGarage-CVector-float", preserve = "ecx,edx", expect = "D9 41 08 83 EC 08", this = "CGarage")]
pub fn is_point_inside_garage_margin(this: GPtr<u8>, px: f32, py: f32, pz: f32, m: f32) -> u8 {
    // (z0 - m) vs z: false on Greater
    if !x(gf(this, 8)).fsub_m(m).fcom(x(pz)).test_any(0x41) {
        return 0;
    }
    // (m + top) vs z: false on Less
    if !x(m).fadd_m(gf(this, 0x1C)).fcom(x(pz)).test_parity(0x05) {
        return 0;
    }
    let dy = x(py).fsub_m(gf(this, 4)).to_f32();
    let dx = x(px).fsub_m(gf(this, 0));
    let dxf = dx.to_f32();
    let t1 = dx.fmul_m(gf(this, 0xC)).fadd(x(dy).fmul_m(gf(this, 0x10)));
    let neg_m = x(m).fchs().to_f32();
    if !t1.fcom(x(neg_m)).test_parity(0x05) {
        return 0;
    }
    if !t1.fcom(x(m).fadd_m(gf(this, 0x20))).test_any(0x41) {
        return 0;
    }
    let t2 = x(dxf).fmul_m(gf(this, 0x14)).fadd(x(dy).fmul_m(gf(this, 0x18)));
    if !t2.fcom(x(neg_m)).test_parity(0x05) {
        return 0;
    }
    if !t2.fcom(x(m).fadd_m(gf(this, 0x24))).test_any(0x41) {
        return 0;
    }
    1
}

/// `CGarages::IsModelIndexADoor(int32)` (0x448AF0): an atomic model info (slot 4, called twice) whose
/// flags word +0x12 has bits 0x7800 == 0x3800.
#[hook(addr = 0x448AF0, conv = cdecl, name = "CGarages::IsModelIndexADoor", expect = "8B 44 24 04 85 C0")]
pub fn is_model_index_a_door(model: i32) -> u8 {
    if model < 0 {
        return 0;
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model).read();
    if mi.is_null() || as_atomic_model_info_ptr(mi).is_null() {
        return 0;
    }
    let a = as_atomic_model_info_ptr(mi);
    (u32::from(rd::<u16>(a, 0x12)) & 0x7800 == 0x3800) as u8
}

/// `CGarages::AllRespraysCloseOrOpen(bool)` (stub 0x448B30 → 0x1563A10): door state of every type-5
/// garage = the flag.
#[hook(addr = 0x448B30, conv = cdecl, name = "CGarages::AllRespraysCloseOrOpen", expect = "E9 DB AE 11 01")]
pub fn all_resprays_close_or_open(open: u8) {
    let n = NUM_GARAGES.get();
    for i in 0..n {
        let gp = garage(i as i32);
        if rd::<u8>(gp, 0x4C) == 5 {
            wr::<u8>(gp, 0x4D, (open != 0) as u8);
        }
    }
}

/// `|cam[k] - garage[off]|` as the original computes it (sign test against 0, `fchs` of a recomputed
/// difference when negative), compared with 40: true when above (`TidyUpGarage`).
fn far_on_axis(cam: GPtr<f32>, k: i32, gp: GPtr<u8>, off: isize) -> bool {
    let d = x(cam.wrapping_index(k).read()).fsub_m(rd::<f32>(gp, off));
    let a = if !d.fcom(X87::zero()).test_parity(0x05) { d.fchs() } else { d };
    !a.fcom(x(40.0)).test_any(0x41)
}

/// `CGarages::Update` (0x44C8C0): not during replay playback or a co-op game; camera-outside flag 0,
/// two camera words reset (0xB6F844 saved to 0x96BFD8), `CGarage::Update(i)` of every garage with a
/// type; every 16th frame (counter & 15 == 12) the next garage in turn is tidied: `TidyUpGarage` when
/// the camera is more than 40 away on x or y from its +0x28/+0x30, else `TidyUpGarageClose`.
#[hook(addr = 0x44C8C0, conv = cdecl, name = "CGarages::Update", expect = "80 3D 88 30 A4 00 01")]
pub fn update() {
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    if is_coop_game_going_on() != 0 {
        return;
    }
    CAM_SHOULD_BE_OUTSIDE.set(0);
    g::<u32>(0x96BFD8).write(g::<u32>(0xB6F844).read());
    g::<u32>(0xB6F844).write(0);
    g::<u32>(0xB6F848).write(0);
    for i in 0..MAX_GARAGES as i32 {
        if rd::<u8>(garage(i), 0x4C) != 0 {
            garage_update(garage(i), i);
        }
    }
    if g::<u32>(0xB7CB4C).read() as u8 & 0xF != 0xC {
        return;
    }
    let mut n = g::<u32>(0x96EA78).read().wrapping_add(1);
    g::<u32>(0x96EA78).write(n);
    if n >= 0x32 {
        n = 0;
        g::<u32>(0x96EA78).write(0);
    }
    let gp = garage(n as i32);
    if rd::<u8>(gp, 0x4C) == 0 {
        return;
    }
    let e = g::<GPtr<u8>>(0xB6F03C).read();
    let cam: GPtr<f32> = if e.is_null() { g(0xB6F02C) } else { e.byte_add(0x30).cast() };
    if far_on_axis(cam, 0, gp, 0x28) || far_on_axis(cam, 1, gp, 0x30) {
        tidy_up_garage(gp);
    } else {
        tidy_up_garage_close(gp);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_UPDATE,
    HOOK_CTOR,
    HOOK_DTOR,
    HOOK_INIT,
    HOOK_SHUTDOWN,
    HOOK_INIT_DOORS_AT_START,
    HOOK_GET_GARAGE_NUMBER_BY_NAME,
    HOOK_CHANGE_GARAGE_TYPE,
    HOOK_VEHICLE_SUITS_GARAGE,
    HOOK_DEACTIVATE_GARAGE,
    HOOK_ACTIVATE_GARAGE,
    HOOK_IS_GARAGE_OPEN,
    HOOK_IS_GARAGE_CLOSED,
    HOOK_OPEN_THIS_GARAGE,
    HOOK_CLOSE_THIS_GARAGE,
    HOOK_CAMERA_SHOULD_BE_OUTSIDE,
    HOOK_PLAYER_ARRESTED_OR_DIED,
    HOOK_IS_POINT_INSIDE_GARAGE,
    HOOK_IS_POINT_INSIDE_GARAGE_MARGIN,
    HOOK_IS_MODEL_INDEX_A_DOOR,
    HOOK_ALL_RESPRAYS_CLOSE_OR_OPEN,
];
