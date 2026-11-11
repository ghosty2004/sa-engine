//! PORT-RENDER-2: the regular `FxInfo*_c` / `FxInterpInfo*_c` bodies (the interp `Load` wrappers, the
//! destructors and scalar deleting destructors, the flag-only `GetValue`s). Generated from the 1.0 US bytes by
//! matching each body against one of these exact instruction shapes; irregular bodies are not here.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::wr;
use crate::renderer::quiet;

sa_core::orig! {
    /// `FxInterpInfo*_c::Load(FILESTREAM)` (vtable +4)
    pub fn interp_load(this: GPtr<u8>, file: u32) = thiscall vslot 0x4 => unported;
    /// CRT `free` (0x8214BD)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
}

sa_core::orig! {
    /// The interp `GetVal(float* out, float time)` at 0x4A8470; the time as a raw dword
    pub fn getval_4a8470(this: GPtr<u8>, out: GPtr<f32>, time: u32) = thiscall 0x4A8470 => unported;
    /// The interp `GetVal(float* out, float time)` at 0x4A8800; the time as a raw dword
    pub fn getval_4a8800(this: GPtr<u8>, out: GPtr<f32>, time: u32) = thiscall 0x4A8800 => unported;
    /// The interp `GetVal(float* out, float time)` at 0x4A89C0; the time as a raw dword
    pub fn getval_4a89c0(this: GPtr<u8>, out: GPtr<f32>, time: u32) = thiscall 0x4A89C0 => unported;
    /// The destructor at 0x4A6A90 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6a90(this: GPtr<u8>) = thiscall 0x4A6A90 => unported;
    /// The destructor at 0x4A6B00 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6b00(this: GPtr<u8>) = thiscall 0x4A6B00 => unported;
    /// The destructor at 0x4A6B70 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6b70(this: GPtr<u8>) = thiscall 0x4A6B70 => unported;
    /// The destructor at 0x4A6BE0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6be0(this: GPtr<u8>) = thiscall 0x4A6BE0 => unported;
    /// The destructor at 0x4A6C50 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6c50(this: GPtr<u8>) = thiscall 0x4A6C50 => unported;
    /// The destructor at 0x4A6CC0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6cc0(this: GPtr<u8>) = thiscall 0x4A6CC0 => unported;
    /// The destructor at 0x4A6D30 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6d30(this: GPtr<u8>) = thiscall 0x4A6D30 => unported;
    /// The destructor at 0x4A6DA0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6da0(this: GPtr<u8>) = thiscall 0x4A6DA0 => unported;
    /// The destructor at 0x4A6E10 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6e10(this: GPtr<u8>) = thiscall 0x4A6E10 => unported;
    /// The destructor at 0x4A6E80 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6e80(this: GPtr<u8>) = thiscall 0x4A6E80 => unported;
    /// The destructor at 0x4A6EF0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6ef0(this: GPtr<u8>) = thiscall 0x4A6EF0 => unported;
    /// The destructor at 0x4A6F60 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6f60(this: GPtr<u8>) = thiscall 0x4A6F60 => unported;
    /// The destructor at 0x4A6FD0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a6fd0(this: GPtr<u8>) = thiscall 0x4A6FD0 => unported;
    /// The destructor at 0x4A7040 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7040(this: GPtr<u8>) = thiscall 0x4A7040 => unported;
    /// The destructor at 0x4A70B0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a70b0(this: GPtr<u8>) = thiscall 0x4A70B0 => unported;
    /// The destructor at 0x4A73B0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a73b0(this: GPtr<u8>) = thiscall 0x4A73B0 => unported;
    /// The destructor at 0x4A7420 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7420(this: GPtr<u8>) = thiscall 0x4A7420 => unported;
    /// The destructor at 0x4A7490 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7490(this: GPtr<u8>) = thiscall 0x4A7490 => unported;
    /// The destructor at 0x4A7500 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7500(this: GPtr<u8>) = thiscall 0x4A7500 => unported;
    /// The destructor at 0x4A7570 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7570(this: GPtr<u8>) = thiscall 0x4A7570 => unported;
    /// The destructor at 0x4A75E0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a75e0(this: GPtr<u8>) = thiscall 0x4A75E0 => unported;
    /// The destructor at 0x4A7650 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7650(this: GPtr<u8>) = thiscall 0x4A7650 => unported;
    /// The destructor at 0x4A76C0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a76c0(this: GPtr<u8>) = thiscall 0x4A76C0 => unported;
    /// The destructor at 0x4A7730 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7730(this: GPtr<u8>) = thiscall 0x4A7730 => unported;
    /// The destructor at 0x4A77A0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a77a0(this: GPtr<u8>) = thiscall 0x4A77A0 => unported;
    /// The destructor at 0x4A7810 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7810(this: GPtr<u8>) = thiscall 0x4A7810 => unported;
    /// The destructor at 0x4A7880 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7880(this: GPtr<u8>) = thiscall 0x4A7880 => unported;
    /// The destructor at 0x4A78F0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a78f0(this: GPtr<u8>) = thiscall 0x4A78F0 => unported;
    /// The destructor at 0x4A7960 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7960(this: GPtr<u8>) = thiscall 0x4A7960 => unported;
    /// The destructor at 0x4A79D0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a79d0(this: GPtr<u8>) = thiscall 0x4A79D0 => unported;
    /// The destructor at 0x4A7A40 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7a40(this: GPtr<u8>) = thiscall 0x4A7A40 => unported;
    /// The destructor at 0x4A7AB0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a7ab0(this: GPtr<u8>) = thiscall 0x4A7AB0 => unported;
    /// The destructor at 0x4A8460 (called at its address, so its hook toggle decides)
    pub fn dtor_4a8460(this: GPtr<u8>) = thiscall 0x4A8460 => unported;
    /// The destructor at 0x4A87F0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a87f0(this: GPtr<u8>) = thiscall 0x4A87F0 => unported;
    /// The destructor at 0x4A89B0 (called at its address, so its hook toggle decides)
    pub fn dtor_4a89b0(this: GPtr<u8>) = thiscall 0x4A89B0 => unported;
    /// The destructor at 0x4A8B70 (called at its address, so its hook toggle decides)
    pub fn dtor_4a8b70(this: GPtr<u8>) = thiscall 0x4A8B70 => unported;
}

