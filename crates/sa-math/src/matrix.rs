//! `CMatrix` (and the `RwMatrix` it wraps), ported from the 1.0 US machine code.
//!
//! The ports follow the original instruction sequences (see `vector.rs` for the conventions): which
//! values stay in x87 registers ([`X87`], 24-bit significand, extended exponent) and which are stored as
//! f32 on the stack and reloaded, the operand order of every add/multiply, register vs memory operands,
//! and the order of the stores. FSIN/FCOS/FPATAN results are not affected by the precision control
//! (see [`X87::fsin`]).
//!
//! Pointers of the original (`m_pAttachMatrix`) are kept as 32-bit addresses; functions that follow
//! them take the pointed-to `RwMatrix` as an argument, and functions that call RenderWare to destroy one
//! take that call as a closure.

use crate::consts as k;
use crate::quaternion::CQuaternion;
use crate::vector::CVector;
use crate::x87::X87;

/// `RwMatrix` (RenderWare `RwMatrixTag`, 64 bytes): right, flags, up, pad, at, pad, pos, pad. The field
/// names follow `CMatrix` (RW's `up` is `forward`, RW's `at` is `up`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RwMatrix {
    pub right: CVector,
    pub flags: u32,
    pub forward: CVector,
    pub pad1: u32,
    pub up: CVector,
    pub pad2: u32,
    pub pos: CVector,
    pub pad3: u32,
}

/// `CMatrix` (0x48 bytes): an `RwMatrix` plus the attached RW matrix.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CMatrix {
    pub right: CVector,
    pub flags: u32,
    pub forward: CVector,
    pub pad1: u32,
    pub up: CVector,
    pub pad2: u32,
    pub pos: CVector,
    pub pad3: u32,
    /// `RwMatrix* m_pAttachMatrix` (address in the game's address space; 0 = none)
    pub attach_matrix: u32,
    /// `bool m_bOwnsAttachedMatrix`. The original reads and writes it as a **dword** (`mov ecx,
    /// [esi+44h]; test ecx, ecx`; `Attach` stores the zero-extended bool), so the three padding bytes
    /// after the bool take part in the test; it is modelled as the whole dword.
    pub owns_attached_matrix: u32,
}

const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(size_of::<RwMatrix>() == 0x40);
    assert!(offset_of!(RwMatrix, flags) == 0xC);
    assert!(offset_of!(RwMatrix, forward) == 0x10);
    assert!(offset_of!(RwMatrix, up) == 0x20);
    assert!(offset_of!(RwMatrix, pos) == 0x30);
    assert!(size_of::<CMatrix>() == 0x48);
    assert!(offset_of!(CMatrix, flags) == 0xC);
    assert!(offset_of!(CMatrix, forward) == 0x10);
    assert!(offset_of!(CMatrix, pad1) == 0x1C);
    assert!(offset_of!(CMatrix, up) == 0x20);
    assert!(offset_of!(CMatrix, pad2) == 0x2C);
    assert!(offset_of!(CMatrix, pos) == 0x30);
    assert!(offset_of!(CMatrix, pad3) == 0x3C);
    assert!(offset_of!(CMatrix, attach_matrix) == 0x40);
    assert!(offset_of!(CMatrix, owns_attached_matrix) == 0x44);
};

