//! WRITER-VEH-BIG (batch `vehbig-1`): `CAutomobile::PreRender`, the per-frame visual update of a
//! four-wheeled vehicle (wheel suspension display, wheel particles, rain splashes, exhaust,
//! emergency / taxi lights, lights and shadow, wheel rotation and matrices, doors, panels and the
//! special moving parts of some models). Ported from the 1.0 US bytes (gta-reversed has no body).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::find_player_vehicle;

use crate::common::{ftol32, g, rd, wr, x, xi};

orig! {
    /// `CVehicle::PreRender()` (0x6D6480)
    fn vehicle_pre_render(v: GPtr<u8>) = thiscall 0x6D6480 => crate::vehicle2::pre_render;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => sa_world::world::entity::get_col_model;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden return pointer)
    fn matrix_mul_vector(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => crate::vehicle_lights::matrix_mul_vector_native;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CDamageManager::GetWheelStatus(int32)` (0x6C21B0)
    fn get_wheel_status(dm: GPtr<u8>, wheel: i32) -> u32 = thiscall 0x6C21B0 => crate::damage::get_wheel_status;
    /// `CVehicle::AddSingleWheelParticles(...)` (0x6DE880)
    fn add_single_wheel_particles(v: GPtr<u8>, state: u32, status: u32, speed: f32, skid: f32, cp: GPtr<u8>, from: GPtr<u8>, id: i32, wheel: i32, kind: u32, blood: GPtr<u8>, flags: u32) -> u8 = thiscall 0x6DE880 => crate::vehicle_wheel_particles::vehicle_add_single_wheel_particles;
    /// `CCullZones::CamNoRain()` (0x72DDB0)
    fn cam_no_rain() -> u8 = cdecl 0x72DDB0 => unported;
    /// `CCullZones::PlayerNoRain()` (0x72DDC0)
    fn player_no_rain() -> u8 = cdecl 0x72DDC0 => unported;
    /// `CVehicle::AddWaterSplashParticles()` (0x6DDF60)
    fn add_water_splash_particles(v: GPtr<u8>) = thiscall 0x6DDF60 => unported;
    /// `CVehicle::AddExhaustParticles()` (0x6DE240)
    fn add_exhaust_particles(v: GPtr<u8>) = thiscall 0x6DE240 => crate::mvpd_v2::vehicle_add_exhaust_particles;
    /// `CVehicle::AddDamagedVehicleParticles()` (0x6D2A80)
    fn add_damaged_vehicle_particles(v: GPtr<u8>) = thiscall 0x6D2A80 => crate::vehicle2::add_damaged_vehicle_particles;
    /// `CPointLights::AddLight(...)` (0x7000E0), floats as bits
    fn add_light(kind: u32, px: u32, py: u32, pz: u32, dx: u32, dy: u32, dz: u32, radius: u32, r: u32, gg: u32, b: u32, fog: u32, shadows: u32, attached: u32) = cdecl 0x7000E0 => unported;
    /// `CCoronas::RegisterCorona(id, attachTo, r, g, b, a, pos, size, range, eCoronaType, ...)` (0x6FC580)
    fn register_corona(id: u32, attach: GPtr<u8>, r: u32, gg: u32, b: u32, a: u32, pos: GPtr<u8>, size: u32, range: u32, kind: u32, flare: u32, refl: u32, los: u32, trail: u32, angle: u32, neon: u32, pull: u32, bright: u32, fade: u32, below: u32, delay: u32) = cdecl 0x6FC580 => unported;
    /// `CPlaceable::GetTopDirection`-style accessor (0x41CCB0)
    fn get_top_direction(e: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x41CCB0 => sa_world::world::entity::get_top_direction;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> u32 = thiscall 0x6D1080 => crate::vehicle2::get_vehicle_appearance;
    /// `CVehicle::DoVehicleLights(CMatrix&, uint32)` (0x6E1A60)
    fn do_vehicle_lights(v: GPtr<u8>, m: GPtr<u8>, flags: u32) = thiscall 0x6E1A60 => crate::vehicle_lights::vehicle_do_vehicle_lights;
    /// `CCamera::GetLookingForwardFirstPerson()` (0x50AED0) on `TheCamera`
    fn get_looking_forward_first_person(cam: GPtr<u8>) -> u8 = thiscall 0x50AED0 => unported;
    /// `CShadows::StoreShadowForVehicle(CVehicle*, VEH_SHD_TYPE)` (0x70BDA0)
    fn store_shadow_for_vehicle(v: GPtr<u8>, kind: u32) = cdecl 0x70BDA0 => unported;
    /// `CVehicle::SetComponentRotation(RwFrame*, eRotationAxis, float, bool)` (0x6DBA30)
    fn set_component_rotation(v: GPtr<u8>, frame: GPtr<u8>, axis: u32, angle: f32, reset: u32) = thiscall 0x6DBA30 => crate::vehicle2::set_component_rotation;
    /// `Multiply3x3(CVector*, const CMatrix*, const CVector*)` (0x59C790)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => sa_world::world::mvpa_math::multiply3x3;
    /// `CPhysical::GetSpeed(CVector)` (0x542CE0)
    fn get_speed(e: GPtr<u8>, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> = thiscall 0x542CE0 => sa_world::physical::forces::get_speed;
    /// `CVehicle::ProcessWheelRotation(tWheelState, const CVector&, const CVector&, float)` (0x6D1230), ST0
    fn process_wheel_rotation(v: GPtr<u8>, state: u32, fwd: GPtr<u8>, speed: GPtr<u8>, radius: f32) -> f64 = thiscall 0x6D1230 => unported;
    /// `CAutomobile::UpdateWheelMatrix(int32 node, int32 flags)` (0x6AA290)
    fn update_wheel_matrix(v: GPtr<u8>, node: i32, flags: u32) = thiscall 0x6AA290 => crate::mvpd_v4::automobile_update_wheel_matrix;
    /// `CAutomobile::ProcessSwingingDoor(int32 node, eDoors)` (0x6A9D70)
    fn process_swinging_door(v: GPtr<u8>, node: i32, door: i32) = thiscall 0x6A9D70 => crate::mvpb_a5::process_swinging_door;
    /// `CBouncingPanel::ProcessPanel(CVehicle*, RwFrame*, CVector, CVector, float, float)` (0x6F49A0), floats as bits
    fn process_panel(p: GPtr<u8>, v: GPtr<u8>, frame: GPtr<u8>, mx: u32, my: u32, mz: u32, tx: u32, ty: u32, tz: u32, ret: u32, damp: u32) = thiscall 0x6F49A0 => unported;
    /// `CDoor::Process(CVehicle*, CVector&, CVector&, CVector&)` (0x6F4040)
    fn door_process(d: GPtr<u8>, v: GPtr<u8>, mf: GPtr<u8>, tf: GPtr<u8>, off: GPtr<u8>) -> u8 = thiscall 0x6F4040 => unported;
    /// `CMatrix::SetRotate(float, float, float)` (0x59B120)
    fn matrix_set_rotate(m: GPtr<u8>, rx: f32, ry: f32, rz: f32) = thiscall 0x59B120 => unported;
    /// `CMatrix::RotateX(float)` (0x59B1E0)
    fn matrix_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B1E0 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CVehicleModelInfo::GetWheelPosn(int32, CVector&, bool)` (0x4C7D20)
    fn get_wheel_posn(mi: GPtr<u8>, wheel: i32, out: GPtr<u8>, local: u32) = thiscall 0x4C7D20 => unported;
    /// `CVehicle::SetTransmissionRotation(RwFrame*, float, float, CVector, bool)` (0x6DBBB0)
    fn set_transmission_rotation(v: GPtr<u8>, frame: GPtr<u8>, l: f32, r: f32, wx: f32, wy: f32, wz: f32, front: u32) = thiscall 0x6DBBB0 => unported;
    /// `CClumpModelInfo::GetFrameFromId(RpClump*, int32)` (0x4C53C0)
    fn get_frame_from_id(clump: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x4C53C0 => unported;
    /// `CMatrix::SetTranslateOnly(float, float, float)` (0x59AF80)
    fn matrix_set_translate_only(m: GPtr<u8>, px: f32, py: f32, pz: f32) = thiscall 0x59AF80 => unported;
    /// `CAutomobile::DoHoverSuspensionRatios()` (vtable +0x114)
    fn v_do_hover_suspension_ratios(v: GPtr<u8>) = thiscall vslot 0x114 => unported;
    /// `CAutomobile::DoBurstAndSoftGroundRatios()` (vtable +0xD0)
    fn v_do_burst_and_soft_ground_ratios(v: GPtr<u8>) = thiscall vslot 0xD0 => unported;
    /// `CAutomobile::GetTowHitchPos(CVector&, bool, CVehicle*)` (vtable +0xEC)
    fn v_get_tow_hitch_pos(v: GPtr<u8>, out: GPtr<u8>, b: u32, other: GPtr<u8>) -> u8 = thiscall vslot 0xEC => unported;
    /// `CAutomobile::GetTowBarPos(CVector&, bool, CVehicle*)` (vtable +0xF0)
    fn v_get_tow_bar_pos(v: GPtr<u8>, out: GPtr<u8>, b: u32, other: GPtr<u8>) -> u8 = thiscall vslot 0xF0 => unported;
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;
const F1: u32 = 0x3F80_0000;
/// The corona far clip `TheCamera.LODDistMultiplier * 150` (0xB6F118).
fn corona_range() -> u32 {
    x(g::<f32>(0xB6_F118).read()).fmul_m(150.0).to_f32().to_bits()
}

fn rv(p: GPtr<u8>, off: isize) -> CVector {
    CVector::new(rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8))
}

fn ld(p: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(p, off))
}

