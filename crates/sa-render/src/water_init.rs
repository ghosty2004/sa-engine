//! `CWaterLevel::WaterLevelInitialise` (STD-6, batch `std6-render-1`). Ported from the 1.0 US bytes
//! (0x6EAE80); gta-reversed's `LoadDataFile` (a `stringstream` parser) is a reading aid only.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, x};
use crate::waterlevel::{
    BLOCK_POLY_INFO, NUM_WATER_QUADS, NUM_WATER_TRIANGLES, NUM_WATER_VERTICES, NUM_WATER_ZONE_POLYS, SEABD32_RASTER,
    TEX_SEABD32, TEX_WATERCLEAR256, TEX_WATERWAKE, WATER_CONFIGURATION, WATERCLEAR256_RASTER, WATERWAKE_RASTER,
};

orig! {
    /// `CFileMgr::OpenFile(name, mode)` (0x538900)
    fn open_file(name: GPtr<u8>, mode: GPtr<u8>) -> i32 = cdecl 0x538900 => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    fn close_file(f: i32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    fn load_line(f: i32) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `sscanf(line, fmt, 29 pointers)` (0x8220AD)
    fn sscanf29(s: GPtr<u8>, fmt: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32, a17: u32, a18: u32, a19: u32, a20: u32, a21: u32, a22: u32, a23: u32, a24: u32, a25: u32, a26: u32, a27: u32, a28: u32) -> i32 = cdecl 0x8220AD => unported;
    /// `CWaterLevel::AddWaterLevelQuad` (0x6E7EF0)
    fn add_quad(x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32, x4: i32, y4: i32, d0: u32, d1: u32, d2: u32, d3: u32, flags: u32) = cdecl 0x6E7EF0 => crate::waterlevel::add_water_level_quad;
    /// `CWaterLevel::AddWaterLevelTriangle` (0x6E7D40)
    fn add_triangle(x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32, flags: u32) = cdecl 0x6E7D40 => crate::waterlevel::add_water_level_triangle;
    /// `CWaterLevel::FillQuadsAndTrianglesList` (0x6E7B30)
    fn fill_lists() = cdecl 0x6E7B30 => crate::waterlevel::fill_quads_and_triangles_list;
    /// `CTxdStore::PushCurrentTxd` (0x7316A0)
    fn push_current_txd() = cdecl 0x7316A0 => unported;
    /// `CTxdStore::FindTxdSlot(name)` (0x731850)
    fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => unported;
    /// `CTxdStore::SetCurrentTxd(slot)` (0x7319C0)
    fn set_current_txd(slot: i32) = cdecl 0x7319C0 => unported;
    /// `CTxdStore::PopCurrentTxd` (0x7316B0)
    fn pop_current_txd() = cdecl 0x7316B0 => unported;
    /// `RwTextureRead(name, mask)` (0x7F3AC0)
    fn rw_texture_read(name: GPtr<u8>, mask: u32) -> u32 = cdecl 0x7F3AC0 => unported;
}

/// One vertex of a water.dat line: x, y, z, flow x, flow y, big waves, small waves.
type Vtx = [f32; 7];

/// The `(X, Y, CRenPar)` arguments of one vertex: `_ftol` of x and y, z, big and small waves, and the
/// flow bytes `(int8)_ftol(flow * 64.0)` (0x859A44). The upper half of the flow dword is stack residue
/// in the original (never read by `AddWaterLevelVertex`, which stores only the two bytes); here 0.
fn args(v: &Vtx) -> (i32, i32, u32, u32, u32, u32) {
    let fx = ftol32(x(v[3]).fmul_m(64.0)) as u8 as u32;
    let fy = ftol32(x(v[4]).fmul_m(64.0)) as u8 as u32;
    (ftol32(x(v[0])), ftol32(x(v[1])), v[2].to_bits(), v[5].to_bits(), v[6].to_bits(), fx | fy << 8)
}

/// `CWaterLevel::WaterLevelInitialise` (0x6EAE80): clears `m_BlockPolyInfo` (0x48 dwords) and the
/// triangle / quad / vertex / zone-poly counts, opens "DATA\\water.dat" (configuration 0) or
/// "DATA\\water1.dat" (1) (`CFileMgr::OpenFile` retried while the result is negative), then for every
/// line not starting with `;`, NUL, `*` or `p`: flags = 1; `sscanf` with 28 `%f` + `%d` (0x8720F0) or,
/// failing 29 fields, 28 `%f` (0x872098): 28 fields make a quad (`AddWaterLevelQuad`, four vertices);
/// otherwise 21 `%f` + `%d` (0x872050, result unchecked) make a triangle. Then
/// `FillQuadsAndTrianglesList`, and with the "particle" TXD current (push / pop) the textures
/// "waterclear256", "seabd32", "waterwake" are read when not loaded yet and their rasters stored.
#[hook(addr = 0x6EAE80, conv = cdecl, name = "CWaterLevel::WaterLevelInitialise", expect = "81 EC 04 01 00 00 53 55",
       unverified = "std6-render-1", route = "L2", hazards = "uninit-locals,heap-writes", recs = "new-game",
       note = "a configuration other than 0 / 1 opens an uninitialised name; a triangle line with fewer fields uses earlier lines' values")]
