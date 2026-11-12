//! WRITER-MVP-D (helper VEH, batch `mvpd-veh-2`): `CBike` construction and suspension lines. Ported
//! from the 1.0 US bytes; gta-reversed as a reading aid. Where they differ, the bytes win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, model_info, rand_at, rd, wr, x, xi};

orig! {
    /// `CVehicle::CVehicle(eVehicleCreatedBy)` (0x6D5F10)
    fn vehicle_ctor(v: GPtr<u8>, created_by: u32) -> GPtr<u8> = thiscall 0x6D5F10 => unported;
    /// `CRideAnimData::CRideAnimData()` (0x6D0B10)
    fn ride_anim_data_ctor(r: GPtr<u8>) -> GPtr<u8> = thiscall 0x6D0B10 => crate::bike2::ride_anim_data_constructor;
    /// `CVehicleModelInfo` virtual +0x38 (`GetAnimFileIndex`)
    fn mi_get_anim_file_index(mi: GPtr<u8>) -> i32 = thiscall vslot 0x38 => unported;
    /// `CVehicle::SetModelIndex(uint32)` (0x6D6A40)
    fn vehicle_set_model_index(v: GPtr<u8>, mi: u32) = thiscall 0x6D6A40 => unported;
    /// `CClumpModelInfo::FillFrameArray(RpClump*, RwFrame**)` (0x4C5440)
    fn fill_frame_array(clump: GPtr<u8>, frames: GPtr<u8>) = cdecl 0x4C5440 => unported;
    /// `cHandlingDataMgr::GetFlyingPointer(uint8)` (0x6F52D0)
    fn get_flying_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x6F52D0 => crate::handling::get_flying_pointer;
    /// `CVehicleModelInfo::ChooseVehicleColour(uint8&, uint8&, uint8&, uint8&, int32)` (0x4C8500)
    fn choose_vehicle_colour(mi: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, inc: i32) = thiscall 0x4C8500 => unported;
    /// `CMemoryMgr::Malloc(uint32)` (0x72F420; the caller pushes a second 0 word)
    fn mem_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CBike::SetupSuspensionLines()` (0x6B89B0)
    fn setup_suspension_lines_call(b: GPtr<u8>) = thiscall 0x6B89B0 => bike_setup_suspension_lines;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CAEVehicleAudioEntity::Initialise(CEntity*)` (0x4F7670)
    fn audio_initialise(a: GPtr<u8>, e: GPtr<u8>) = thiscall 0x4F7670 => unported;
    /// `RwMatrixTransform(RwMatrix*, const RwMatrix*, RwOpCombineType)` (0x7F25A0)
    fn rw_matrix_transform(m: GPtr<u8>, t: GPtr<u8>, op: i32) -> GPtr<u8> = cdecl 0x7F25A0 => unported;
}

/// `CBike::CBike(int32 modelIndex, eVehicleCreatedBy)` (0x6BF430, catalogue `CBike::Constructor`, `ret 8`;
/// the SEH prologue through the .HOODLUM fragment 0x405895).
///
/// `CVehicle(createdBy)`, vtable 0x871360, +0x60C/+0x610 = 0, `CRideAnimData` at +0x640. For a model
/// info of type 9 (+0x3C) the ride anim group (+0x640) becomes the anim block word at 0xB5D4BC +
/// 32 * `GetAnimFileIndex()` (vslot 0x38), 2 unless in 2..=6. Type/subtype 9, +0x7BC = 0, word
/// +0x7A8 = 0, byte +0x614 = 0, `SetModelIndex`, the 10 frames at +0x5A0 cleared and filled. Handling
/// (the model's handling id word, low byte): car entry 0xC2B9DC + 0xE0*id (+0x384), bike entry
/// 0xC3491C + 0x40*id (+0x63C), flags (+0x38C = handling +0xD0), flying pointer (+0x388); +0x740 = 20.0
/// (0x871288); colours; +0x798..+0x7A0 = 0; +0x7A4 = `tan(model +0x58 * pi/180)` (FPTAN); bytes
/// +0x7B4/+0x7B5 = 0; mass, turn mass, centre of mass (+0xA4..; +0xAC then overwritten by 0.1);
/// buoyancy +0x98 = `percentSubmerged / 1000 * 0.5` when > 0.01, else copied; +0x9C = 0.05,
/// elasticity +0xA0. Then the per-wheel fields (2 wheels: +0x750/+0x758/+0x80C/+0x744, bytes
/// +0x74C/+0x74E/+0x65C; 4 suspension slots: +0x7D0 vectors, +0x7C0 = 0, +0x710/+0x720 = 1.0, +0x730 =
/// 0), bytes +0x804..+0x806 = 0, +0x790 = 0, +0x794 = 1.0. The col model's line array is allocated
/// (0x80 bytes, 4 lines, line count byte 4) when missing, with line 1 start.z = 99999.99; line 0
/// start.z = 99999.99 always (the "not set up" marker), then `SetupSuspensionLines`. Flags: +0x3DB
/// bit 2 cleared, +0x3BA/+0x3BB = 0, status (+0x36 bits 3..7) = 2, +0x3AC = `CTimer::m_snTimeInMilliseconds`,
/// +0x484 = 0, +0x429 low nibble cleared, +0x5C8 = 0, the lean matrix +0x5CC = the matrix, +0x630..
/// = 0, audio `Initialise`. Returns `this`.
#[hook(addr = 0x6BF430, conv = thiscall, name = "CBike::Constructor", this = "CBike", expect = "6A FF E9 5E 64 D4 FF",
       unverified = "mvpd-veh-2", route = "L2", hazards = "stack-residue,x87-transcendental,heap-writes",
       recs = "driving",
       note = "the original uses its modelIndex slot as the suspension loop counter (ends 0) and stores the handling id into its createdBy slot; FPTAN result stored to f32; writes the shared col model's line array (malloc)")]