fn c(bits: u32) -> X87 {
    X87::constant(bits)
}

fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}

fn ts() -> f32 {
    g::<f32>(TIME_STEP).read()
}

/// `fcomp` + `test ah, 0x41` + `jne` taken: `a <= b` or unordered.
fn le(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `fcomp` + `test ah, 5` + `jp` taken: `a >= b` or unordered.
fn ge(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

fn pos_ptr(this: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        this.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

fn frame(this: GPtr<u8>, node: isize) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x648 + 4 * node)
}

/// A stack `CMatrix` (0x48 bytes) as the original builds it inline: attach matrix (+0x40) and the
/// owns flag (+0x44) zero, the rest written by `Attach`.
struct StackMatrix([u32; 0x12]);

impl StackMatrix {
    fn new() -> StackMatrix {
        StackMatrix([0; 0x12])
    }
    fn ptr(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr()).cast()
    }
}

/// `CAutomobile::PreRender()` (0x6AAB50).
///
/// `CVehicle::PreRender`; with the cars-on-water cheat the virtual hover suspension ratios. With
/// +0x42B bit 0 (not subclass 1) the burst/soft-ground ratios and the displayed wheel heights
/// +0x838.. (`lineStart.z - q * spring` with `q = (c - r) / (1 - r)`, `r = 1 - spring / line`;
/// set directly when the wheel touches and the value rises, else eased by 0.75). Forward speed
/// (`/ g 0xC1BFF0`) and speed; simple cars place their four wheel contacts from the wheel frames;
/// driven cars emit the wheel particles (`AddSingleWheelParticles` per wheel, the middle pair of
/// six-wheelers from 2 m ahead), rain splashes (`|fwdSpeed| < 20`, rain > 0.02), exhaust (engine
/// on, slower than 130) and damage particles. Emergency vehicles flash their light bars (four
/// coronas per bar walking with the time, a coloured point light, colours by model), taxis their
/// roof light, 0x1EA its siren corona; then `DoVehicleLights` and the shadow. Wheels: front and
/// rear wheel directions (steer angles), per touching wheel the contact speed and
/// `ProcessWheelRotation` (radius = model wheel scale / 2) and the rotation integration; wheel
/// matrices; doors (`ProcessSwingingDoor`), three bouncing panels, the boot door of 0x220; the
/// move / turn speeds with their friction parts into +0x8B4 / +0x8C0 for the next frame; the
/// model specials (0x1A8 shovel and wheel struts, 0x1E6/0x1BB/0x20C/0x20D/0x213/0x212 rams and
/// arms from +0x86C, the 0x214 combine reel, 0x238/0x1B2 struts, 0x1B0/0x259/0x197/0x1DD turrets
/// and lights) and the transmission of 0x1EF and of hydraulic-and-bounce cars of subclass 0.
///
/// The light-bar and taxi corona colour dwords carry stack residue above their byte in the
/// original (passed zero-extended here).
#[hook(addr = 0x6aab50, conv = thiscall, name = "CAutomobile::PreRender", this = "CAutomobile",
       expect = "6A FF 90 E9 34 E1 D5 FF",
       route = "L2", hazards = "x87-transcendental,uninit-locals",
       recs = "driving,car-ai,grove-walk",
       note = "entry `push -1; nop; jmp 0x408c8c` (SEH setup in a protection fragment) and the light-bar timer read through the fragment 0x401740 (CTimer::m_snTimeInMilliseconds); fsin/fcos/fpatan via X87, _CIasin/_CIpow via orig; stack CMatrix locals zeroed")]
