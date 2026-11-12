//! Memory access of the backend: the engine's memory ([`EngineView`]) or a fake in tests.

use sa_samp_api::{Matrix, Vec3};

use crate::addr;

/// Byte-level access to the engine's memory at 32-bit addresses (original layouts, see the crate docs).
///
/// Only the game thread uses it (the adapter contract), and only for addresses the engine's own state
/// says are live: a global, or an object whose pool slot was validated first.
pub trait View {
    /// Reads `buf.len()` bytes at `addr`.
    fn read(&self, addr: u32, buf: &mut [u8]);
    /// Writes `data` at `addr`.
    fn write(&mut self, addr: u32, data: &[u8]);

    /// A byte.
    fn read_u8(&self, a: u32) -> u8 {
        let mut b = [0; 1];
        self.read(a, &mut b);
        b[0]
    }
    /// A little-endian u16.
    fn read_u16(&self, a: u32) -> u16 {
        let mut b = [0; 2];
        self.read(a, &mut b);
        u16::from_le_bytes(b)
    }
    /// A little-endian i16.
    fn read_i16(&self, a: u32) -> i16 {
        self.read_u16(a) as i16
    }
    /// A little-endian u32.
    fn read_u32(&self, a: u32) -> u32 {
        let mut b = [0; 4];
        self.read(a, &mut b);
        u32::from_le_bytes(b)
    }
    /// A little-endian i32.
    fn read_i32(&self, a: u32) -> i32 {
        self.read_u32(a) as i32
    }
    /// An f32.
    fn read_f32(&self, a: u32) -> f32 {
        f32::from_bits(self.read_u32(a))
    }
    /// A `CVector` (three f32).
    fn read_vec3(&self, a: u32) -> Vec3 {
        Vec3::new(self.read_f32(a), self.read_f32(a + 4), self.read_f32(a + 8))
    }
    /// A `CMatrix` / `RwMatrix` body: right +0x0, forward +0x10, up +0x20, position +0x30.
    fn read_matrix(&self, a: u32) -> Matrix {
        Matrix {
            right: self.read_vec3(a + addr::matrix::RIGHT),
            forward: self.read_vec3(a + addr::matrix::FORWARD),
            up: self.read_vec3(a + addr::matrix::UP),
            pos: self.read_vec3(a + addr::matrix::POS),
        }
    }
    /// Writes a little-endian u32.
    fn write_u32(&mut self, a: u32, v: u32) {
        self.write(a, &v.to_le_bytes());
    }
}

/// The engine's memory in this process: globals in the legacy address arena, objects in the engine's pools
/// (32-bit targets: native sa-game on i686, wasm32 with the layouts in linear memory).
#[derive(Clone, Copy, Debug, Default)]
pub struct EngineView;

impl View for EngineView {
    fn read(&self, addr: u32, buf: &mut [u8]) {
        // Callers never pass 0 (null objects are filtered before any field read); a wild address here would
        // be a backend bug, as it would be in the port that owns the state.
        debug_assert_ne!(addr, 0, "sa-samp read at address 0");
        for (i, b) in buf.iter_mut().enumerate() {
            *b = sa_core::mem::GPtr::<u8>::from_addr(addr.wrapping_add(i as u32) as usize).read();
        }
    }

    fn write(&mut self, addr: u32, data: &[u8]) {
        debug_assert_ne!(addr, 0, "sa-samp write at address 0");
        for (i, b) in data.iter().enumerate() {
            sa_core::mem::GPtr::<u8>::from_addr(addr.wrapping_add(i as u32) as usize).write(*b);
        }
    }
}

/// A pool of the engine (`CPool<T>`: objects +0x0, byte map +0x4, size +0x8; one byte per slot, bit 7 =
/// free, bits 0..6 = reuse counter), as `sa_core::pool` ports it.
#[derive(Clone, Copy, Debug)]
pub struct Pool {
    /// Address of the `CPools::ms_p*Pool` pointer.
    pub global: u32,
    /// Slot size (`sizeof` of the pool's widest type).
    pub element: u32,
}

impl Pool {
    /// The pool header address, or `None` before `CPools::Initialise`.
    fn header<V: View + ?Sized>(&self, v: &V) -> Option<(u32, u32, i32)> {
        let pool = v.read_u32(self.global);
        if pool == 0 {
            return None;
        }
        let (objects, map, size) = (v.read_u32(pool), v.read_u32(pool + 4), v.read_i32(pool + 8));
        (objects != 0 && map != 0 && size > 0).then_some((objects, map, size))
    }

    /// The object of reference `r` (`CPool::GetAtRef`: slot `r >> 8` whose map byte equals `r & 0xFF`),
    /// bounds-checked (the original is not; the adapter must not read outside the pool).
    pub fn at_ref<V: View + ?Sized>(&self, v: &V, r: u32) -> Option<u32> {
        let (objects, map, size) = self.header(v)?;
        let slot = (r as i32) >> 8;
        if slot < 0 || slot >= size {
            return None;
        }
        // Same test as `sa_core::pool::at_ref_slot`; the byte equals the counter only for a used slot
        // (the counter never has bit 7 set).
        (v.read_u8(map + slot as u32) == r as u8).then(|| objects + slot as u32 * self.element)
    }

    /// The reference of the object at `obj` (`CPool::GetRef`: `slot << 8 | map[slot]`), or `None` when `obj`
    /// is not the start of a used slot of this pool.
    pub fn ref_of<V: View + ?Sized>(&self, v: &V, obj: u32) -> Option<u32> {
        let (objects, map, size) = self.header(v)?;
        if obj < objects || !(obj - objects).is_multiple_of(self.element) {
            return None;
        }
        let slot = (obj - objects) / self.element;
        if slot as i64 >= i64::from(size) {
            return None;
        }
        let b = v.read_u8(map + slot);
        (b & sa_core::pool::FREE == 0).then_some((slot << 8) | u32::from(b))
    }
}

/// A fake memory for host tests: sparse pages of zeroes.
#[cfg(test)]
#[derive(Default)]
pub struct FakeView {
    pages: std::collections::HashMap<u32, Box<[u8; 0x1000]>>,
}

#[cfg(test)]
impl View for FakeView {
    fn read(&self, addr: u32, buf: &mut [u8]) {
        for (i, b) in buf.iter_mut().enumerate() {
            let a = addr.wrapping_add(i as u32);
            *b = self
                .pages
                .get(&(a & !0xFFF))
                .map_or(0, |p| p[(a & 0xFFF) as usize]);
        }
    }

    fn write(&mut self, addr: u32, data: &[u8]) {
        for (i, b) in data.iter().enumerate() {
            let a = addr.wrapping_add(i as u32);
            self.pages
                .entry(a & !0xFFF)
                .or_insert_with(|| Box::new([0; 0x1000]))[(a & 0xFFF) as usize] = *b;
        }
    }
}
