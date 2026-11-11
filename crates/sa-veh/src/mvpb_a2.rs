//! WRITER-MVP-B (helper A, batch `mvpb-veh-2`): vehicle functions on the MVP call path. Ported from
//! the 1.0 US bytes; gta-reversed and docs/re/specs/VEH as reading aids. Where they differ, the bytes
//! win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, g, rd, wr, x};

orig! {
    /// `CVehicle::Render()` (0x6D0E60)
    fn vehicle_render(v: GPtr<u8>) = thiscall 0x6D0E60 => crate::vehicle2::render;
    /// `CPlayerPed::SetWantedLevelNoDrop(int32)` (0x609F30)
    fn player_set_wanted_level_no_drop(p: GPtr<u8>, level: i32) = thiscall 0x609F30 => unported;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u8) = thiscall 0x59BD10 => unported;
    /// `CMatrix::SetRotateX(float)` (0x59B060)
    fn matrix_set_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B060 => unported;
    /// `CMatrix::RotateY(float)` (0x59B2C0)
    fn matrix_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59B2C0 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CDamageManager::GetPanelStatus(ePanels)` (0x6C2180)
    fn dm_get_panel_status(dm: GPtr<u8>, panel: u32) -> u32 = thiscall 0x6C2180 => crate::damage::get_panel_status;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, returns the out pointer)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970, ST0)
    fn normalise_and_mag(v: GPtr<u8>) -> f32 = thiscall 0x59C970 => unported;
    /// `CCoronas::RegisterCorona(...)` (0x6FC580, 21 arguments)
    fn register_corona(id: u32, e: u32, r: u32, g: u32, b: u32, a: u32, pos: GPtr<u8>, radius: f32, far: f32, ty: u32, flare: u32, refl: u32, los: u32, unused: u32, angle: f32, long_distance: u32, near_clip: f32, fade_state: u32, fade_speed: f32, below: u32, delay: u32) = cdecl 0x6FC580 => unported;
    /// `CShadows::StoreCarLightShadow(CVehicle*, int32 id, RwTexture*, CVector* pos, float frontX,
    /// float frontY, float sideX, float sideY, uint8 r, uint8 g, uint8 b, float maxViewAngle)` (0x70C500)
    fn store_car_light_shadow(v: GPtr<u8>, id: u32, tex: GPtr<u8>, pos: GPtr<u8>, fx: f32, fy: f32, sx: f32, sy: f32, r: u32, gg: u32, b: u32, angle: f32) = cdecl 0x70C500 => unported;
}

/// `cTransmission::CalculateGearForSimpleCar(float speed, uint8& gear)` (0x6D0530, `ret 8`): stores
/// the speed (+0x64); above the current gear's change-up velocity (gear entry `this + gear * 0xC`, +4)
/// the gear goes up by one, capped at the gear count (+0x4A); below its change-down velocity (+8) it
/// goes down by one, floored at 0.
#[hook(addr = 0x6D0530, conv = thiscall, name = "cTransmission::CalculateGearForSimpleCar", this = "cTransmission", expect = "8B 44 24 04 D9 44 24 04",
       unverified = "mvpb-veh-2", route = "L2", hazards = "none")]
pub fn calculate_gear_for_simple_car(this: GPtr<u8>, speed: f32, gear: GPtr<u8>) {
    wr::<f32>(this, 0x64, speed);
    let g = u32::from(gear.read());
    let e = this.byte_add((g * 0xC) as isize);
    if !x(speed).fcom(x(rd::<f32>(e, 4))).test_any(0x41) {
        let n = u32::from(rd::<u8>(this, 0x4A));
        gear.write(if n as i32 >= (g + 1) as i32 { (g + 1) as u8 } else { n as u8 });
    } else if !x(speed).fcom(x(rd::<f32>(e, 8))).test_parity(0x05) {
        let c = g as i32 - 1;
        gear.write(if c < 0 { 0 } else { c as u8 });
    }
}

/// `CTrain::Render()` (0x6F55C0): a tail `jmp` to `CVehicle::Render` (0x6D0E60). `expect` takes the
/// following `nop` too so the 5-byte `jmp` is not mistaken for a `.HOODLUM` stub.
#[hook(addr = 0x6F55C0, conv = thiscall, name = "CTrain::Render", this = "CTrain", expect = "E9 9B B8 FD FF 90",
       unverified = "mvpb-veh-2", route = "L2", hazards = "none")]
pub fn train_render(this: GPtr<u8>) {
    vehicle_render(this);
}

