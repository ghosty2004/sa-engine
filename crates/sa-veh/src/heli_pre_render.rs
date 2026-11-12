//! PORT-VEH-4 (batch `veh4-2`): `CHeli::PreRender`. Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, g, jne_mask, jp5, model_info, rd, wr, x};

orig! {
    /// `CVehicle::PreRender()` (0x6D6480)
    fn vehicle_pre_render(v: GPtr<u8>) = thiscall 0x6D6480 => unported;
    /// `CClock::GetIsTimeInRange(uint8 from, uint8 to)` (0x52CEE0, AL)
    fn clock_get_is_time_in_range(from: u32, to: u32) -> u8 = cdecl 0x52CEE0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, the result through `out`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CHeli::AddHeliSearchLight(const CVector& origin, const CVector& target, float targetRadius,
    /// float power, uint32 coronaIndex, uint8, uint8 drawShadow)` (0x6C45B0)
    fn add_heli_search_light(origin: GPtr<u8>, target: GPtr<u8>, radius: f32, power: u32, corona: u32, a: u32, shadow: u32) = cdecl 0x6C45B0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// virtual (vtable +0xD0) of `CAutomobile`: the burst/soft-ground suspension ratios
    fn v_slot_d0(v: GPtr<u8>) = thiscall vslot 0xD0 => unported;
    /// `CVehicleModelInfo::GetWheelPosn(int32 wheel, CVector& out, bool local)` (0x4C7D20)
    fn get_wheel_posn(mi: GPtr<u8>, wheel: i32, out: GPtr<u8>, local: u32) = thiscall 0x4C7D20 => unported;
    /// `CAutomobile::UpdateWheelMatrix(int32 node, int32 flags)` (0x6AA290)
    fn update_wheel_matrix(v: GPtr<u8>, node: i32, flags: u32) = thiscall 0x6AA290 => unported;
    /// `CAutomobile::DoHeliDustEffect(float, float)` (0x6B0690)
    fn do_heli_dust_effect(v: GPtr<u8>, a: f32, b: f32) = thiscall 0x6B0690 => unported;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::SetRotateX(float)` (0x59B060)
    fn matrix_set_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B060 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0)
    fn matrix_set_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B0E0 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CShadows::StoreShadowForVehicle(CVehicle*, VEH_SHD_TYPE)` (0x70BDA0)
    fn store_shadow_for_vehicle(v: GPtr<u8>, kind: u32) = cdecl 0x70BDA0 => unported;
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Rotates a rotor frame through the shared `CMatrix`: attached to the frame's matrix, the rotation
/// replaced (`SetRotateZ` for the main rotor, `SetRotateX` for the tail), the position kept.
fn spin(m: GPtr<u8>, frame: GPtr<u8>, angle: f32, tail: bool) {
    if frame.is_null() {
        return;
    }
    matrix_attach(m, frame.byte_add(0x10), 0);
    let saved = [rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)];
    if tail {
        matrix_set_rotate_x(m, angle);
    } else {
        matrix_set_rotate_z(m, angle);
    }
    for (k, o) in [(0usize, 0x30isize), (1, 0x34), (2, 0x38)] {
        wr::<f32>(m, o, f32_of(x(rd::<f32>(m, o)).fadd_m(saved[k])));
    }
    matrix_update_rw(m);
}

/// `CHeli::PreRender()` (0x6C5420): the vehicle pre-render; the search light (when on, with power
/// above 0, between 19:00 and 06:00) from `matrix * (0, 3.5, -0.3)` to its target (+0x9F4) with radius
/// 20; with the suspension flag (+0x42B bit 0) the slot-0xD0 update and per wheel the suspension
/// position (+0x838) eased toward the wheel's z (+ handling +0xB8, minus the compression-scaled length)
/// at 0.75 unless it rises or the vehicle is flagged to snap (+0x40 bit 26 and +0x38C bit 17); the
/// four wheel matrices; dust (except the RC Raider/Goblin, 465/501); the main rotor angle (+0x9A0)
/// turns by `timeStep * rotorSpeed` (x1.66 for 447/469/487/488/497), wrapped above -2pi; the tail rotor
/// (+0x9A4) by x2 (Leviathan 417) or x2.3, wrapped below 2pi; the rotor frames (+0x678/+0x67C main,
/// +0x680/+0x684 tail) are set to them; the vehicle shadow (type 2) is stored.
#[hook(addr = 0x6C5420, conv = thiscall, name = "CHeli::PreRender", this = "CHeli",
       expect = "6A FF 68 88 84 84 00",
       unverified = "veh4-2", route = "L2", hazards = "heap-writes",
       recs = "driving",
       note = "the rotor CMatrix lives on the Rust stack; writes the rotor frames' RwMatrix and the shadow list; the corona index passed to AddHeliSearchLight is `this + 0xB` as the original computes it; the SEH frame is not reproduced")]
