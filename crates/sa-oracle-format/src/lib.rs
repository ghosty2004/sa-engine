//! Platform-independent oracle data formats shared by `sa-hook` (the in-process DLL) and the host-side
//! harness (`sa-oracle`):
//!
//! - [`fnv`]: the hash function every frame-state hash uses (FNV-1a 64, stable forever).
//! - [`subsys`]: the per-subsystem frame-state hash definitions (ids, names, what each one covers).
//! - [`trace`]: the per-frame hash trace file (`SATRACE`) and divergence search.
//! - [`recording`]: the versioned input recording (`SAREC`): start conditions + per-tick pad input.
//! - [`program`]: scripted input programs (text) that generate pad input per frame/tick.
//! - [`pad`]: the raw input block captured/injected per input tick (original memory layouts).

pub mod fnv;
pub mod l2;
pub mod normalise;
pub mod pad;
pub mod program;
pub mod recording;
pub mod subsys;
pub mod trace;

/// Little-endian cursor used by the binary readers.
pub(crate) struct Reader<'a> {
    pub buf: &'a [u8],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.buf.len())
            .ok_or_else(|| format!("truncated at byte {} (wanted {n} more)", self.pos))?;
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }
    pub fn u32(&mut self) -> Result<u32, String> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub fn u64(&mut self) -> Result<u64, String> {
        let b = self.bytes(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }
    pub fn at_end(&self) -> bool {
        self.pos == self.buf.len()
    }
}