pub fn bike_constructor(this: GPtr<u8>, mi: u32, created_by: u32) -> GPtr<u8> {
    vehicle_ctor(this, created_by);
    wr::<u32>(this, 0, 0x0087_1360);
    wr::<u32>(this, 0x60C, 0);
    wr::<u32>(this, 0x610, 0);
    ride_anim_data_ctor(this.byte_add(0x640));
    let mip = model_info(mi as i32);
    let hid = i32::from(rd::<i16>(mip, 0x4A)) as u32;
    if rd::<u32>(mip, 0x3C) == 9 {
        let idx = mi_get_anim_file_index(mip);
        let v = g::<i32>((idx << 5).wrapping_add(0xB5_D4BC) as u32 as usize).read();
        wr::<i32>(this, 0x640, v);
        if !(2..=6).contains(&v) {
            wr::<u32>(this, 0x640, 2);
        }
    }
    wr::<u32>(this, 0x594, 9);
    wr::<u32>(this, 0x590, 9);
    wr::<u32>(this, 0x7BC, 0);
    wr::<u16>(this, 0x7A8, 0);
    wr::<u8>(this, 0x614, 0);
    vehicle_set_model_index(this, mi);
    for k in 0..10 {
        wr::<u32>(this, 0x5A0 + 4 * k, 0);
    }
    fill_frame_array(rd::<GPtr<u8>>(this, 0x18), this.byte_add(0x5A0));
    let id = (hid & 0xFF) as usize;
    let h = GPtr::<u8>::from_addr(0xC2_B9DC + 0xE0 * id);
    wr::<GPtr<u8>>(this, 0x384, h);
    wr::<u32>(this, 0x63C, (0xC3_491C + 0x40 * id) as u32);
    wr::<u32>(this, 0x38C, rd::<u32>(h, 0xD0));
    let fp = get_flying_pointer(GPtr::from_addr(0xC2_B9C8), hid);
    wr::<GPtr<u8>>(this, 0x388, fp);
    wr::<u32>(this, 0x740, g::<u32>(0x87_1288).read());
    choose_vehicle_colour(mip, this.byte_add(0x434), this.byte_add(0x435), this.byte_add(0x436), this.byte_add(0x437), 1);
    wr::<u32>(this, 0x798, 0);
    wr::<u32>(this, 0x79C, 0);
    wr::<u32>(this, 0x7A0, 0);
    let arg = x(rd::<f32>(mip, 0x58)).fmul_m(f32::from_bits(0x3C8E_FA35));
    let h = rd::<GPtr<u8>>(this, 0x384);
    wr::<u8>(this, 0x7B4, 0);
    // FPTAN leaves the operand when |x| >= 2^63 (nothing pushed); `fstp st(0)` then pops it and the
    // store reads an empty register (the indefinite NaN)
    let tan = match arg.fptan() {
        Some((t, _one)) => f32_of(t),
        None => f32::from_bits(0xFFC0_0000),
    };
    wr::<u8>(this, 0x7B5, 0);
    wr::<f32>(this, 0x7A4, tan);
    wr::<u32>(this, 0x8C, rd::<u32>(h, 4));
    wr::<u32>(this, 0x90, rd::<u32>(h, 0xC));
    for k in 0..3 {
        wr::<u32>(this, 0xA4 + 4 * k, rd::<u32>(h, 0x14 + 4 * k));
    }
    wr::<u32>(this, 0xAC, 0x3DCC_CCCD);
    if !x(rd::<f32>(h, 0x10)).fcom(x(0.01)).test_any(0x41) {
        wr::<f32>(this, 0x98, f32_of(x(rd::<f32>(h, 0x10)).fdiv_m(1000.0).fmul_m(0.5)));
    } else {
        wr::<u32>(this, 0x98, rd::<u32>(h, 0x10));
    }
    wr::<u32>(this, 0x9C, 0x3D4C_CCCD);
    wr::<u32>(this, 0xA0, rd::<u32>(h, 0x24));
    for off in [0x494, 0x49C, 0x4A0, 0x800, 0x4CC, 0x808] {
        wr::<u32>(this, off, 0);
    }
    wr::<u32>(this, 0x7AC, 0x3F80_0000);
    wr::<u32>(this, 0x7B0, 0);
    wr::<u32>(this, 0x7B8, 0);
    for k in 0..2 {
        wr::<u32>(this, 0x750 + 4 * k, 0);
        wr::<u32>(this, 0x758 + 4 * k, 0);
        wr::<u32>(this, 0x80C + 4 * k, 0);
        wr::<u32>(this, 0x744 + 4 * k, 0);
        wr::<u8>(this, 0x74C + k, 0);
        wr::<u8>(this, 0x74E + k, 0);
        wr::<u8>(this, 0x65C + k, 0);
    }
    for k in 0..4 {
        for c in 0..3 {
            wr::<u32>(this, 0x7D0 + 12 * k + 4 * c, 0);
        }
        wr::<u32>(this, 0x7C0 + 4 * k, 0);
        wr::<u32>(this, 0x710 + 4 * k, 0x3F80_0000);
        wr::<u32>(this, 0x720 + 4 * k, 0x3F80_0000);
        wr::<u32>(this, 0x730 + 4 * k, 0);
    }
    wr::<u8>(this, 0x804, 0);
    wr::<u8>(this, 0x805, 0);
    wr::<u8>(this, 0x806, 0);
    wr::<u32>(this, 0x790, 0);
    wr::<u32>(this, 0x794, 0x3F80_0000);
    let cd = rd::<GPtr<u8>>(rd::<GPtr<u8>>(mip, 0x14), 0x2C);
    const FAR: u32 = 0x47C3_4FFF;
    if rd::<u32>(cd, 0x10) == 0 {
        wr::<u8>(cd, 6, 4);
        let lines = mem_malloc(0x80, 0);
        wr::<GPtr<u8>>(cd, 0x10, lines);
        wr::<u32>(lines, 0x28, FAR);
    }
    wr::<u32>(rd::<GPtr<u8>>(cd, 0x10), 8, FAR);
    setup_suspension_lines_call(this);
    let f3db = rd::<u8>(this, 0x3DB) & 0xFB;
    wr::<u8>(this, 0x3BA, 0);
    wr::<u8>(this, 0x3BB, 0);
    wr::<u8>(this, 0x3DB, f3db);
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x10);
    let f429 = rd::<u8>(this, 0x429) & 0xF0;
    wr::<u32>(this, 0x3AC, g::<u32>(0xB7_CB84).read());
    wr::<u8>(this, 0x484, 0);
    wr::<u8>(this, 0x429, f429);
    wr::<u8>(this, 0x5C8, 0);
    matrix_assign(this.byte_add(0x5CC), rd::<GPtr<u8>>(this, 0x14));
    wr::<u32>(this, 0x630, 0);
    wr::<u32>(this, 0x634, 0);
    wr::<u32>(this, 0x638, 0);
    audio_initialise(this.byte_add(0x138), this);
    this
}