/// `FxInfoEmRate_c::Load(FILESTREAM file, int32 version)` (0x4A4B40): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A4B40, conv = thiscall, name = "FxInfoEmRate_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a4b40(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmSize_c::Load(FILESTREAM file, int32 version)` (0x4A4C90): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A4C90, conv = thiscall, name = "FxInfoEmSize_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a4c90(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmSize_c::GetValue(float time, float, float, float, bool, void* info)` (0x4A4CB0): the interp member's
/// `GetVal(out, time)` (0x4A89C0) into a stack buffer; `out[0]` through `fld` / `fstp` (a signalling NaN is
/// quieted) to `info+0x4`, out[1] to +0x8, out[2] to +0x14, out[3] to +0xc, out[4] to +0x18, out[5] to +0x10, out[6] to +0x1c as dwords. EAX = `info`.
#[hook(addr = 0x4A4CB0, conv = thiscall, name = "FxInfoEmSize_c::GetValue", expect = "8B 44 24 04 83 EC 40")]
pub fn fx_4a4cb0(
    this: GPtr<u8>,
    time: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    let mut o = [0u32; 16];
    getval_4a89c0(
        this.byte_add(8),
        GPtr::<u32>::from_raw(o.as_mut_ptr()).cast(),
        time,
    );
    wr::<f32>(info, 0x4, quiet(f32::from_bits(o[0])));
    wr::<u32>(info, 0x8, o[1]);
    wr::<u32>(info, 0x14, o[2]);
    wr::<u32>(info, 0xc, o[3]);
    wr::<u32>(info, 0x18, o[4]);
    wr::<u32>(info, 0x10, o[5]);
    wr::<u32>(info, 0x1c, o[6]);
    info
}

/// `FxInfoEmSpeed_c::Load(FILESTREAM file, int32 version)` (0x4A4D70): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A4D70, conv = thiscall, name = "FxInfoEmSpeed_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a4d70(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmSpeed_c::GetValue(float time, float, float, float, bool, void* info)` (0x4A4D90): the interp member's
/// `GetVal(out, time)` (0x4A89C0) into a stack buffer; `out[0]` through `fld` / `fstp` (a signalling NaN is
/// quieted) to `info+0x20`, out[1] to +0x24 as dwords. EAX = `info`.
#[hook(addr = 0x4A4D90, conv = thiscall, name = "FxInfoEmSpeed_c::GetValue", expect = "8B 44 24 04 83 EC 40")]
pub fn fx_4a4d90(
    this: GPtr<u8>,
    time: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    let mut o = [0u32; 16];
    getval_4a89c0(
        this.byte_add(8),
        GPtr::<u32>::from_raw(o.as_mut_ptr()).cast(),
        time,
    );
    wr::<f32>(info, 0x20, quiet(f32::from_bits(o[0])));
    wr::<u32>(info, 0x24, o[1]);
    info
}

/// `FxInfoEmDir_c::Load(FILESTREAM file, int32 version)` (0x4A4E30): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A4E30, conv = thiscall, name = "FxInfoEmDir_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a4e30(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmDir_c::GetValue(float time, float, float, float, bool, void* info)` (0x4A4E50): the interp member's
/// `GetVal(out, time)` (0x4A89C0) into a stack buffer; `out[0]` through `fld` / `fstp` (a signalling NaN is
/// quieted) to `info+0x28`, out[1] to +0x2c, out[2] to +0x30 as dwords. EAX = `info`.
#[hook(addr = 0x4A4E50, conv = thiscall, name = "FxInfoEmDir_c::GetValue", expect = "8B 44 24 04 83 EC 40")]
pub fn fx_4a4e50(
    this: GPtr<u8>,
    time: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    let mut o = [0u32; 16];
    getval_4a89c0(
        this.byte_add(8),
        GPtr::<u32>::from_raw(o.as_mut_ptr()).cast(),
        time,
    );
    wr::<f32>(info, 0x28, quiet(f32::from_bits(o[0])));
    wr::<u32>(info, 0x2c, o[1]);
    wr::<u32>(info, 0x30, o[2]);
    info
}

/// `FxInfoEmAngle_c::Load(FILESTREAM file, int32 version)` (0x4A4F00): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A4F00, conv = thiscall, name = "FxInfoEmAngle_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a4f00(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmAngle_c::GetValue(float time, float, float, float, bool, void* info)` (0x4A4F20): the interp member's
/// `GetVal(out, time)` (0x4A8470) into a stack buffer; `out[0]` through `fld` / `fstp` (a signalling NaN is
/// quieted) to `info+0x34`, out[1] to +0x38 as dwords. EAX = `info`.
#[hook(addr = 0x4A4F20, conv = thiscall, name = "FxInfoEmAngle_c::GetValue", expect = "8B 44 24 04 83 EC 40")]
pub fn fx_4a4f20(
    this: GPtr<u8>,
    time: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    let mut o = [0u32; 16];
    getval_4a8470(
        this.byte_add(8),
        GPtr::<u32>::from_raw(o.as_mut_ptr()).cast(),
        time,
    );
    wr::<f32>(info, 0x34, quiet(f32::from_bits(o[0])));
    wr::<u32>(info, 0x38, o[1]);
    info
}