fn ld(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `fld a; fmul b` (b a memory operand).
fn mul(a: f32, b: f32) -> X87 {
    ld(a).fmul_m(b)
}

fn one() -> f32 {
    f32::from_bits(k::F1)
}

/// RenderWare `RwMatrixUpdate` (0x7F18A0, cdecl): `flags &= ~(rwMATRIXTYPEMASK | rwMATRIXINTERNALIDENTITY)`
/// (`and [eax+0Ch], 0FFFDFFFCh`). Called by the CMatrix copy-to-RW functions.
pub fn rw_matrix_update(m: &mut RwMatrix) {
    m.flags &= 0xFFFD_FFFC;
}

impl RwMatrix {
    /// The 16 dwords.
    pub fn to_words(&self) -> [u32; 16] {
        let mut w = [0u32; 16];
        for (r, v) in [self.right, self.forward, self.up, self.pos].iter().enumerate() {
            w[4 * r] = v.x.to_bits();
            w[4 * r + 1] = v.y.to_bits();
            w[4 * r + 2] = v.z.to_bits();
        }
        w[3] = self.flags;
        w[7] = self.pad1;
        w[11] = self.pad2;
        w[15] = self.pad3;
        w
    }

    pub fn from_words(w: &[u32]) -> RwMatrix {
        let v = |i: usize| {
            CVector::new(
                f32::from_bits(w[i]),
                f32::from_bits(w[i + 1]),
                f32::from_bits(w[i + 2]),
            )
        };
        RwMatrix {
            right: v(0),
            flags: w[3],
            forward: v(4),
            pad1: w[7],
            up: v(8),
            pad2: w[11],
            pos: v(12),
            pad3: w[15],
        }
    }
}

impl CMatrix {
    /// The 18 dwords.
    pub fn to_words(&self) -> [u32; 18] {
        let mut w = [0u32; 18];
        w[..16].copy_from_slice(&self.rw().to_words());
        w[16] = self.attach_matrix;
        w[17] = self.owns_attached_matrix;
        w
    }

    pub fn from_words(w: &[u32]) -> CMatrix {
        let mut m = CMatrix::default();
        m.set_rw(&RwMatrix::from_words(&w[..16]));
        m.attach_matrix = w[16];
        m.owns_attached_matrix = w[17];
        m
    }

    /// The first 64 bytes as an `RwMatrix`.
    pub fn rw(&self) -> RwMatrix {
        RwMatrix {
            right: self.right,
            flags: self.flags,
            forward: self.forward,
            pad1: self.pad1,
            up: self.up,
            pad2: self.pad2,
            pos: self.pos,
            pad3: self.pad3,
        }
    }

    fn set_rw(&mut self, m: &RwMatrix) {
        self.right = m.right;
        self.flags = m.flags;
        self.forward = m.forward;
        self.pad1 = m.pad1;
        self.up = m.up;
        self.pad2 = m.pad2;
        self.pos = m.pos;
        self.pad3 = m.pad3;
    }

    /// The 12 vector components (no flags/padding), as the copy loops of the original see them.
    fn set_vectors(&mut self, right: CVector, forward: CVector, up: CVector, pos: CVector) {
        self.right = right;
        self.forward = forward;
        self.up = up;
        self.pos = pos;
    }

    // ------------------------------------------------------------------ setters

    /// `CMatrix::SetUnity` (0x59AE70).
    pub fn set_unity(&mut self) {
        self.reset_orientation();
        self.pos = CVector::default();
    }

    /// `CMatrix::ResetOrientation` (0x59AEA0): identity 3x3, position untouched.
    pub fn reset_orientation(&mut self) {
        self.right = CVector::new(one(), 0.0, 0.0);
        self.forward = CVector::new(0.0, one(), 0.0);
        self.up = CVector::new(0.0, 0.0, one());
    }

    /// `CMatrix::SetScale(float)` (0x59AED0): raw copies of the argument.
    pub fn set_scale(&mut self, s: f32) {
        self.set_scale_xyz(s, s, s);
    }

    /// `CMatrix::SetScale(float, float, float)` (0x59AF00).
    pub fn set_scale_xyz(&mut self, x: f32, y: f32, z: f32) {
        self.right = CVector::new(x, 0.0, 0.0);
        self.forward = CVector::new(0.0, y, 0.0);
        self.up = CVector::new(0.0, 0.0, z);
        self.pos = CVector::default();
    }

    /// `CMatrix::SetTranslate` (0x59AF40): identity 3x3 and the position.
    pub fn set_translate(&mut self, x: f32, y: f32, z: f32) {
        self.reset_orientation();
        self.pos = CVector::new(x, y, z);
    }

    /// `CMatrix::SetTranslateOnly` (0x59AF80).
    pub fn set_translate_only(&mut self, x: f32, y: f32, z: f32) {
        self.pos = CVector::new(x, y, z);
    }

    /// `CMatrix::SetRotateXOnly` (0x59AFA0): `fcos`/`fsin` of the angle, stored as f32.
    pub fn set_rotate_x_only(&mut self, a: f32) {
        let (c, s) = (ld(a).fcos(), ld(a).fsin());
        self.right = CVector::new(one(), 0.0, 0.0);
        self.forward = CVector::new(0.0, c.to_f32(), s.to_f32());
        self.up = CVector::new(0.0, s.fchs().to_f32(), c.to_f32());
    }

    /// `CMatrix::SetRotateYOnly` (0x59AFE0).
    pub fn set_rotate_y_only(&mut self, a: f32) {
        let (c, s) = (ld(a).fcos(), ld(a).fsin());
        self.right = CVector::new(c.to_f32(), 0.0, s.fchs().to_f32());
        self.forward = CVector::new(0.0, one(), 0.0);
        self.up = CVector::new(s.to_f32(), 0.0, c.to_f32());
    }

    /// `CMatrix::SetRotateZOnly` (0x59B020).
    pub fn set_rotate_z_only(&mut self, a: f32) {
        let (c, s) = (ld(a).fcos(), ld(a).fsin());
        self.right = CVector::new(c.to_f32(), s.to_f32(), 0.0);
        self.forward = CVector::new(s.fchs().to_f32(), c.to_f32(), 0.0);
        self.up = CVector::new(0.0, 0.0, one());
    }

    /// `CMatrix::SetRotateX` (0x59B060): `SetRotateXOnly` plus position 0.
    pub fn set_rotate_x(&mut self, a: f32) {
        self.set_rotate_x_only(a);
        self.pos = CVector::default();
    }

    /// `CMatrix::SetRotateY` (0x59B0A0).
    pub fn set_rotate_y(&mut self, a: f32) {
        self.set_rotate_y_only(a);
        self.pos = CVector::default();
    }

    /// `CMatrix::SetRotateZ` (0x59B0E0).
    pub fn set_rotate_z(&mut self, a: f32) {
        self.set_rotate_z_only(a);
        self.pos = CVector::default();
    }

    /// `CMatrix::SetRotate(float x, float y, float z)` (0x59B120): rotation matrix from three angles, with
    /// the original's mix of register values and f32 temporaries (`cy`, `sz`, `cz*sx` are stored into the
    /// argument slots and reloaded); position 0.
    pub fn set_rotate(&mut self, x: f32, y: f32, z: f32) {
        let cx = ld(x).fcos();
        let sx = ld(x).fsin();
        let cyf = ld(y).fcos().to_f32();
        let sy = ld(y).fsin();
        let cz = ld(z).fcos();
        let sz = ld(z).fsin();
        let szf = sz.to_f32();
        let szsx = sz.fmul(sx);
        let czsxf = cz.fmul(sx).to_f32();
        self.pos = CVector::default();
        self.right = CVector::new(
            cz.fmul_m(cyf).fsub(szsx.fmul(sy)).to_f32(),
            ld(czsxf).fmul(sy).fadd(mul(szf, cyf)).to_f32(),
            sy.fmul(cx).fchs().to_f32(),
        );
        self.forward = CVector::new(
            ld(szf).fmul(cx).fchs().to_f32(),
            cz.fmul(cx).to_f32(),
            sx.to_f32(),
        );
        self.up = CVector::new(
            cz.fmul(sy).fadd(szsx.fmul_m(cyf)).to_f32(),
            ld(szf).fmul(sy).fsub(mul(czsxf, cyf)).to_f32(),
            ld(cyf).fmul(cx).to_f32(),
        );
    }

    /// `CMatrix::Scale(float)` (0x459350): the 3x3 part times `s` (`fld s; fmul m[i]`), rows in order.
    pub fn scale(&mut self, s: f32) {
        for v in [&mut self.right, &mut self.forward, &mut self.up] {
            v.x = ld(s).fmul_m(v.x).to_f32();
            v.y = ld(s).fmul_m(v.y).to_f32();
            v.z = ld(s).fmul_m(v.z).to_f32();
        }
    }

    // ------------------------------------------------------------------ rotations

    /// `CMatrix::RotateX` (0x59B1E0): rotates every row (and the position) about X. `sin` is stored as
    /// f32 into the argument slot and reloaded, `cos` stays in a register.
    pub fn rotate_x(&mut self, a: f32) {
        let c = ld(a).fcos();
        let sf = ld(a).fsin().to_f32();
        // right row stays in registers until the end
        let ry = c.fmul_m(self.right.y).fsub(mul(sf, self.right.z));
        let rz = mul(sf, self.right.y).fadd(c.fmul_m(self.right.z));
        let (fy, fz) = rot_pair(c, sf, self.forward.y, self.forward.z);
        self.forward.y = fy;
        self.forward.z = fz;
        let (uy, uz) = rot_pair(c, sf, self.up.y, self.up.z);
        self.up.y = uy;
        self.up.z = uz;
        let py = c.fmul_m(self.pos.y).fsub(mul(sf, self.pos.z)).to_f32();
        let pz = mul(sf, self.pos.y).fadd(c.fmul_m(self.pos.z)).to_f32();
        self.pos.y = py;
        self.pos.z = pz;
        self.right.y = ry.to_f32();
        self.right.z = rz.to_f32();
    }

    /// `CMatrix::RotateY` (0x59B2C0). Unlike X and Z, the right row's first product uses the register
    /// `sin` (`fst` instead of `fstp`).
    pub fn rotate_y(&mut self, a: f32) {
        let c = ld(a).fcos();
        let s = ld(a).fsin();
        let sf = s.to_f32();
        let rx = s.fmul_m(self.right.z).fadd(c.fmul_m(self.right.x));
        let rz = c.fmul_m(self.right.z).fsub(mul(sf, self.right.x));
        for v in [&mut self.forward, &mut self.up, &mut self.pos] {
            // new x = sf*z + c*x; new z = c*z - sf*x (old x)
            let nx = mul(sf, v.z).fadd(c.fmul_m(v.x)).to_f32();
            let nz = c.fmul_m(v.z).fsub(mul(sf, v.x)).to_f32();
            v.x = nx;
            v.z = nz;
        }
        self.right.x = rx.to_f32();
        self.right.z = rz.to_f32();
    }

    /// `CMatrix::RotateZ` (0x59B390).
    pub fn rotate_z(&mut self, a: f32) {
        let c = ld(a).fcos();
        let sf = ld(a).fsin().to_f32();
        let rx = c.fmul_m(self.right.x).fsub(mul(sf, self.right.y));
        let ry = c.fmul_m(self.right.y).fadd(mul(sf, self.right.x));
        let (fx, fy) = rot_pair(c, sf, self.forward.x, self.forward.y);
        self.forward.x = fx;
        self.forward.y = fy;
        let (ux, uy) = rot_pair(c, sf, self.up.x, self.up.y);
        self.up.x = ux;
        self.up.y = uy;
        let px = c.fmul_m(self.pos.x).fsub(mul(sf, self.pos.y)).to_f32();
        let py = mul(sf, self.pos.x).fadd(c.fmul_m(self.pos.y)).to_f32();
        self.pos.x = px;
        self.pos.y = py;
        self.right.x = rx.to_f32();
        self.right.y = ry.to_f32();
    }

    /// `CMatrix::Rotate(float x, float y, float z)` (0x59B460): `this = R(x, y, z) * this` (3x3 and
    /// position), R built as in [`CMatrix::set_rotate`] but kept on the stack as f32 (except R[2][2],
    /// which stays in a register).
    pub fn rotate(&mut self, x: f32, y: f32, z: f32) {
        let cx = ld(x).fcos();
        let sx = ld(x).fsin().to_f32();
        let cy = ld(y).fcos();
        let sy = ld(y).fsin();
        let cz = ld(z).fcos();
        let sz = ld(z).fsin();
        let szf = sz.to_f32();
        let szsx = sz.fmul_m(sx);
        let czsxf = cz.fmul_m(sx).to_f32();
        let r0 = cz.fmul(cy).fsub(szsx.fmul(sy)).to_f32();
        let r1 = ld(czsxf).fmul(sy).fadd(ld(szf).fmul(cy)).to_f32();
        let r2 = sy.fmul(cx).fchs().to_f32();
        let r4 = ld(szf).fmul(cx).fchs().to_f32();
        let r5 = cz.fmul(cx).to_f32();
        let r8 = cz.fmul(sy).fadd(szsx.fmul(cy)).to_f32();
        let r9 = ld(szf).fmul(sy).fsub(ld(czsxf).fmul(cy)).to_f32();
        let r10 = cy.fmul(cx);
        let t = *self;
        // new right
        let nrx = mul(r8, t.right.z)
            .fadd(mul(r4, t.right.y))
            .fadd(mul(r0, t.right.x));
        let nry = mul(r9, t.right.z)
            .fadd(mul(r5, t.right.y))
            .fadd(mul(r1, t.right.x));
        let nrz = r10
            .fmul_m(t.right.z)
            .fadd(mul(sx, t.right.y))
            .fadd(mul(r2, t.right.x));
        // rows: (R0*v.x + R8*v.z) + R4*v.y etc.
        let row = |v: CVector| {
            CVector::new(
                mul(r0, v.x).fadd(mul(r8, v.z)).fadd(mul(r4, v.y)).to_f32(),
                mul(r1, v.x).fadd(mul(r9, v.z)).fadd(mul(r5, v.y)).to_f32(),
                mul(r2, v.x).fadd(r10.fmul_m(v.z)).fadd(mul(sx, v.y)).to_f32(),
            )
        };
        let f = row(t.forward);
        let u = row(t.up);
        let p = row(t.pos);
        self.right = CVector::new(nrx.to_f32(), nry.to_f32(), nrz.to_f32());
        self.forward = f;
        self.up = u;
        self.pos = p;
    }

    /// `CMatrix::Reorthogonalise` (0x59B6A0): `up = normalise(right x forward)`,
    /// `right = normalise(forward x up)`, `forward = up x right`, with the original's registers and f32
    /// temporaries.
    pub fn reorthogonalise(&mut self) {
        let (r, f) = (self.right, self.forward);
        // a = right x forward
        let ax = mul(r.y, f.z).fsub(mul(f.y, r.z));
        let ay = mul(f.x, r.z).fsub(mul(r.x, f.z)).to_f32();
        let az = mul(f.y, r.x).fsub(mul(f.x, r.y));
        let azf = az.to_f32();
        let q = az.fmul_m(azf).fadd(mul(ay, ay)).fadd(ax.fmul(ax));
        let inv = q.fsqrt().fdivr_m(one());
        let invf = inv.to_f32();
        let nax = ax.fmul(inv);
        let nay = mul(invf, ay);
        let naz = mul(invf, azf);
        let nazf = naz.to_f32();
        // b = n x forward (with the forward row reloaded from memory)
        let bx = naz.fmul_m(f.y).fsub(nay.fmul_m(f.z));
        let by = nax.fmul_m(f.z).fsub(mul(nazf, f.x)).to_f32();
        let bz = nay.fmul_m(f.x).fsub(nax.fmul_m(f.y));
        let bzf = bz.to_f32();
        let q2 = bz.fmul_m(bzf).fadd(mul(by, by)).fadd(bx.fmul(bx));
        let inv2f = q2.fsqrt().fdivr_m(one()).to_f32();
        let nbx = bx.fmul_m(inv2f);
        let nby = mul(by, inv2f);
        let nbz = mul(bzf, inv2f);
        let nbzf = nbz.to_f32();
        // c = n x b
        let cx = nbz.fmul(nay).fsub(nby.fmul_m(nazf)).to_f32();
        let cy = nbx.fmul_m(nazf).fsub(ld(nbzf).fmul(nax)).to_f32();
        let cz = nby.fmul(nax).fsub(nbx.fmul(nay)).to_f32();
        self.right = CVector::new(nbx.to_f32(), nby.to_f32(), nbzf);
        self.forward = CVector::new(cx, cy, cz);
        self.up = CVector::new(nax.to_f32(), nay.to_f32(), nazf);
    }

    /// `CMatrix::ForceUpVector(CVector up)` (0x59B7E0, vector by value): `up = arg` (raw),
    /// `right = forward x up`, `forward = up x right`.
    pub fn force_up_vector(&mut self, x: f32, y: f32, z: f32) {
        let f = self.forward;
        let rx = mul(z, f.y).fsub(mul(y, f.z));
        let ry = mul(x, f.z).fsub(mul(z, f.x));
        let rz = mul(y, f.x).fsub(mul(x, f.y));
        let rzf = rz.to_f32();
        let fx = rz.fmul_m(y).fsub(ry.fmul_m(z)).to_f32();
        let fy = rx.fmul_m(z).fsub(mul(rzf, x)).to_f32();
        let fz = ld(x).fmul(ry).fsub(rx.fmul_m(y)).to_f32();
        self.right = CVector::new(rx.to_f32(), ry.to_f32(), rzf);
        self.forward = CVector::new(fx, fy, fz);
        self.up = CVector::new(x, y, z);
    }

    // ------------------------------------------------------------------ copies

    /// `CMatrix::UpdateMatrix(RwMatrix*)` (0x59AD20): the 12 vector components from `m` (flags and
    /// padding untouched).
    pub fn update_matrix(&mut self, m: &RwMatrix) {
        self.set_vectors(m.right, m.forward, m.up, m.pos);
    }

    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70): the 12 vector components into `m`, then
    /// `RwMatrixUpdate(m)`.
    pub fn update_rw_matrix(&self, m: &mut RwMatrix) {
        m.right = self.right;
        m.forward = self.forward;
        m.up = self.up;
        m.pos = self.pos;
        rw_matrix_update(m);
    }

    /// `CMatrix::CopyToRwMatrix(RwMatrix*)` (0x59B8B0, returns `m`): as `UpdateRwMatrix`.
    pub fn copy_to_rw_matrix(&self, m: &mut RwMatrix) {
        self.update_rw_matrix(m);
    }

    /// `CMatrix::CopyOnlyMatrix(const CMatrix&)` (0x59ADD0): the first 16 dwords (`rep movsd`), flags
    /// and padding included.
    pub fn copy_only_matrix(&mut self, src: &CMatrix) {
        self.set_rw(&src.rw());
    }

    /// `CMatrix::Update()` (0x59BB60): `UpdateMatrix(m_pAttachMatrix)`; `attached` is the RwMatrix at
    /// `self.attach_matrix` (the original does not check it for null).
    pub fn update(&mut self, attached: &RwMatrix) {
        self.update_matrix(attached);
    }

    /// `CMatrix::UpdateRW()` (0x59BBB0): `UpdateRwMatrix(m_pAttachMatrix)` when attached. `attached` is
    /// the RwMatrix at `self.attach_matrix`; it is used only when that address is non-zero.
    pub fn update_rw(&self, attached: Option<&mut RwMatrix>) {
        if self.attach_matrix != 0 {
            self.update_rw_matrix(attached.expect("CMatrix::UpdateRW: attached matrix missing"));
        }
    }

    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, returns `this`): copies 16 dwords (attach fields
    /// kept), then `UpdateRwMatrix(m_pAttachMatrix)` when attached (`attached` as for
    /// [`CMatrix::update_rw`]).
    pub fn assign(&mut self, src: &CMatrix, attached: Option<&mut RwMatrix>) {
        let att = self.attach_matrix;
        self.set_rw(&src.rw());
        if att != 0 {
            self.update_rw_matrix(attached.expect("CMatrix::operator=: attached matrix missing"));
        }
    }

    /// `CMatrix::Detach()` (0x59ACF0): when attached and owning (dword test), `RwMatrixDestroy(m)`
    /// (0x7F2A20, passed in as `destroy`); then `m_pAttachMatrix = nullptr`.
    pub fn detach(&mut self, destroy: impl FnOnce(u32)) {
        if self.attach_matrix != 0 && self.owns_attached_matrix != 0 {
            destroy(self.attach_matrix);
        }
        self.attach_matrix = 0;
    }

    /// `CMatrix::Attach(RwMatrix* m, bool owns)` (0x59BD10): the inlined `Detach` (without clearing the
    /// pointer), then `m_pAttachMatrix = m`, `owns` zero-extended into the dword, and `Update()`.
    pub fn attach(&mut self, m_addr: u32, m: &RwMatrix, owns: u8, destroy: impl FnOnce(u32)) {
        if self.attach_matrix != 0 && self.owns_attached_matrix != 0 {
            destroy(self.attach_matrix);
        }
        self.attach_matrix = m_addr;
        self.owns_attached_matrix = u32::from(owns);
        self.update_matrix(m);
    }

    /// `CMatrix::operator+=(const CMatrix&)` (0x59ADF0): `this[i] = rhs[i] + this[i]` (`fld rhs; fadd
    /// this`) in the order right.x, forward.x, up.x, right.y, ... pos.z. Alias-safe.
    pub fn add_assign(&mut self, rhs: &CMatrix) {
        let add = |a: f32, b: f32| ld(a).fadd_m(b).to_f32();
        self.right.x = add(rhs.right.x, self.right.x);
        self.forward.x = add(rhs.forward.x, self.forward.x);
        self.up.x = add(rhs.up.x, self.up.x);
        self.right.y = add(rhs.right.y, self.right.y);
        self.forward.y = add(rhs.forward.y, self.forward.y);
        self.up.y = add(rhs.up.y, self.up.y);
        self.right.z = add(rhs.right.z, self.right.z);
        self.forward.z = add(rhs.forward.z, self.forward.z);
        self.up.z = add(rhs.up.z, self.up.z);
        self.pos.x = add(rhs.pos.x, self.pos.x);
        self.pos.y = add(rhs.pos.y, self.pos.y);
        self.pos.z = add(rhs.pos.z, self.pos.z);
    }

    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80, returns `this`): `this = this * rhs` through a
    /// temporary (`operator*`, then `operator=`, which updates the attached RwMatrix). The flags and
    /// padding words come from the temporary's uninitialised stack slots (see [`mul`]); the port writes
    /// 0 there.
    pub fn mul_assign(&mut self, rhs: &CMatrix, attached: Option<&mut RwMatrix>) {
        let t = mul_matrix(self, rhs);
        self.assign(&t, attached);
    }

    /// `CMatrix::ConvertFromEulerAngles(float x, float y, float z, uint32 flags)` (0x59AA40).
    ///
    /// Flags as the original decodes them: bit 0 swaps x and z, bit 1 selects proper Euler angles (else
    /// Tait-Bryan), bit 2 negates the angles and swaps the 2nd/3rd axes, bits 3-4 pick the first axis
    /// from the exe's table {0, 1, 2, 0}. Only the 3x3 part is written.
    pub fn convert_from_euler_angles(&mut self, x: f32, y: f32, z: f32, flags: u32) {
        let (i1, i2, i3, neg) = euler_indices(flags);
        // swap: st0 = z, z slot = x
        let (mut xv, mut zs) = if flags & 1 != 0 { (ld(z), x) } else { (ld(x), z) };
        let mut yv = ld(y);
        if neg {
            xv = xv.fchs();
            yv = yv.fchs();
            zs = ld(zs).fchs().to_f32();
        }
        let cx = xv.fcos().to_f32();
        let cy = yv.fcos().to_f32();
        let cz = ld(zs).fcos().to_f32();
        let sx = xv.fsin().to_f32();
        let sy = yv.fsin().to_f32();
        let sz = ld(zs).fsin();
        let czcx = mul(cz, cx).to_f32();
        let cxsz = ld(cx).fmul(sz).to_f32();
        let sxcz = mul(sx, cz).to_f32();
        let sxsz = ld(sx).fmul(sz).to_f32();
        let mut a = [0f32; 9];
        let at = |r: usize, c: usize| 3 * r + c;
        if flags & 2 != 0 {
            a[at(i1, i1)] = cy;
            a[at(i1, i2)] = mul(sy, sx).to_f32();
            a[at(i1, i3)] = mul(sy, cx).to_f32();
            a[at(i2, i1)] = sz.fmul_m(sy).to_f32();
            a[at(i2, i2)] = mul(sxsz, cy).fsubr_m(czcx).to_f32();
            a[at(i2, i3)] = mul(cxsz, cy).fchs().fsub_m(sxcz).to_f32();
            a[at(i3, i1)] = mul(sy, cz).fchs().to_f32();
            a[at(i3, i2)] = mul(sxcz, cy).fadd_m(cxsz).to_f32();
            a[at(i3, i3)] = mul(czcx, cy).fsub_m(sxsz).to_f32();
        } else {
            a[at(i1, i1)] = mul(cz, cy).to_f32();
            a[at(i1, i2)] = mul(sxcz, sy).fsub_m(cxsz).to_f32();
            a[at(i1, i3)] = mul(czcx, sy).fadd_m(sxsz).to_f32();
            a[at(i2, i1)] = sz.fmul_m(cy).to_f32();
            a[at(i2, i2)] = mul(sxsz, sy).fadd_m(czcx).to_f32();
            a[at(i2, i3)] = mul(cxsz, sy).fsub_m(sxcz).to_f32();
            a[at(i3, i1)] = ld(sy).fchs().to_f32();
            a[at(i3, i2)] = mul(sx, cy).to_f32();
            a[at(i3, i3)] = mul(cy, cx).to_f32();
        }
        self.right = CVector::new(a[0], a[1], a[2]);
        self.forward = CVector::new(a[3], a[4], a[5]);
        self.up = CVector::new(a[6], a[7], a[8]);
    }

    /// `CMatrix::ConvertToEulerAngles(float* x, float* y, float* z, uint32 flags)` (0x59A840): the values
    /// the original leaves in `*x`, `*y`, `*z` (written in that order; the flags as in
    /// [`CMatrix::convert_from_euler_angles`]).
    pub fn convert_to_euler_angles(&self, flags: u32) -> (f32, f32, f32) {
        let (i1, i2, i3, neg) = euler_indices(flags);
        let a = [
            self.right.x,
            self.right.y,
            self.right.z,
            self.forward.x,
            self.forward.y,
            self.forward.z,
            self.up.x,
            self.up.y,
            self.up.z,
        ];
        let e = |r: usize, c: usize| ld(a[3 * r + c]);
        let eps = X87::constant(k::EULER_EPSILON);
        let (mut x, mut y, mut z);
        if flags & 2 != 0 {
            let (r12, r13) = (e(i1, i2), e(i1, i3));
            let cy = r12.fmul(r12).fadd(r13.fmul(r13)).fsqrt();
            if !cy.fcom(eps).test_any(0x41) {
                x = r12.fpatan(r13).to_f32();
                y = cy.fpatan(e(i1, i1)).to_f32();
                z = e(i2, i1).fpatan(e(i3, i1).fchs()).to_f32();
            } else {
                x = e(i2, i3).fchs().fpatan(e(i2, i2)).to_f32();
                y = cy.fpatan(e(i1, i1)).to_f32();
                z = 0.0;
            }
        } else {
            let (r21, r11) = (e(i2, i1), e(i1, i1));
            let cy = r21.fmul(r21).fadd(r11.fmul(r11)).fsqrt();
            if !cy.fcom(eps).test_any(0x41) {
                x = e(i3, i2).fpatan(e(i3, i3)).to_f32();
                y = e(i3, i1).fchs().fpatan(cy).to_f32();
                z = r21.fpatan(r11).to_f32();
            } else {
                x = e(i2, i3).fchs().fpatan(e(i2, i2)).to_f32();
                y = e(i3, i1).fchs().fpatan(cy).to_f32();
                z = 0.0;
            }
        }
        if neg {
            // fld; fchs; fstp (each value was stored, so a NaN is already quiet)
            x = ld(x).fchs().to_f32();
            y = ld(y).fchs().to_f32();
            z = ld(z).fchs().to_f32();
        }
        if flags & 1 != 0 {
            // *x = *z (raw); *z = fld old *x
            let ox = x;
            x = z;
            z = ld(ox).to_f32();
        }
        (x, y, z)
    }

    /// `CMatrix::SetRotate(const CQuaternion&)` (0x59BBF0): the rotation part from a quaternion.
    pub fn set_rotate_quat(&mut self, q: &CQuaternion) {
        let (x, y) = (ld(q.x), ld(q.y));
        let (qz, qw) = (q.z, q.w);
        let x2 = x.fdouble().to_f32();
        let y2 = y.fadd(y).to_f32();
        let z2 = ld(qz).fdouble().to_f32();
        let xx2 = ld(x2).fmul(x).to_f32();
        let xy2 = ld(y2).fmul(x).to_f32();
        let xz2 = ld(z2).fmul(x).to_f32();
        let yy2 = ld(y2).fmul(y).to_f32();
        let yz2 = ld(z2).fmul(y); // stays in a register
        let zz2 = mul(z2, qz);
        let xw2 = mul(x2, qw).to_f32();
        let yw2 = mul(y2, qw);
        let zw2 = mul(z2, qw);
        self.right.x = zz2.fadd_m(yy2).fsubr_m(one()).to_f32();
        self.right.y = ld(xy2).fadd(zw2).to_f32();
        self.right.z = ld(xz2).fsub(yw2).to_f32();
        self.forward.x = ld(xy2).fsub(zw2).to_f32();
        self.forward.y = zz2.fadd_m(xx2).fsubr_m(one()).to_f32();
        self.forward.z = ld(xw2).fadd(yz2).to_f32();
        self.up.x = yw2.fadd_m(xz2).to_f32();
        self.up.y = yz2.fsub_m(xw2).to_f32();
        self.up.z = ld(yy2).fadd_m(xx2).fsubr_m(one()).to_f32();
    }
}

