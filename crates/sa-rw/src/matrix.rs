//! `RwMatrixTag` (0x40 bytes in the original: right, flags, up, pad, at, pad, pos, pad).

/// An affine RW matrix. RW names: `right` (x axis), `up` (y axis), `at` (z axis), `pos`.
/// A point transforms as `p' = right * p.x + up * p.y + at * p.z + pos` (row vectors).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwMatrix {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub at: [f32; 3],
    pub pos: [f32; 3],
    /// `RwMatrix::flags`, kept as read.
    pub flags: u32,
}

impl Default for RwMatrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl RwMatrix {
    pub const IDENTITY: RwMatrix = RwMatrix {
        right: [1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
        at: [0.0, 0.0, 1.0],
        pos: [0.0, 0.0, 0.0],
        flags: 0,
    };

    /// `self` applied after `parent`'s transform is *not* what this computes: it returns
    /// `self * parent`, i.e. the LTM of a frame whose modelling matrix is `self` and whose
    /// parent LTM is `parent` (RW `_rwFrameSyncHierarchyLTM`: `ltm = modelling × parent.ltm`).
    pub fn mul(&self, parent: &RwMatrix) -> RwMatrix {
        let t = |v: [f32; 3]| parent.transform_vector(v);
        let p = parent.transform_point(self.pos);
        RwMatrix { right: t(self.right), up: t(self.up), at: t(self.at), pos: p, flags: self.flags }
    }

    pub fn transform_vector(&self, v: [f32; 3]) -> [f32; 3] {
        let mut o = [0.0; 3];
        for (i, out) in o.iter_mut().enumerate() {
            *out = self.right[i] * v[0] + self.up[i] * v[1] + self.at[i] * v[2];
        }
        o
    }

    pub fn transform_point(&self, v: [f32; 3]) -> [f32; 3] {
        let r = self.transform_vector(v);
        [r[0] + self.pos[0], r[1] + self.pos[1], r[2] + self.pos[2]]
    }

    /// Column-major 4x4 (as WGSL `mat4x4<f32>` expects: columns right, up, at, pos).
    pub fn to_cols(&self) -> [[f32; 4]; 4] {
        [
            [self.right[0], self.right[1], self.right[2], 0.0],
            [self.up[0], self.up[1], self.up[2], 0.0],
            [self.at[0], self.at[1], self.at[2], 0.0],
            [self.pos[0], self.pos[1], self.pos[2], 1.0],
        ]
    }

    /// Largest axis length (for scaling bounding spheres).
    pub fn max_scale(&self) -> f32 {
        let l = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        l(self.right).max(l(self.up)).max(l(self.at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_composes_child_then_parent() {
        let mut parent = RwMatrix::IDENTITY;
        parent.pos = [10.0, 0.0, 0.0];
        // parent rotated 90 degrees about z: x -> y
        parent.right = [0.0, 1.0, 0.0];
        parent.up = [-1.0, 0.0, 0.0];
        let mut child = RwMatrix::IDENTITY;
        child.pos = [1.0, 0.0, 0.0];
        let ltm = child.mul(&parent);
        assert_eq!(ltm.pos, [10.0, 1.0, 0.0]);
        assert_eq!(ltm.transform_point([1.0, 0.0, 0.0]), [10.0, 2.0, 0.0]);
    }
}
