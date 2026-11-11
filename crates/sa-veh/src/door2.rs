//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `CDoor` and `CBouncingPanel`. Ported from the 1.0 US
//! bytes; gta-reversed and docs/re/specs/VEH as reading aids.
//!
//! `CDoor` (0x18): +0 open angle, +4 closed angle, +8 dirn (low nibble: direction 0..5), +0xA axis,
//! +0xB state, +0xC angle, +0x10 previous angle, +0x14 angular velocity.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use crate::common::{g, jne_mask, jp5, rd, wr, x, xi};

orig! {
    /// `CPhysical::GetSpeed(CVector)` (0x542CE0), hidden return pointer first
    pub fn physical_get_speed(p: GPtr<u8>, out: GPtr<u8>, x: u32, y: u32, z: u32) -> GPtr<u8> = thiscall 0x542CE0 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730)
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0)
    pub fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `Multiply3x3(CVector* out, const CVector*, const CMatrix*)` (0x59C810: vector times matrix, the
    /// inverse rotation)
    /// `Multiply3x3(CVector* out, const CMatrix*, const CVector*)` (0x59C790: matrix times vector)
    pub fn multiply3x3_mv(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    pub fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, temp: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    pub fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    pub fn multiply3x3_vm(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
}

/// `CTimer::ms_fTimeStep`.
fn time_step() -> f32 {
    g::<f32>(0xB7CB5C).read()
}

/// `fld dword [addr]` of a `.rdata` constant.
fn k(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `fabs`: clears the sign bit (of a NaN too).
fn fabs(v: X87) -> X87 {
    X87::from_f64_exact(f64::from_bits(v.to_f64().to_bits() & !(1 << 63)))
}

/// A stack `CVector` handed to original code.
fn vec_ptr(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v as *mut [f32; 3]).cast()
}

/// `CDoor::ProcessImpact(CVehicle*, CVector& oldMoveSpeed, CVector& oldTurnSpeed, CVector& offset)`
/// (0x6F4540): acceleration `a = GetSpeed(offset) - (oldTurn x offset + oldMove)` (the old velocity's
/// x and y stored as f32, z at register precision); a `CMatrix` copy of the vehicle matrix is built and
/// destroyed unused; the direction is the matrix's right/forward/up row (dirn 0..2) or its negation
/// (3..5). `ang` = (a x dir).x for axis 0, .z for axis 2; for any other axis the stack slot still holds
/// the old velocity's y, which is used (kept). limit = `(rand() * [0x858C7C] * 0.75 + 0.75) * 0.1`;
/// returns `ang > limit` when `openAngle >= closedAngle` (or unordered), else `-limit > ang`. Only AL
/// is defined. Deviation: a dirn nibble above 5 leaves the original's direction uninitialised (stack
/// garbage); this uses (0, 0, 0).
#[hook(addr = 0x6F4540, conv = thiscall, name = "CDoor::ProcessImpact", expect = "6A FF 68 48 8A 84 00", this = "CDoor")]
pub fn process_impact(
    door: GPtr<u8>,
    veh: GPtr<u8>,
    old_move: GPtr<u8>,
    old_turn: GPtr<u8>,
    offset: GPtr<u8>,
) -> u8 {
    let mut vel = [0f32; 3];
    physical_get_speed(
        veh,
        vec_ptr(&mut vel),
        rd::<u32>(offset, 0),
        rd::<u32>(offset, 4),
        rd::<u32>(offset, 8),
    );
    let mut c = [0f32; 3];
    let cp = cross_product(vec_ptr(&mut c), old_turn, offset);
    let oz = x(rd::<f32>(cp, 8)).fadd_m(rd::<f32>(old_move, 8));
    let oy = x(rd::<f32>(cp, 4)).fadd_m(rd::<f32>(old_move, 4)).to_f32();
    let ox = x(rd::<f32>(old_move, 0)).fadd_m(rd::<f32>(cp, 0)).to_f32();
    let mut acc = [
        x(vel[0]).fsub_m(ox).to_f32(),
        x(vel[1]).fsub_m(oy).to_f32(),
        oz.fsubr_m(vel[2]).to_f32(),
    ];
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let mut mat = [0u32; 0x12];
    let mp = GPtr::from_raw(&raw mut mat).cast::<u8>();
    matrix_copy_ctor(mp, m);

    let mut dir = [0f32; 3];
    let dirn = rd::<u8>(door, 8) & 0xF;
    if dirn <= 5 {
        let row = isize::from(dirn % 3) * 0x10;
        for (k, d) in dir.iter_mut().enumerate() {
            let v = rd::<f32>(m, row + 4 * k as isize);
            *d = if dirn >= 3 { x(v).fchs().to_f32() } else { v };
        }
    }
    let ang = match rd::<u8>(door, 0xA) {
        0 => {
            let mut o = [0f32; 3];
            let r = cross_product(vec_ptr(&mut o), vec_ptr(&mut acc), vec_ptr(&mut dir));
            rd::<f32>(r, 0)
        }
        2 => {
            let mut o = [0f32; 3];
            let r = cross_product(vec_ptr(&mut o), vec_ptr(&mut acc), vec_ptr(&mut dir));
            rd::<f32>(r, 8)
        }
        _ => oy,
    };

    let r = crate::common::rand_at(0x6F46F1);
    let k75 = g::<f32>(0x858F34).read();
    let limit = xi(r)
        .fmul_m(g::<f32>(0x858C7C).read())
        .fmul_m(k75)
        .fadd_m(k75)
        .fmul_m(g::<f32>(0x8D3968).read());
    let open_lt_closed = !x(rd::<f32>(door, 0))
        .fcom(x(rd::<f32>(door, 4)))
        .test_parity(0x05);
    let c = if open_lt_closed {
        limit.fchs().fcom(x(ang))
    } else {
        x(ang).fcom(limit)
    };
    matrix_dtor(mp);
    u8::from(!c.test_any(0x41))
}

/// `CDoor::Process(CVehicle*, CVector& oldMoveSpeed, CVector& oldTurnSpeed, CVector& offset)` (0x6F4040,
/// the SEH prologue through the .HOODLUM fragment 0x407DD5). Acceleration as in ProcessImpact (old
/// velocity x/y stored as f32; here `C.x + M.x`), z plus `timeStep * (dirn & 0x20 ? 0.0016 : 0.008)`,
/// rotated by `Multiply3x3(acc, CMatrix copy of the vehicle matrix)`. The unit direction (dirn 0..5 =
/// +x, +y, +z, -x, -y, -z; zero above 5) is rotated by the angle (sin/cos stored as f32): axis 0 ->
/// `(dx, cs*dy - sn*dz, cs*dz + sn*dy)`, `ang = (acc x dir).x`; axis 2 -> `(cs*dx - sn*dy, cs*dy +
/// sn*dx, dz)`, `ang = (acc x dir).z`; any other axis: `ang = sin(angle)` (kept). Angular velocity:
/// with dirn bit 6 `+= rate * ang` (rate 0 in widescreen or for an automobile without contact wheels,
/// else 0.025) then `*= pow(t, timeStep)` (t 0.95 / 0.98 by the same test) and `-= timeStep * angle *
/// 0.06`; otherwise `+= ang` when `|ang|` or `|angVel|` > 0.001, then the damping (bikes 0.98, model
/// 0x220 `0.92 * v - timeStep * angle * 0.01`, dirn bit 5 0.97, else 0.97). Clamped to ±(bit 6: 0.02,
/// `+9` bit 0: 0.05, else [0x8D3950]). `angle += angVel`; state 0 unless dirn bit 7; at the open limit
/// angle = open, angVel *= -0.8, state 1; at the closed limit angle = closed and either (dirn bit 4, not
/// bit 7, |angVel| > 0.1) angVel 0, state 4 and true, or angVel *= (bit 5 ? -0.9 : -0.4), state 2.
/// Returns false otherwise. Only AL is defined. Ported but unhookable: the protection code at 0x4072EE
/// (`sbb edi, [0x6F4043]`) reads the entry bytes as a key.
pub fn process(
    door: GPtr<u8>,
    veh: GPtr<u8>,
    old_move: GPtr<u8>,
    old_turn: GPtr<u8>,
    offset: GPtr<u8>,
) -> u8 {
    let mut vel = [0f32; 3];
    physical_get_speed(
        veh,
        vec_ptr(&mut vel),
        rd::<u32>(offset, 0),
        rd::<u32>(offset, 4),
        rd::<u32>(offset, 8),
    );
    let mut c = [0f32; 3];
    let cp = cross_product(vec_ptr(&mut c), old_turn, offset);
    let oz = x(rd::<f32>(cp, 8)).fadd_m(rd::<f32>(old_move, 8));
    let oy = x(rd::<f32>(cp, 4)).fadd_m(rd::<f32>(old_move, 4)).to_f32();
    let dirn = rd::<u8>(door, 8);
    let ox = x(rd::<f32>(cp, 0)).fadd_m(rd::<f32>(old_move, 0)).to_f32();
    let az = oz.fsubr_m(vel[2]);
    let ay = x(vel[1]).fsub_m(oy).to_f32();
    let ax = x(vel[0]).fsub_m(ox).to_f32();
    let grav = x(time_step()).fmul_m(if dirn & 0x20 != 0 {
        k(0x87149C)
    } else {
        k(0x863984)
    });
    let mut acc = [ax, ay, grav.fadd(az).to_f32()];
    let mut mat = [0u32; 0x12];
    let mp = GPtr::from_raw(&raw mut mat).cast::<u8>();
    matrix_copy_ctor(mp, rd::<GPtr<u8>>(veh, 0x14));
    let mut tmp = [0f32; 3];
    let r = multiply3x3_vm(vec_ptr(&mut tmp), vec_ptr(&mut acc), mp);
    acc = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];

    let mut dir = [0f32; 3];
    match dirn & 0xF {
        0 => dir[0] = 1.0,
        1 => dir[1] = 1.0,
        2 => dir[2] = 1.0,
        3 => dir[0] = -1.0,
        4 => dir[1] = -1.0,
        5 => dir[2] = -1.0,
        _ => {}
    }
    let angle = rd::<f32>(door, 0xC);
    let sn = x(angle).fsin().to_f32();
    let cs = x(angle).fcos().to_f32();
    let ang = match rd::<u8>(door, 0xA) {
        0 => {
            let p = x(cs).fmul_m(dir[2]).fadd(x(sn).fmul_m(dir[1]));
            let q = x(cs).fmul_m(dir[1]).fsub(x(sn).fmul_m(dir[2]));
            dir[1] = q.to_f32();
            dir[2] = p.to_f32();
            let mut o = [0f32; 3];
            let r = cross_product(vec_ptr(&mut o), vec_ptr(&mut acc), vec_ptr(&mut dir));
            x(rd::<f32>(r, 0))
        }
        2 => {
            let p = x(cs).fmul_m(dir[1]).fadd(x(sn).fmul_m(dir[0]));
            let q = x(cs).fmul_m(dir[0]).fsub(x(sn).fmul_m(dir[1]));
            dir[0] = q.to_f32();
            dir[1] = p.to_f32();
            let mut o = [0f32; 3];
            let r = cross_product(vec_ptr(&mut o), vec_ptr(&mut acc), vec_ptr(&mut dir));
            x(rd::<f32>(r, 8))
        }
        _ => x(sn),
    };

    let flags = rd::<u16>(door, 8);
    let chassis = flags & 0x40 != 0;
    // widescreen, or an automobile (type 0) without contact wheels
    let slow = || g::<u8>(0xB6F065).read() != 0 || (rd::<u32>(veh, 0x590) == 0 && rd::<u8>(veh, 0x960) == 0);
    if chassis {
        let rate = if slow() { k(0x87232C) } else { k(0x872328) };
        wr::<f32>(
            door,
            0x14,
            x(rate).fmul(ang).fadd_m(rd::<f32>(door, 0x14)).to_f32(),
        );
    } else if !jne_mask(fabs(ang), x(k(0x872304)), 0x41)
        || !jne_mask(fabs(x(rd::<f32>(door, 0x14))), x(k(0x872304)), 0x41)
    {
        wr::<f32>(door, 0x14, ang.fadd_m(rd::<f32>(door, 0x14)).to_f32());
    }

    if chassis {
        let t = if slow() { k(0x872320) } else { k(0x87231C) };
        let p = X87::from_f64_exact(sa_core::orig::x87_binary(
            0x822130,
            x(t).to_f64(),
            x(time_step()).to_f64(),
        ));
        let v = p.fmul_m(rd::<f32>(door, 0x14));
        wr::<f32>(door, 0x14, v.to_f32());
        let d = x(time_step()).fmul_m(rd::<f32>(door, 0xC)).fmul_m(k(0x872324));
        wr::<f32>(door, 0x14, v.fsub(d).to_f32());
    } else if rd::<u32>(veh, 0x590) == 5 {
        wr::<f32>(door, 0x14, x(k(0x872310)).fmul_m(rd::<f32>(door, 0x14)).to_f32());
    } else if rd::<u16>(veh, 0x22) == 0x220 {
        let v = x(k(0x872314)).fmul_m(rd::<f32>(door, 0x14));
        wr::<f32>(door, 0x14, v.to_f32());
        let d = x(time_step()).fmul_m(rd::<f32>(door, 0xC)).fmul_m(k(0x872338));
        wr::<f32>(door, 0x14, v.fsub(d).to_f32());
    } else {
        let f = if flags & 0x20 != 0 {
            k(0x872318)
        } else {
            k(0x87230C)
        };
        wr::<f32>(door, 0x14, x(f).fmul_m(rd::<f32>(door, 0x14)).to_f32());
    }

    let lim = if chassis {
        k(0x872330)
    } else if flags & 0x100 != 0 {
        k(0x872334)
    } else {
        k(0x8D3950)
    };
    let lim = x(lim);
    if !jp5(lim, x(rd::<f32>(door, 0x14))) {
        wr::<f32>(door, 0x14, lim.to_f32());
    } else if !jne_mask(lim.fchs(), x(rd::<f32>(door, 0x14)), 0x41) {
        wr::<f32>(door, 0x14, lim.fchs().to_f32());
    }

    let fixed = flags & 0x80 != 0;
    let na = x(rd::<f32>(door, 0x14)).fadd_m(rd::<f32>(door, 0xC));
    wr::<f32>(door, 0xC, na.to_f32());
    if !fixed {
        wr::<u8>(door, 0xB, 0);
    }
    let open = rd::<f32>(door, 0);
    let closed = rd::<f32>(door, 4);
    let open_lt_closed = !jp5(x(open), x(closed));
    let (at_open, at_closed) = if !open_lt_closed {
        let o = !na.fcom(x(open)).test_any(0x41);
        (o, !o && !jp5(na, x(closed)))
    } else {
        let o = !jp5(na, x(open));
        (o, !o && !jne_mask(na, x(closed), 0x41))
    };
    let mut ret = 0u8;
    if at_open {
        let v = x(rd::<f32>(door, 0x14)).fmul_m(k(0x858C9C));
        wr::<u32>(door, 0xC, rd::<u32>(door, 0));
        wr::<f32>(door, 0x14, v.to_f32());
        if !fixed {
            wr::<u8>(door, 0xB, 1);
        }
    } else if at_closed {
        wr::<u32>(door, 0xC, rd::<u32>(door, 4));
        if flags & 0x10 != 0 && !fixed && !jne_mask(fabs(x(rd::<f32>(door, 0x14))), x(k(0x87233C)), 0x41) {
            wr::<u32>(door, 0x14, 0);
            wr::<u8>(door, 0xB, 4);
            ret = 1;
        } else {
            let f = if flags & 0x20 != 0 {
                k(0x858CAC)
            } else {
                k(0x858F0C)
            };
            wr::<f32>(door, 0x14, x(rd::<f32>(door, 0x14)).fmul_m(f).to_f32());
            if !fixed {
                wr::<u8>(door, 0xB, 2);
            }
        }
    }
    matrix_dtor(mp);
    ret
}

/// `BOUNCE_VEL_CHANGE_LIMIT` clamp as ProcessPanel inlines it: `m = (-L > v) ? -L : v`; `L < m` -> L,
/// else `(-L > v) ? -L : v` (a NaN v stays).
fn bounce_clamp(v: f32) -> X87 {
    let l = x(k(0x8D395C));
    let nl = l.fchs();
    let m = if jne_mask(nl, x(v), 0x41) { x(v) } else { nl };
    if !jp5(l, m) {
        l
    } else if !jne_mask(nl, x(v), 0x41) {
        nl
    } else {
        x(v)
    }
}

/// `CBouncingPanel::ProcessPanel(CVehicle*, RwFrame*, CVector moveForce, CVector turnForce, float
/// returnMult, float dampMult)` (0x6F49A0). `cp = Multiply3x3(vehicle matrix, frame position)` (the
/// frame matrix wrapped in a CMatrix); `d = GetSpeed(cp) - (move + turn x cp)` (operand order of the
/// bytes; gta-reversed adds `cp x turn`), the sum's x/y stored as f32; `av = Multiply3x3(d, vehicle
/// matrix)` (inverse rotation). Per axis the velocity `+0x14..` gains `clamp(av.c) * angleLimit`
/// (axis 0: y += z, z += y; 1: x += z, z += y; 2: x += y, y += x). Return: `pos -= mult * rot * timeStep`
/// (mult = returnMult when both multipliers are `> -1.0`, else 0.02 / 0.1 for axis 2 / others; the x
/// product is stored as f32 before the time step multiply, the z one after); `pos *= pow(damp,
/// timeStep)` (damp = dampMult, or 0.98 / 0.95); `rot += timeStep * pos`. The frame matrix gets the
/// rotation (axis 0: right.y = rot.z, right.z = rot.y; 1: fwd.x = rot.z, fwd.z = rot.x; 2: up.y =
/// rot.x, up.x = rot.y), `UpdateRW`, destroyed.
#[hook(addr = 0x6F49A0, conv = thiscall, name = "CBouncingPanel::ProcessPanel", expect = "6A FF 68 68 8A 84 00", this = "CBouncingPanel")]
#[allow(clippy::too_many_arguments)]
pub fn process_panel(
    pn: GPtr<u8>,
    veh: GPtr<u8>,
    frame: GPtr<u8>,
    mx: f32,
    my: f32,
    mz: f32,
    tx: f32,
    ty: f32,
    tz: f32,
    ret_mult: f32,
    damp_mult: f32,
) {
    let mut fm = [0u32; 0x12];
    let fmp = GPtr::from_raw(&raw mut fm).cast::<u8>();
    matrix_ctor_rw(fmp, frame.byte_add(0x10), 0);
    let mut cp = [0f32; 3];
    multiply3x3_mv(vec_ptr(&mut cp), rd::<GPtr<u8>>(veh, 0x14), fmp.byte_add(0x30));
    let mut vel = [0f32; 3];
    physical_get_speed(
        veh,
        vec_ptr(&mut vel),
        cp[0].to_bits(),
        cp[1].to_bits(),
        cp[2].to_bits(),
    );
    let mut turn = [tx, ty, tz];
    let mut xo = [0f32; 3];
    let xr = cross_product(vec_ptr(&mut xo), vec_ptr(&mut turn), vec_ptr(&mut cp));
    let sz = x(mz).fadd_m(rd::<f32>(xr, 8));
    let sy = x(my).fadd_m(rd::<f32>(xr, 4)).to_f32();
    let sx = x(mx).fadd_m(rd::<f32>(xr, 0)).to_f32();
    let mut d = [
        x(vel[0]).fsub_m(sx).to_f32(),
        x(vel[1]).fsub_m(sy).to_f32(),
        sz.fsubr_m(vel[2]).to_f32(),
    ];
    let mut avo = [0f32; 3];
    let r = multiply3x3_vm(vec_ptr(&mut avo), vec_ptr(&mut d), rd::<GPtr<u8>>(veh, 0x14));
    let av = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];

    let axis = rd::<u16>(pn, 2);
    let lim = |pn: GPtr<u8>, off: isize, v: f32| {
        let n = bounce_clamp(v)
            .fmul_m(rd::<f32>(pn, 4))
            .fadd_m(rd::<f32>(pn, off));
        wr::<f32>(pn, off, n.to_f32());
    };
    match axis {
        0 => {
            lim(pn, 0x18, av[2]);
            lim(pn, 0x1C, av[1]);
        }
        1 => {
            lim(pn, 0x14, av[2]);
            lim(pn, 0x1C, av[1]);
        }
        2 => {
            lim(pn, 0x14, av[1]);
            lim(pn, 0x18, av[0]);
        }
        _ => {}
    }

    let ts = time_step();
    let neg1 = x(k(0x858C1C));
    let (rm, dm) = if !jne_mask(x(ret_mult), neg1, 0x41) && !jne_mask(x(damp_mult), neg1, 0x41) {
        (ret_mult, damp_mult)
    } else if axis == 2 {
        (k(0x8D3964), k(0x8D3960))
    } else {
        (k(0x8D3958), k(0x8D3954))
    };
    let pz = x(rm).fmul_m(rd::<f32>(pn, 0x10));
    let py = x(rm).fmul_m(rd::<f32>(pn, 0xC));
    let px = x(rm).fmul_m(rd::<f32>(pn, 8)).to_f32();
    let az = pz.fmul_m(ts).to_f32();
    let ay = py.fmul_m(ts);
    let ax = x(px).fmul_m(ts);
    wr::<f32>(pn, 0x14, x(rd::<f32>(pn, 0x14)).fsub(ax).to_f32());
    wr::<f32>(pn, 0x18, x(rd::<f32>(pn, 0x18)).fsub(ay).to_f32());
    wr::<f32>(pn, 0x1C, x(rd::<f32>(pn, 0x1C)).fsub_m(az).to_f32());
    let p = X87::from_f64_exact(sa_core::orig::x87_binary(
        0x822130,
        x(dm).to_f64(),
        x(ts).to_f64(),
    ));
    for off in [0x14, 0x18, 0x1C] {
        wr::<f32>(pn, off, p.fmul_m(rd::<f32>(pn, off)).to_f32());
    }
    let ts = time_step();
    let cz = x(ts).fmul_m(rd::<f32>(pn, 0x1C));
    let cy = x(ts).fmul_m(rd::<f32>(pn, 0x18));
    let cx = x(ts).fmul_m(rd::<f32>(pn, 0x14));
    wr::<f32>(pn, 8, cx.fadd_m(rd::<f32>(pn, 8)).to_f32());
    wr::<f32>(pn, 0xC, cy.fadd_m(rd::<f32>(pn, 0xC)).to_f32());
    wr::<f32>(pn, 0x10, cz.fadd_m(rd::<f32>(pn, 0x10)).to_f32());

    match rd::<u16>(pn, 2) {
        0 => {
            wr::<u32>(fmp, 4, rd::<u32>(pn, 0x10));
            wr::<u32>(fmp, 8, rd::<u32>(pn, 0xC));
        }
        1 => {
            wr::<u32>(fmp, 0x10, rd::<u32>(pn, 0x10));
            wr::<u32>(fmp, 0x18, rd::<u32>(pn, 8));
        }
        2 => {
            wr::<u32>(fmp, 0x24, rd::<u32>(pn, 8));
            wr::<u32>(fmp, 0x20, rd::<u32>(pn, 0xC));
        }
        _ => {}
    }
    matrix_update_rw(fmp);
    matrix_dtor(fmp);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_IMPACT, HOOK_PROCESS_PANEL];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
