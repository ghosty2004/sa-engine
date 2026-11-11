//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `CBoat`. Ported from the 1.0 US bytes; gta-reversed
//! and docs/re/specs/VEH as reading aids.
//!
//! Wake trail: `+0x644` point count (u16), `+0x648 + 8 * i` point x/y, `+0x748 + 4 * i` lifetime,
//! `+0x7C8 + i` intensity byte (32 entries each).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, jp5, rd, wr, x};

/// `CBoat*`.
type Boat = GPtr<u8>;

sa_core::globals! {
    list GLOBALS;
    /// `CBoat::apFrameWakeGeneratingBoats[4]`
    pub static FRAME_WAKE_GENERATING_BOATS @ 0xC27994: [GPtr<u8>; 4] = [GPtr::NULL; 4];
}

orig! {
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    pub fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CAEVehicleAudioEntity::Terminate()` (0x4FB8C0)
    pub fn audio_terminate(a: GPtr<u8>) = thiscall 0x4FB8C0 => unported;
    /// `CVehicle::~CVehicle` body (0x6E2B40)
    pub fn vehicle_destructor(v: GPtr<u8>) = thiscall 0x6E2B40 => unported;
    /// `CVehicle::operator delete(void*)` (0x6E2D90)
    pub fn vehicle_delete(p: GPtr<u8>) = cdecl 0x6E2D90 => crate::vehicle2::operator_delete;
    /// `CVehicle::CVehicle(eVehicleCreatedBy)` (0x6D5F10)
    pub fn vehicle_ctor(v: GPtr<u8>, created_by: u32) -> GPtr<u8> = thiscall 0x6D5F10 => unported;
    /// `CVehicle::SetModelIndex(uint32)` (0x6D6A40)
    pub fn vehicle_set_model_index(v: GPtr<u8>, mi: u32) = thiscall 0x6D6A40 => unported;
    /// `CClumpModelInfo::FillFrameArray(RpClump*, RwFrame**)` (0x4C5440)
    pub fn fill_frame_array(clump: GPtr<u8>, frames: GPtr<u8>) = cdecl 0x4C5440 => unported;
    /// `CVehicleModelInfo::ChooseVehicleColour(uint8&, uint8&, uint8&, uint8&, int32)` (0x4C8500)
    pub fn choose_vehicle_colour(mi: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, inc: i32) = thiscall 0x4C8500 => unported;
    /// `CAEVehicleAudioEntity::Initialise(CEntity*)` (0x4F7670)
    pub fn audio_initialise(a: GPtr<u8>, e: GPtr<u8>) = thiscall 0x4F7670 => unported;
    /// `cHandlingDataMgr::GetFlyingPointer(uint8)` (0x6F52D0)
    pub fn get_flying_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x6F52D0 => crate::handling::get_flying_pointer;
    /// `cHandlingDataMgr::GetBoatPointer(uint8)` (0x6F5300)
    pub fn get_boat_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x6F5300 => crate::handling::get_boat_pointer;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetBrake()` (0x540080)
    pub fn pad_get_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540080 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0)
    pub fn pad_get_accelerate(pad: GPtr<u8>) -> i16 = thiscall 0x5403F0 => unported;
    /// `CPad::GetSteeringLeftRight()` (0x53FB80)
    pub fn pad_get_steering_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FB80 => unported;
    /// `CBoat::~CBoat` (0x6F00F0)
    pub fn boat_destructor_o(b: GPtr<u8>) = thiscall 0x6F00F0 => crate::boat2::destructor;
}

/// `CBoat::WAKE_LIFETIME` (0x8D3940).
const WAKE_LIFETIME: usize = 0x8D3940;
/// `CBoat::MIN_WAKE_INTERVAL` (0x8D393C).
const MIN_WAKE_INTERVAL: usize = 0x8D393C;

