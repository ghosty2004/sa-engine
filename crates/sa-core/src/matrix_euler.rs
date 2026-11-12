//! STD-9: `CMatrix::ConvertToEulerAngles`, `ConvertFromEulerAngles` and `Rotate` (batch `std9-math-1`): the hooks run
//! sa-math's models of the 1.0 US bytes (`sa_math::matrix::CMatrix`, with L1 specs in `sa-oracle`'s `math_specs.rs`).
//! `CMatrix`: right +0, forward +0x10, up +0x20, position +0x30. Unverified.

use sa_math::matrix::CMatrix;

use crate::abi::HookReg;
use crate::hook;
use crate::mem::GPtr;

/// The 18 dwords of the `CMatrix` at `m`.
fn load(m: GPtr<u8>) -> CMatrix {
    let w: [u32; 18] = core::array::from_fn(|i| m.byte_add(4 * i as isize).cast::<u32>().read());
    CMatrix::from_words(&w)
}

/// Writes back the dwords `idx` of `v` (the fields the original stores).
fn store(m: GPtr<u8>, v: &CMatrix, idx: &[usize]) {
    let w = v.to_words();
    for &i in idx {
        m.byte_add(4 * i as isize).cast::<u32>().write(w[i]);
    }
}

/// The 3x3 part's dwords (right, forward, up).
const ROT: [usize; 9] = [0, 1, 2, 4, 5, 6, 8, 9, 10];
/// The 3x3 part and the position.
const ROT_POS: [usize; 12] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14];

/// `CMatrix::ConvertToEulerAngles(float* x, float* y, float* z, uint32 flags)` (0x59A840, thiscall, `ret 0x10`):
/// [`CMatrix::convert_to_euler_angles`] (FPATAN); the three results are stored in order, then negated in place (flag
/// bit 2) and x / z swapped (bit 0), which the model folds into its result.
#[hook(addr = 0x59A840, conv = thiscall, name = "CMatrix::ConvertToEulerAngles", this = "CMatrix",
       expect = "83 EC 28 8B 01 8B 51 04", unverified = "std9-math-1", route = "L1-pure",
       hazards = "x87-transcendental", note = "sa-math model (FPATAN); writes through three out pointers")]
pub fn matrix_convert_to_euler_angles(m: GPtr<u8>, px: GPtr<f32>, py: GPtr<f32>, pz: GPtr<f32>, flags: u32) {
    let (x, y, z) = load(m).convert_to_euler_angles(flags);
    px.write(x);
    py.write(y);
    pz.write(z);
}

/// `CMatrix::ConvertFromEulerAngles(float x, float y, float z, uint32 flags)` (0x59AA40, thiscall, `ret 0x10`):
/// [`CMatrix::convert_from_euler_angles`] (FSIN / FCOS); only the 3x3 part is written.
#[hook(addr = 0x59AA40, conv = thiscall, name = "CMatrix::ConvertFromEulerAngles", this = "CMatrix",
       expect = "8B 44 24 10 83 EC 38 53", unverified = "std9-math-1", route = "L1-pure",
       hazards = "x87-transcendental", note = "sa-math model (FSIN/FCOS)")]
pub fn matrix_convert_from_euler_angles(m: GPtr<u8>, x: f32, y: f32, z: f32, flags: u32) {
    let mut v = load(m);
    v.convert_from_euler_angles(x, y, z, flags);
    store(m, &v, &ROT);
}

/// `CMatrix::Rotate(float x, float y, float z)` (0x59B460, thiscall, `ret 0xC`): [`CMatrix::rotate`], `this = R(x, y,
/// z) * this` for the 3x3 part and the position (FSIN / FCOS).
#[hook(addr = 0x59B460, conv = thiscall, name = "CMatrix::Rotate", this = "CMatrix",
       expect = "83 EC 54 D9 44 24 58 D9 FF", unverified = "std9-math-1", route = "L1-pure",
       hazards = "x87-transcendental", note = "sa-math model (FSIN/FCOS)")]
pub fn matrix_rotate_xyz(m: GPtr<u8>, x: f32, y: f32, z: f32) {
    let mut v = load(m);
    v.rotate(x, y, z);
    store(m, &v, &ROT_POS);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] =
    &[HOOK_MATRIX_CONVERT_TO_EULER_ANGLES, HOOK_MATRIX_CONVERT_FROM_EULER_ANGLES, HOOK_MATRIX_ROTATE_XYZ];