pub fn heli_pre_render(this: GPtr<u8>) {
    vehicle_pre_render(this);
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    let mut mat = [0u32; 0x12];
    let m = buf(&mut mat);
    if rd::<u8>(this, 0xA11) != 0
        && !jne_mask(x(rd::<f32>(this, 0xA00)), x(0.0), 0x41)
        && clock_get_is_time_in_range(0x13, 6) != 0
    {
        let mut target = [rd::<u32>(this, 0x9F4), rd::<u32>(this, 0x9F8), rd::<u32>(this, 0x9FC)];
        let mut offset = [0u32, 0x4060_0000, 0xBE99_999A];
        let mut out = [0u32; 3];
        let r = matrix_mul_vec(buf(&mut out), rd::<GPtr<u8>>(this, 0x14), buf(&mut offset));
        add_heli_search_light(r, buf(&mut target), 20.0, rd::<u32>(this, 0xA00), (this.addr() as u32).wrapping_add(0xB), 1, 1);
    }
    entity_get_col_model(this);
    if rd::<u8>(this, 0x42B) & 1 != 0 {
        v_slot_d0(this);
        let handling = || rd::<GPtr<u8>>(this, 0x384);
        for wheel in 0..4i32 {
            let o = 4 * wheel as isize;
            let t = x(rd::<f32>(this, 0x878 + o)).fdiv_m(rd::<f32>(this, 0x888 + o)).fsubr_m(1.0);
            let v = f32_of(x(rd::<f32>(this, 0x7D4 + o)).fsub(t).fdiv(x(1.0).fsub(t)));
            let mut pos = [0u32; 3];
            get_wheel_posn(mi, wheel, buf(&mut pos), 1);
            let mut z = x(f32::from_bits(pos[2])).fadd_m(rd::<f32>(handling(), 0xB8));
            if !jne_mask(x(v), x(0.0), 0x41) {
                z = z.fsub(x(v).fmul_m(rd::<f32>(this, 0x878 + o)));
            }
            let cur = rd::<f32>(this, 0x838 + o);
            if z.fcom(x(cur)).test_any(0x41)
                && !(rd::<u32>(this, 0x40) & 0x400_0000 != 0 && rd::<u32>(this, 0x38C) & 0x2_0000 != 0)
            {
                z = z.fsub_m(cur).fmul_m(0.75).fadd_m(cur);
            }
            wr::<f32>(this, 0x838 + o, f32_of(z));
        }
    }
    for node in [4, 7, 2, 5] {
        update_wheel_matrix(this, node, 1);
    }
    let model = rd::<i16>(this, 0x22);
    if model != 0x1D1 && model != 0x1F5 {
        do_heli_dust_effect(this, 1.0, 1.0);
    }
    let ts = || g::<f32>(0xB7_CB5C).read();
    let mut d = x(ts()).fmul_m(rd::<f32>(this, 0x84C));
    if matches!(model, 0x1D5 | 0x1BF | 0x1E7 | 0x1E8 | 0x1F1) {
        d = d.fmul_m(g::<f32>(0x8D_33A0).read());
    }
    wr::<f32>(this, 0x9A0, f32_of(d.fsubr_m(rd::<f32>(this, 0x9A0))));
    let low = f32::from_bits(0xC0C9_0FDB);
    let two_pi = f32::from_bits(0x40C9_0FDB);
    if !jp5(x(rd::<f32>(this, 0x9A0)), x(low)) {
        let mut v = x(rd::<f32>(this, 0x9A0));
        loop {
            v = v.fadd_m(two_pi);
            if jp5(v, x(low)) {
                break;
            }
        }
        wr::<f32>(this, 0x9A0, f32_of(v));
    }
    let d = x(ts()).fmul_m(rd::<f32>(this, 0x84C));
    let d = if model == 0x1A1 { d.fdouble() } else { d.fmul_m(f32::from_bits(0x4013_3333)) };
    wr::<f32>(this, 0x9A4, f32_of(d.fsubr_m(rd::<f32>(this, 0x9A4))));
    if !jne_mask(x(rd::<f32>(this, 0x9A4)), x(two_pi), 0x41) {
        let mut v = x(rd::<f32>(this, 0x9A4));
        loop {
            v = v.fsub_m(two_pi);
            if jne_mask(v, x(two_pi), 0x41) {
                break;
            }
        }
        wr::<f32>(this, 0x9A4, f32_of(v));
    }
    spin(m, rd::<GPtr<u8>>(this, 0x678), rd::<f32>(this, 0x9A0), false);
    spin(m, rd::<GPtr<u8>>(this, 0x67C), rd::<f32>(this, 0x9A0), false);
    spin(m, rd::<GPtr<u8>>(this, 0x680), rd::<f32>(this, 0x9A4), true);
    spin(m, rd::<GPtr<u8>>(this, 0x684), rd::<f32>(this, 0x9A4), true);
    store_shadow_for_vehicle(this, 2);
    matrix_dtor(m);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_HELI_PRE_RENDER];