/// The entity position: `m_matrix->pos` (+0x14, +0x30) or the placement position (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CVehicle::DoHeadLightReflectionTwin(CMatrix& m)` (0x6E1600, `ret 4`): with `P` = the vehicle
/// struct's first dummy (model info +0x5C, +0/+4/+8), `n` = the matrix forward xy normalised (`inv =
/// 1/sqrt(fy² + fx²)`, or the zero sum itself; nx stored as f32 into the argument slot, ny kept on the
/// x87 stack), `p4 = P.x * 4`, `w = p4 + p4` (f32), `o = (p4 + p4) + 1 + P.y` (x87): the light position
/// is the entity position + `(nx * o, o * ny, +2)` and `CShadows::StoreCarLightShadow(this, this + 0x16,
/// [0xC403F8], &pos, w * nx, w * ny, p4 * ny, -(nx * p4), 45, 45, 45, 7.0)`.
#[hook(addr = 0x6E1600, conv = thiscall, name = "CVehicle::DoHeadLightReflectionTwin", this = "CVehicle", expect = "83 EC 1C 0F BF 41 22",
       unverified = "mvpb-veh-2", route = "L2", hazards = "x87-transcendental",
       note = "the original stores nx into its own matrix argument slot (entry esp+4); the port does not")]
pub fn do_head_light_reflection_twin(this: GPtr<u8>, mat: GPtr<u8>) {
    let vs = rd::<GPtr<u8>>(crate::common::model_info(rd::<i16>(this, 0x22) as i32), 0x5C);
    let (px, py) = (rd::<f32>(vs, 0), rd::<f32>(vs, 4));
    let src = entity_pos(this);
    let mut pos = [rd::<f32>(src, 0), rd::<f32>(src, 4), rd::<f32>(src, 8)];
    let fy = x(rd::<f32>(mat, 0x14));
    let fxm = rd::<f32>(mat, 0x10);
    let s = fy.fmul(fy).fadd(x(fxm).fmul_m(fxm));
    let inv = if !s.fcom(x(0.0)).test_parity(0x44) { s } else { s.fsqrt().fdivr_m(1.0) };
    let nx = f32_of(x(fxm).fmul(inv));
    let ny = inv.fmul(fy);
    let p4 = x(px).fmul_m(4.0);
    let two = p4.fadd(p4);
    let w = f32_of(two);
    let o = two.fadd_m(1.0).fadd_m(py);
    pos[0] = f32_of(x(nx).fmul(o).fadd_m(pos[0]));
    pos[1] = f32_of(o.fmul(ny).fadd_m(pos[1]));
    pos[2] = f32_of(x(pos[2]).fadd_m(2.0));
    let side_y = f32_of(x(nx).fmul(p4).fchs());
    let side_x = f32_of(p4.fmul(ny));
    let front_y = f32_of(x(w).fmul(ny));
    let front_x = f32_of(x(w).fmul_m(nx));
    let tex = g::<GPtr<u8>>(0xC403F8).read();
    store_car_light_shadow(
        this,
        this.addr() as u32 + 0x16,
        tex,
        GPtr::from_raw(pos.as_mut_ptr()).cast(),
        front_x,
        front_y,
        side_x,
        side_y,
        0x2D,
        0x2D,
        0x2D,
        7.0,
    );
}

/// `CAutomobile::ScanForCrimes()` (0x6ADFF0): when the player's vehicle (`FindPlayerVehicle(-1,
/// false)`, called again for every access) is an automobile (+0x590 == 0) with a running alarm
/// (+0x45C not 0/0xFFFF) and not wrecked (+0x36 & 0xF8 != 0x28), and its squared distance to this car
/// (`(dz² + dx²) + dy²`, x87) is below 400: `Players[PlayerInFocus].m_pPed->SetWantedLevelNoDrop(1)`.
#[hook(addr = 0x6ADFF0, conv = thiscall, name = "CAutomobile::ScanForCrimes", this = "CAutomobile", expect = "56 6A 00 6A FF 8B F1",
       unverified = "mvpb-veh-2", route = "L2", hazards = "none")]
pub fn scan_for_crimes(this: GPtr<u8>) {
    let fpv = || sa_world::world::common::find_player_vehicle(-1, 0);
    if fpv().is_null() || rd::<u32>(fpv(), 0x590) != 0 {
        return;
    }
    let pv = fpv();
    let alarm = rd::<u16>(pv, 0x45C);
    if alarm == 0 || alarm == 0xFFFF || rd::<u8>(pv, 0x36) & 0xF8 == 0x28 {
        return;
    }
    let a = entity_pos(this);
    let b = entity_pos(fpv());
    let dx = x(rd::<f32>(b, 0)).fsub_m(rd::<f32>(a, 0));
    let dy = x(rd::<f32>(b, 4)).fsub_m(rd::<f32>(a, 4));
    let dz = x(rd::<f32>(b, 8)).fsub_m(rd::<f32>(a, 8));
    let d = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    if d.fcom(x(400.0)).test_parity(0x05) {
        return;
    }
    let idx = usize::from(g::<u8>(0xB7CD74).read());
    let ped = g::<GPtr<u8>>(0xB7CD98 + idx * 0x190).read();
    player_set_wanted_level_no_drop(ped, 1);
}

