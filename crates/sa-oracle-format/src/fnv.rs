//! FNV-1a 64-bit. Every frame-state hash and data-file fingerprint uses this exact function, so traces
//! from different builds and from the host harness stay comparable. Never change it; add a new trace
//! version instead.

pub const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
pub const PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fnv64(pub u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Fnv64(OFFSET)
    }
}

impl Fnv64 {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline]
    pub fn write(&mut self, bytes: &[u8]) {
        let mut h = self.0;
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(PRIME);
        }
        self.0 = h;
    }
    #[inline]
    pub fn u8(&mut self, v: u8) {
        self.write(&[v]);
    }
    #[inline]
    pub fn u16(&mut self, v: u16) {
        self.write(&v.to_le_bytes());
    }
    #[inline]
    pub fn u32(&mut self, v: u32) {
        self.write(&v.to_le_bytes());
    }
    #[inline]
    pub fn u64(&mut self, v: u64) {
        self.write(&v.to_le_bytes());
    }
    pub fn finish(&self) -> u64 {
        self.0
    }
}

/// One-shot hash of a byte slice.
pub fn hash(bytes: &[u8]) -> u64 {
    let mut h = Fnv64::new();
    h.write(bytes);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_vectors() {
        // Published FNV-1a 64 test vectors.
        assert_eq!(hash(b""), 0xcbf29ce484222325);
        assert_eq!(hash(b"a"), 0xaf63dc4c8601ec8c);
        assert_eq!(hash(b"foobar"), 0x85944171f73967e8);
        let mut h = Fnv64::new();
        h.write(b"foo");
        h.write(b"bar");
        assert_eq!(h.finish(), hash(b"foobar"));
        let mut h = Fnv64::new();
        h.u32(0x6f6f66);
        assert_eq!(h.finish(), hash(&[0x66, 0x6f, 0x6f, 0]));
    }
}
