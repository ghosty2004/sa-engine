//! A fake engine memory for host tests (of this crate and of crates that read the engine through it): sparse
//! pages of zeroes, and a game laid out like the engine's with its three pools.

use crate::NativeGame;
use crate::addr;
use crate::view::View;

/// A fake memory for host tests: sparse pages of zeroes.
#[derive(Default)]
pub struct FakeView {
    pages: std::collections::HashMap<u32, Box<[u8; 0x1000]>>,
}

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

/// The ped pool's header.
pub const PED_HEADER: u32 = 0x0100_0000;
/// The ped pool's byte map.
pub const PED_MAP: u32 = 0x0100_1000;
/// The ped pool's objects.
pub const PEDS: u32 = 0x0110_0000;
/// The vehicle pool's header.
pub const VEH_HEADER: u32 = 0x0120_0000;
/// The vehicle pool's byte map.
pub const VEH_MAP: u32 = 0x0120_1000;
/// The vehicle pool's objects.
pub const VEHS: u32 = 0x0130_0000;
/// The object pool's header.
pub const OBJ_HEADER: u32 = 0x0140_0000;
/// The object pool's byte map.
pub const OBJ_MAP: u32 = 0x0140_1000;
/// The object pool's objects.
pub const OBJS: u32 = 0x0150_0000;
/// A free address for a matrix.
pub const MATRIX: u32 = 0x0160_0000;

/// Writes `v` at `a`.
pub fn put<T: Copy>(g: &mut NativeGame<FakeView>, a: u32, v: T) {
    // SAFETY: test values are plain integers / floats.
    let bytes = unsafe { std::slice::from_raw_parts((&raw const v).cast::<u8>(), size_of::<T>()) };
    g.view.write(a, bytes);
}

/// Lays out a pool of `size` free slots whose `CPools` pointer is at `global`.
pub fn pool(g: &mut NativeGame<FakeView>, global: u32, header: u32, objects: u32, map: u32, size: i32) {
    put(g, global, header);
    put(g, header, objects);
    put(g, header + 4, map);
    put(g, header + 8, size);
    for i in 0..size as u32 {
        put(g, map + i, 0x80u8);
    }
}

/// A game with the three pools and the local player in ped slot 2 (reuse counter 5).
pub fn game() -> NativeGame<FakeView> {
    let mut g = NativeGame::with_executor(FakeView::default(), |_| 0);
    pool(&mut g, addr::PED_POOL.global, PED_HEADER, PEDS, PED_MAP, 140);
    pool(&mut g, addr::VEHICLE_POOL.global, VEH_HEADER, VEHS, VEH_MAP, 110);
    pool(&mut g, addr::OBJECT_POOL.global, OBJ_HEADER, OBJS, OBJ_MAP, 350);
    put(&mut g, PED_MAP + 2, 0x05u8);
    put(&mut g, addr::PLAYERS, PEDS + 2 * addr::PED_POOL.element);
    g
}

/// The local player's ped (slot 2 of the ped pool).
pub const LOCAL: u32 = PEDS + 2 * 0x7C4;

/// Sets `gGameState`.
pub fn set_game_state(g: &mut NativeGame<FakeView>, raw: i32) {
    put(g, addr::GAME_STATE, raw);
}

/// Sets the local player's health and position (`m_matrix` at [`MATRIX`]).
pub fn set_local_player(g: &mut NativeGame<FakeView>, health: f32, pos: [f32; 3]) {
    use addr::off;
    put(g, LOCAL + off::PED_HEALTH, health);
    put(g, LOCAL + off::MATRIX, MATRIX);
    for (i, v) in pos.into_iter().enumerate() {
        put(g, MATRIX + addr::matrix::POS + 4 * i as u32, v);
    }
}