/// The frame's LTM up to (not including) `stop`: its modelling matrix (+0x10, 16 dwords) transformed
/// (`rwCOMBINEPOSTCONCAT`) by every parent's modelling matrix, the walk ending at a null parent or at
/// `stop`. The 64-byte result is written to `out`.
fn frame_matrix(frame: GPtr<u8>, stop: GPtr<u8>, out: &mut [u32; 16]) {
    for (k, o) in out.iter_mut().enumerate() {
        *o = rd::<u32>(frame, 0x10 + 4 * k as isize);
    }
    let m = GPtr::from_raw(out.as_mut_ptr()).cast::<u8>();
    let mut p = rd::<GPtr<u8>>(frame, 4);
    while !p.is_null() {
        rw_matrix_transform(m, p.byte_add(0x10), 2);
        p = rd::<GPtr<u8>>(p, 4);
        if p == stop {
            break;
        }
    }
}

fn lt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x05)
}

/// `sqrt(x^2 + y^2 + z^2)` of three f32s at `p` as the inlined `CVector::Magnitude` sums them
/// (x*x + y*y, + z*z).
fn magnitude(p: GPtr<u8>) -> X87 {
    let (vx, vy, vz) = (x(rd::<f32>(p, 0)), x(rd::<f32>(p, 4)), x(rd::<f32>(p, 8)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt()
}

/// `CBike::SetupSuspensionLines()` (0x6B89B0).
///
/// Col data of the model's col model; `already = line[0].start.z != 99999.99` and `first =
/// line[1].start.z == 99999.99` (bit compares). For the 4 lines (0/1 front, 2/3 rear): when
/// already set up, the start is the old line start with z = the stored front/rear wheel z (+0x768 /
/// +0x76C). Otherwise the wheel position is the frame (+0x5B0 front, +0x5B4 rear) position in the
/// clump frame's space (parents up to the clump frame), with y shifted by ±0.25 * the wheel scale
/// (model +0x40 front / +0x44 rear; the shift is applied after the z is recorded: line 0 stores
/// +0x768, line 2 stores +0x76C and +0x798 = the y/z distance to the frame +0x5AC, 0 without it).
/// start.z += handling upper limit (+0xB8); end = start with z = `(lower - upper) - scale * 0.5 +
/// start.z`; +0x770[i] = upper - lower; +0x780[i] = start.z - end.z. When not set up before, +0x79C/
/// +0x7A0 = the y/z of frame +0x5A8 (its parent walk compares against the frame itself, so it
/// runs to the top). Then +0x790 = `(1 - 1/(4 * handling +0xAC)) * +0x770[0] + (scale front * 0.5 -
/// line[0].start.z)`, +0x760[k] = scale[k] * 0.5 - +0x790; the bounding box min.z drops to line 0's
/// end.z, the sphere radius grows to the larger box corner magnitude; with handling flag
/// 0x40000000 on the first setup the col spheres reaching below `0.25 - +0x790` are shrunk (radius
/// = max(0.4, centre.z - t) for radius > 0.4) and lifted to `t + radius`.
#[hook(addr = 0x6B89B0, conv = thiscall, name = "CBike::SetupSuspensionLines", this = "CBike", expect = "83 EC 68 53 8B D9",
       unverified = "mvpd-veh-2", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "driving",
       note = "writes the shared col model (lines, bounding box/sphere, spheres); fsqrt results stored")]