/// `FxInfoEmLife_c::Load(FILESTREAM file, int32 version)` (0x4A4FC0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A4FC0, conv = thiscall, name = "FxInfoEmLife_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a4fc0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmLife_c::GetValue(float time, float, float, float, bool, void* info)` (0x4A4FE0): the interp member's
/// `GetVal(out, time)` (0x4A8800) into a stack buffer; `out[0]` through `fld` / `fstp` (a signalling NaN is
/// quieted) to `info+0x3c`, out[1] to +0x40 as dwords. EAX = `info`.
#[hook(addr = 0x4A4FE0, conv = thiscall, name = "FxInfoEmLife_c::GetValue", expect = "8B 44 24 04 83 EC 40")]
pub fn fx_4a4fe0(
    this: GPtr<u8>,
    time: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    let mut o = [0u32; 16];
    getval_4a8800(
        this.byte_add(8),
        GPtr::<u32>::from_raw(o.as_mut_ptr()).cast(),
        time,
    );
    wr::<f32>(info, 0x3c, quiet(f32::from_bits(o[0])));
    wr::<u32>(info, 0x40, o[1]);
    info
}

/// `FxInfoEmPos_c::Load(FILESTREAM file, int32 version)` (0x4A5080): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5080, conv = thiscall, name = "FxInfoEmPos_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5080(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmWeather_c::Load(FILESTREAM file, int32 version)` (0x4A5150): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5150, conv = thiscall, name = "FxInfoEmWeather_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5150(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmWeather_c::GetValue(float time, float, float, float, bool, void* info)` (0x4A5170): the interp member's
/// `GetVal(out, time)` (0x4A8470) into a stack buffer; `out[0]` through `fld` / `fstp` (a signalling NaN is
/// quieted) to `info+0x58`, out[1] to +0x5c, out[2] to +0x60, out[3] to +0x64 as dwords. EAX = `info`.
#[hook(addr = 0x4A5170, conv = thiscall, name = "FxInfoEmWeather_c::GetValue", expect = "8B 44 24 04 83 EC 40")]
pub fn fx_4a5170(
    this: GPtr<u8>,
    time: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    let mut o = [0u32; 16];
    getval_4a8470(
        this.byte_add(8),
        GPtr::<u32>::from_raw(o.as_mut_ptr()).cast(),
        time,
    );
    wr::<f32>(info, 0x58, quiet(f32::from_bits(o[0])));
    wr::<u32>(info, 0x5c, o[1]);
    wr::<u32>(info, 0x60, o[2]);
    wr::<u32>(info, 0x64, o[3]);
    info
}

/// `FxInfoEmRotation_c::Load(FILESTREAM file, int32 version)` (0x4A5220): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5220, conv = thiscall, name = "FxInfoEmRotation_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5220(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmRotation_c::GetValue(float time, float, float, float, bool, void* info)` (0x4A5240): the interp member's
/// `GetVal(out, time)` (0x4A8470) into a stack buffer; `out[0]` through `fld` / `fstp` (a signalling NaN is
/// quieted) to `info+0x50`, out[1] to +0x54 as dwords. EAX = `info`.
#[hook(addr = 0x4A5240, conv = thiscall, name = "FxInfoEmRotation_c::GetValue", expect = "8B 44 24 04 83 EC 40")]
pub fn fx_4a5240(
    this: GPtr<u8>,
    time: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    let mut o = [0u32; 16];
    getval_4a8470(
        this.byte_add(8),
        GPtr::<u32>::from_raw(o.as_mut_ptr()).cast(),
        time,
    );
    wr::<f32>(info, 0x50, quiet(f32::from_bits(o[0])));
    wr::<u32>(info, 0x54, o[1]);
    info
}

/// `FxInfoNoise_c::Load(FILESTREAM file, int32 version)` (0x4A52E0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A52E0, conv = thiscall, name = "FxInfoNoise_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a52e0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoForce_c::Load(FILESTREAM file, int32 version)` (0x4A54C0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A54C0, conv = thiscall, name = "FxInfoForce_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a54c0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoFriction_c::Load(FILESTREAM file, int32 version)` (0x4A55C0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A55C0, conv = thiscall, name = "FxInfoFriction_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a55c0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoAttractPt_c::Load(FILESTREAM file, int32 version)` (0x4A56C0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A56C0, conv = thiscall, name = "FxInfoAttractPt_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a56c0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoGroundCollide_c::Load(FILESTREAM file, int32 version)` (0x4A59F0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A59F0, conv = thiscall, name = "FxInfoGroundCollide_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a59f0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoWind_c::Load(FILESTREAM file, int32 version)` (0x4A5A80): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5A80, conv = thiscall, name = "FxInfoWind_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5a80(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoJitter_c::Load(FILESTREAM file, int32 version)` (0x4A5B90): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5B90, conv = thiscall, name = "FxInfoJitter_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5b90(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoRotSpeed_c::Load(FILESTREAM file, int32 version)` (0x4A5D20): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5D20, conv = thiscall, name = "FxInfoRotSpeed_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5d20(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoFloat_c::Load(FILESTREAM file, int32 version)` (0x4A5DF0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5DF0, conv = thiscall, name = "FxInfoFloat_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5df0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoFloat_c::GetValue(float, float, float, float, bool, void* info)` (0x4A5E10): `info+0x28 = 1`.
#[hook(addr = 0x4A5E10, conv = thiscall, name = "FxInfoFloat_c::GetValue", expect = "8B 44 24 18 C6 40 28 01")]
pub fn fx_4a5e10(
    _this: GPtr<u8>,
    _t: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    wr::<u8>(info, 0x28, 1);
    info
}

/// `FxInfoUnderwater_c::Load(FILESTREAM file, int32 version)` (0x4A5E70): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5E70, conv = thiscall, name = "FxInfoUnderwater_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5e70(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoUnderwater_c::GetValue(float, float, float, float, bool, void* info)` (0x4A5E90): `info+0x29 = 1`.
#[hook(addr = 0x4A5E90, conv = thiscall, name = "FxInfoUnderwater_c::GetValue", expect = "8B 44 24 18 C6 40 29 01")]
pub fn fx_4a5e90(
    _this: GPtr<u8>,
    _t: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    wr::<u8>(info, 0x29, 1);
    info
}