pub fn automobile_pre_render(this: GPtr<u8>) {
    vehicle_pre_render(this);
    if g::<u8>(0x96_9152).read() != 0 {
        v_do_hover_suspension_ratios(this);
    }
    // L124: the collision data
    let col_data = rd::<GPtr<u8>>(get_col_model(this), 0x2C);
    if rd::<u8>(this, 0x42B) & 1 != 0 && rd::<u32>(this, 0x594) != 1 {
        v_do_burst_and_soft_ground_ratios(this);
        for i in 0..4isize {
            let r = ld(this, 0x878 + 4 * i)
                .fdiv_m(rd::<f32>(this, 0x888 + 4 * i))
                .fsubr_m(1.0);
            let q = ld(this, 0x7D4 + 4 * i).fsub(r).fdiv(c(F1).fsub(r)).to_f32();
            let lines = rd::<GPtr<u8>>(col_data, 0x10);
            let mut v = ld(lines, 0x20 * i + 8);
            if !le(x(q), X87::zero()) {
                v = v.fsub(x(q).fmul_m(rd::<f32>(this, 0x878 + 4 * i)));
            }
            let cur = 0x838 + 4 * i;
            let direct = !ge(ld(this, 0x7D4 + 4 * i), c(F1)) && !v.fcom(ld(this, cur)).test_any(0x41);
            if !direct {
                v = v
                    .fsub_m(rd::<f32>(this, cur))
                    .fmul_m(0.75)
                    .fadd_m(rd::<f32>(this, cur));
            }
            wr::<f32>(this, cur, v.to_f32());
        }
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    // L144: forward speed scaled, L154: speed
    let fwd_speed = ld(this, 0x4C)
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(ld(this, 0x48).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(ld(this, 0x44).fmul_m(rd::<f32>(m, 0x10)))
        .fdiv_m(g::<f32>(0xC1_BFF0).read())
        .to_f32();
    let (vx, vy, vz) = (ld(this, 0x44), ld(this, 0x48), ld(this, 0x4C));
    let speed = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt().to_f32();
    let model = rd::<u16>(this, 0x22);
    if model != 0xFFFE && model != 0x21B && model != 0x1B9 {
        if rd::<u8>(this, 0x36) & 0xF8 == 0x10 {
            simple_wheel_contacts(this);
        }
        let st = rd::<u8>(this, 0x36) >> 3;
        if st <= 3 {
            wheel_particles(this, speed);
        }
    }
    if rd::<u32>(this, 0x654) != 0 {
        middle_wheel_particles(this, speed);
    }
    if cam_no_rain() == 0
        && player_no_rain() == 0
        && !ge(x(fwd_speed).fabs(), c(0x41A0_0000))
        && !le(x(g::<f32>(0xC8_1324).read()), c(0x3CA3_D70A))
        && g::<u32>(0xB7_2914).read() == 0
    {
        add_water_splash_particles(this);
    }
    if rd::<u8>(this, 0x428) & 0x10 != 0
        && (rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) >> 8) & 0x10 == 0
        && !ge(x(fwd_speed), c(0x4302_0000))
        && rd::<u8>(this, 0x42B) & 0x40 == 0
        && rd::<u8>(this, 0x42E) & 0x80 == 0
    {
        add_exhaust_particles(this);
    }
    add_damaged_vehicle_particles(this);
    model_lights(this, model);

    let model = rd::<u16>(this, 0x22);
    if model != 0x1B9
        && model != 0xFFFE
        && model != 0x1B0
        && rd::<u8>(this, 0x42C) & 0x80 == 0
        && get_vehicle_appearance(this) != 3
    {
        let model = rd::<u16>(this, 0x22);
        let flags = if model == 0x214 || model == 0x1D7 { 0 } else { 3 };
        do_vehicle_lights(this, rd::<GPtr<u8>>(this, 0x14), flags);
    }
    if !(find_player_vehicle(-1, 0) == this
        && get_looking_forward_first_person(GPtr::from_addr(0xB6_F028)) != 0)
    {
        let k = if rd::<u8>(this, 0x42C) & 0x80 != 0 && rd::<u32>(this, 0x594) != 0 {
            4
        } else {
            0
        };
        store_shadow_for_vehicle(this, k);
    }

    // L100: the stack matrix (inline constructor: attach / owns zero)
    let mut l100 = StackMatrix::new();
    let mi = crate::common::model_info(i32::from(rd::<i16>(this, 0x22)));
    if rd::<u16>(this, 0x22) == 0x1B0 {
        set_component_rotation(this, frame(this, 0x10), 2, rd::<f32>(this, 0x94C), 1);
    }
    wheels(this, mi);
    let edi = if rd::<u16>(this, 0x22) == 0x1B0 { 7 } else { 0 };
    for node in [2, 5, 4, 7] {
        update_wheel_matrix(this, node, edi);
    }
    if rd::<u32>(this, 0x654) != 0 {
        update_wheel_matrix(this, 3, edi);
    }
    if rd::<u32>(this, 0x660) != 0 {
        update_wheel_matrix(this, 6, edi);
    }
    if rd::<u16>(this, 0x22) != 0x1B0 {
        doors_and_panels(this, &mut l100);
    }
    // the speeds of this frame for the next frame's panels and doors
    let vz = ld(this, 0x4C).fadd_m(rd::<f32>(this, 0x64));
    let vy = ld(this, 0x48).fadd_m(rd::<f32>(this, 0x60));
    let vx = ld(this, 0x44).fadd_m(rd::<f32>(this, 0x5C));
    wr::<f32>(this, 0x8B4, vx.to_f32());
    wr::<f32>(this, 0x8B8, vy.to_f32());
    wr::<f32>(this, 0x8BC, vz.to_f32());
    let tz = ld(this, 0x58).fadd_m(rd::<f32>(this, 0x70));
    let ty = ld(this, 0x54).fadd_m(rd::<f32>(this, 0x6C));
    let tx = ld(this, 0x50).fadd_m(rd::<f32>(this, 0x68));
    wr::<f32>(this, 0x8C0, tx.to_f32());
    wr::<f32>(this, 0x8C4, ty.to_f32());
    wr::<f32>(this, 0x8C8, tz.to_f32());
    model_specials(this, mi, col_data, &mut l100);
    let model = rd::<u16>(this, 0x22);
    let f = rd::<u32>(this, 0x38C);
    if model == 0x1EF || (f & 0x1_0000 != 0 && f & 0x2_0000 != 0 && rd::<u32>(this, 0x594) == 0) {
        let mut p = CVector::default();
        let pp = GPtr::from_raw(&mut p).cast::<u8>();
        get_wheel_posn(mi, 0, pp, 0);
        set_transmission_rotation(
            this,
            frame(this, 0x14),
            rd::<f32>(this, 0x838),
            rd::<f32>(this, 0x840),
            p.x,
            p.y,
            p.z,
            1,
        );
        get_wheel_posn(mi, 1, pp, 0);
        let p = rv(pp, 0);
        set_transmission_rotation(
            this,
            frame(this, 0x15),
            rd::<f32>(this, 0x83C),
            rd::<f32>(this, 0x844),
            p.x,
            p.y,
            p.z,
            0,
        );
    }
    matrix_dtor(l100.ptr());
}

/// 0x6AAE48..0x6AB06E: a simple car's wheel contact points from its wheel frames (the height from
/// the displayed suspension * 1.5), surface B 0.
fn simple_wheel_contacts(this: GPtr<u8>) {
    // L84: the stack matrix of this block
    let mut l84 = StackMatrix::new();
    for (node, comp, out, surf) in [
        (4isize, 0x844isize, 0x7A8isize, 0x7CBisize),
        (7, 0x83C, 0x750, 0x773),
        (2, 0x840, 0x77C, 0x79F),
        (5, 0x838, 0x724, 0x747),
    ] {
        matrix_attach(l84.ptr(), frame(this, node).byte_add(0x10), 0);
        let mp = l84.ptr();
        let mut local = CVector::new(
            rd::<f32>(mp, 0x30),
            rd::<f32>(mp, 0x34),
            ld(this, comp).fmul_m(1.5).to_f32(),
        );
        let mut o = CVector::default();
        let r = matrix_mul_vector(
            GPtr::from_raw(&mut o).cast(),
            rd::<GPtr<u8>>(this, 0x14),
            GPtr::from_raw(&mut local).cast(),
        );
        let v = rv(r, 0);
        wr::<f32>(this, out, v.x);
        wr::<f32>(this, out + 4, v.y);
        wr::<f32>(this, out + 8, v.z);
        wr::<u8>(this, surf, 0);
    }
    matrix_dtor(l84.ptr());
}

/// 0x6AB088..0x6AB154: the four wheels' particles.
fn wheel_particles(this: GPtr<u8>, speed: f32) {
    let spinning = rd::<u32>(this, 0x96C) == 2 || rd::<u32>(this, 0x974) == 2;
    for i in 0..4i32 {
        let ii = i as isize;
        let mut flags = if (i == 0 || i == 2) && !spinning { 4 } else { 0 };
        if rd::<u8>(this, 0x820 + ii) != 0 {
            flags += 1;
        }
        if rd::<u8>(this, 0x824 + ii) != 0 {
            flags += 2;
        }
        let side = if i == 0 || i == 1 { 0xBF80_0000u32 } else { F1 };
        let cp = this.byte_add(0x724 + 0x2C * ii);
        let status = get_wheel_status(this.byte_add(0x5A0), i);
        add_single_wheel_particles(
            this,
            rd::<u32>(this, 0x968 + 4 * ii),
            status,
            rd::<f32>(this, 0x7E4 + 4 * ii),
            speed,
            cp,
            cp,
            side as i32,
            i,
            rd::<u32>(this, 0x810 + 4 * ii),
            this.byte_add(0x820 + ii),
            flags,
        );
    }
}

/// 0x6AB162..0x6AB2BC: the middle wheel pair of six-wheelers (from 2 m ahead of contacts 1 / 3).
fn middle_wheel_particles(this: GPtr<u8>, speed: f32) {
    for (blood, b2, cp_off, state, wheel, comp, skid, id) in [
        (
            0x821isize, 0x825isize, 0x750isize, 0x96Cisize, 1i32, 0x7E8isize, 0x814isize, 5i32,
        ),
        (0x823, 0x827, 0x7A8, 0x974, 3, 0x7F0, 0x81C, 6),
    ] {
        let mut flags = u32::from(rd::<u8>(this, blood) != 0);
        if rd::<u8>(this, b2) != 0 {
            flags += 2;
        }
        let m = rd::<GPtr<u8>>(this, 0x14);
        let fz = ld(m, 0x18).fadd(ld(m, 0x18));
        let fy = ld(m, 0x14).fadd(ld(m, 0x14)).to_f32();
        let fx = ld(m, 0x10).fadd(ld(m, 0x10)).to_f32();
        let cp = this.byte_add(cp_off);
        let mut from = CVector::new(
            x(fx).fadd_m(rd::<f32>(cp, 0)).to_f32(),
            x(fy).fadd_m(rd::<f32>(cp, 4)).to_f32(),
            fz.fadd_m(rd::<f32>(cp, 8)).to_f32(),
        );
        let status = get_wheel_status(this.byte_add(0x5A0), wheel);
        add_single_wheel_particles(
            this,
            rd::<u32>(this, state),
            status,
            rd::<f32>(this, comp),
            speed,
            cp,
            GPtr::from_raw(&mut from).cast(),
            F1 as i32,
            id,
            rd::<u32>(this, skid),
            this.byte_add(blood),
            flags,
        );
    }
}

/// 0x6AB350..0x6ABC71: emergency light bars, taxi lights and the 0x1EA siren corona.
fn model_lights(this: GPtr<u8>, model: u16) {
    let k = model.wrapping_sub(0x197);
    if k > 0xC0 {
        return;
    }
    match model {
        0x197 | 0x1A0 | 0x1AB | 0x254 | 0x255 | 0x256 | 0x257 => {
            if rd::<u8>(this, 0x42D) & 0x80 == 0 {
                return;
            }
            light_bar(this, model);
        }
        0x1A4 | 0x1B6 => {
            if rd::<u8>(this, 0x868) & 1 == 0 {
                return;
            }
            let (ly, lz) = if model == 0x1A4 {
                (0xBECC_CCCDu32, 0x3F73_3333u32)
            } else {
                (0, 0x3F59_999A)
            };
            let mut local = CVector::new(0.0, f32::from_bits(ly), f32::from_bits(lz));
            let lp = GPtr::from_raw(&mut local).cast::<u8>();
            let mut w = CVector::default();
            let wp = matrix_mul_vector(GPtr::from_raw(&mut w).cast(), rd::<GPtr<u8>>(this, 0x14), lp);
            let world = rv(wp, 0);
            let b = ftol32(x(g::<f32>(0xB7_C4E4).read()).fmul_m(10.0)) as u32;
            register_corona(
                (this.addr() as u32).wrapping_add(0x11),
                this,
                b,
                b,
                0,
                0xFF,
                lp,
                0x3F4C_CCCD,
                corona_range(),
                1,
                0,
                1,
                0,
                0,
                0,
                0,
                0x3FC0_0000,
                0,
                0x4170_0000,
                0,
                1,
            );
            add_light(
                0,
                world.x.to_bits(),
                world.y.to_bits(),
                world.z.to_bits(),
                0,
                0,
                0,
                0x4120_0000,
                0x3DCC_CCCD,
                0x3DCC_CCCD,
                0x3D4C_CCCD,
                0,
                1,
                0,
            );
        }
        0x1EA => {
            if rd::<u8>(this, 0x42D) & 0x80 == 0 {
                return;
            }
            let mut local = CVector::new(0.0, f32::from_bits(0x3F99_999A), 0.5);
            if g::<u32>(0xB7_CB84).read() & 0x100 == 0 {
                return;
            }
            let m = rd::<GPtr<u8>>(this, 0x14);
            let mut top = [0f32; 3];
            let t =
                get_top_direction(GPtr::from_addr(0xB6_F028), GPtr::from_raw(top.as_mut_ptr())).cast::<u8>();
            let d = ld(m, 0x18)
                .fmul_m(rd::<f32>(t, 8))
                .fadd(ld(m, 0x14).fmul_m(rd::<f32>(t, 4)))
                .fadd(ld(m, 0x10).fmul_m(rd::<f32>(t, 0)));
            if ge(d, X87::zero()) {
                return;
            }
            register_corona(
                (this.addr() as u32).wrapping_add(0x15),
                this,
                0,
                0,
                0xFF,
                0xFF,
                GPtr::from_raw(&mut local).cast(),
                0x3ECC_CCCD,
                corona_range(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0x3FC0_0000,
                0,
                0x4170_0000,
                0,
                1,
            );
        }
        _ => {}
    }
}

/// 0x6AB382..0x6ABA7F: an emergency light bar.
fn light_bar(this: GPtr<u8>, model: u16) {
    // p1 (L150..), p2 (L140..) and the four colour bytes L144, L15c, L154, L158
    let (p1, p2, c144, c15c, c154, c158) = match model {
        0x257 => (
            [0x3F33_3333u32, 0xBDCC_CCCD, 0x3F99_999A],
            [0xBF33_3333u32, 0xBDCC_CCCD, 0x3F99_999A],
            0xFFu8,
            0u8,
            0u8,
            0xFFu8,
        ),
        0x254..=0x256 => (
            [0x3F33_3333, 0xBECC_CCCD, F1],
            [0xBF33_3333, 0xBECC_CCCD, F1],
            0xFF,
            0,
            0,
            0xFF,
        ),
        0x1AB => (
            [0x3F0C_CCCD, 0x3F8C_CCCD, 0x3FB3_3333],
            [0xBF0C_CCCD, 0x3F8C_CCCD, 0x3FB3_3333],
            0xFF,
            0,
            0,
            0xFF,
        ),
        0x1A0 => (
            [0x3F19_999A, 0x3F66_6666, 0x3F99_999A],
            [0xBF19_999A, 0x3F66_6666, 0x3F99_999A],
            0xFF,
            0xFF,
            0xFF,
            0xFF,
        ),
        _ => (
            [0x3F66_6666, 0x404C_CCCD, 0x3FA6_6666],
            [0xBF66_6666, 0x404C_CCCD, 0x3FA6_6666],
            0xFF,
            0xFF,
            0xFF,
            0,
        ),
    };
    let fv = |b: u32| f32::from_bits(b);
    // registers al = c144, cl = c158, dl = c15c
    let time = g::<u32>(0xB7_CB84).read();
    let mut edi = time & 0x3FF;
    let div6 = |v: u8| (i32::from(v) / 6) as u8;
    let (mut bl, mut c181, mut c175) = if edi < 0x200 {
        (div6(c144), 0u8, 0u8)
    } else {
        (div6(c15c), div6(c154), div6(c158))
    };
    edi &= 0x1FF;
    let scale = |v: u8, f: X87| ftol32(xi(i32::from(v)).fmul(f).fmul_m(f32::from_bits(0x3C23_D70A))) as u8;
    if edi < 0x64 {
        let f = xi(edi as i32);
        bl = scale(bl, f);
        c181 = scale(c181, f);
        c175 = scale(c175, f);
    } else if edi > 0x19C {
        let f = xi((0x200 - edi) as i32);
        bl = scale(bl, f);
        c181 = scale(c181, f);
        c175 = scale(c175, f);
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let p = pos_ptr(this);
    let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    let uy2 = ld(m, 0x24).fadd(ld(m, 0x24)).to_f32();
    let ux2 = ld(m, 0x20).fadd(ld(m, 0x20)).to_f32();
    let uz2 = ld(m, 0x28).fadd(ld(m, 0x28));
    let lz = uz2.fadd_m(pz).to_f32();
    let ly = x(py).fadd_m(uy2).to_f32();
    let lx = x(px).fadd_m(ux2).to_f32();
    let k005 = f32::from_bits(0x3BA3_D70A);
    add_light(
        0,
        lx.to_bits(),
        ly.to_bits(),
        lz.to_bits(),
        0,
        0,
        0,
        0x4120_0000,
        xi(i32::from(bl)).fmul_m(k005).to_f32().to_bits(),
        xi(i32::from(c181)).fmul_m(k005).to_f32().to_bits(),
        xi(i32::from(c175)).fmul_m(k005).to_f32().to_bits(),
        0,
        1,
        0,
    );
    let gb = g::<f32>(0xB7_C4E4).read();
    let br = |v: u8| ftol32(xi(i32::from(v)).fmul_m(gb).fmul_m(f32::from_bits(0x3DCC_CCCD))) as u8;
    let r144 = br(c144);
    let zero = ftol32(x(gb).fmul_m(0.0)) as u8;
    let r15c = br(c15c);
    let r154 = br(c154);
    let r158 = br(c158);
    let third = f32::from_bits(0x3EAA_AAAB);
    for i in 0..4u32 {
        let fi = xi(i as i32);
        let t = c(0x4040_0000).fsub(fi);
        let phase = (time.wrapping_add(0x40 * i) >> 8) & 3;
        let z = x(fv(p1[2]))
            .fmul(fi)
            .fadd(x(x(fv(p2[2])).fmul(t).to_f32()))
            .to_f32();
        let yv = x(x(fv(p1[1])).fmul(fi).to_f32())
            .fadd(x(x(fv(p2[1])).fmul(t).to_f32()))
            .to_f32();
        let xv = x(x(fv(p1[0])).fmul(fi).to_f32())
            .fadd(t.fmul_m(fv(p2[0])))
            .to_f32();
        let mut lp = CVector::new(
            x(xv).fmul_m(third).to_f32(),
            x(yv).fmul_m(third).to_f32(),
            x(z).fmul_m(third).to_f32(),
        );
        let (r, gg, b) = match phase {
            0 => (r144, zero, zero),
            2 => (r15c, r154, r158),
            _ => continue,
        };
        register_corona(
            (this.addr() as u32).wrapping_add(0x15 + i),
            this,
            u32::from(r),
            u32::from(gg),
            u32::from(b),
            0xFF,
            GPtr::from_raw(&mut lp).cast(),
            0x3ECC_CCCD,
            corona_range(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0x3FC0_0000,
            0,
            0x4170_0000,
            0,
            1,
        );
    }
}

/// 0x6ABD47..0x6ABFC6: wheel directions, contact speeds and wheel rotations.
fn wheels(this: GPtr<u8>, mi: GPtr<u8>) {
    let heli = rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x200_0000 != 0;
    let m = rd::<GPtr<u8>>(this, 0x14);
    let steer = ld(this, 0x494);
    let mut d = CVector::new(steer.fsin().fchs().to_f32(), steer.fcos().to_f32(), 0.0);
    // La8: front wheel direction, L118: rear wheel direction
    let mut front = CVector::default();
    multiply3x3(
        GPtr::from_raw(&mut front).cast(),
        m,
        GPtr::from_raw(&mut d).cast(),
    );
    let m = rd::<GPtr<u8>>(this, 0x14);
    let mut rear = rv(m, 0x10);
    let f = rd::<u32>(this, 0x38C);
    let rear_dir = if f & 0x20 != 0 {
        front = rv(m, 0x10);
        let s = ld(this, 0x494);
        Some((s.fsin().to_f32(), s.fcos().to_f32()))
    } else if f & 0x40 != 0 && ld(this, 0x498).fcom(X87::zero()).test_parity(0x44) {
        let s = ld(this, 0x498);
        Some((s.fsin().fchs().to_f32(), s.fcos().to_f32()))
    } else {
        None
    };
    if let Some((a, b)) = rear_dir {
        let mut d = CVector::new(a, b, 0.0);
        let mut o = CVector::default();
        rear = rv(
            multiply3x3(GPtr::from_raw(&mut o).cast(), m, GPtr::from_raw(&mut d).cast()),
            0,
        );
    }
    for i in 0..4isize {
        if le(ld(this, 0x7F4 + 4 * i), X87::zero()) {
            continue;
        }
        if heli && i != 0 && i != 2 {
            continue;
        }
        let p = pos_ptr(this);
        let cp = this.byte_add(0x724 + 0x2C * i);
        let rel = CVector::new(
            ld(cp, 0).fsub_m(rd::<f32>(p, 0)).to_f32(),
            ld(cp, 4).fsub_m(rd::<f32>(p, 4)).to_f32(),
            ld(cp, 8).fsub_m(rd::<f32>(p, 8)).to_f32(),
        );
        let mut tmp = CVector::default();
        let mut sp = rv(get_speed(this, GPtr::from_raw(&mut tmp), rel).cast(), 0);
        let (radius, dir) = if i == 0 || i == 2 {
            (ld(mi, 0x40).fmul_m(0.5).to_f32(), &mut front)
        } else {
            (ld(mi, 0x44).fmul_m(0.5).to_f32(), &mut rear)
        };
        let w = st0(process_wheel_rotation(
            this,
            rd::<u32>(this, 0x968 + 4 * i),
            GPtr::from_raw(dir).cast(),
            GPtr::from_raw(&mut sp).cast(),
            radius,
        ))
        .to_f32();
        wr::<f32>(this, 0x848 + 4 * i, w);
        let rot = x(ts()).fmul_m(w).fadd_m(rd::<f32>(this, 0x828 + 4 * i)).to_f32();
        wr::<f32>(this, 0x828 + 4 * i, rot);
    }
}

/// 0x6AC031..0x6AC232: swinging doors, bouncing panels and the 0x220 / boot door.
fn doors_and_panels(this: GPtr<u8>, l100: &mut StackMatrix) {
    for (node, door) in [(0xA, 2), (8, 3), (0xB, 4), (9, 5), (0x10, 0), (0x11, 1)] {
        process_swinging_door(this, node, door);
    }
    for k in 0..3isize {
        let pn = this.byte_add(0x6AC + 0x20 * k);
        let comp = rd::<i16>(pn, 0);
        if comp <= -1 {
            continue;
        }
        let ms = rv(this, 0x8B4);
        let tsp = rv(this, 0x8C0);
        process_panel(
            pn,
            this,
            frame(this, isize::from(comp)),
            ms.x.to_bits(),
            ms.y.to_bits(),
            ms.z.to_bits(),
            tsp.x.to_bits(),
            tsp.y.to_bits(),
            tsp.z.to_bits(),
            0xBF80_0000,
            0xBF80_0000,
        );
    }
    let node = if rd::<u16>(this, 0x22) == 0x220 { 0x15 } else { 1 };
    if rd::<u8>(this, 0x717) != 1 {
        return;
    }
    let fr = frame(this, node);
    if fr.is_null() {
        return;
    }
    matrix_attach(l100.ptr(), fr.byte_add(0x10), 0);
    let mp = l100.ptr();
    // L130: the frame position, L10c: the hinge offset in world axes, L150: the rotation angles
    let mut pos = rv(mp, 0x30);
    let mut off = CVector::default();
    let offp = GPtr::from_raw(&mut off).cast::<u8>();
    let mut ang = [0f32; 3];
    multiply3x3(offp, rd::<GPtr<u8>>(this, 0x14), GPtr::from_raw(&mut pos).cast());
    door_process(
        this.byte_add(0x70C),
        this,
        this.byte_add(0x8B4),
        this.byte_add(0x8C0),
        offp,
    );
    ang[usize::from(rd::<u8>(this, 0x716))] = rd::<f32>(this, 0x718);
    matrix_set_rotate(mp, ang[0], ang[1], ang[2]);
    if rd::<u16>(this, 0x22) == 0x220 {
        let a = xi(i32::from(rd::<u16>(this, 0x86C)))
            .fmul_m(f32::from_bits(0xB8D1_B717))
            .to_f32();
        matrix_rotate_x(mp, a);
    }
    wr::<f32>(mp, 0x30, x(pos.x).fadd_m(rd::<f32>(mp, 0x30)).to_f32());
    wr::<f32>(mp, 0x34, x(pos.y).fadd_m(rd::<f32>(mp, 0x34)).to_f32());
    wr::<f32>(mp, 0x38, ld(mp, 0x38).fadd_m(pos.z).to_f32());
    matrix_update_rw(mp);
}

/// The strut tilt the shovel / strut models compute per wheel (stored into the attached matrix at
/// `slot`, then `UpdateRW`).
#[allow(clippy::too_many_arguments)]
fn strut(
    this: GPtr<u8>,
    l100: &mut StackMatrix,
    col_data: GPtr<u8>,
    node: isize,
    wheel: isize,
    slot: isize,
    along: isize,
    pos_off: isize,
) {
    let fr = frame(this, node);
    if fr.is_null() {
        return;
    }
    matrix_attach(l100.ptr(), fr.byte_add(0x10), 0);
    let mp = l100.ptr();
    let line = rd::<GPtr<u8>>(col_data, 0x10).byte_add(0x20 * wheel);
    let h = rd::<GPtr<u8>>(this, 0x384);
    let den = ld(line, along).fsub_m(rd::<f32>(mp, pos_off));
    let num = ld(line, 8)
        .fsub_m(rd::<f32>(h, 0xB8))
        .fsubr_m(rd::<f32>(this, 0x838 + 4 * wheel));
    wr::<f32>(mp, slot, num.fdiv(den).to_f32());
    matrix_update_rw(mp);
}

/// 0x6AC2AA..0x6ACBC7: the moving parts of special models.
fn model_specials(this: GPtr<u8>, mi: GPtr<u8>, col_data: GPtr<u8>, l100: &mut StackMatrix) {
    let model = rd::<u16>(this, 0x22);
    let ts = ts();
    let u86c = || xi(i32::from(rd::<u16>(this, 0x86C)));
    let gas_rate = |this: GPtr<u8>| {
        let r = if le(ld(this, 0x49C).fabs(), X87::zero()) {
            f32::from_bits(0x3E99_999A)
        } else {
            0.5
        };
        x(ts).fmul_m(r).to_f32()
    };
    match model {
        0x1A8 => {
            let a = gas_rate(this);
            set_component_rotation(this, frame(this, 0x14), 1, a, 0);
            set_component_rotation(this, frame(this, 0x18), 1, x(a).fmul_m(1.5).to_f32(), 0);
            get_col_model(this);
            for (k, node) in [21isize, 0, 22].into_iter().enumerate() {
                if node == 0 {
                    continue;
                }
                strut(this, l100, col_data, node, k as isize, 0x18, 4, 0x34);
            }
            let mut p = CVector::default();
            let pp = GPtr::from_raw(&mut p).cast::<u8>();
            get_wheel_posn(mi, 1, pp, 0);
            set_transmission_rotation(
                this,
                frame(this, 0x17),
                rd::<f32>(this, 0x83C),
                rd::<f32>(this, 0x844),
                p.x,
                p.y,
                p.z,
                0,
            );
        }
        0x1E6 => {
            let a = u86c().fmul_m(f32::from_bits(0x3951_B717)).to_f32();
            set_component_rotation(this, frame(this, 0x14), 0, a, 1);
        }
        0x20C => {
            if rd::<u8>(this, 0x36) & 0xF8 == 0x28 {
                return;
            }
            let k = f32::from_bits(0xBE32_B8C3);
            set_component_rotation(this, frame(this, 0x14), 0, k, 1);
            let v = u86c()
                .fdiv(xi(i32::from(g::<i16>(0x8D_314C).read())))
                .fadd_m(0.5)
                .fmul_m(ts)
                .fmul_m(f32::from_bits(0x3CA3_D70A))
                .fadd_m(rd::<f32>(this, 0x958))
                .to_f32();
            wr::<f32>(this, 0x958, v);
            set_component_rotation(this, frame(this, 0x15), 1, v, 1);
            set_component_rotation(this, frame(this, 0x15), 0, k, 0);
        }
        0x1BB => {
            let a = u86c().fmul_m(f32::from_bits(0xB8D1_B717)).to_f32();
            set_component_rotation(this, frame(this, 0x14), 0, a, 1);
        }
        0x20D => tow_arm(this, l100),
        0x213 => {
            let a = u86c()
                .fmul_m(g::<f32>(0x8D_3144).read())
                .fdiv(xi(i32::from(g::<u16>(0x8D_313C).read())))
                .to_f32();
            set_component_rotation(this, frame(this, 0x14), 0, a, 1);
        }
        0x212 => {
            if frame(this, 0x14).is_null() {
                return;
            }
            let mi212 = g::<GPtr<u8>>(0xA9_B910).read();
            let f = get_frame_from_id(rd::<GPtr<u8>>(mi212, 0x1C), 0x14).byte_add(0x40);
            let (px, py, pz) = (rd::<f32>(f, 0), rd::<f32>(f, 4), rd::<f32>(f, 8));
            let z = u86c().fmul_m(f32::from_bits(0x3A1D_4952)).fadd_m(pz).to_f32();
            matrix_attach(l100.ptr(), frame(this, 0x14).byte_add(0x10), 0);
            matrix_set_translate_only(l100.ptr(), px, py, z);
            matrix_update_rw(l100.ptr());
        }
        0x214 => combine(this, mi, l100),
        0x238 | 0x1B2 => {
            get_col_model(this);
            for (k, node) in [20isize, 22, 21, 23].into_iter().enumerate() {
                strut(this, l100, col_data, node, k as isize, 8, 0, 0x30);
            }
            let a = gas_rate(this);
            set_component_rotation(this, frame(this, 0x18), 1, a, 0);
        }
        0x1B0 | 0x259 => {
            set_component_rotation(this, frame(this, 0x14), 2, rd::<f32>(this, 0x94C), 1);
            set_component_rotation(this, frame(this, 0x15), 0, rd::<f32>(this, 0x950), 1);
        }
        0x197 => {
            set_component_rotation(this, frame(this, 0x14), 0, rd::<f32>(this, 0x950), 1);
            set_component_rotation(this, frame(this, 0x14), 2, rd::<f32>(this, 0x94C), 0);
        }
        0x1DD => {
            set_component_rotation(this, frame(this, 0x14), 0, rd::<f32>(this, 0x958), 1);
        }
        _ => {}
    }
}

/// 0x6AC511..0x6AC6D4: the tow truck arm (0x20D).
fn tow_arm(this: GPtr<u8>, l100: &mut StackMatrix) {
    // L144: the arm angle
    let mut a = xi(i32::from(rd::<u16>(this, 0x86C)))
        .fmul_m(g::<f32>(0x8D_3144).read())
        .fdiv(xi(i32::from(g::<u16>(0x8D_313C).read())))
        .to_f32();
    set_component_rotation(this, frame(this, 0x14), 0, a, 1);
    let trailer = rd::<GPtr<u8>>(this, 0x4C8);
    if trailer.is_null() {
        set_component_rotation(this, frame(this, 0x15), 0, x(a).fchs().to_f32(), 1);
        if rd::<u16>(this, 0x6AC) != 0x15 {
            return;
        }
        matrix_attach(l100.ptr(), frame(this, 0x15).byte_add(0x10), 0);
        let mp = l100.ptr();
        wr::<f32>(mp, 0x24, ld(mp, 0x24).fadd_m(rd::<f32>(this, 0x6B4)).to_f32());
        wr::<f32>(mp, 0x20, ld(mp, 0x20).fadd_m(rd::<f32>(this, 0x6B8)).to_f32());
        matrix_update_rw(mp);
        return;
    }
    a = x(a).fmul_m(-1.0).to_f32();
    let mut hitch = CVector::default();
    let hp = GPtr::from_raw(&mut hitch).cast::<u8>();
    if v_get_tow_hitch_pos(trailer, hp, 1, this) == 0 {
        return;
    }
    let mut bar = CVector::default();
    if v_get_tow_bar_pos(
        this,
        GPtr::from_raw(&mut bar).cast(),
        1,
        rd::<GPtr<u8>>(this, 0x4C8),
    ) == 0
    {
        return;
    }
    let d = CVector::new(
        x(hitch.x).fsub_m(bar.x).to_f32(),
        x(hitch.y).fsub_m(bar.y).to_f32(),
        x(hitch.z).fsub_m(bar.z).to_f32(),
    );
    let m = rd::<GPtr<u8>>(this, 0x14);
    let gl = g::<f32>(0x8D_3148).read();
    let v = x(d.z)
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x(d.y).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x(d.x).fmul_m(rd::<f32>(m, 0x10)))
        .fdiv_m(gl)
        .to_f32();
    let asin = |v: f32| st0(sa_core::orig::x87_unary(0x82_1E70, x(v).to_f64()));
    let r = asin(v).fadd_m(a).to_f32();
    set_component_rotation(this, frame(this, 0x15), 0, r, 1);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let s = x(d.z)
        .fmul_m(rd::<f32>(m, 8))
        .fadd(x(d.y).fmul_m(rd::<f32>(m, 4)))
        .fadd(x(d.x).fmul_m(rd::<f32>(m, 0)));
    let v2 = c(0xBF80_0000).fdiv_m(gl).fmul(s).to_f32();
    let r2 = asin(v2).to_f32();
    set_component_rotation(this, frame(this, 0x15), 1, r2, 0);
}

/// 0x6AC7B5..0x6ACA32: the combine harvester's reel, cutter and body roll.
fn combine(this: GPtr<u8>, mi: GPtr<u8>, l100: &mut StackMatrix) {
    let two_pi = c(0x40C9_0FDB);
    if !le(ld(this, 0x958), two_pi) {
        wr::<f32>(this, 0x958, ld(this, 0x958).fsub(two_pi).to_f32());
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let sp = || {
        ld(this, 0x4C)
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(ld(this, 0x48).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(ld(this, 0x44).fmul_m(rd::<f32>(m, 0x10)))
    };
    let v = if !le(sp(), c(0x3DCC_CCCD)) {
        if le(sp(), c(0x3DCC_CCCD)) {
            sp()
        } else {
            c(0x3DCC_CCCD)
        }
    } else if !le(X87::zero(), sp()) {
        X87::zero()
    } else if le(sp(), c(0x3DCC_CCCD)) {
        sp()
    } else {
        c(0x3DCC_CCCD)
    };
    let r = v.fmul_m(ts()).fsubr_m(rd::<f32>(this, 0x958)).to_f32();
    wr::<f32>(this, 0x958, r);
    set_component_rotation(this, frame(this, 0x15), 0, r, 1);
    // Lb4: the front wheel position
    let mut wp = CVector::default();
    get_wheel_posn(mi, 0, GPtr::from_raw(&mut wp).cast(), 0);
    matrix_attach(l100.ptr(), frame(this, 0x14).byte_add(0x10), 0);
    let mp = l100.ptr();
    let avg = ld(this, 0x838).fadd_m(rd::<f32>(this, 0x840)).fmul_m(0.5);
    let diff = ld(this, 0x838).fsub_m(rd::<f32>(this, 0x840));
    let dy = ld(mp, 0x34).fsub_m(wp.y);
    let r1 = avg.fsub_m(wp.z).fpatan(dy).fchs().to_f32();
    let r2 = diff.fpatan(x(wp.x).fabs().fadd(x(wp.x).fabs())).to_f32();
    set_component_rotation(
        this,
        frame(this, 0x14),
        0,
        x(r1).fadd_m(f32::from_bits(0x4049_0FDB)).to_f32(),
        1,
    );
    set_component_rotation(this, frame(this, 0x14), 1, r2, 0);
    let m = rd::<GPtr<u8>>(this, 0x14);
    // L154: the target roll
    let mut roll = 0.0f32;
    if !le(ld(m, 0x28), X87::zero()) && rd::<u8>(this, 0x486) == 0 && rd::<u8>(this, 0x487) == 0 {
        let a = st0(sa_core::orig::x87_unary(0x82_1E70, ld(m, 8).to_f64()));
        roll = a.to_f32();
        let lim = g::<f32>(0x8D_31EC).read();
        if !le(a, x(lim)) {
            roll = lim;
        } else if !ge(x(roll), x(lim).fchs()) {
            roll = x(lim).fchs().to_f32();
        }
    }
    let p = st0(sa_core::orig::x87_binary(
        0x82_2130,
        x(g::<f32>(0x8D_31E8).read()).to_f64(),
        x(ts()).to_f64(),
    ));
    let v = c(F1)
        .fsub(p)
        .fmul_m(roll)
        .fadd(p.fmul_m(rd::<f32>(this, 0x94C)))
        .to_f32();
    wr::<f32>(this, 0x94C, v);
    set_component_rotation(this, frame(this, 1), 1, v, 1);
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_PRE_RENDER];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