pub fn bike_setup_suspension_lines(this: GPtr<u8>) {
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    let colm = rd::<GPtr<u8>>(mi, 0x14);
    let cd = rd::<GPtr<u8>>(colm, 0x2C);
    let lines0 = rd::<GPtr<u8>>(cd, 0x10);
    const FAR: u32 = 0x47C3_4FFF;
    let first = rd::<u32>(lines0, 0x28) == FAR;
    let already = rd::<u32>(lines0, 8) != FAR;
    let mut mat = [0u32; 16];
    for i in 0..4isize {
        let (px, mut py, mut pz);
        if already {
            let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
            px = rd::<u32>(line, 0);
            py = rd::<u32>(line, 4);
            pz = rd::<u32>(this, if i < 2 { 0x768 } else { 0x76C });
        } else {
            let (o, frame) = match i {
                0 => (x(rd::<f32>(mi, 0x40)).fmul_m(0.25), rd::<GPtr<u8>>(this, 0x5B0)),
                1 => (x(rd::<f32>(mi, 0x40)).fmul_m(-0.25), rd::<GPtr<u8>>(this, 0x5B0)),
                2 => (x(rd::<f32>(mi, 0x44)).fmul_m(0.25), rd::<GPtr<u8>>(this, 0x5B4)),
                _ => (x(rd::<f32>(mi, 0x44)).fmul_m(-0.25), rd::<GPtr<u8>>(this, 0x5B4)),
            };
            let off = f32_of(o);
            let root = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x18), 4);
            frame_matrix(frame, root, &mut mat);
            px = mat[12];
            py = mat[13];
            pz = mat[14];
            if i == 0 {
                wr::<u32>(this, 0x768, pz);
            } else if i == 2 {
                wr::<u32>(this, 0x76C, pz);
                let f = rd::<GPtr<u8>>(this, 0x5AC);
                if !f.is_null() {
                    frame_matrix(f, root, &mut mat);
                    let dy = x(f32::from_bits(py)).fsub_m(f32::from_bits(mat[13]));
                    let dz = x(f32::from_bits(pz)).fsub_m(f32::from_bits(mat[14]));
                    wr::<f32>(this, 0x798, f32_of(dz.fmul(dz).fadd(dy.fmul(dy)).fsqrt()));
                } else {
                    wr::<u32>(this, 0x798, 0);
                }
            }
            py = f32_of(x(f32::from_bits(py)).fadd_m(off)).to_bits();
        }
        let h = rd::<GPtr<u8>>(this, 0x384);
        pz = f32_of(x(f32::from_bits(pz)).fadd_m(rd::<f32>(h, 0xB8))).to_bits();
        let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
        wr::<u32>(line, 0, px);
        wr::<u32>(line, 4, py);
        wr::<u32>(line, 8, pz);
        let scale = x(rd::<f32>(mi, if i < 2 { 0x40 } else { 0x44 }));
        let h = rd::<GPtr<u8>>(this, 0x384);
        let d = x(rd::<f32>(h, 0xBC)).fsub_m(rd::<f32>(h, 0xB8));
        let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
        wr::<u32>(line, 0x10, px);
        wr::<u32>(line, 0x14, py);
        pz = f32_of(d.fsub(scale.fmul_m(0.5)).fadd_m(f32::from_bits(pz))).to_bits();
        wr::<u32>(line, 0x18, pz);
        let h = rd::<GPtr<u8>>(this, 0x384);
        wr::<f32>(this, 0x770 + 4 * i, f32_of(x(rd::<f32>(h, 0xB8)).fsub_m(rd::<f32>(h, 0xBC))));
        let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
        wr::<f32>(this, 0x780 + 4 * i, f32_of(x(rd::<f32>(line, 8)).fsub_m(rd::<f32>(line, 0x18))));
    }
    if !already {
        let f = rd::<GPtr<u8>>(this, 0x5A8);
        frame_matrix(f, f, &mut mat);
        wr::<u32>(this, 0x79C, mat[13]);
        wr::<u32>(this, 0x7A0, mat[14]);
    }
    let h = rd::<GPtr<u8>>(this, 0x384);
    let lines = rd::<GPtr<u8>>(cd, 0x10);
    let a = x(rd::<f32>(h, 0xAC)).fmul_m(4.0).fdivr_m(1.0).fsubr_m(1.0).fmul_m(rd::<f32>(this, 0x770));
    let b = x(rd::<f32>(mi, 0x40)).fmul_m(0.5).fsub_m(rd::<f32>(lines, 8));
    wr::<f32>(this, 0x790, f32_of(a.fadd(b)));
    for k in 0..2isize {
        let s = x(rd::<f32>(mi, if k == 0 { 0x40 } else { 0x44 }));
        wr::<f32>(this, 0x760 + 4 * k, f32_of(s.fmul_m(0.5).fsub_m(rd::<f32>(this, 0x790))));
    }
    let lines = rd::<GPtr<u8>>(cd, 0x10);
    if lt(x(rd::<f32>(lines, 0x18)), x(rd::<f32>(colm, 8))) {
        wr::<u32>(colm, 8, rd::<u32>(lines, 0x18));
    }
    let r1 = magnitude(colm);
    let r2 = magnitude(colm.byte_add(0xC));
    let r = if !r1.fcom(r2).test_any(0x41) { r1 } else { magnitude(colm.byte_add(0xC)) };
    if !r.fcom(x(rd::<f32>(colm, 0x24))).test_any(0x41) {
        wr::<f32>(colm, 0x24, f32_of(r));
    }
    let h = rd::<GPtr<u8>>(this, 0x384);
    if rd::<u32>(h, 0xCC) & 0x4000_0000 == 0 || !first {
        return;
    }
    let t = f32_of(x(0.25).fsub_m(rd::<f32>(this, 0x790)));
    let mut j = 0i32;
    while j < i32::from(rd::<i16>(cd, 0)) {
        let s = rd::<GPtr<u8>>(cd, 8).byte_add(0x14 * j as isize);
        if lt(x(rd::<f32>(s, 8)).fsub_m(rd::<f32>(s, 0xC)), x(t)) {
            if !x(rd::<f32>(s, 0xC)).fcom(x(0.4)).test_any(0x41) {
                let r = x(rd::<f32>(s, 8)).fsub_m(t);
                let r = if x(0.4).fcom(r).test_any(0x41) { r } else { x(0.4) };
                wr::<f32>(s, 0xC, f32_of(r));
            }
            let s = rd::<GPtr<u8>>(cd, 8).byte_add(0x14 * j as isize);
            wr::<f32>(s, 8, f32_of(x(t).fadd_m(rd::<f32>(s, 0xC))));
        }
        j += 1;
    }
}