/// `FxInfoColour_c::Load(FILESTREAM file, int32 version)` (0x4A5F10): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5F10, conv = thiscall, name = "FxInfoColour_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5f10(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoSpriteRect_c::Load(FILESTREAM file, int32 version)` (0x4A6120): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A6120, conv = thiscall, name = "FxInfoSpriteRect_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a6120(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoHeatHaze_c::Load(FILESTREAM file, int32 version)` (0x4A61F0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A61F0, conv = thiscall, name = "FxInfoHeatHaze_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a61f0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoHeatHaze_c::GetValue(float, float, float, float, bool, void* info)` (0x4A6210): `info+0x36 = 1`.
#[hook(addr = 0x4A6210, conv = thiscall, name = "FxInfoHeatHaze_c::GetValue", expect = "8B 44 24 18 C6 40 36 01")]
pub fn fx_4a6210(
    _this: GPtr<u8>,
    _t: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    wr::<u8>(info, 0x36, 1);
    info
}

/// `FxInfoTrail_c::Load(FILESTREAM file, int32 version)` (0x4A6290): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A6290, conv = thiscall, name = "FxInfoTrail_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a6290(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoFlat_c::Load(FILESTREAM file, int32 version)` (0x4A6390): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A6390, conv = thiscall, name = "FxInfoFlat_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a6390(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoDir_c::Load(FILESTREAM file, int32 version)` (0x4A64B0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A64B0, conv = thiscall, name = "FxInfoDir_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a64b0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoAnimTexture_c::Load(FILESTREAM file, int32 version)` (0x4A65C0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A65C0, conv = thiscall, name = "FxInfoAnimTexture_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a65c0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoSelfLit_c::Load(FILESTREAM, int32)` (0x4A67B0): `ret 8` (nothing to load). Not hooked: the entry is 3 bytes.
pub fn fx_4a67b0(_this: GPtr<u8>, _file: u32, _version: u32) {}

/// `FxInfoSelfLit_c::GetValue(float, float, float, float, bool, void* info)` (0x4A67C0): `info+0x37 = 1`.
#[hook(addr = 0x4A67C0, conv = thiscall, name = "FxInfoSelfLit_c::GetValue", expect = "8B 44 24 18 C6 40 37 01")]
pub fn fx_4a67c0(
    _this: GPtr<u8>,
    _t: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    wr::<u8>(info, 0x37, 1);
    info
}

/// `FxInfoColourBright_c::Load(FILESTREAM file, int32 version)` (0x4A6840): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A6840, conv = thiscall, name = "FxInfoColourBright_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a6840(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmPos_c::GetValue(float time, float, float, float, bool, void* info)` (0x4A50A0): the interp member's
/// `GetVal(out, time)` (0x4A8470) into a stack buffer; `out[0]` through `fld` / `fstp` (a signalling NaN is
/// quieted) to `info+0x44`, out[1] to +0x48, out[2] to +0x4c as dwords. EAX = `info`.
#[hook(addr = 0x4A50A0, conv = thiscall, name = "FxInfoEmPos_c::GetValue", expect = "8B 44 24 04 83 EC 40")]
pub fn fx_4a50a0(
    this: GPtr<u8>,
    time: u32,
    _m: u32,
    _tt: u32,
    _len: u32,
    _c: u32,
    info: GPtr<u8>,
) -> GPtr<u8> {
    let mut o = [0u32; 16];
    getval_4a8470(
        this.byte_add(8),
        GPtr::<u32>::from_raw(o.as_mut_ptr()).cast(),
        time,
    );
    wr::<f32>(info, 0x44, quiet(f32::from_bits(o[0])));
    wr::<u32>(info, 0x48, o[1]);
    wr::<u32>(info, 0x4c, o[2]);
    info
}

/// `FxInfoAttractLine_c::Load(FILESTREAM file, int32 version)` (0x4A5830): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A5830, conv = thiscall, name = "FxInfoAttractLine_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a5830(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoColourRange_c::Load(FILESTREAM file, int32 version)` (0x4A66A0): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A66A0, conv = thiscall, name = "FxInfoColourRange_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a66a0(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoSmoke_c::Load(FILESTREAM file, int32 version)` (0x4A6950): the interp member's `Load(file)` (vtable +4); the
/// version is not read. Void: EAX is whatever the interp `Load` leaves (the original enters it with the vtable in EAX).
#[hook(addr = 0x4A6950, conv = thiscall, name = "FxInfoSmoke_c::Load", expect = "8B 54 24 04 8B 41 08")]
pub fn fx_4a6950(this: GPtr<u8>, file: u32, _version: u32) {
    interp_load(this.byte_add(8), file);
}

/// `FxInfoEmRate_c::~FxInfoEmRate_c(uint8 flags)` (0x4A6A70, scalar deleting destructor): the destructor 0x4A6A90, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6A70, conv = thiscall, name = "FxInfoEmRate_c::~FxInfoEmRate_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6a70(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6a90(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoEmRate_c::~FxInfoEmRate_c()` (0x4A6A90): the member at +0x8 destroyed (0x4a8460), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6A90, conv = thiscall, name = "FxInfoEmRate_c::~FxInfoEmRate_c", expect = "6A FF 68 B8 92 83 00")]
pub fn fx_4a6a90(this: GPtr<u8>) {
    dtor_4a8460(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoEmSize_c::~FxInfoEmSize_c(uint8 flags)` (0x4A6AE0, scalar deleting destructor): the destructor 0x4A6B00, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6AE0, conv = thiscall, name = "FxInfoEmSize_c::~FxInfoEmSize_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6ae0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6b00(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FUN_004a6b00()` (0x4A6B00): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6B00, conv = thiscall, name = "FUN_004a6b00", expect = "6A FF 68 D8 92 83 00")]
