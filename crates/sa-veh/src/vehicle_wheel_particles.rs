//! WRITER-VEH-BIG (batch `vehbig-1`): `CVehicle::AddSingleWheelParticles`, a wheel's sparks, dirt /
//! water / smoke particles and skid marks. Ported from the 1.0 US bytes; gta-reversed as a
//! structural guide only.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::{find_player_ped, find_player_vehicle};

use crate::common::{ftol32, g, rand_at, rd, x, xi};

orig! {
    /// `SurfaceInfos_c::IsWater(uint32)` (0x55E710): bool in AL
    fn is_water(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E710 => unported;
    /// `CWaterLevel::GetWaterLevel(float, float, float, float*, bool, CVector*)` (0x6EB690)
    fn get_water_level(px: f32, py: f32, pz: f32, out: GPtr<f32>, waves: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `SurfaceInfos_c::GetFrictionEffect(uint32)` (0x55E650)
    fn get_friction_effect(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E650 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, hidden return pointer)
    fn vector_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => sa_world::world::mvpa_math::vector_mul_scalar;
    /// `CVector::NormaliseAndMag()` (0x59C970), ST0
    fn normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => normalise_and_mag_native;
    /// `Fx_c::AddSparks(CVector&, CVector&, float, int32, CVector, eSparkType, float, float)` (0x49F040) on `g_fx` (0xA9AE00)
    fn add_sparks(fx: GPtr<u8>, origin: GPtr<u8>, dir: GPtr<u8>, force: f32, amount: i32, across: CVector, kind: u32, spray: f32, life: f32) = thiscall 0x49F040 => unported;
    /// `CAudioEngine::ReportCollision(CEntity*, CEntity*, eSurfaceType, eSurfaceType, CVector&, CVector*, float, float, bool, bool)` (0x506EB0) on `AudioEngine` (0xB6BC90)
    fn report_collision(ae: GPtr<u8>, e1: GPtr<u8>, e2: GPtr<u8>, s1: u32, s2: u32, pos: GPtr<u8>, normal: u32, impulse: f32, volume: f32, bullet: u32, b2: u32) = thiscall 0x506EB0 => unported;
    /// `CVehicle::MakeDirty(CColPoint&)` (0x6D2BF0)
    fn make_dirty(v: GPtr<u8>, cp: GPtr<u8>) = thiscall 0x6D2BF0 => crate::vehicle2::make_dirty;
    /// `CVehicle::AddWheelDirtAndWater(CColPoint&, uint32, uint8, uint8)` (0x6D2D50)
    fn add_wheel_dirt_and_water(v: GPtr<u8>, cp: GPtr<u8>, below: u32, band: u32, in_water: u32) -> u32 = thiscall 0x6D2D50 => crate::vehicle2::add_wheel_dirt_and_water;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => sa_world::world::mvpa_math::vector_add_assign;
    /// `CVector::operator-=(const CVector&)` (0x406D70)
    fn vector_sub_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x406D70 => sa_world::world::mvpa_math::vector_sub_assign;
    /// `FxPrtMult_c::FxPrtMult_c(float r, float g, float b, float a, float size, float, float life)` (0x4AB290)
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, gg: f32, b: f32, a: f32, size: f32, rot: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90), ST0
    fn random_in_range(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => random_in_range_native;
    /// `VectorSub(CVector*, const CVector*, const CVector*)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => sa_world::world::mvpa_math::vector_sub;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    fn fx_add_particle(sys: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: f32, mult: GPtr<u8>, rot: f32, bright: u32, z: f32, local: u32) = thiscall 0x4AA440 => unported;
    /// `CSkidmarks::RegisterOne(uint32, const CVector&, float, float, eSkidmarkType, bool*, float)` (0x720930)
    fn skidmarks_register_one(id: u32, pos: GPtr<u8>, dx: u32, dy: u32, kind: u32, blood: GPtr<u8>, length: f32) = cdecl 0x720930 => unported;
    /// `CVehicle::FindWheelWidth(bool rear)` (vtable +0xFC), ST0
    fn v_find_wheel_width(v: GPtr<u8>, rear: u32) -> f64 = thiscall vslot 0xFC => unported;
}

/// Phases B/C side of `CVector::NormaliseAndMag` 0x59C970.
pub fn normalise_and_mag_native(v: GPtr<u8>) -> f64 {
    let mut c = rv(v, 0);
    let m = c.normalise_and_mag_x87();
    v.cast::<f32>().write(c.x);
    v.byte_add(4).cast::<f32>().write(c.y);
    v.byte_add(8).cast::<f32>().write(c.z);
    m.to_f64()
}

/// Phases B/C side of `CGeneral::GetRandomNumberInRange(float, float)` 0x41BD90.
pub fn random_in_range_native(min: f32, max: f32) -> f64 {
    sa_core::general::get_random_number_in_range_f32(min, max).to_f64()
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

/// `FxPrtMult_c` (0x1C bytes: r, g, b, a, size, rotation, life), accessed through its pointer.
fn pm(p: GPtr<u8>, k: isize) -> f32 {
    rd::<f32>(p, 4 * k)
}

fn set_pm(p: GPtr<u8>, k: isize, v: f32) {
    crate::common::wr::<f32>(p, 4 * k, v);
}

const SURFACE_INFOS: usize = 0xB7_9538;

/// `CVehicle::AddSingleWheelParticles(tWheelState state, uint32 status, float speedRatio, float
/// skidRatio, CColPoint* colPoint, CVector* from, int32 id, int32 wheelId, eSkidmarkType skidType,
/// bool* bloodState, uint32 flags)` (0x6DE880, `ret 0x2C`, always returns false).
///
/// Nothing for +0x1C bit 17, or farther than 25 m from the camera unless +0x42D bit 0. Particles
/// are allowed (`Lc5`) within 8 m (or 8..20 m for the player's car), else every second / fourth
/// frame by `(modelIndex low byte + frame)`. Nothing when `speedRatio >= 1`. In water (the surface
/// or the touching-water flag with the water level above `from.z`) no skid mark is drawn. Status 1
/// (sparking rim) on a sparking surface with `skidRatio > 0.1` or a spinning wheel: sparks along
/// `moveSpeed * -50 (+2.5 z)` (`skidRatio * 32` of them; spinning slowly: `forward * gas * -12`,
/// 10) and a collision sound. `MakeDirty`; then per state: SPINNING (1): dirt/water and up to one
/// smoke particle with random spray (`rand() % 10000` at 0x6DF294); SKIDDING (2, not with flag 4)
/// and FIXED (3) above 0.03: dirt/water and `max(1, ftol(|moveSpeed * timeStep| * 2|3))` smoke
/// particles along the travel; NORMAL: dirt/water above 0.03, and the skid mark only for
/// sliding odd wheels of +0x42D-bit-0 cars, a bloody wheel or flag 2. Skid marks (not status 1):
/// at `from` (0x1D7: shifted by `right * ±0.15`), id `this + wheelId`, the wheel width of the
/// virtual `FindWheelWidth(wheelId & 1)`.
#[hook(addr = 0x6de880, conv = thiscall, name = "CVehicle::AddSingleWheelParticles", this = "CVehicle",
       expect = "90 E9 F0 6F D2 FF",
       unverified = "vehbig-1", route = "split", hazards = "uninit-locals",
       recs = "driving,car-ai",
       note = "entry is `nop; jmp 0x405876` (frame setup in a protection fragment); rand site 0x6DF299 plus GetRandomNumberInRange calls; the ReportCollision surface dwords keep the original's float residue above the byte; AddWheelDirtAndWater's in-water dword is zero-extended (stack residue above the byte in the original)")]
#[allow(clippy::too_many_arguments)]
pub fn vehicle_add_single_wheel_particles(
    this: GPtr<u8>,
    state: u32,
    status: u32,
    speed_ratio: f32,
    skid_ratio: f32,
    cp: GPtr<u8>,
    from: GPtr<u8>,
    _id: i32,
    wheel_id: i32,
    skid_type: u32,
    blood: GPtr<u8>,
    flags: u32,
) -> u8 {
    let pv = find_player_vehicle(-1, 0);
    if rd::<u32>(this, 0x1C) & 0x2_0000 != 0 {
        return 0;
    }
    let (cam, pos) = (cam_pos(), pos_ptr(this));
    let (dx, dy, dz) = (
        ld(cam, 0).fsub_m(rd::<f32>(pos, 0)),
        ld(cam, 4).fsub_m(rd::<f32>(pos, 4)),
        ld(cam, 8).fsub_m(rd::<f32>(pos, 8)),
    );
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if !le(d2, c(0x441C_4000)) && rd::<u8>(this, 0x42D) & 1 == 0 {
        return 0;
    }
    let fc = g::<u32>(0xB7_CB4C).read();
    let mlow = u32::from(rd::<u8>(this, 0x22));
    // Lc5: particles allowed this frame
    let particles = if !le(d2, c(0x43C8_0000)) {
        (mlow.wrapping_add(fc) as u8) & 3 == 0
    } else if !le(d2, c(0x4280_0000)) || pv.is_null() {
        (mlow.wrapping_add(fc) as u8) & 1 == 0
    } else {
        true
    };
    if !x(speed_ratio).fcom(c(0x3F80_0000)).test_any(1) {
        return 0;
    }
    let si = GPtr::from_addr(SURFACE_INFOS);
    // Lb0: in water
    let mut in_water = is_water(si, u32::from(rd::<u8>(cp, 0x23))) as u8 != 0;
    if !in_water && rd::<u32>(this, 0x40) & 0x800_0000 != 0 {
        let mut level = 0f32;
        if get_water_level(
            rd::<f32>(from, 0),
            rd::<f32>(from, 4),
            rd::<f32>(from, 8),
            GPtr::from_raw(&mut level),
            1,
            0,
        ) != 0
            && !x(level).fcom(x(rd::<f32>(from, 8))).test_any(1)
        {
            in_water = true;
        }
    }
    let below = !ge(x(skid_ratio), c(0x3F80_0000));
    if status == 1
        && (!le(x(skid_ratio), c(0x3DCC_CCCD)) || state == 1)
        && get_friction_effect(si, u32::from(rd::<u8>(cp, 0x23))) == 1
    {
        sparks(this, state, skid_ratio, cp, from);
    }
    make_dirty(this, cp);
    let ts = g::<f32>(0xB7_CB5C).read();
    let skid_section = |this: GPtr<u8>| {
        if status == 1 {
            return;
        }
        skid_mark(this, from, wheel_id, skid_type, blood, in_water);
    };
    match state {
        1 => {
            if add_wheel_dirt_and_water(this, cp, u32::from(below), 1, u32::from(in_water)) != 0 && particles
            {
                spin_smoke(this, from);
            }
            skid_section(this);
        }
        2 => {
            if flags & 4 != 0 {
                return 0;
            }
            if !le(x(skid_ratio), c(0x3CF5_C28F))
                && add_wheel_dirt_and_water(this, cp, u32::from(below), 0, u32::from(in_water)) != 0
                && particles
            {
                skid_smoke(this, from, ts);
            }
            skid_section(this);
        }
        3 => {
            if !le(x(skid_ratio), c(0x3CF5_C28F))
                && add_wheel_dirt_and_water(this, cp, u32::from(below), 0, u32::from(in_water)) != 0
                && particles
            {
                skid_smoke(this, from, ts);
            }
            skid_section(this);
        }
        _ => {
            if !le(x(skid_ratio), c(0x3CF5_C28F)) {
                add_wheel_dirt_and_water(this, cp, u32::from(below), 0, u32::from(in_water));
            }
            let sliding = rd::<u8>(this, 0x42D) & 1 != 0 && wheel_id & 1 != 0 && {
                let (vx, vy) = (ld(this, 0x44), ld(this, 0x48));
                !le(vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt(), c(0x3D23_D70A))
            };
            if !sliding && blood.read() == 0 && flags & 2 == 0 {
                return 0;
            }
            skid_section(this);
        }
    }
    0
}

fn cam_pos() -> GPtr<u8> {
    let m = g::<GPtr<u8>>(0xB6_F03C).read();
    if m.is_null() {
        GPtr::from_addr(0xB6_F02C)
    } else {
        m.byte_add(0x30)
    }
}

/// 0x6DEA43..0x6DEB99: the sparks of a rim on a sparking surface and the scrape sound.
fn sparks(this: GPtr<u8>, state: u32, skid_ratio: f32, cp: GPtr<u8>, from: GPtr<u8>) {
    let k = f32::from_bits(0xC248_0000);
    let mut d = CVector::new(
        ld(this, 0x44).fmul_m(k).to_f32(),
        ld(this, 0x48).fmul_m(k).to_f32(),
        ld(this, 0x4C).fmul_m(k).fadd_m(2.5).to_f32(),
    );
    // Lc4: the spark amount as a float
    let mut amount = x(skid_ratio).fmul_m(32.0).to_f32();
    if state == 1 && !ge(x(skid_ratio), c(0x3E4C_CCCD)) {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let mut t = CVector::default();
        let tp = vector_mul_scalar(
            GPtr::from_raw(&mut t).cast(),
            m.byte_add(0x10),
            rd::<f32>(this, 0x49C),
        );
        let mut t2 = CVector::default();
        let r = rv(vector_mul_scalar(GPtr::from_raw(&mut t2).cast(), tp, -12.0), 0);
        d = CVector::new(r.x, r.y, x(r.z).fadd_m(2.5).to_f32());
        amount = 10.0;
    }
    let force = st0(normalise_and_mag(GPtr::from_raw(&mut d).cast())).to_f32();
    let ms = rv(this, 0x44);
    add_sparks(
        GPtr::from_addr(0xA9_AE00),
        from,
        GPtr::from_raw(&mut d).cast(),
        force,
        ftol32(x(amount)),
        ms,
        1,
        f32::from_bits(0x3DCC_CCCD),
        f32::from_bits(0x3E99_999A),
    );
    let s2 = (force.to_bits() & 0xFFFF_FF00) | u32::from(rd::<u8>(cp, 0x23));
    let s1 = (amount.to_bits() & 0xFFFF_FF00) | u32::from(rd::<u8>(cp, 0x20));
    let ped = find_player_ped(0);
    report_collision(
        GPtr::from_addr(0xB6_BC90),
        this,
        ped,
        s1,
        s2,
        from,
        0,
        1.0,
        1.0,
        0,
        1,
    );
}

/// 0x6DF1BE..0x6DF303: the spinning wheel's smoke particle.
fn spin_smoke(this: GPtr<u8>, from: GPtr<u8>) {
    let mut m = [0f32; 7];
    let mp = GPtr::from_raw(m.as_mut_ptr()).cast::<u8>();
    let k9 = f32::from_bits(0x3F66_6666);
    fx_prt_mult_ctor(mp, k9, k9, 1.0, 0.5, 1.0, 1.0, 0.5);
    if !le(st0(vector_magnitude(this.byte_add(0x44))), c(0x3E19_999A)) {
        set_pm(mp, 3, f32::from_bits(0x3E99_999A));
        set_pm(mp, 4, 0.5);
    }
    scale_by_class(this, mp);
    let gas = ld(this, 0x49C).fabs().to_f32();
    let k = f32::from_bits(0xC1F0_0000);
    let vx = st0(random_in_range(
        0.0,
        x(gas).fmul_m(rd::<f32>(this, 0x44)).fmul_m(k).to_f32(),
    ))
    .to_f32();
    let vy = st0(random_in_range(
        0.0,
        x(gas).fmul_m(rd::<f32>(this, 0x48)).fmul_m(k).to_f32(),
    ))
    .to_f32();
    let r = rand_at(0x6D_F299) % 10000;
    let vz = xi(r)
        .fmul_m(f32::from_bits(0x38D1_B717))
        .fmul_m(f32::from_bits(0x3F4C_CCCD))
        .to_f32();
    let mut vel = CVector::new(vx, vy, vz);
    let b = st0(random_in_range(0.5, 1.0));
    set_pm(mp, 2, b.to_f32());
    let gg = b.fmul_m(k_09());
    set_pm(mp, 1, gg.to_f32());
    set_pm(mp, 0, gg.to_f32());
    fx_add_particle(
        g::<GPtr<u8>>(0xA9_AE24).read(),
        from,
        GPtr::from_raw(&mut vel).cast(),
        0.0,
        mp,
        -1.0,
        rd::<u32>(this, 0x12C),
        f32::from_bits(0x3F19_999A),
        0,
    );
}

fn k_09() -> f32 {
    f32::from_bits(0x3F66_6666)
}

/// The class scaling of the particle multiplier (+0x594 9/2: size * 0.5; 10: size * 0.2, life *
/// 0.3). Returns true when the class branch was taken (the amount factor becomes 3).
fn scale_by_class(this: GPtr<u8>, m: GPtr<u8>) -> bool {
    match rd::<u32>(this, 0x594) {
        9 | 2 => {
            set_pm(m, 4, x(pm(m, 4)).fmul_m(0.5).to_f32());
            true
        }
        10 => {
            set_pm(m, 4, x(pm(m, 4)).fmul_m(f32::from_bits(0x3E4C_CCCD)).to_f32());
            set_pm(m, 6, x(pm(m, 6)).fmul_m(f32::from_bits(0x3E99_999A)).to_f32());
            true
        }
        _ => false,
    }
}

/// 0x6DED1A..0x6DEE8F (and the SKIDDING copy): smoke along the travel of this frame.
fn skid_smoke(this: GPtr<u8>, from: GPtr<u8>, ts: f32) {
    let mut m = [0f32; 7];
    let mp = GPtr::from_raw(m.as_mut_ptr()).cast::<u8>();
    fx_prt_mult_ctor(
        mp,
        k_09(),
        k_09(),
        1.0,
        0.5,
        1.0,
        f32::from_bits(0x3F33_3333),
        f32::from_bits(0x3E99_999A),
    );
    let mut vel = CVector::new(0.0, 0.0, 0.5);
    let factor = if scale_by_class(this, mp) { 3.0f32 } else { 2.0 };
    let mut travel = CVector::default();
    let tp = GPtr::from_raw(&mut travel).cast::<u8>();
    vector_mul_scalar(tp, this.byte_add(0x44), ts);
    let n = ftol32(st0(vector_magnitude(tp)).fmul_m(factor)).max(1);
    let nf = xi(n).to_f32();
    for i in 0..n {
        let b = st0(random_in_range(0.5, 1.0));
        set_pm(mp, 2, b.to_f32());
        let gg = b.fmul_m(k_09());
        set_pm(mp, 1, gg.to_f32());
        set_pm(mp, 0, gg.to_f32());
        let t = xi(i).fdiv_m(nf).fsubr_m(1.0).to_f32();
        let mut s = CVector::default();
        let sp = vector_mul_scalar(GPtr::from_raw(&mut s).cast(), tp, t);
        let mut p = CVector::default();
        let pp = GPtr::from_raw(&mut p).cast::<u8>();
        vector_sub(pp, from, sp);
        fx_add_particle(
            g::<GPtr<u8>>(0xA9_AE24).read(),
            pp,
            GPtr::from_raw(&mut vel).cast(),
            f32::from_bits(0x3E99_999A),
            mp,
            -1.0,
            rd::<u32>(this, 0x12C),
            f32::from_bits(0x3F19_999A),
            0,
        );
    }
}

/// 0x6DEC3C.. / 0x6DEE95.. / 0x6DF0D5.. / 0x6DF308..: the skid mark at the wheel.
fn skid_mark(this: GPtr<u8>, from: GPtr<u8>, wheel_id: i32, skid_type: u32, blood: GPtr<u8>, in_water: bool) {
    let mut p = rv(from, 0);
    let pp = GPtr::from_raw(&mut p).cast::<u8>();
    if rd::<u16>(this, 0x22) == 0x1D7 {
        let mut o = CVector::default();
        let op = vector_mul_scalar(
            GPtr::from_raw(&mut o).cast(),
            rd::<GPtr<u8>>(this, 0x14),
            f32::from_bits(0x3E19_999A),
        );
        if wheel_id < 2 {
            vector_add_assign(pp, op);
        } else {
            vector_sub_assign(pp, op);
        }
    }
    if in_water {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let width = st0(v_find_wheel_width(this, (wheel_id as u32) & 1)).to_f32();
    skidmarks_register_one(
        (this.addr() as u32).wrapping_add(wheel_id as u32),
        pp,
        rd::<u32>(m, 0x10),
        rd::<u32>(m, 0x14),
        skid_type,
        blood,
        width,
    );
}

pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_ADD_SINGLE_WHEEL_PARTICLES];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