orig! {
    /// `CDamageManager::GetEngineStatus()` (0x6C22C0)
    fn dm_get_engine_status(dm: GPtr<u8>) -> u32 = thiscall 0x6C22C0 => unported;
    /// `CAutomobile::IsInAir()` (0x6A6140)
    fn automobile_is_in_air(a: GPtr<u8>) -> u8 = thiscall 0x6A6140 => unported;
    /// `CVehicle::BlowUpCar(CEntity*, bool)` (virtual, vtable +0xA4)
    fn blow_up_car_virtual(v: GPtr<u8>, damager: GPtr<u8>, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float x7)` (0x4AB290), the floats as bits
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, size: u32, unk: u32, life: u32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90, ST0)
    fn random_float(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `FxSystem_c::AddParticle(CVector*, CVector*, float, FxPrtMult_c*, float, float, float, int32)` (0x4AA440)
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, rot: u32, bright: u32, cam: u32, local: u32) = thiscall 0x4AA440 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_i32(min: i32, max: i32) -> i32 = cdecl 0x407180 => sa_core::general::get_random_number_in_range_i32;
    /// `CExplosion::AddExplosion(CEntity*, CPed*, eExplosionType, CVector, uint32, bool, float, bool)` (0x736A50)
    fn add_explosion(victim: GPtr<u8>, creator: GPtr<u8>, ty: u32, x: u32, y: u32, z: u32, lifetime: u32, make_sound: u32, cam_shake: u32, visibility: u32) = cdecl 0x736A50 => unported;
    /// `FxManager_c::CreateFxSystem(char*, CVector*, RwMatrix*, bool)` (0x4A9BE0)
    fn fx_create_fx_system(mgr: GPtr<u8>, name: u32, pos: GPtr<u8>, m: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `FxSystem_c::SetVelAdd(CVector*)` (0x4AA730)
    fn fx_set_vel_add(fx: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4AA730 => unported;
    /// `CEventVehicleOnFire::CEventVehicleOnFire(CVehicle*)` (0x4B10C0)
    fn event_vehicle_on_fire_ctor(ev: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B10C0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventVehicleOnFire::~CEventVehicleOnFire()` (0x4B1170)
    fn event_vehicle_on_fire_dtor(ev: GPtr<u8>) = thiscall 0x4B1170 => unported;
    /// `CVehicle::ProcessDelayedExplosion()` (0x6D1340)
    fn process_delayed_explosion(v: GPtr<u8>) = thiscall 0x6D1340 => unported;
}

/// `ftol(ms_fTimeStep * 0.02 * 1000)` reloaded with `fild` as unsigned (+2^32 when negative).
fn step_ms() -> X87 {
    let t = crate::common::ftol32(x(g::<f32>(0xB7_CB5C).read()).fmul_m(0.02).fmul_m(1000.0));
    let v = crate::common::xi(t);
    if t < 0 {
        v.fadd_m(4_294_967_296.0)
    } else {
        v
    }
}

fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The RC models 0x1D0, 0x1D1, 0x1F5, 0x1B9, 0x234.
fn is_rc(this: GPtr<u8>) -> bool {
    matches!(rd::<u16>(this, 0x22), 0x1D0 | 0x1D1 | 0x1F5 | 0x1B9 | 0x234)
}

/// `CAutomobile::ProcessCarOnFireAndExplode(bool explodeImmediately)` (0x6A7090, `ret 4`).
///
/// With health (+0x4C0) < 250, not wrecked and not drowning (+0x42B bit 6): `kind` = 1 ("fire_car")
/// when there is no fire fx (+0x57C) and the subtype (+0x594) is not plane (3) or heli (4). Cars:
/// burn timer +0x8E4 += `ftol(timeStep * 0.02 * 1000)` (as unsigned), `BlowUpCar(+0x928, false)`
/// (vslot 0xA4) above 5000. Planes/helis: +0x981 = `IsInAir() ? 2 : 1` when 0; `explode` blows up
/// directly; else the burn timer grows by the step (times 0.2 unless +0x981 == 1 or an RC model),
/// above 5000 it blows up, else every even frame (0xB7CB4C) a SmokeHuge (0xA9AE20) particle at the
/// position plus a random offset (5 `GetRandomNumberInRange<float>`: vel x, y, z, then pos x, y), with
/// the colour multiplier (0,0,0,0.4,1,1,0.3) / RC (0.15/0.3/0.3, remote 0.5/0.1/0.1 at +0xC/+0x10/
/// +0x18); then a player/remote vehicle with no fire fx and burn > 2500 creates "fire_large" (kind 2),
/// other non-RC ones explode with a 3/250 chance (`AddExplosion(this, +0x928, rocket, pos, 0, true,
/// -1, false)`). A kind 1/2 creates the fx at the engine dummy (model info +0x5C, +0x54) on the RW
/// frame's modelling matrix, plays it and adds a `CEventVehicleOnFire` to the global group; an
/// existing fire fx gets `SetVelAdd(moveSpeed * 50)`. Otherwise the burn timer is 0 and the fx is
/// killed. Finally an engine status > 225 with health > 250 loses 2 health, and
/// `ProcessDelayedExplosion`.
#[hook(addr = 0x6A7090, conv = thiscall, name = "CAutomobile::ProcessCarOnFireAndExplode", this = "CAutomobile", expect = "6A FF 68 B8 7F 84 00",
       route = "L2", hazards = "stack-residue",
       recs = "driving",
       note = "the original stores the ftol step into its own explodeImmediately slot (the plane path reads the flag before that store)")]