/// `CVehicle::SetTransmissionRotation(RwFrame*, float angleL, float angleR, CVector wheelPos, bool
/// isFront)` (0x6DBBB0, `ret 0x1C`, `this` unused): with a frame, a stack `CMatrix` attached to its
/// modelling matrix (+0x10, not owned); saved = its position; `rx = -fpatan(((L + R) * 0.5) - wz, pos.y -
/// wy)` (+ PI 0x858CB8 when isFront, after the f32 store), `ry = fpatan(L - R, |wx| + |wx|)`, both stored
/// as f32 into the original's L/R argument slots; `SetRotateX(rx)`, `RotateY(ry)`, position += saved,
/// `UpdateRW`, destructor.
#[hook(addr = 0x6DBBB0, conv = thiscall, name = "CVehicle::SetTransmissionRotation", this = "CVehicle", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "mvpb-veh-2", route = "L2", hazards = "x87-transcendental",
       note = "the original keeps rx/ry in its own angle argument slots (entry esp+8/+0xC); the port uses locals")]
#[allow(clippy::too_many_arguments)]
pub fn set_transmission_rotation(_this: GPtr<u8>, frame: GPtr<u8>, l: f32, r: f32, wx: f32, wy: f32, wz: f32, front: u8) {
    if frame.is_null() {
        return;
    }
    let mut mat = [0u32; 18];
    let m = GPtr::from_raw(mat.as_mut_ptr()).cast::<u8>();
    matrix_attach(m, frame.byte_add(0x10), 0);
    let h = x(l).fadd_m(r).fmul_m(0.5);
    let saved = [rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)];
    let dlr = x(l).fsub_m(r);
    let dy = x(rd::<f32>(m, 0x34)).fsub_m(wy);
    let mut rx = f32_of(h.fsub_m(wz).fpatan(dy).fchs());
    let ry = f32_of(dlr.fpatan(x(wx).fabs().fadd(x(wx).fabs())));
    if front != 0 {
        rx = f32_of(x(rx).fadd_m(f32::from_bits(0x4049_0FDB)));
    }
    matrix_set_rotate_x(m, rx);
    matrix_rotate_y(m, ry);
    for (k, s) in saved.iter().enumerate() {
        let off = 0x30 + 4 * k as isize;
        wr::<f32>(m, off, f32_of(x(rd::<f32>(m, off)).fadd_m(*s)));
    }
    matrix_update_rw(m);
    matrix_dtor(m);
}

/// `CVehicle::DoTailLightEffect(int32 lightId, CMatrix& m, bool isRight, bool forcedOff, uint32 flags,
/// bool lightOn)` (0x6E1780, `ret 0x18`; a protection jump to 0x408076 clears the result byte and
/// comes back). Models 0x1B7/0x1DB with panel 6 damaged: false. `P` = vehicle struct +0xC + id * 0x18;
/// light 1 at (0,0,0): false. `Q = P` with `Q.x = P.x - (P.x + P.x)` on the left side;
/// `W = m * Q` (into P's local); `D = cam - W` normalised (`mag` kept); `dot = (-fz * Dz + -fy * Dy) +
/// -fx * Dx` with the matrix forward. False when forcedOff, `dot <= 0`, or the active camera is in mode
/// 0x10 (first person) on the player's own vehicle. `a = dot * 0.5 + 0.2` (x87), `size = (1 - mag *
/// 0.006666) * [0x8D3688] * dot`; trailers (+0x594 == 6, model not 0x1C1): `a *= 3` capped at 1, size
/// * 4. `dot = sqrt(dot)`. Red = `ftol(a * 128)` when braking (+0x4A0 > 0, +0x428 bit 5 clear, a
/// driver), else `ftol(a * 96)` when lightOn, else nothing is registered but the corona call still
/// happens with red 0. `CCoronas::RegisterCorona(this + isRight + 2 * id + 8, this, red, 0, 0, 0x80,
/// &Q, size, [0xB6F118] * 150, 1, 0, true, false, 0, dot, false, 0.5, 0, 15, false, false)`; returns
/// whether a red value was computed.
#[hook(addr = 0x6E1780, conv = thiscall, name = "CVehicle::DoTailLightEffect", this = "CVehicle", expect = "83 EC 28 56 8B F1",
       unverified = "mvpb-veh-2", route = "L2", hazards = "x87-transcendental",
       note = "the original keeps mag/dot/size in its own argument slots (entry esp+4/+8/+0xC); the port uses locals")]