/// `CBoat::~CBoat()` (0x6F00F0, catalogue name `CBoat::Destructor`): vtable 0x8721A0; the fx systems
/// `+0x57C`, `+0x628`, `+0x62C` killed and cleared when set; audio Terminate; `CVehicle::~CVehicle`.
#[hook(addr = 0x6F00F0, conv = thiscall, name = "CBoat::Destructor", expect = "6A FF 68 F8 88 84 00", this = "CBoat")]
pub fn destructor(b: Boat) {
    wr::<u32>(b, 0, 0x0087_21A0);
    for off in [0x57C, 0x628, 0x62C] {
        let fx = rd::<GPtr<u8>>(b, off);
        if !fx.is_null() {
            fx_kill(fx);
            wr::<u32>(b, off, 0);
        }
    }
    audio_terminate(b.byte_add(0x138));
    vehicle_destructor(b);
}

/// `CBoat::scalar deleting destructor(uint8)` (0x6F1120).
#[hook(addr = 0x6F1120, conv = thiscall, name = "CBoat::`scalar deleting destructor'", expect = "56 8B F1 E8 C8 EF FF FF", this = "CBoat")]
pub fn scalar_deleting_dtor(b: Boat, flags: u32) -> Boat {
    boat_destructor_o(b);
    if flags & 1 != 0 {
        vehicle_delete(b);
    }
    b
}

/// `CBoat::AddWakePoint(CVector posn)` (0x6F2550): intensity = low byte of `_ftol(|moveSpeed| *
/// [0x858628])` (`sqrt((x² + y²) + z²)`). When the newest lifetime is not `> 0` the trail restarts with
/// this point (count 1). Otherwise, when the boat moved more than `MIN_WAKE_INTERVAL` from the newest
/// point (squared, `dy² + dx²`, matrix position or `+4` without a matrix), the points shift up by one
/// (at most 31 for status 0, 20 for `+0x4A4 == 2`, else 15 entries move) and the new point goes to 0;
/// the count grows up to 32.
#[hook(addr = 0x6F2550, conv = thiscall, name = "CBoat::AddWakePoint", expect = "53 56 8B F1 D9 46 4C", this = "CBoat")]
pub fn add_wake_point(b: Boat, px: u32, py: u32, _pz: u32) {
    let (vz, vy, vx) = (
        x(rd::<f32>(b, 0x4C)),
        x(rd::<f32>(b, 0x48)),
        x(rd::<f32>(b, 0x44)),
    );
    let speed = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt();
    let intensity = ftol32(speed.fmul_m(g::<f32>(0x858628).read())) as u8;
    if !x(rd::<f32>(b, 0x748))
        .fcom(x(g::<f32>(0x858B50).read()))
        .test_parity(0x41)
    {
        wr::<u8>(b, 0x7C8, intensity);
        wr::<u32>(b, 0x648, px);
        wr::<u32>(b, 0x64C, py);
        wr::<u32>(b, 0x748, g::<u32>(WAKE_LIFETIME).read());
        wr::<u16>(b, 0x644, 1);
        return;
    }
    let m = rd::<GPtr<u8>>(b, 0x14);
    let pos = if m.is_null() {
        b.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(b, 0x64C));
    let dy2 = dy.fmul(x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(b, 0x64C)));
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(b, 0x648));
    let dx2 = dx.fmul(x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(b, 0x648)));
    let sum = dy2.fadd(dx2);
    let mi = g::<f32>(MIN_WAKE_INTERVAL).read();
    if jp5(x(mi).fmul_m(mi), sum) {
        return;
    }
    let count = rd::<u16>(b, 0x644);
    let limit: u16 = if rd::<u8>(b, 0x36) & 0xF8 == 0 {
        0x1F
    } else if rd::<u8>(b, 0x4A4) == 2 {
        0x14
    } else {
        0xF
    };
    let n = if count >= limit { limit } else { count };
    let mut k = isize::from(n as u8);
    while k > 0 {
        wr::<u32>(b, 0x648 + 8 * k, rd::<u32>(b, 0x640 + 8 * k));
        wr::<u32>(b, 0x64C + 8 * k, rd::<u32>(b, 0x644 + 8 * k));
        wr::<u32>(b, 0x748 + 4 * k, rd::<u32>(b, 0x744 + 4 * k));
        wr::<u8>(b, 0x7C8 + k, rd::<u8>(b, 0x7C7 + k));
        k -= 1;
    }
    wr::<u32>(b, 0x648, px);
    let count = rd::<u16>(b, 0x644);
    wr::<u32>(b, 0x64C, py);
    wr::<u32>(b, 0x748, g::<u32>(WAKE_LIFETIME).read());
    wr::<u8>(b, 0x7C8, intensity);
    if count < 0x20 {
        wr::<u16>(b, 0x644, count + 1);
    }
}