pub fn automobile_process_car_on_fire_and_explode(this: GPtr<u8>, explode: u32) {
    let eng = dm_get_engine_status(this.byte_add(0x5A0)) as u8;
    let burning = !x(rd::<f32>(this, 0x4C0)).fcom(x(250.0)).test_parity(0x05)
        && rd::<u8>(this, 0x36) & 0xF8 != 0x28
        && rd::<u8>(this, 0x42B) & 0x40 == 0;
    if burning {
        let sub = rd::<u32>(this, 0x594);
        let mut kind = u32::from(rd::<u32>(this, 0x57C) == 0 && sub != 4 && sub != 3);
        let mut player_check = false;
        if sub != 4 && sub != 3 {
            let burn = step_ms().fadd_m(rd::<f32>(this, 0x8E4));
            wr::<f32>(this, 0x8E4, f32_of(burn));
            if !burn.fcom(x(5000.0)).test_any(0x41) {
                blow_up_car_virtual(this, rd::<GPtr<u8>>(this, 0x928), 0);
            }
        } else {
            if rd::<u8>(this, 0x981) == 0 {
                let air = automobile_is_in_air(this);
                wr::<u8>(this, 0x981, if air != 0 { 2 } else { 1 });
            }
            if explode as u8 != 0 {
                blow_up_car_virtual(this, rd::<GPtr<u8>>(this, 0x928), 0);
            } else {
                let rc = is_rc(this);
                let step = if rd::<u8>(this, 0x981) == 1 || rc { step_ms() } else { step_ms().fmul_m(f32::from_bits(0x3E4C_CCCD)) };
                wr::<f32>(this, 0x8E4, f32_of(step.fadd_m(rd::<f32>(this, 0x8E4))));
                if !x(rd::<f32>(this, 0x8E4)).fcom(x(5000.0)).test_any(0x41) {
                    blow_up_car_virtual(this, rd::<GPtr<u8>>(this, 0x928), 0);
                } else {
                    let mut mult = [0u32; 8];
                    let mp = GPtr::from_raw(mult.as_mut_ptr()).cast::<u8>();
                    fx_prt_mult_ctor(mp, 0, 0, 0, 0x3ECC_CCCD, 0x3F80_0000, 0x3F80_0000, 0x3E99_999A);
                    if rc {
                        if rd::<u8>(this, 0x36) & 0xF8 == 0x40 {
                            wr::<u32>(mp, 0xC, 0x3F00_0000);
                            wr::<u32>(mp, 0x10, 0x3DCC_CCCD);
                            wr::<u32>(mp, 0x18, 0x3DCC_CCCD);
                        } else {
                            wr::<u32>(mp, 0xC, 0x3E19_999A);
                            wr::<u32>(mp, 0x10, 0x3E99_999A);
                            wr::<u32>(mp, 0x18, 0x3E99_999A);
                        }
                    }
                    if g::<u8>(0xB7_CB4C).read() & 1 == 0 {
                        let p = entity_pos(this);
                        let mut pos = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
                        let (v, z, o) = if rc { (0.5f32, 0.4f32, 0.7f32) } else { (1.5, 1.0, 2.0) };
                        let vx = f32_of(X87::from_f64_exact(random_float(-v, v)));
                        let vy = f32_of(X87::from_f64_exact(random_float(-v, v)));
                        let vz = f32_of(X87::from_f64_exact(random_float(0.0, z)));
                        let mut vel = [vx, vy, vz];
                        pos[0] = f32_of(X87::from_f64_exact(random_float(-o, o)).fadd_m(pos[0]));
                        pos[1] = f32_of(X87::from_f64_exact(random_float(-o, o)).fadd_m(pos[1]));
                        fx_add_particle(
                            g::<GPtr<u8>>(0xA9_AE20).read(),
                            GPtr::from_raw(pos.as_mut_ptr()).cast(),
                            GPtr::from_raw(vel.as_mut_ptr()).cast(),
                            0,
                            mp,
                            0xBF80_0000,
                            0x3F99_999A,
                            0x3F19_999A,
                            0,
                        );
                    }
                }
                let st = rd::<u8>(this, 0x36) >> 3;
                if st == 0 || st == 8 {
                    if rd::<u32>(this, 0x57C) == 0 && !x(rd::<f32>(this, 0x8E4)).fcom(x(2500.0)).test_any(0x41) {
                        kind = 2;
                    }
                } else {
                    player_check = !is_rc(this) && random_i32(0, 250) < 3;
                }
            }
        }
        if player_check {
            let p = entity_pos(this);
            add_explosion(this, rd::<GPtr<u8>>(this, 0x928), 2, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), 0, 1, 0xBF80_0000, 0);
        }
        if kind != 0 {
            let dummies = rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(this, 0x22))), 0x5C);
            let mut pos = [rd::<u32>(dummies, 0x54), rd::<u32>(dummies, 0x58), rd::<u32>(dummies, 0x5C)];
            let rw = rd::<GPtr<u8>>(this, 0x18);
            if !rw.is_null() {
                let mat = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
                if !mat.is_null() {
                    let name = match kind {
                        1 => Some(0x0085_AA68),
                        2 => Some(0x0085_AA74),
                        _ => None,
                    };
                    if let Some(name) = name {
                        let fx = fx_create_fx_system(GPtr::from_addr(0xA9_AE80), name, GPtr::from_raw(pos.as_mut_ptr()).cast(), mat, 0);
                        wr::<GPtr<u8>>(this, 0x57C, fx);
                    }
                }
            }
            let fx = rd::<GPtr<u8>>(this, 0x57C);
            if !fx.is_null() {
                fx_play(fx);
                let mut ev = [0u32; 8];
                let evp = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
                event_vehicle_on_fire_ctor(evp, this);
                event_group_add(get_event_global_group(), evp, 0);
                event_vehicle_on_fire_dtor(evp);
            }
        }
        let fx = rd::<GPtr<u8>>(this, 0x57C);
        if !fx.is_null() {
            let k = 50.0f32;
            let mut v = [
                f32_of(x(rd::<f32>(this, 0x44)).fmul_m(k)),
                f32_of(x(rd::<f32>(this, 0x48)).fmul_m(k)),
                f32_of(x(rd::<f32>(this, 0x4C)).fmul_m(k)),
            ];
            fx_set_vel_add(fx, GPtr::from_raw(v.as_mut_ptr()).cast());
        }
    } else {
        wr::<u32>(this, 0x8E4, 0);
        let fx = rd::<GPtr<u8>>(this, 0x57C);
        if !fx.is_null() {
            fx_kill(fx);
            wr::<u32>(this, 0x57C, 0);
        }
    }
    if eng > 0xE1 && !x(rd::<f32>(this, 0x4C0)).fcom(x(250.0)).test_any(0x41) {
        wr::<f32>(this, 0x4C0, f32_of(x(rd::<f32>(this, 0x4C0)).fsub_m(2.0)));
    }
    process_delayed_explosion(this);
}