#[allow(clippy::too_many_arguments)]
pub fn do_tail_light_effect(this: GPtr<u8>, id: i32, mat: GPtr<u8>, right: u8, forced_off: u8, _flags: u32, light_on: u8) -> u8 {
    let mi = rd::<u16>(this, 0x22);
    if (mi == 0x1B7 || mi == 0x1DB) && dm_get_panel_status(this.byte_add(0x5A0), 6) != 0 {
        return 0;
    }
    let vs = rd::<GPtr<u8>>(crate::common::model_info(rd::<i16>(this, 0x22) as i32), 0x5C);
    let pp = vs.byte_add(0xC + id as isize * 0x18);
    let mut p = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)];
    let ne = |v: f32| x(v).fcom(x(0.0)).test_parity(0x44);
    if id == 1 && !ne(p[0]) && !ne(p[1]) && !ne(p[2]) {
        return 0;
    }
    let mut q = p;
    if right == 0 {
        q[0] = f32_of(x(p[0]).fadd(x(p[0])).fsubr_m(p[0]));
    }
    let cam = sa_world::world::common::cam_pos();
    let pptr = GPtr::from_raw(p.as_mut_ptr()).cast::<u8>();
    let qptr = GPtr::from_raw(q.as_mut_ptr()).cast::<u8>();
    let w = matrix_mul_vec(pptr, mat, qptr);
    let dz = f32_of(x(cam.wrapping_index(2).read()).fsub_m(rd::<f32>(w, 8)));
    let dy = f32_of(x(cam.wrapping_index(1).read()).fsub_m(rd::<f32>(w, 4)));
    let dx = f32_of(x(cam.read()).fsub_m(rd::<f32>(w, 0)));
    let mut d = [dx, dy, dz];
    let mag = normalise_and_mag(GPtr::from_raw(d.as_mut_ptr()).cast());
    let fx = x(rd::<f32>(mat, 0x10)).fchs();
    let fy = x(rd::<f32>(mat, 0x14)).fchs();
    let fz = x(rd::<f32>(mat, 0x18)).fchs();
    let dot = f32_of(fz.fmul_m(d[2]).fadd(fy.fmul_m(d[1])).fadd(fx.fmul_m(d[0])));
    if forced_off != 0 {
        return 0;
    }
    if x(dot).fcom(x(0.0)).test_any(0x41) {
        return 0;
    }
    let active = usize::from(g::<u8>(0xB6F081).read());
    if g::<u16>(0xB6F1A8 + active * 0x238).read() == 0x10 && sa_world::world::common::find_player_vehicle(-1, 0) == this {
        return 0;
    }
    let mut a = x(dot).fmul_m(0.5).fadd_m(f32::from_bits(0x3E4C_CCCD));
    let mut size = f32_of(
        x(mag)
            .fmul_m(f32::from_bits(0x3BDA_740E))
            .fsubr_m(1.0)
            .fmul_m(g::<f32>(0x8D3688).read())
            .fmul_m(dot),
    );
    if rd::<u32>(this, 0x594) == 6 && rd::<u16>(this, 0x22) != 0x1C1 {
        a = a.fmul_m(3.0);
        if !x(1.0).fcom(a).test_parity(0x05) {
            a = x(1.0);
        }
        size = f32_of(x(size).fmul_m(4.0));
    }
    let sdot = f32_of(x(dot).fsqrt());
    let mut red: u32 = 0;
    let mut done = false;
    if !x(rd::<f32>(this, 0x4A0)).fcom(x(0.0)).test_any(0x41)
        && rd::<u8>(this, 0x428) & 0x20 == 0
        && !rd::<GPtr<u8>>(this, 0x460).is_null()
    {
        red = crate::common::ftol32(a.fmul_m(128.0)) as u32;
        done = true;
    } else if light_on != 0 {
        red = crate::common::ftol32(a.fmul_m(96.0)) as u32;
        done = true;
    }
    let far = f32_of(x(g::<f32>(0xB6F118).read()).fmul_m(150.0));
    let cid = (this.addr() as u32).wrapping_add(u32::from(right)).wrapping_add((id as u32).wrapping_mul(2)).wrapping_add(8);
    register_corona(cid, this.addr() as u32, red, 0, 0, 0x80, qptr, size, far, 1, 0, 1, 0, 0, sdot, 0, 0.5, 0, 15.0, 0, 0);
    u8::from(done)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DO_TAIL_LIGHT_EFFECT,
    HOOK_CALCULATE_GEAR_FOR_SIMPLE_CAR,
    HOOK_TRAIN_RENDER,
    HOOK_DO_HEAD_LIGHT_REFLECTION_TWIN,
    HOOK_SCAN_FOR_CRIMES,
    HOOK_SET_TRANSMISSION_ROTATION,
];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