/// `CBoat::CBoat(int32 modelIndex, eVehicleCreatedBy)` (0x6F2940, catalogue `CBoat::Constructor`; the
/// SEH prologue through the .HOODLUM fragment 0x4022B2): `CVehicle(createdBy)`, vtable 0x8721A0, type
/// and subtype 5, `SetModelIndex`, the 12 frames `+0x5B0`, car / flying / boat handling (the model's
/// handling id word sign-extended, its low byte indexing the car entries), colours, mass, turn mass
/// `* [0x858B8C]`, centre of mass, elasticity 0.1, buoyancy (`drag / [0x872184] * [0x858B8C]` when drag
/// `>` [0x858C58]), flags, the 32 wake lifetimes cleared, the door (model 0x1E4: ±0.314, dirn 4;
/// else ±0.628, dirn 3; axis 2), audio `Initialise`. Returns `this`.
#[hook(addr = 0x6F2940, conv = thiscall, name = "CBoat::Constructor", expect = "6A FF E9 6B F9 D0 FF", this = "CBoat")]
pub fn constructor(b: Boat, mi: u32, created_by: u32) -> Boat {
    vehicle_ctor(b, created_by);
    wr::<u32>(b, 0, 0x0087_21A0);
    for k in 0..6 {
        wr::<u32>(b, 0x5E0 + 4 * k, 0);
    }
    let mip = g::<GPtr<u8>>(0xA9B0C8 + 4 * mi as usize).read();
    let hid = i32::from(rd::<i16>(mip, 0x4A)) as u32;
    wr::<u32>(b, 0x594, 5);
    wr::<u32>(b, 0x590, 5);
    for off in [0x610, 0x614, 0x618, 0x61C, 0x620, 0x624] {
        wr::<u32>(b, off, 0);
    }
    wr::<u8>(b, 0x63D, 0);
    for off in [0x5A0, 0x5A4, 0x5A8] {
        wr::<u32>(b, off, 0);
    }
    wr::<u32>(b, 0x600, g::<u32>(0xB7CB84).read());
    vehicle_set_model_index(b, mi);
    for k in 0..0xC {
        wr::<u32>(b, 0x5B0 + 4 * k, 0);
    }
    fill_frame_array(rd::<GPtr<u8>>(b, 0x18), b.byte_add(0x5B0));
    let h = GPtr::<u8>::from_addr(0xC2B9DC + 0xE0 * (hid & 0xFF) as usize);
    wr::<GPtr<u8>>(b, 0x384, h);
    wr::<u32>(b, 0x38C, rd::<u32>(h, 0xD0));
    let mgr = GPtr::<u8>::from_addr(0xC2B9C8);
    let fp = get_flying_pointer(mgr, hid);
    wr::<GPtr<u8>>(b, 0x388, fp);
    let bp = get_boat_pointer(mgr, hid);
    wr::<GPtr<u8>>(b, 0x5F8, bp);
    choose_vehicle_colour(
        mip,
        b.byte_add(0x434),
        b.byte_add(0x435),
        b.byte_add(0x436),
        b.byte_add(0x437),
        1,
    );
    let h = rd::<GPtr<u8>>(b, 0x384);
    let k = g::<f32>(0x858B8C).read();
    wr::<u32>(b, 0x8C, rd::<u32>(h, 4));
    wr::<f32>(b, 0x90, x(rd::<f32>(h, 0xC)).fmul_m(k).to_f32());
    for i in 0..3 {
        wr::<u32>(b, 0xA4 + 4 * i, rd::<u32>(h, 0x14 + 4 * i));
    }
    wr::<u32>(b, 0x9C, 0x3DCC_CCCD);
    wr::<u32>(b, 0xA0, rd::<u32>(h, 0x24));
    if !crate::common::jne_mask(x(rd::<f32>(h, 0x10)), x(g::<f32>(0x858C58).read()), 0x41) {
        let v = x(rd::<f32>(h, 0x10)).fdiv_m(g::<f32>(0x872184).read()).fmul_m(k);
        wr::<f32>(b, 0x98, v.to_f32());
    } else {
        wr::<u32>(b, 0x98, rd::<u32>(h, 0x10));
    }
    wr::<u32>(b, 0x40, rd::<u32>(b, 0x40) | 0x0800_0100);
    let b5ac = rd::<u8>(b, 0x5AC) | 7;
    for off in [0x494, 0x49C, 0x4A0, 0x58C] {
        wr::<u32>(b, off, 0);
    }
    wr::<u8>(b, 0x63C, 0);
    wr::<u32>(b, 0x640, 0x40E0_0000);
    wr::<u32>(b, 0x604, 0);
    wr::<u8>(b, 0x5AC, b5ac);
    wr::<u32>(b, 0x5FC, 0xC61C_3FF6);
    wr::<u32>(b, 0x608, 0);
    wr::<u32>(b, 0x60C, 0);
    wr::<u16>(b, 0x644, 0);
    for i in 0..0x20 {
        wr::<u32>(b, 0x748 + 4 * i, 0);
    }
    wr::<u8>(b, 0x4AB, 0x14);
    wr::<u8>(b, 0x5EA, 2);
    if rd::<u16>(b, 0x22) == 0x1E4 {
        wr::<u32>(b, 0x5E0, 0x3EA0_D97C);
        wr::<u32>(b, 0x5E4, 0xBEA0_D97C);
        wr::<u16>(b, 0x5E8, 4);
    } else {
        wr::<u32>(b, 0x5E0, 0x3F20_D97C);
        wr::<u32>(b, 0x5E4, 0xBF20_D97C);
        wr::<u16>(b, 0x5E8, 3);
    }
    audio_initialise(b.byte_add(0x138), b);
    wr::<u32>(b, 0x62C, 0);
    wr::<u32>(b, 0x628, 0);
    b
}