orig! {
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CBike::CalculateLeanMatrix()` (0x6B7150)
    fn calculate_lean_matrix(b: GPtr<u8>) = thiscall 0x6B7150 => crate::bike2::calculate_lean_matrix;
    /// `operator!=(const CVector&, const CVector&)` (0x509760)
    fn vector_ne(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x509760 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, hidden result pointer)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CWaterLevel::GetWaterLevel(float, float, float, float*, bool, CVector*)` (0x6EB690), the floats as bits
    fn get_water_level(x: u32, y: u32, z: u32, level: GPtr<u8>, waves: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
}

/// `rand()` static guard / value of `AddExhaustParticles` (0xC1CDB8 bit 0 / 0xC1CDB4).
const EXHAUST_GUARD: usize = 0xC1_CDB8;
const EXHAUST_BACK: usize = 0xC1_CDB4;

/// `max(v, 0)` as `fld 0; fcomp st(1); test ah, 0x41; jne`: v is kept unless `0 > v`.
fn clamp0(v: X87) -> f32 {
    if x(0.0).fcom(v).test_any(0x41) {
        f32_of(v)
    } else {
        0.0
    }
}

/// `CVehicle::AddExhaustParticles()` (0x6DE240).
///
/// Nothing for an entity flagged +0x1C bit 17 (off-screen/not visible), beyond 256 squared distance
/// from the camera (`TheCamera` matrix 0xB6F03C / placement 0xB6F02C), or beyond 64 on odd
/// `frameCounter + (uint8) model` frames. The exhaust dummy (model info +0x5C, +0x48) gives the left
/// position and its x-mirror; a copy of the matrix (the bike lean matrix +0x5CC for bikes) with the
/// double-exhaust handling flag (bit 13 of +0xCC): FCR-900 (0x209) with +0x438 1 or 2 and BF-400
/// (0x245) with 2 are double; NRG-500 (0x20A) with 0/1 moves the second exhaust to the secondary
/// dummy (+0x84). A zero dummy position stops. Velocity: below 0.05 forward speed `fwd * r` with a
/// once-drawn `r = rand(-1.8, -0.9)` (static), else `moveSpeed * 30` (x, y; z 0). The world
/// positions; touching water (+0x40 bit 27) with the water level not below an exhaust marks it
/// as in water. One `rand`: emit only when `(2 * rand/32767 + 1) * (gas + 1.1) > 2.5`. Colour
/// multiplier (0.9, 0.9, 1.0, a1, 0.2, 1.0, a2) with `a1 = max(0.25 - |v|/2, 0)`, `a2 = max(0.2 -
/// |v|/2, 0)`. Twice: a particle per exhaust (in water on 0xA9AE08 with alpha `a1 * 0.5` and size
/// 0.6 written into the multiplier, which stays changed; else on 0xA9AE24), and with gas > 0.5 and
/// gear byte +0x4B4 < 3 one more on the left (odd `rand`) or right exhaust.
#[hook(addr = 0x6DE240, conv = thiscall, name = "CVehicle::AddExhaustParticles", this = "CVehicle", expect = "64 A1 00 00 00 00",
       unverified = "mvpd-veh-2", route = "L2", hazards = "none",
       recs = "driving",
       note = "two rand sites (0x6DE572, 0x6DE744); the colour multiplier is changed in place by in-water particles")]