pub fn fx_4a6b00(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoEmSpeed_c::~FxInfoEmSpeed_c(uint8 flags)` (0x4A6B50, scalar deleting destructor): the destructor 0x4A6B70, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6B50, conv = thiscall, name = "FxInfoEmSpeed_c::~FxInfoEmSpeed_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6b50(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6b70(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoEmSpeed_c::~FxInfoEmSpeed_c()` (0x4A6B70): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6B70, conv = thiscall, name = "FxInfoEmSpeed_c::~FxInfoEmSpeed_c", expect = "6A FF 68 F8 92 83 00")]
pub fn fx_4a6b70(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoEmDir_c::~FxInfoEmDir_c(uint8 flags)` (0x4A6BC0, scalar deleting destructor): the destructor 0x4A6BE0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6BC0, conv = thiscall, name = "FxInfoEmDir_c::~FxInfoEmDir_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6bc0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6be0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoEmDir_c::~FxInfoEmDir_c()` (0x4A6BE0): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6BE0, conv = thiscall, name = "FxInfoEmDir_c::~FxInfoEmDir_c", expect = "6A FF 68 18 93 83 00")]
pub fn fx_4a6be0(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoEmAngle_c::~FxInfoEmAngle_c(uint8 flags)` (0x4A6C30, scalar deleting destructor): the destructor 0x4A6C50, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6C30, conv = thiscall, name = "FxInfoEmAngle_c::~FxInfoEmAngle_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6c30(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6c50(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoEmAngle_c::~FxInfoEmAngle_c()` (0x4A6C50): the member at +0x8 destroyed (0x4a8460), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6C50, conv = thiscall, name = "FxInfoEmAngle_c::~FxInfoEmAngle_c", expect = "6A FF 68 38 93 83 00")]
pub fn fx_4a6c50(this: GPtr<u8>) {
    dtor_4a8460(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoEmLife_c::~FxInfoEmLife_c(uint8 flags)` (0x4A6CA0, scalar deleting destructor): the destructor 0x4A6CC0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6CA0, conv = thiscall, name = "FxInfoEmLife_c::~FxInfoEmLife_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6ca0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6cc0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoEmLife_c::~FxInfoEmLife_c()` (0x4A6CC0): the member at +0x8 destroyed (0x4a87f0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6CC0, conv = thiscall, name = "FxInfoEmLife_c::~FxInfoEmLife_c", expect = "6A FF 68 58 93 83 00")]
pub fn fx_4a6cc0(this: GPtr<u8>) {
    dtor_4a87f0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoEmPos_c::~FxInfoEmPos_c(uint8 flags)` (0x4A6D10, scalar deleting destructor): the destructor 0x4A6D30, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6D10, conv = thiscall, name = "FxInfoEmPos_c::~FxInfoEmPos_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6d10(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6d30(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoEmPos_c::~FxInfoEmPos_c()` (0x4A6D30): the member at +0x8 destroyed (0x4a8460), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6D30, conv = thiscall, name = "FxInfoEmPos_c::~FxInfoEmPos_c", expect = "6A FF 68 78 93 83 00")]
