//! The D3DX9 matrix and vector helpers RW's vertex-shader path calls (D3DX9 is linked statically into gta_sa.exe;
//! sa-rwcore `d3d9vsmat.rs` declares them). `D3DXMATRIX` is 16 row-major floats used with row vectors
//! (`v' = v * M`), `D3DXVECTOR3` three floats. Each result is computed before it is stored, so the output may alias an
//! input as D3DX allows. D3DX picks a CPU-specific implementation at run time; these are the plain arithmetic, so
//! the last bit of a sum can differ from the SSE path's.

use sa_core::mem::GPtr;

type Mat = [f32; 16];
type Vec3 = [f32; 3];

pub fn multiply(a: &Mat, b: &Mat) -> Mat {
    std::array::from_fn(|i| {
        let (r, c) = (i / 4, i % 4);
        (0..4).map(|k| a[r * 4 + k] * b[k * 4 + c]).sum()
    })
}

pub fn transpose(m: &Mat) -> Mat {
    std::array::from_fn(|i| m[(i % 4) * 4 + i / 4])
}

pub fn transform_normal(v: &Vec3, m: &Mat) -> Vec3 {
    std::array::from_fn(|c| v[0] * m[c] + v[1] * m[4 + c] + v[2] * m[8 + c])
}

/// `(x, y, z, 1) * M`, divided by the resulting w.
pub fn transform_coord(v: &Vec3, m: &Mat) -> Vec3 {
    let row = |c: usize| v[0] * m[c] + v[1] * m[4 + c] + v[2] * m[8 + c] + m[12 + c];
    let w = row(3);
    std::array::from_fn(|c| row(c) / w)
}

fn load<const N: usize>(p: u32) -> [f32; N] {
    std::array::from_fn(|i| GPtr::<f32>::from_addr(p as usize + 4 * i).read())
}

fn store<const N: usize>(p: u32, v: [f32; N]) -> u32 {
    for (i, x) in v.into_iter().enumerate() {
        GPtr::<f32>::from_addr(p as usize + 4 * i).write(x);
    }
    p
}

/// `D3DXMatrixMultiply(out, a, b)` (0x7672E1): `out = a * b`; returns `out`.
pub fn d3dx_matrix_multiply(out: u32, a: u32, b: u32) -> u32 {
    store(out, multiply(&load(a), &load(b)))
}

/// `D3DXMatrixMultiplyTranspose(out, a, b)` (0x7676C7): `out = transpose(a * b)`; returns `out`.
pub fn d3dx_matrix_multiply_transpose(out: u32, a: u32, b: u32) -> u32 {
    store(out, transpose(&multiply(&load(a), &load(b))))
}

/// `D3DXMatrixTranspose(out, m)` (0x7675EE); returns `out`.
pub fn d3dx_matrix_transpose(out: u32, m: u32) -> u32 {
    store(out, transpose(&load(m)))
}

/// `D3DXVec3TransformNormal(out, v, m)` (0x766AB7): `(x, y, z, 0) * m`; returns `out`.
pub fn d3dx_vec3_transform_normal(out: u32, v: u32, m: u32) -> u32 {
    store(out, transform_normal(&load(v), &load(m)))
}

/// `D3DXVec3TransformCoord(out, v, m)` (0x766A32): `(x, y, z, 1) * m` projected back to w = 1; returns `out`.
pub fn d3dx_vec3_transform_coord(out: u32, v: u32, m: u32) -> u32 {
    store(out, transform_coord(&load(v), &load(m)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Mat = [1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12., 13., 14., 15., 16.];
    const B: Mat = [2., 0., 0., 0., 0., 3., 0., 0., 0., 0., 4., 0., 1., 2., 3., 1.];

    #[test]
    fn multiply_is_row_major_row_vector_product() {
        let p = multiply(&A, &B);
        assert_eq!(&p[..4], &[2. + 4., 6. + 8., 12. + 12., 4.]);
        assert_eq!(&p[12..], &[26. + 16., 42. + 32., 60. + 48., 16.]);
        assert_eq!(transpose(&transpose(&A)), A);
        assert_eq!(transpose(&A)[1], 5.);
    }

    #[test]
    fn vectors_transform_as_rows() {
        let v = [1., 1., 1.];
        assert_eq!(transform_normal(&v, &B), [2., 3., 4.]);
        assert_eq!(transform_coord(&v, &B), [3., 5., 7.]);
        let mut proj = B;
        proj[15] = 2.;
        assert_eq!(transform_coord(&v, &proj), [1.5, 2.5, 3.5]);
    }
}