pub fn vehicle_add_exhaust_particles(this: GPtr<u8>) {
    if rd::<u32>(this, 0x1C) & 0x2_0000 != 0 {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let p = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
    let cm = g::<GPtr<u8>>(0xB6_F03C).read();
    let c = if cm.is_null() { GPtr::<u8>::from_addr(0xB6_F02C) } else { cm.byte_add(0x30) };
    let dz = x(rd::<f32>(c, 8)).fsub_m(rd::<f32>(p, 8));
    let dy = x(rd::<f32>(c, 4)).fsub_m(rd::<f32>(p, 4));
    let dx = x(rd::<f32>(c, 0)).fsub_m(rd::<f32>(p, 0));
    let d2 = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz));
    if !d2.fcom(x(256.0)).test_any(0x41) {
        return;
    }
    if !d2.fcom(x(64.0)).test_any(0x41) {
        let fc = g::<u32>(0xB7_CB4C).read();
        if (u32::from(rd::<u8>(this, 0x22)).wrapping_add(fc)) & 1 == 0 {
            return;
        }
    }
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    let dummies = rd::<GPtr<u8>>(mi, 0x5C);
    let mut pos_l = [rd::<u32>(dummies, 0x48), rd::<u32>(dummies, 0x4C), rd::<u32>(dummies, 0x50)];
    let mut pos_r = [f32_of(x(f32::from_bits(pos_l[0])).fmul_m(-1.0)).to_bits(), pos_l[1], pos_l[2]];
    let mut mat = [0u32; 0x12];
    let mp = GPtr::from_raw(mat.as_mut_ptr()).cast::<u8>();
    matrix_copy_ctor(mp, m);
    let mut double = (rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) >> 13) & 1 != 0;
    if rd::<u32>(this, 0x594) == 9 {
        calculate_lean_matrix(this);
        matrix_assign(mp, this.byte_add(0x5CC));
        let extra = rd::<u8>(this, 0x438);
        match rd::<u16>(this, 0x22) {
            0x209 => double |= extra == 1 || extra == 2,
            0x20A => {
                if extra == 0 || extra == 1 {
                    let d = rd::<GPtr<u8>>(mi, 0x5C);
                    pos_r = [rd::<u32>(d, 0x84), rd::<u32>(d, 0x88), rd::<u32>(d, 0x8C)];
                }
            }
            0x245 => double |= extra == 2,
            _ => {}
        }
    }
    let mut world_r = [0u32; 3];
    let pl = GPtr::from_raw(pos_l.as_mut_ptr()).cast::<u8>();
    if vector_ne(pl, GPtr::from_raw(world_r.as_mut_ptr()).cast()) == 0 {
        matrix_dtor(mp);
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let fwd_speed = x(rd::<f32>(m, 0x18))
        .fmul_m(rd::<f32>(this, 0x4C))
        .fadd(x(rd::<f32>(m, 0x14)).fmul_m(rd::<f32>(this, 0x48)))
        .fadd(x(rd::<f32>(m, 0x10)).fmul_m(rd::<f32>(this, 0x44)));
    let mut vel = [0f32; 3];
    if !fwd_speed.fcom(x(0.05)).test_parity(0x05) {
        if g::<u32>(EXHAUST_GUARD).read() & 1 == 0 {
            g::<u32>(EXHAUST_GUARD).write(g::<u32>(EXHAUST_GUARD).read() | 1);
            let r = random_float(f32::from_bits(0xBFE6_6666), f32::from_bits(0xBF66_6666));
            g::<f32>(EXHAUST_BACK).write(f32_of(X87::from_f64_exact(r)));
        }
        let m = rd::<GPtr<u8>>(this, 0x14);
        vel[0] = f32_of(x(g::<f32>(EXHAUST_BACK).read()).fmul_m(rd::<f32>(m, 0x10)));
        vel[1] = f32_of(x(g::<f32>(EXHAUST_BACK).read()).fmul_m(rd::<f32>(m, 0x14)));
    } else {
        vel[0] = f32_of(x(rd::<f32>(this, 0x44)).fmul_m(30.0));
        vel[1] = f32_of(x(rd::<f32>(this, 0x48)).fmul_m(30.0));
    }
    let mut world_l = [0u32; 3];
    matrix_mul_vec(GPtr::from_raw(world_l.as_mut_ptr()).cast(), mp, pl);
    let mut level = 0f32;
    let lp = GPtr::from_raw(&mut level).cast::<u8>();
    let mut water_l = false;
    let mut water_r = false;
    if rd::<u32>(this, 0x40) & 0x800_0000 != 0
        && get_water_level(world_l[0], world_l[1], world_l[2], lp, 1, 0) != 0
        && !x(level).fcom(x(f32::from_bits(world_l[2]))).c0()
    {
        water_l = true;
    }
    if double {
        let mut tmp = [0u32; 3];
        let r = matrix_mul_vec(GPtr::from_raw(tmp.as_mut_ptr()).cast(), mp, GPtr::from_raw(pos_r.as_mut_ptr()).cast());
        world_r = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
        if rd::<u32>(this, 0x40) & 0x800_0000 != 0
            && get_water_level(world_r[0], world_r[1], world_r[2], lp, 1, 0) != 0
            && !x(level).fcom(x(f32::from_bits(world_r[2]))).c0()
        {
            water_r = true;
        }
    }
    let gas = rd::<f32>(this, 0x49C);
    let r = xi(rand_at(0x6D_E572)).fmul_m(f32::from_bits(0x3800_0100));
    let prod = r.fadd(r).fadd_m(1.0).fmul(x(gas).fadd_m(1.1));
    if prod.fcom(x(g::<f32>(0x8D_3680).read())).test_any(0x41) {
        matrix_dtor(mp);
        return;
    }
    let s = X87::from_f64_exact(vector_magnitude(this.byte_add(0x44))).fmul_m(0.5);
    let a1 = clamp0(x(0.25).fsub(s));
    let a2 = clamp0(s.fsubr_m(0.2));
    let mut mult = [0u32; 8];
    let mlt = GPtr::from_raw(mult.as_mut_ptr()).cast::<u8>();
    fx_prt_mult_ctor(mlt, 0x3F66_6666, 0x3F66_6666, 0x3F80_0000, a1.to_bits(), 0x3E4C_CCCD, 0x3F80_0000, a2.to_bits());
    let wl = GPtr::from_raw(world_l.as_mut_ptr()).cast::<u8>();
    let wrp = GPtr::from_raw(world_r.as_mut_ptr()).cast::<u8>();
    let vp = GPtr::from_raw(vel.as_mut_ptr()).cast::<u8>();
    let emit = |pos: GPtr<u8>, in_water: bool| {
        let bright = rd::<u32>(this, 0x12C);
        if in_water {
            wr::<f32>(mlt, 0xC, f32_of(x(a1).fmul_m(0.5)));
            wr::<u32>(mlt, 0x10, 0x3F19_999A);
            fx_add_particle(g::<GPtr<u8>>(0xA9_AE08).read(), pos, vp, 0, mlt, 0xBF80_0000, bright, 0x3F19_999A, 0);
        } else {
            fx_add_particle(g::<GPtr<u8>>(0xA9_AE24).read(), pos, vp, 0, mlt, 0xBF80_0000, bright, 0x3F19_999A, 0);
        }
    };
    for _ in 0..2 {
        emit(wl, water_l);
        if double {
            emit(wrp, water_r);
        }
        if !x(rd::<f32>(this, 0x49C)).fcom(x(g::<f32>(0x8D_367C).read())).test_any(0x41) && rd::<u8>(this, 0x4B4) < 3 {
            if rand_at(0x6D_E744) & 1 != 0 {
                emit(wl, water_l);
            } else if double {
                emit(wrp, water_r);
            }
        }
    }
    matrix_dtor(mp);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_BIKE_CONSTRUCTOR,
    HOOK_BIKE_SETUP_SUSPENSION_LINES,
    HOOK_AUTOMOBILE_PROCESS_CAR_ON_FIRE_AND_EXPLODE,
    HOOK_VEHICLE_ADD_EXHAUST_PARTICLES,
];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