pub fn water_level_initialise() {
    BLOCK_POLY_INFO.ptr().cast::<[u32; 0x48]>().write([0; 0x48]);
    NUM_WATER_TRIANGLES.set(0);
    NUM_WATER_QUADS.set(0);
    NUM_WATER_VERTICES.set(0);
    NUM_WATER_ZONE_POLYS.set(0);
    let mut name = [0u8; 16];
    let src: &[u8] = match WATER_CONFIGURATION.get() {
        0 => b"DATA\\water.dat\0",
        1 => b"DATA\\water1.dat\0",
        _ => b"\0",
    };
    name[..src.len()].copy_from_slice(src);
    let f = loop {
        let f = open_file(GPtr::from_raw(name.as_mut_ptr()), GPtr::from_addr(0x859C6C));
        if f >= 0 {
            break f;
        }
    };
    NUM_WATER_TRIANGLES.set(0);
    NUM_WATER_QUADS.set(0);
    NUM_WATER_VERTICES.set(0);
    // the locals keep their values from line to line, as the original's stack slots do
    let mut store: [f32; 28] = [0.0; 28];
    let mut flags_cell: i32 = 0;
    // every access goes through these pointers (the sscanf targets)
    let vbase = GPtr::from_raw(store.as_mut_ptr());
    let fptr = GPtr::from_raw(&mut flags_cell as *mut i32);
    let vtx = |k: usize| -> Vtx { core::array::from_fn(|i| vbase.byte_add(((k * 7 + i) * 4) as isize).read()) };
    let mut line = load_line(f);
    while !line.is_null() {
        let c = line.read();
        if c != b';' && c != 0 && c != b'*' && c != b'p' {
            fptr.write(1);
            let p = |i: usize| -> u32 { vbase.addr() as u32 + 4 * i as u32 };
            let fl = fptr.addr() as u32;
            let scan = |fmt: usize, last: u32| -> i32 {
                sscanf29(
                    line, GPtr::from_addr(fmt), p(0), p(1), p(2), p(3), p(4), p(5), p(6), p(7), p(8), p(9), p(10),
                    p(11), p(12), p(13), p(14), p(15), p(16), p(17), p(18), p(19), p(20), p(21), p(22), p(23), p(24),
                    p(25), p(26), p(27), last,
                )
            };
            // the 28-float format ignores its 29th argument; the 22-field one takes the flags 22nd
            let quad = scan(0x8720F0, fl) == 0x1D || scan(0x872098, 0) == 0x1C;
            if quad {
                let (a, b, c, d) = (args(&vtx(0)), args(&vtx(1)), args(&vtx(2)), args(&vtx(3)));
                add_quad(
                    a.0, a.1, a.2, a.3, a.4, a.5, b.0, b.1, b.2, b.3, b.4, b.5, c.0, c.1, c.2, c.3, c.4, c.5, d.0, d.1, d.2,
                    d.3, d.4, d.5, fptr.read() as u32,
                );
            } else {
                let p21 = fl;
                sscanf29(
                    line, GPtr::from_addr(0x872050), p(0), p(1), p(2), p(3), p(4), p(5), p(6), p(7), p(8), p(9), p(10),
                    p(11), p(12), p(13), p(14), p(15), p(16), p(17), p(18), p(19), p(20), p21, 0, 0, 0, 0, 0, 0, 0,
                );
                let (a, b, c) = (args(&vtx(0)), args(&vtx(1)), args(&vtx(2)));
                add_triangle(
                    a.0, a.1, a.2, a.3, a.4, a.5, b.0, b.1, b.2, b.3, b.4, b.5, c.0, c.1, c.2, c.3, c.4, c.5, fptr.read() as u32,
                );
            }
        }
        line = load_line(f);
    }
    close_file(f);
    fill_lists();
    push_current_txd();
    set_current_txd(find_txd_slot(GPtr::from_addr(0x85C648)));
    for (tex, raster, n) in [
        (&TEX_WATERCLEAR256, &WATERCLEAR256_RASTER, 0x872040usize),
        (&TEX_SEABD32, &SEABD32_RASTER, 0x872038),
        (&TEX_WATERWAKE, &WATERWAKE_RASTER, 0x87202C),
    ] {
        if tex.get() == 0 {
            tex.set(rw_texture_read(GPtr::from_addr(n), 0));
        }
        raster.set(g::<u32>(tex.get() as usize).read());
    }
    pop_current_txd();
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_WATER_LEVEL_INITIALISE];