/// `CEntity::GetPosition()`: the matrix position, or the placement `+4` without a matrix.
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CBoat::FillBoatList()` (0x6F2710): clears the four wake-generating boats, then walks the vehicle pool
/// from the top. A boat (`+0x590 == 5`) with wake points whose offset from the camera (x stored as f32,
/// y at register precision) has a dot product with the normalised 2D camera front (left unnormalised
/// when its length is not `> 0`) in [[0x866D14], [0x858628]] and a squared distance (from the f32
/// offsets, stored as f32) not above [0x859AEC] is appended while there is room; otherwise the list
/// entry with the smallest squared distance to the camera (`< 999999.9`, stored as f32) is replaced when
/// this boat's distance is smaller or the boat is player-controlled (status 0). (gta-reversed replaces
/// the farthest entry; the bytes keep the minimum.)
#[hook(addr = 0x6F2710, conv = cdecl, name = "CBoat::FillBoatList", expect = "83 EC 1C 33 C0")]
pub fn fill_boat_list() {
    let list = FRAME_WAKE_GENERATING_BOATS.ptr().cast::<GPtr<u8>>();
    FRAME_WAKE_GENERATING_BOATS.set([GPtr::NULL; 4]);
    let cm = g::<GPtr<u8>>(0xB6F03C).read();
    let cpos = if cm.is_null() {
        GPtr::<u8>::from_addr(0xB6F02C)
    } else {
        cm.byte_add(0x30)
    };
    let (cx, cy) = (rd::<f32>(cpos, 0), rd::<f32>(cpos, 4));
    let fx = x(g::<f32>(0xB6F9AC).read());
    let fy = x(g::<f32>(0xB6F9B0).read());
    let len = fy.fmul(fy).fadd(fx.fmul(fx)).fsqrt();
    let (dx, dy) = if crate::common::jne_mask(len, x(g::<f32>(0x858B50).read()), 0x41) {
        (fx, fy)
    } else {
        let inv = len.fdivr_m(g::<f32>(0x858624).read());
        (fx.fmul(inv), fy.fmul(inv))
    };
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let mut i = rd::<i32>(pool, 8);
    let mut count: i16 = 0;
    while i != 0 {
        i -= 1;
        let pool = g::<GPtr<u8>>(0xB74494).read();
        if rd::<u8>(rd::<GPtr<u8>>(pool, 4), i as isize) & 0x80 != 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if v.is_null() || rd::<u32>(v, 0x590) != 5 || rd::<u16>(v, 0x644) == 0 {
            continue;
        }
        let p = pos_of(v);
        let ddx = x(rd::<f32>(p, 0)).fsub_m(cx).to_f32();
        let ddy = x(rd::<f32>(pos_of(v), 4)).fsub_m(cy);
        let ddy_f = ddy.to_f32();
        let dot = ddy.fmul(dy).fadd(x(ddx).fmul(dx));
        if !dot.fcom(x(g::<f32>(0x858628).read())).test_any(0x41) {
            continue;
        }
        if !crate::common::jp5(dot, x(g::<f32>(0x866D14).read())) {
            continue;
        }
        let d2 = x(ddy_f).fmul_m(ddy_f).fadd(x(ddx).fmul_m(ddx)).to_f32();
        if !x(d2).fcom(x(g::<f32>(0x859AEC).read())).test_any(0x41) {
            continue;
        }
        if count < 4 {
            list.wrapping_index(i32::from(count)).write(v);
            count += 1;
            continue;
        }
        let mut best_i: i16 = -1;
        let mut best = f32::from_bits(0x4974_23FE);
        for k in 0..4i16 {
            let e = pos_of(list.wrapping_index(i32::from(k)).read());
            let ex = x(rd::<f32>(e, 0)).fsub_m(cx);
            let ey = x(rd::<f32>(e, 4)).fsub_m(cy);
            let sum = ey.fmul(ey).fadd(ex.fmul(ex));
            if !crate::common::jp5(sum, x(best)) {
                best = sum.to_f32();
                best_i = k;
            }
        }
        if best_i != -1 && (!crate::common::jp5(x(d2), x(best)) || rd::<u8>(v, 0x36) & 0xF8 == 0) {
            list.wrapping_index(i32::from(best_i)).write(v);
        }
    }
}

/// `fabs` (clears the sign bit, of a NaN too).
fn fabs(v: sa_math::x87::X87) -> sa_math::x87::X87 {
    sa_math::x87::X87::from_f64_exact(f64::from_bits(v.to_f64().to_bits() & !(1 << 63)))
}

/// `CBoat::ProcessControlInputs(uint8 pad)` (0x6F0A10): `+0x63D` = pad clamped to 3 (the unclamped
/// number is used for `GetPad`). Brake `+0x4A0` eases toward `GetBrake / 255` (`* [0x858B1C]`), stored,
/// then clamped to [0, 1] (NaN -> 0) and stored again; below [0x858C28] it is zeroed and the gas `+0x49C`
/// = `GetAccelerate / 255`, else gas = brake * [0x858EE4]. Steering `+0x58C`: with mouse steering
/// (0xB6EC2E and 0xC1CC02) the mouse x (0xB73424, when non-zero or while the stick is idle in mouse
/// mode) adds `* [0x872190]` unless pad `+0x2C`, mode 0xC1CC04 = 2, and `*= pow([0x872198], timeStep)`
/// when small or pad `+0x2C`; otherwise `+= ((-stick * [0x858B88]) - s) * [0x87218C] * timeStep`, mode
/// 0. Then clamped to [-1, 1], stored, and `+0x494 = steerLock * [0x8595EC] * (s * |s|)`.
#[hook(addr = 0x6F0A10, conv = thiscall, name = "CBoat::ProcessControlInputs", expect = "51 8A 44 24 08", this = "CBoat")]
pub fn process_control_inputs(b: Boat, pad: u32) {
    let k = |a: usize| g::<f32>(a).read();
    let pn = pad as u8;
    wr::<u8>(b, 0x63D, pn);
    if pn > 3 {
        wr::<u8>(b, 0x63D, 3);
    }
    let pn = i32::from(pn);
    let br = crate::common::xi(i32::from(pad_get_brake(get_pad(pn))));
    let v = br
        .fmul_m(k(0x859A3C))
        .fsub_m(rd::<f32>(b, 0x4A0))
        .fmul_m(k(0x858B1C))
        .fadd_m(rd::<f32>(b, 0x4A0));
    wr::<f32>(b, 0x4A0, v.to_f32());
    let zero = x(k(0x858B50));
    let v = if !crate::common::jne_mask(v, zero, 0x41) && !jp5(v, x(k(0x858624))) {
        v
    } else if !crate::common::jne_mask(v, zero, 0x41) {
        if jp5(v, x(k(0x858624))) { x(k(0x858624)) } else { v }
    } else {
        zero
    };
    wr::<f32>(b, 0x4A0, v.to_f32());
    if !jp5(v, x(k(0x858C28))) {
        wr::<u32>(b, 0x4A0, 0);
        let acc = crate::common::xi(i32::from(pad_get_accelerate(get_pad(pn))));
        wr::<f32>(b, 0x49C, acc.fmul_m(k(0x859A3C)).to_f32());
    } else {
        wr::<f32>(b, 0x49C, v.fmul_m(k(0x858EE4)).to_f32());
    }

    let mode = g::<u32>(0xC1CC04);
    // None: skip the steering update; Some(true): mouse path; Some(false): stick path
    let path: Option<bool> = if g::<u8>(0xB6EC2E).read() != 0 && g::<u8>(0xC1CC02).read() != 0 {
        get_pad(pn);
        // mouse x non-zero, or the stick idle in mouse mode while still steering
        if x(k(0xB73424)).fcom(zero).test_parity(0x44)
            || (!crate::common::jne_mask(fabs(x(rd::<f32>(b, 0x58C))), zero, 0x41)
                && mode.read() == 2
                && pad_get_steering_left_right(get_pad(pn)) == 0)
        {
            Some(true)
        } else if pad_get_steering_left_right(get_pad(pn)) == 0 && mode.read() == 2 {
            None
        } else {
            Some(false)
        }
    } else {
        Some(false)
    };
    match path {
        Some(true) => {
            if rd::<u16>(get_pad(pn), 0x2C) == 0 {
                get_pad(pn);
                let s = x(k(0xB73424)).fmul_m(k(0x872190)).fadd_m(rd::<f32>(b, 0x58C));
                wr::<f32>(b, 0x58C, s.to_f32());
            }
            mode.write(2);
            if !jp5(fabs(x(rd::<f32>(b, 0x58C))), x(k(0x872194))) || rd::<u16>(get_pad(pn), 0x2C) != 0 {
                let p = sa_math::x87::X87::from_f64_exact(sa_core::orig::x87_binary(
                    0x822130,
                    x(k(0x872198)).to_f64(),
                    x(k(0xB7CB5C)).to_f64(),
                ));
                wr::<f32>(b, 0x58C, p.fmul_m(rd::<f32>(b, 0x58C)).to_f32());
            }
        }
        Some(false) => {
            let ts = k(0xB7CB5C);
            let st = -i32::from(pad_get_steering_left_right(get_pad(pn)));
            let s = crate::common::xi(st)
                .fmul_m(k(0x858B88))
                .fsub_m(rd::<f32>(b, 0x58C))
                .fmul_m(k(0x87218C))
                .fmul_m(ts)
                .fadd_m(rd::<f32>(b, 0x58C));
            wr::<f32>(b, 0x58C, s.to_f32());
            mode.write(0);
        }
        None => {}
    }
    let s = rd::<f32>(b, 0x58C);
    let one = x(k(0x858624));
    let v = if !jp5(one, x(s)) {
        one
    } else if crate::common::jne_mask(x(k(0x858C1C)), x(s), 0x41) {
        x(s)
    } else {
        x(k(0x858C1C))
    };
    wr::<f32>(b, 0x58C, v.to_f32());
    let q = if v.fcom(zero).test_any(1) {
        v.fmul(v).fchs()
    } else {
        v.fmul(v)
    };
    let h = rd::<GPtr<u8>>(b, 0x384);
    let r = x(rd::<f32>(h, 0xA0)).fmul_m(k(0x8595EC)).fmul(q);
    wr::<f32>(b, 0x494, r.to_f32());
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_CONTROL_INPUTS,
    HOOK_FILL_BOAT_LIST,
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_SCALAR_DELETING_DTOR,
    HOOK_ADD_WAKE_POINT,
];