/// Indices of `ConvertTo/FromEulerAngles`: `i1 = T1[(f >> 3) & 3]`, `i2 = T2[i1 + neg]`,
/// `i3 = T2[i1 + 1 - neg]` with the exe's byte tables T1 (0x866D9C) and T2 (0x866D94).
fn euler_indices(flags: u32) -> (usize, usize, usize, bool) {
    let neg = (flags >> 2) & 1 == 1;
    let i1 = k::EULER_T1[((flags >> 3) & 3) as usize] as usize;
    let n = neg as usize;
    let i2 = k::EULER_T2[i1 + n] as usize;
    let i3 = k::EULER_T2[i1 + 1 - n] as usize;
    (i1, i2, i3, neg)
}

/// `cos*a - sf*b` and `sf*a + cos*b`, each stored as f32 (the RotateX/RotateZ row pattern: the second
/// one reloads the old `a` from memory).
fn rot_pair(c: X87, sf: f32, a: f32, b: f32) -> (f32, f32) {
    let na = c.fmul_m(a).fsub(mul(sf, b)).to_f32();
    let nb = mul(sf, a).fadd(c.fmul_m(b)).to_f32();
    (na, nb)
}

/// `operator*(const CMatrix&, const CMatrix&)` (0x59BE30, cdecl, hidden return pointer): `a * b`
/// (3x3 product, `pos = a.3x3 * b.pos + a.pos`). The result is built in a stack temporary whose flags
/// and padding words (offsets 0xC, 0x1C, 0x2C, 0x3C) are never written, then copied whole: those four
/// words of the result are uninitialised stack memory in the original (ledger: original UB); the port
/// writes 0. `m_pAttachMatrix` and the owns dword of the result are 0.
pub fn mul_matrix(a: &CMatrix, b: &CMatrix) -> CMatrix {
    // `fld x; fmul y` pairs as the original orders them (x loaded, y the memory operand)
    let rx = mul(a.forward.x, b.right.y)
        .fadd(mul(a.right.x, b.right.x))
        .fadd(mul(b.right.z, a.up.x));
    let ry = mul(a.up.y, b.right.z)
        .fadd(mul(a.right.y, b.right.x))
        .fadd(mul(a.forward.y, b.right.y));
    let rz = mul(a.forward.z, b.right.y)
        .fadd(mul(a.up.z, b.right.z))
        .fadd(mul(b.right.x, a.right.z));
    let fx = mul(b.forward.z, a.up.x)
        .fadd(mul(a.forward.x, b.forward.y))
        .fadd(mul(b.forward.x, a.right.x));
    let fy = mul(a.up.y, b.forward.z)
        .fadd(mul(a.forward.y, b.forward.y))
        .fadd(mul(a.right.y, b.forward.x));
    let fz = mul(b.forward.x, a.right.z)
        .fadd(mul(a.up.z, b.forward.z))
        .fadd(mul(a.forward.z, b.forward.y));
    let ux = mul(a.forward.x, b.up.y)
        .fadd(mul(b.up.z, a.up.x))
        .fadd(mul(b.up.x, a.right.x));
    let uy = mul(a.up.y, b.up.z)
        .fadd(mul(a.forward.y, b.up.y))
        .fadd(mul(b.up.x, a.right.y));
    let uz = mul(b.up.x, a.right.z)
        .fadd(mul(a.up.z, b.up.z))
        .fadd(mul(a.forward.z, b.up.y));
    let px = mul(a.forward.x, b.pos.y)
        .fadd(mul(b.pos.z, a.up.x))
        .fadd(mul(b.pos.x, a.right.x))
        .fadd_m(a.pos.x);
    let py = mul(b.pos.z, a.up.y)
        .fadd(mul(b.pos.y, a.forward.y))
        .fadd(mul(b.pos.x, a.right.y))
        .fadd_m(a.pos.y);
    let pz = mul(b.pos.x, a.right.z)
        .fadd(mul(a.forward.z, b.pos.y))
        .fadd(mul(a.up.z, b.pos.z))
        .fadd_m(a.pos.z);
    let v = |x: X87, y: X87, z: X87| CVector::new(x.to_f32(), y.to_f32(), z.to_f32());
    CMatrix {
        right: v(rx, ry, rz),
        forward: v(fx, fy, fz),
        up: v(ux, uy, uz),
        pos: v(px, py, pz),
        ..CMatrix::default()
    }
}