pub fn fx_4a6d30(this: GPtr<u8>) {
    dtor_4a8460(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoEmWeather_c::~FxInfoEmWeather_c(uint8 flags)` (0x4A6D80, scalar deleting destructor): the destructor 0x4A6DA0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6D80, conv = thiscall, name = "FxInfoEmWeather_c::~FxInfoEmWeather_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6d80(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6da0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoEmRotation_c::~FxInfoEmRotation_c(uint8 flags)` (0x4A6DF0, scalar deleting destructor): the destructor 0x4A6E10, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6DF0, conv = thiscall, name = "FxInfoEmRotation_c::~FxInfoEmRotation_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6df0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6e10(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoNoise_c::~FxInfoNoise_c(uint8 flags)` (0x4A6E60, scalar deleting destructor): the destructor 0x4A6E80, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6E60, conv = thiscall, name = "FxInfoNoise_c::~FxInfoNoise_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6e60(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6e80(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoForce_c::~FxInfoForce_c(uint8 flags)` (0x4A6ED0, scalar deleting destructor): the destructor 0x4A6EF0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6ED0, conv = thiscall, name = "FxInfoForce_c::~FxInfoForce_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6ed0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6ef0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoForce_c::~FxInfoForce_c()` (0x4A6EF0): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6EF0, conv = thiscall, name = "FxInfoForce_c::~FxInfoForce_c", expect = "6A FF 68 F8 93 83 00")]
pub fn fx_4a6ef0(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoFriction_c::~FxInfoFriction_c(uint8 flags)` (0x4A6F40, scalar deleting destructor): the destructor 0x4A6F60, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6F40, conv = thiscall, name = "FxInfoFriction_c::~FxInfoFriction_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6f40(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6f60(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoFriction_c::~FxInfoFriction_c()` (0x4A6F60): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6F60, conv = thiscall, name = "FxInfoFriction_c::~FxInfoFriction_c", expect = "6A FF 68 18 94 83 00")]
pub fn fx_4a6f60(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoAttractPt_c::~FxInfoAttractPt_c(uint8 flags)` (0x4A6FB0, scalar deleting destructor): the destructor 0x4A6FD0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A6FB0, conv = thiscall, name = "FxInfoAttractPt_c::~FxInfoAttractPt_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a6fb0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a6fd0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoAttractPt_c::~FxInfoAttractPt_c()` (0x4A6FD0): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A6FD0, conv = thiscall, name = "FxInfoAttractPt_c::~FxInfoAttractPt_c", expect = "6A FF 68 38 94 83 00")]
pub fn fx_4a6fd0(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoAttractLine_c::~FxInfoAttractLine_c(uint8 flags)` (0x4A7020, scalar deleting destructor): the destructor 0x4A7040, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7020, conv = thiscall, name = "FxInfoAttractLine_c::~FxInfoAttractLine_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7020(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7040(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoAttractLine_c::~FxInfoAttractLine_c()` (0x4A7040): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A7040, conv = thiscall, name = "FxInfoAttractLine_c::~FxInfoAttractLine_c", expect = "6A FF 68 58 94 83 00")]
pub fn fx_4a7040(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoGroundCollide_c::~FxInfoGroundCollide_c(uint8 flags)` (0x4A7090, scalar deleting destructor): the destructor 0x4A70B0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7090, conv = thiscall, name = "FxInfoGroundCollide_c::~FxInfoGroundCollide_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7090(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a70b0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoWind_c::~FxInfoWind_c(uint8 flags)` (0x4A7390, scalar deleting destructor): the destructor 0x4A73B0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7390, conv = thiscall, name = "FxInfoWind_c::~FxInfoWind_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7390(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a73b0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoJitter_c::~FxInfoJitter_c(uint8 flags)` (0x4A7400, scalar deleting destructor): the destructor 0x4A7420, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7400, conv = thiscall, name = "FxInfoJitter_c::~FxInfoJitter_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7400(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7420(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoRotSpeed_c::~FxInfoRotSpeed_c(uint8 flags)` (0x4A7470, scalar deleting destructor): the destructor 0x4A7490, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7470, conv = thiscall, name = "FxInfoRotSpeed_c::~FxInfoRotSpeed_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7470(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7490(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoFloat_c::~FxInfoFloat_c(uint8 flags)` (0x4A74E0, scalar deleting destructor): the destructor 0x4A7500, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A74E0, conv = thiscall, name = "FxInfoFloat_c::~FxInfoFloat_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a74e0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7500(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoFloat_c::~FxInfoFloat_c()` (0x4A7500): the member at +0x8 destroyed (0x4a8460), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A7500, conv = thiscall, name = "FxInfoFloat_c::~FxInfoFloat_c", expect = "6A FF 68 F8 94 83 00")]
pub fn fx_4a7500(this: GPtr<u8>) {
    dtor_4a8460(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoUnderwater_c::~FxInfoUnderwater_c(uint8 flags)` (0x4A7550, scalar deleting destructor): the destructor 0x4A7570, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7550, conv = thiscall, name = "FxInfoUnderwater_c::~FxInfoUnderwater_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7550(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7570(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoUnderwater_c::~FxInfoUnderwater_c()` (0x4A7570): the member at +0x8 destroyed (0x4a87f0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A7570, conv = thiscall, name = "FxInfoUnderwater_c::~FxInfoUnderwater_c", expect = "6A FF 68 18 95 83 00")]
pub fn fx_4a7570(this: GPtr<u8>) {
    dtor_4a87f0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoColour_c::~FxInfoColour_c(uint8 flags)` (0x4A75C0, scalar deleting destructor): the destructor 0x4A75E0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A75C0, conv = thiscall, name = "FxInfoColour_c::~FxInfoColour_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a75c0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a75e0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoColour_c::~FxInfoColour_c()` (0x4A75E0): the member at +0x8 destroyed (0x4a87f0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A75E0, conv = thiscall, name = "FxInfoColour_c::~FxInfoColour_c", expect = "6A FF 68 38 95 83 00")]
pub fn fx_4a75e0(this: GPtr<u8>) {
    dtor_4a87f0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoSize_c::~FxInfoSize_c(uint8 flags)` (0x4A7630, scalar deleting destructor): the destructor 0x4A7650, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7630, conv = thiscall, name = "FxInfoSize_c::~FxInfoSize_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7630(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7650(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoSize_c::~FxInfoSize_c()` (0x4A7650): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A7650, conv = thiscall, name = "FxInfoSize_c::~FxInfoSize_c", expect = "6A FF 68 58 95 83 00")]
pub fn fx_4a7650(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoSpriteRect_c::~FxInfoSpriteRect_c(uint8 flags)` (0x4A76A0, scalar deleting destructor): the destructor 0x4A76C0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A76A0, conv = thiscall, name = "FxInfoSpriteRect_c::~FxInfoSpriteRect_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a76a0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a76c0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoHeatHaze_c::~FxInfoHeatHaze_c(uint8 flags)` (0x4A7710, scalar deleting destructor): the destructor 0x4A7730, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7710, conv = thiscall, name = "FxInfoHeatHaze_c::~FxInfoHeatHaze_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7710(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7730(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoTrail_c::~FxInfoTrail_c(uint8 flags)` (0x4A7780, scalar deleting destructor): the destructor 0x4A77A0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7780, conv = thiscall, name = "FxInfoTrail_c::~FxInfoTrail_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7780(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a77a0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoTrail_c::~FxInfoTrail_c()` (0x4A77A0): the member at +0x8 destroyed (0x4a87f0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A77A0, conv = thiscall, name = "FxInfoTrail_c::~FxInfoTrail_c", expect = "6A FF 68 B8 95 83 00")]
pub fn fx_4a77a0(this: GPtr<u8>) {
    dtor_4a87f0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoFlat_c::~FxInfoFlat_c(uint8 flags)` (0x4A77F0, scalar deleting destructor): the destructor 0x4A7810, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A77F0, conv = thiscall, name = "FxInfoFlat_c::~FxInfoFlat_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a77f0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7810(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoDir_c::~FxInfoDir_c(uint8 flags)` (0x4A7860, scalar deleting destructor): the destructor 0x4A7880, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7860, conv = thiscall, name = "FxInfoDir_c::~FxInfoDir_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7860(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7880(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoDir_c::~FxInfoDir_c()` (0x4A7880): the member at +0x8 destroyed (0x4a8460), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A7880, conv = thiscall, name = "FxInfoDir_c::~FxInfoDir_c", expect = "6A FF 68 F8 95 83 00")]
pub fn fx_4a7880(this: GPtr<u8>) {
    dtor_4a8460(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoAnimTexture_c::~FxInfoAnimTexture_c(uint8 flags)` (0x4A78D0, scalar deleting destructor): the destructor 0x4A78F0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A78D0, conv = thiscall, name = "FxInfoAnimTexture_c::~FxInfoAnimTexture_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a78d0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a78f0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoAnimTexture_c::~FxInfoAnimTexture_c()` (0x4A78F0): the member at +0x8 destroyed (0x4a89b0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A78F0, conv = thiscall, name = "FxInfoAnimTexture_c::~FxInfoAnimTexture_c", expect = "6A FF 68 18 96 83 00")]
pub fn fx_4a78f0(this: GPtr<u8>) {
    dtor_4a89b0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoColourRange_c::~FxInfoColourRange_c(uint8 flags)` (0x4A7940, scalar deleting destructor): the destructor 0x4A7960, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7940, conv = thiscall, name = "FxInfoColourRange_c::~FxInfoColourRange_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7940(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7960(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoColourRange_c::~FxInfoColourRange_c()` (0x4A7960): the member at +0x8 destroyed (0x4a87f0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A7960, conv = thiscall, name = "FxInfoColourRange_c::~FxInfoColourRange_c", expect = "6A FF 68 38 96 83 00")]
pub fn fx_4a7960(this: GPtr<u8>) {
    dtor_4a87f0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoSelfLit_c::~FxInfoSelfLit_c(uint8 flags)` (0x4A79B0, scalar deleting destructor): the destructor 0x4A79D0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A79B0, conv = thiscall, name = "FxInfoSelfLit_c::~FxInfoSelfLit_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a79b0(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a79d0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoSelfLit_c::~FxInfoSelfLit_c()` (0x4A79D0): the member at +0x8 destroyed (0x4a87f0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A79D0, conv = thiscall, name = "FxInfoSelfLit_c::~FxInfoSelfLit_c", expect = "6A FF 68 58 96 83 00")]
pub fn fx_4a79d0(this: GPtr<u8>) {
    dtor_4a87f0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoColourBright_c::~FxInfoColourBright_c(uint8 flags)` (0x4A7A20, scalar deleting destructor): the destructor 0x4A7A40, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7A20, conv = thiscall, name = "FxInfoColourBright_c::~FxInfoColourBright_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7a20(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7a40(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoColourBright_c::~FxInfoColourBright_c()` (0x4A7A40): the member at +0x8 destroyed (0x4a87f0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A7A40, conv = thiscall, name = "FxInfoColourBright_c::~FxInfoColourBright_c", expect = "6A FF 68 78 96 83 00")]
pub fn fx_4a7a40(this: GPtr<u8>) {
    dtor_4a87f0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInfoSmoke_c::~FxInfoSmoke_c(uint8 flags)` (0x4A7A90, scalar deleting destructor): the destructor 0x4A7AB0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A7A90, conv = thiscall, name = "FxInfoSmoke_c::~FxInfoSmoke_c", expect = "56 8B F1 E8 18 00 00 00")]
pub fn fx_4a7a90(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a7ab0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInfoSmoke_c::~FxInfoSmoke_c()` (0x4A7AB0): the member at +0x8 destroyed (0x4a87f0), then the base vtable 0x85a7dc. The SEH frame
/// (`push -1; push handler`) is not reproduced (the frame is torn down before returning; fs:0 ends unchanged).
#[hook(addr = 0x4A7AB0, conv = thiscall, name = "FxInfoSmoke_c::~FxInfoSmoke_c", expect = "6A FF 68 98 96 83 00")]
pub fn fx_4a7ab0(this: GPtr<u8>) {
    dtor_4a87f0(this.byte_add(0x8));
    wr::<u32>(this, 0, 0x85a7dc);
}

/// `FxInterpInfoFloat_c::~FxInterpInfoFloat_c()` (0x4A8460): the base vtable 0x85a968.
#[hook(addr = 0x4A8460, conv = thiscall, name = "FxInterpInfoFloat_c::~FxInterpInfoFloat_c", expect = "C7 01 68 A9 85 00")]
pub fn fx_4a8460(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85a968);
}

/// `FxInterpInfoU255_c::~FxInterpInfoU255_c()` (0x4A87F0): the base vtable 0x85a968.
#[hook(addr = 0x4A87F0, conv = thiscall, name = "FxInterpInfoU255_c::~FxInterpInfoU255_c", expect = "C7 01 68 A9 85 00")]
pub fn fx_4a87f0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85a968);
}

/// `FxInterpInfo32_c::~FxInterpInfo32_c()` (0x4A89B0): the base vtable 0x85a968.
#[hook(addr = 0x4A89B0, conv = thiscall, name = "FxInterpInfo32_c::~FxInterpInfo32_c", expect = "C7 01 68 A9 85 00")]
pub fn fx_4a89b0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85a968);
}

/// `FxInterpInfo255_c::~FxInterpInfo255_c()` (0x4A8B70): the base vtable 0x85a968.
#[hook(addr = 0x4A8B70, conv = thiscall, name = "FxInterpInfo255_c::~FxInterpInfo255_c", expect = "C7 01 68 A9 85 00")]
pub fn fx_4a8b70(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85a968);
}

/// `(missed by ghidra: data-vtable)(uint8 flags)` (0x4A8D10, scalar deleting destructor with the inlined destructor): the base
/// vtable 0x85a968, then `free(this)` when `flags & 1`; returns `this`.
#[hook(addr = 0x4A8D10, conv = thiscall, name = "(missed by ghidra: data-vtable)", expect = "F6 44 24 04 01")]
pub fn fx_4a8d10(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x85a968);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInterpInfoFloat_c::`scalar deleting destructor'(uint8 flags)` (0x4A8D30, scalar deleting destructor): the destructor 0x4A8460, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A8D30, conv = thiscall, name = "FxInterpInfoFloat_c::`scalar deleting destructor'", expect = "56 8B F1 E8 28 F7 FF FF")]
pub fn fx_4a8d30(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a8460(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInterpInfoU255_c::`scalar deleting destructor'(uint8 flags)` (0x4A8D50, scalar deleting destructor): the destructor 0x4A87F0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A8D50, conv = thiscall, name = "FxInterpInfoU255_c::`scalar deleting destructor'", expect = "56 8B F1 E8 98 FA FF FF")]
pub fn fx_4a8d50(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a87f0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInterpInfo32_c::`scalar deleting destructor'(uint8 flags)` (0x4A8D70, scalar deleting destructor): the destructor 0x4A89B0, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A8D70, conv = thiscall, name = "FxInterpInfo32_c::`scalar deleting destructor'", expect = "56 8B F1 E8 38 FC FF FF")]
pub fn fx_4a8d70(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a89b0(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxInterpInfo255_c::`scalar deleting destructor'(uint8 flags)` (0x4A8D90, scalar deleting destructor): the destructor 0x4A8B70, then `free(this)`
/// when `flags & 1`; returns `this`.
#[hook(addr = 0x4A8D90, conv = thiscall, name = "FxInterpInfo255_c::`scalar deleting destructor'", expect = "56 8B F1 E8 D8 FD FF FF")]
pub fn fx_4a8d90(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dtor_4a8b70(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FX_4A4B40,
    HOOK_FX_4A4C90,
    HOOK_FX_4A4CB0,
    HOOK_FX_4A4D70,
    HOOK_FX_4A4D90,
    HOOK_FX_4A4E30,
    HOOK_FX_4A4E50,
    HOOK_FX_4A4F00,
    HOOK_FX_4A4F20,
    HOOK_FX_4A4FC0,
    HOOK_FX_4A4FE0,
    HOOK_FX_4A5080,
    HOOK_FX_4A5150,
    HOOK_FX_4A5170,
    HOOK_FX_4A5220,
    HOOK_FX_4A5240,
    HOOK_FX_4A52E0,
    HOOK_FX_4A54C0,
    HOOK_FX_4A55C0,
    HOOK_FX_4A56C0,
    HOOK_FX_4A59F0,
    HOOK_FX_4A5A80,
    HOOK_FX_4A5B90,
    HOOK_FX_4A5D20,
    HOOK_FX_4A5DF0,
    HOOK_FX_4A5E10,
    HOOK_FX_4A5E70,
    HOOK_FX_4A5E90,
    HOOK_FX_4A5F10,
    HOOK_FX_4A6120,
    HOOK_FX_4A61F0,
    HOOK_FX_4A6210,
    HOOK_FX_4A6290,
    HOOK_FX_4A6390,
    HOOK_FX_4A64B0,
    HOOK_FX_4A65C0,
    HOOK_FX_4A67C0,
    HOOK_FX_4A6840,
    HOOK_FX_4A50A0,
    HOOK_FX_4A5830,
    HOOK_FX_4A66A0,
    HOOK_FX_4A6950,
    HOOK_FX_4A6A70,
    HOOK_FX_4A6A90,
    HOOK_FX_4A6AE0,
    HOOK_FX_4A6B00,
    HOOK_FX_4A6B50,
    HOOK_FX_4A6B70,
    HOOK_FX_4A6BC0,
    HOOK_FX_4A6BE0,
    HOOK_FX_4A6C30,
    HOOK_FX_4A6C50,
    HOOK_FX_4A6CA0,
    HOOK_FX_4A6CC0,
    HOOK_FX_4A6D10,
    HOOK_FX_4A6D30,
    HOOK_FX_4A6D80,
    HOOK_FX_4A6DF0,
    HOOK_FX_4A6E60,
    HOOK_FX_4A6ED0,
    HOOK_FX_4A6EF0,
    HOOK_FX_4A6F40,
    HOOK_FX_4A6F60,
    HOOK_FX_4A6FB0,
    HOOK_FX_4A6FD0,
    HOOK_FX_4A7020,
    HOOK_FX_4A7040,
    HOOK_FX_4A7090,
    HOOK_FX_4A7390,
    HOOK_FX_4A7400,
    HOOK_FX_4A7470,
    HOOK_FX_4A74E0,
    HOOK_FX_4A7500,
    HOOK_FX_4A7550,
    HOOK_FX_4A7570,
    HOOK_FX_4A75C0,
    HOOK_FX_4A75E0,
    HOOK_FX_4A7630,
    HOOK_FX_4A7650,
    HOOK_FX_4A76A0,
    HOOK_FX_4A7710,
    HOOK_FX_4A7780,
    HOOK_FX_4A77A0,
    HOOK_FX_4A77F0,
    HOOK_FX_4A7860,
    HOOK_FX_4A7880,
    HOOK_FX_4A78D0,
    HOOK_FX_4A78F0,
    HOOK_FX_4A7940,
    HOOK_FX_4A7960,
    HOOK_FX_4A79B0,
    HOOK_FX_4A79D0,
    HOOK_FX_4A7A20,
    HOOK_FX_4A7A40,
    HOOK_FX_4A7A90,
    HOOK_FX_4A7AB0,
    HOOK_FX_4A8460,
    HOOK_FX_4A87F0,
    HOOK_FX_4A89B0,
    HOOK_FX_4A8B70,
    HOOK_FX_4A8D10,
    HOOK_FX_4A8D30,
    HOOK_FX_4A8D50,
    HOOK_FX_4A8D70,
    HOOK_FX_4A8D90,
];