/// `operator+(const CMatrix&, const CMatrix&)` (0x59BFA0, cdecl, hidden return pointer): element-wise
/// `fld a; fadd b`. Flags/padding of the result: uninitialised stack, as in [`mul_matrix`] (port: 0).
pub fn add_matrix(a: &CMatrix, b: &CMatrix) -> CMatrix {
    let add = |x: f32, y: f32| ld(x).fadd_m(y).to_f32();
    let v = |p: CVector, q: CVector| CVector::new(add(p.x, q.x), add(p.y, q.y), add(p.z, q.z));
    CMatrix {
        right: v(a.right, b.right),
        forward: v(a.forward, b.forward),
        up: v(a.up, b.up),
        pos: v(a.pos, b.pos),
        ..CMatrix::default()
    }
}

/// `Invert(CMatrix& src, CMatrix& dst)` (0x59B920, cdecl, returns `dst`) on dword memory: `src` and
/// `dst` are word offsets into `mem` (they may overlap, as in the original). `dst` = transpose of the
/// 3x3 part, `dst.pos = -(dst.3x3 * src.pos)`, every partial sum stored as f32 in `dst.pos`. Flags,
/// padding and the attach fields of `dst` are not written.
pub fn invert_mem(mem: &mut [u32], src: usize, dst: usize) {
    let f = |mem: &[u32], i: usize| f32::from_bits(mem[i]);
    for i in 12..15 {
        mem[dst + i] = 0;
    }
    for (d, s) in [
        (0, 0),
        (1, 4),
        (2, 8),
        (4, 1),
        (5, 5),
        (6, 9),
        (8, 2),
        (9, 6),
        (10, 10),
    ] {
        mem[dst + d] = mem[src + s];
    }
    // (loaded, memory operand[, added]) as word offsets: s = src, d = dst
    let st = |mem: &mut [u32], i: usize, v: X87| mem[dst + i] = v.to_f32().to_bits();
    let v = mul(f(mem, src + 12), f(mem, dst));
    st(mem, 12, v);
    let v = mul(f(mem, dst + 1), f(mem, src + 12));
    st(mem, 13, v);
    let v = mul(f(mem, dst + 2), f(mem, src + 12));
    st(mem, 14, v);
    let v = mul(f(mem, src + 13), f(mem, dst + 4)).fadd_m(f(mem, dst + 12));
    st(mem, 12, v);
    let v = mul(f(mem, dst + 5), f(mem, src + 13)).fadd_m(f(mem, dst + 13));
    st(mem, 13, v);
    let v = mul(f(mem, dst + 6), f(mem, src + 13)).fadd_m(f(mem, dst + 14));
    st(mem, 14, v);
    let v = mul(f(mem, dst + 8), f(mem, src + 14)).fadd_m(f(mem, dst + 12));
    st(mem, 12, v);
    let v = mul(f(mem, src + 14), f(mem, dst + 9)).fadd_m(f(mem, dst + 13));
    st(mem, 13, v);
    let v = mul(f(mem, dst + 10), f(mem, src + 14)).fadd_m(f(mem, dst + 14));
    st(mem, 14, v);
    for i in 12..15 {
        let v = mul(f(mem, dst + i), f32::from_bits(k::MINUS_ONE));
        st(mem, i, v);
    }
}

/// `Invert(CMatrix& src, CMatrix& dst)` (0x59B920) for distinct matrices.
pub fn invert_into(src: &CMatrix, dst: &mut CMatrix) {
    let mut mem = [0u32; 36];
    mem[..18].copy_from_slice(&src.to_words());
    mem[18..].copy_from_slice(&dst.to_words());
    invert_mem(&mut mem, 0, 18);
    *dst = CMatrix::from_words(&mem[18..]);
}

/// `Invert(m, m)` (0x59B920 with `src == dst`): the original zeroes `dst.pos` before reading
/// `src.pos`, and transposes in place, so this is not an inverse.
pub fn invert_in_place(m: &mut CMatrix) {
    let mut mem = m.to_words();
    invert_mem(&mut mem, 0, 0);
    *m = CMatrix::from_words(&mem);
}

/// `Invert(const CMatrix&)` (0x59BDD0, cdecl, hidden return pointer): `Invert(src, tmp)` into a stack
/// temporary, returned with attach fields 0. As in [`mul_matrix`], the temporary's flags/padding words
/// are uninitialised stack (port: 0).
pub fn invert(src: &CMatrix) -> CMatrix {
    let mut t = CMatrix::default();
    invert_into(src, &mut t);
    t
}
