//! MVP-C writer batch `mvpc-render-1`: boot/loading-time RENDER functions (`CDraw::SetFOV`, the plant
//! manager's list set-up, static constructors of the scripted 2D effects and the inform-friends queue).
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid. Every hook is unverified.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use crate::common::wr;

orig! {
    /// `CPlantSurfPropMgr::Initialise()` (0x5DD6C0)
    pub fn plant_surf_prop_initialise() -> u8 = cdecl 0x5DD6C0 => unported;
    /// CRT `eh vector constructor iterator(arr, size, n, ctor, dtor)` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `FUN_006f9eb0` (0x6F9EB0): resets a `CScripted2dEffects` pair list
    pub fn effect_pairs_reset_call(this: GPtr<u8>) -> u32 = thiscall 0x6F9EB0 => effect_pairs_reset;
}

fn g<T: Copy>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

/// `CDraw::SetFOV(float)` (0x6FF410, cdecl): `CDraw::ms_fFOV` (0x8D5038) = fov. The store sits in a
/// SecuROM fragment (0x401025, the target address de-obfuscated from `[0x15605DC] ^ [0x419F03]`);
/// the protection reads the entry's `jmp` bytes (0x6FF415) as data.
#[hook(addr = 0x6FF410, conv = cdecl, name = "CDraw::SetFOV", expect = "8B 44 24 04 E9 0C 1C D0 FF",
       route = "L2", hazards = "entry-protection-read", recs = "grove-walk",
       note = "protection code reads 0x6FF415 (inside the patched entry) as a key")]
pub fn draw_set_fov(fov: u32) {
    g::<u32>(0x8D5038).write(fov);
}

/// `(missed by ghidra: data-code-immediate)` (0x4AC0D0, thiscall): the
/// `CInformFriendsEventQueue::ms_informFriendsEvents` element constructor: +0 = +4 = 0, +8 = -1;
/// returns this.
#[hook(addr = 0x4AC0D0, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 C7 00 00 00 00 00",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn inform_friends_event_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0xFFFF_FFFF);
    this
}

/// `(missed by ghidra: data-code-immediate)` (0x6FA6D0, thiscall): the `CScripted2dEffects::ms_userLists`
/// element constructor: dwords +0..+0xC and +0x10..+0x1C = -1, byte +0x20 = 0; returns this.
#[hook(addr = 0x6FA6D0, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 33 D2 83 C9 FF",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn scripted_2d_user_list_ctor(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..4isize {
        wr::<u32>(this, 4 * i, 0xFFFF_FFFF);
        wr::<u32>(this, 0x10 + 4 * i, 0xFFFF_FFFF);
    }
    wr::<u8>(this, 0x20, 0);
    this
}

/// `(missed by ghidra: data-code-immediate)` (0x6FA790, thiscall): the
/// `CScripted2dEffects::ms_effectPairs` element constructor: `eh vector constructor iterator(this + 4,
/// 0x24, 4, 0x6FA770, 0x6F9E30)`, then `FUN_006f9eb0(this)`; returns this.
#[hook(addr = 0x6FA790, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "56 68 30 9E 6F 00",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn scripted_2d_effect_pairs_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this.byte_add(4), 0x24, 4, 0x6FA770, 0x6F9E30);
    effect_pairs_reset_call(this);
    this
}

/// `FUN_006f9eb0` (0x6F9EB0, thiscall): four 0x24-byte entries from +4: dwords +4..+0x20 of each = -1
/// (offsets as the original lists them), byte +0x24 of each = 0, +0 = 0. Returns -1 in EAX; ECX is left
/// unchanged (its caller 0x6FA740 relies on it).
#[hook(addr = 0x6F9EB0, conv = thiscall, name = "FUN_006f9eb0", expect = "83 C8 FF 33 D2", preserve = "ecx",
       unverified = "mvpc-render-1", route = "L2", hazards = "preserved-regs",
       note = "resets a CScripted2dEffects effect-pair record; the caller 0x6FA740 keeps ECX across the call (0x6FA75A)")]
pub fn effect_pairs_reset(this: GPtr<u8>) -> u32 {
    for e in 0..4isize {
        let b = this.byte_add(0x24 * e);
        for o in [4, 8, 0xC, 0x10, 0x14, 0x18, 0x1C, 0x20] {
            if o != 0x1C {
                wr::<u32>(b, o, 0xFFFF_FFFF);
            }
        }
        wr::<u8>(b, 0x24, 0);
    }
    wr::<u32>(this, 0, 0);
    0xFFFF_FFFF
}

/// `CPlantMgr::ReloadConfig()` (0x5DD780, cdecl, bool in AL): `CPlantSurfPropMgr::Initialise()` false →
/// returns its AL. Otherwise the 256 `CPlantLocTri` (0x54 bytes at 0xC03A48) get their 12 position
/// dwords, +0x40, word +0x44 and byte +0x46 cleared and form a doubly linked list (+0x4C next, +0x50
/// prev; ends null); the four list heads 0xC03988..0xC03994 = 0, free head 0xC03984 = 0xC03A48. The 40
/// `CPlantColEntEntry` (0x14 bytes at 0xC08E48): +0, +4, word +8 cleared, linked through +0xC / +0x10;
/// 0xC0399C = 0, free head 0xC03998 = 0xC08E48. Returns 1.
#[hook(addr = 0x5DD780, conv = cdecl, name = "CPlantMgr::ReloadConfig", expect = "83 EC 30 E8 38 FF FF FF",
       route = "split", hazards = "ax-al-return", recs = "grove-walk",
       note = "loads PLANTS.DAT through CPlantSurfPropMgr::Initialise (file effects)")]
pub fn plant_mgr_reload_config() -> u8 {
    let r = plant_surf_prop_initialise();
    if r == 0 {
        return r;
    }
    for a in [0xC03988, 0xC0398C, 0xC03990, 0xC03994] {
        g::<u32>(a).write(0);
    }
    g::<u32>(0xC03984).write(0xC03A48);
    for i in 0..0x100usize {
        let e = g::<u8>(0xC03A48 + i * 0x54);
        for o in 0..12isize {
            wr::<u32>(e, 4 * o, 0);
        }
        wr::<u8>(e, 0x46, 0);
        wr::<u32>(e, 0x40, 0);
        wr::<u16>(e, 0x44, 0);
        if i == 0 {
            wr::<u32>(e, 0x50, 0);
        } else {
            wr::<u32>(e, 0x50, (e.addr() - 0x54) as u32);
            if i == 0xFF {
                wr::<u32>(e, 0x4C, 0);
                continue;
            }
        }
        wr::<u32>(e, 0x4C, (e.addr() + 0x54) as u32);
    }
    g::<u32>(0xC0399C).write(0);
    g::<u32>(0xC03998).write(0xC08E48);
    for i in 0..0x28usize {
        let e = g::<u8>(0xC08E48 + i * 0x14);
        wr::<u32>(e, 0, 0);
        wr::<u32>(e, 4, 0);
        wr::<u16>(e, 8, 0);
        if i == 0 {
            wr::<u32>(e, 0x10, 0);
        } else {
            wr::<u32>(e, 0x10, (e.addr() - 0x14) as u32);
            if i == 0x27 {
                wr::<u32>(e, 0xC, 0);
                continue;
            }
        }
        wr::<u32>(e, 0xC, (e.addr() + 0x14) as u32);
    }
    1
}

orig! {
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn txd_push_current() = cdecl 0x7316A0 => sa_stream::txdstore::push_current_txd;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn txd_pop_current() = cdecl 0x7316B0 => sa_stream::txdstore::pop_current_txd;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn txd_find_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => sa_stream::txdstore::find_txd_slot_name;
    /// `CTxdStore::SetCurrentTxd(int)` (0x7319C0)
    pub fn txd_set_current(slot: i32) = cdecl 0x7319C0 => sa_stream::txdstore::set_current_txd;
    /// `RwTextureRead(const char* name, const char* mask)` (0x7F3AC0)
    pub fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// `RwRasterDestroy(RwRaster*)` (0x7FB020)
    pub fn rw_raster_destroy(r: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
    /// `C3dMarkers::Init()` (0x724E40)
    pub fn markers_init() = cdecl 0x724E40 => crate::markers::init;
    /// `CCheckpoints::Init()` (0x722880)
    pub fn checkpoints_init() = cdecl 0x722880 => crate::mvpa_render1::checkpoints_init;
}

/// `"particle"` (0x85C648).
const PARTICLE_TXD: usize = 0x85C648;

/// `CShadows::Init()` (0x706CD0, cdecl): with the "particle" TXD current (push / set / pop) reads the 12
/// shadow textures (names 0x872710 .. 0x872684, null mask) into 0xC403E0 .. 0xC4040C; the 24-entry
/// word index table at 0xC403A8 (the shadow quads' triangle indices); the 48 static shadows (0x40 bytes
/// from 0xC4A030): +0 and +4 = 0; the poly-bunch pool: free head 0xC403D8 = 0xC40DF0 and each bunch's
/// +0x54 (from 0xC40E44, 0x68 apart) = bunch + 0x68 (the next), the last (0xC4A01C) = 0; byte +0x32 of
/// the 48 permanent shadows (0x38 apart from 0xC4AC62) = 0.
#[hook(addr = 0x706CD0, conv = cdecl, name = "CShadows::Init", expect = "53 E8 CA A9 02 00",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "reads 12 textures (one-time initialiser)")]
pub fn shadows_init() {
    txd_push_current();
    txd_set_current(txd_find_slot(g(PARTICLE_TXD)));
    const NAMES: [usize; 12] = [
        0x872710, 0x872704, 0x8726F8, 0x8726EC, 0x8726DC, 0x8726D0, 0x8726C4, 0x8726B8, 0x8726A8, 0x8726A0, 0x872694,
        0x872684,
    ];
    for (i, n) in NAMES.iter().enumerate() {
        let t = rw_texture_read(g(*n), GPtr::NULL);
        g::<GPtr<u8>>(0xC403E0 + 4 * i).write(t);
    }
    txd_pop_current();
    const IDX: [u16; 24] = [0, 2, 1, 0, 3, 2, 0, 4, 3, 0, 5, 4, 0, 6, 5, 0, 7, 6, 0, 8, 7, 0, 9, 8];
    for (i, v) in IDX.iter().enumerate() {
        g::<u16>(0xC403A8 + 2 * i).write(*v);
    }
    let mut a = 0xC4A034usize;
    while a < 0xC4AC34 {
        g::<u32>(a - 4).write(0);
        g::<u32>(a).write(0);
        a += 0x40;
    }
    g::<u32>(0xC403D8).write(0xC40DF0);
    let mut a = 0xC40E44usize;
    while a < 0xC4A01C {
        g::<u32>(a).write((a + 0x14) as u32);
        a += 0x68;
    }
    g::<u32>(0xC4A01C).write(0);
    let mut a = 0xC4AC62usize;
    while a < 0xC4B6E2 {
        g::<u8>(a).write(0);
        a += 0x38;
    }
}

/// `CSpecialFX::Init()` (0x7268F0, cdecl): clears byte +0 of the 16 `CBulletTrace`-sized records
/// (0x2C apart from 0xC7C760), of 0xC80258.. (0x1C apart) and 0xC802E8.. (0x30 apart); the 12-word
/// index table at 0xC7C6B8 (0,1,2,1,3,2,0,2,1,1,2,3); the two 0x48-byte vertex records: +0x644/+0x648 =
/// 0, 0x668 = 1.0, 0x66C = 0, 0x68C/0x690 = 0, 0x6B0 = 1.0, 0x6B4 = 0 (all at 0xC7C6xx); dword 0 of the
/// four 0x54-byte records from 0xC7CA08; 0xC7C6FC = 0xC7C6F8 = 0; `C3dMarkers::Init`,
/// `CCheckpoints::Init`; bytes 0xC7C70C/0xC7C70D/0xC7C714 and dword 0xC7C710 = 0. With "particle"
/// current: the texture 0xC7C718 is read (0x872A04) when null. The rasters 0xC7C71C and 0xC7C720 are
/// destroyed when set (and cleared); 0xC7C724 = 0xC7C618 = 0.
#[hook(addr = 0x7268F0, conv = cdecl, name = "CSpecialFX::Init", expect = "53 B8 60 C7 C7 00",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "reads a texture and destroys rasters (one-time initialiser)")]
pub fn special_fx_init() {
    for (start, end, step) in [(0xC7C760usize, 0xC7CA20usize, 0x2Cusize), (0xC80258, 0xC802E4, 0x1C), (0xC802E8, 0xC803D8, 0x30)] {
        let mut a = start;
        while a < end {
            g::<u8>(a).write(0);
            a += step;
        }
    }
    const IDX: [u16; 12] = [0, 1, 2, 1, 3, 2, 0, 2, 1, 1, 2, 3];
    for (i, v) in IDX.iter().enumerate() {
        g::<u16>(0xC7C6B8 + 2 * i).write(*v);
    }
    for (a, v) in [
        (0xC7C644usize, 0u32),
        (0xC7C648, 0),
        (0xC7C668, 0x3F80_0000),
        (0xC7C66C, 0),
        (0xC7C68C, 0),
        (0xC7C690, 0),
        (0xC7C6B0, 0x3F80_0000),
        (0xC7C6B4, 0),
    ] {
        g::<u32>(a).write(v);
    }
    let mut a = 0xC7CA08usize;
    while a < 0xC7CB58 {
        g::<u32>(a).write(0);
        a += 0x54;
    }
    g::<u32>(0xC7C6FC).write(0);
    g::<u32>(0xC7C6F8).write(0);
    markers_init();
    checkpoints_init();
    g::<u8>(0xC7C70C).write(0);
    g::<u8>(0xC7C70D).write(0);
    g::<u32>(0xC7C710).write(0);
    g::<u8>(0xC7C714).write(0);
    txd_push_current();
    txd_set_current(txd_find_slot(g(PARTICLE_TXD)));
    if g::<GPtr<u8>>(0xC7C718).read().is_null() {
        g::<GPtr<u8>>(0xC7C718).write(rw_texture_read(g(0x872A04), GPtr::NULL));
    }
    txd_pop_current();
    for a in [0xC7C71Cusize, 0xC7C720] {
        let r = g::<GPtr<u8>>(a).read();
        if !r.is_null() {
            rw_raster_destroy(r);
            g::<u32>(a).write(0);
        }
    }
    g::<u32>(0xC7C724).write(0);
    g::<u32>(0xC7C618).write(0);
}

/// The camera position the light registrations measure from: `TheCamera.m_matrix` (0xB6F03C) + 0x30
/// when set, else `TheCamera.m_placement.pos` (0xB6F02C).
fn camera_pos() -> GPtr<f32> {
    let m = g::<GPtr<u8>>(0xB6F03C).read();
    if m.is_null() { g(0xB6F02C) } else { m.byte_add(0x30).cast() }
}

/// `sqrt(dz*dz + dy*dy + dx*dx)` on the x87 stack, `d = pos - cam` (`fsub` from the f32 arguments).
fn cam_dist(x: u32, y: u32, z: u32) -> X87 {
    let c = camera_pos();
    let dx = X87::from_f32(f32::from_bits(x)).fsub_m(c.read());
    let dy = X87::from_f32(f32::from_bits(y)).fsub_m(c.wrapping_index(1).read());
    let dz = X87::from_f32(f32::from_bits(z)).fsub_m(c.wrapping_index(2).read());
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// `CBrightLights::RegisterOne(CVector pos, CVector up, CVector right, CVector front, u8 colour, u8 a,
/// u8 b, u8 c)` (0x724770, cdecl; vectors by value): nothing for colour 0 or with 32 lights
/// (0xC7C6FC). Otherwise `d = |pos - cam| / [0xB6F118]` (x87, `fsqrt`) is stored (f32) into the next
/// slot's +0x30 (0x38-byte records at 0xC7CB58) before the test; `d <= 60.0` (ordered) registers: the
/// 12 floats → +0..+0x2C, colour → +0x34, a/b/c → +0x35..+0x37, count + 1.
#[hook(addr = 0x724770, conv = cdecl, name = "CBrightLights::RegisterOne", expect = "53 8A 5C 24 38",
       route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "fsqrt of the camera distance")]
#[allow(clippy::too_many_arguments)]
pub fn bright_lights_register_one(
    px: u32, py: u32, pz: u32, ux: u32, uy: u32, uz: u32, rx: u32, ry: u32, rz: u32, fx: u32, fy: u32, fz: u32,
    colour: u32, a: u32, b: u32, c: u32,
) {
    let col = colour as u8;
    let n = g::<i32>(0xC7C6FC).read();
    if col == 0 || n >= 0x20 {
        return;
    }
    let e = g::<u8>(0xC7CB58 + (n as usize) * 0x38);
    let d = cam_dist(px, py, pz).fdiv_m(g::<f32>(0xB6F118).read());
    wr::<f32>(e, 0x30, d.to_f32());
    if crate::common::jp(d, X87::from_f32(f32::from_bits(0x4270_0000)), 0x41) {
        return;
    }
    for (i, v) in [px, py, pz, ux, uy, uz, rx, ry, rz, fx, fy, fz].iter().enumerate() {
        wr::<u32>(e, 4 * i as isize, *v);
    }
    wr::<u8>(e, 0x34, col);
    wr::<u8>(e, 0x35, a as u8);
    wr::<u8>(e, 0x36, b as u8);
    wr::<u8>(e, 0x37, c as u8);
    g::<i32>(0xC7C6FC).write(n + 1);
}

/// `CShinyTexts::RegisterOne(CVector c0, c1, c2, c3, float u0, v0, u1, v1, u2, v2, u3, v3, u8 type, u8
/// r, u8 g, u8 b, float maxDist)` (0x724B60, cdecl; vectors by value): nothing with 32 texts
/// (0xC7C6F8). `d = |c0 - cam|` (x87, `fsqrt`) is stored (f32) into the next slot's +0x50 (0x58-byte
/// records at 0xC7D258) before the test; `d <= maxDist` (ordered) registers: the 20 floats → +0..+0x4C,
/// type/r/g/b → +0x54..+0x57; when `h = maxDist * 0.5` is below the stored d (ordered) the colour fades:
/// `m = 1 - (d - h) / h`, each of r/g/b = `_ftol(c * m)` (low byte). Count + 1.
#[hook(addr = 0x724B60, conv = cdecl, name = "CShinyTexts::RegisterOne", expect = "51 57 8B 3D F8 C6 C7 00",
       unverified = "mvpc-render-1", route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "fsqrt of the camera distance")]
#[allow(clippy::too_many_arguments)]
pub fn shiny_texts_register_one(
    x0: u32, y0: u32, z0: u32, x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32, x3: u32, y3: u32, z3: u32,
    u0: u32, v0: u32, u1: u32, v1: u32, u2: u32, v2: u32, u3: u32, v3: u32, ty: u32, r: u32, gr: u32, b: u32,
    max_dist: u32,
) {
    let n = g::<i32>(0xC7C6F8).read();
    if n >= 0x20 {
        return;
    }
    let e = g::<u8>(0xC7D258 + (n as usize) * 0x58);
    let d = cam_dist(x0, y0, z0);
    wr::<f32>(e, 0x50, d.to_f32());
    let maxd = f32::from_bits(max_dist);
    if crate::common::jp(d, X87::from_f32(maxd), 0x41) {
        return;
    }
    let h = X87::from_f32(maxd).fmul_m(f32::from_bits(0x3F00_0000));
    let vals = [x0, y0, z0, x1, y1, z1, x2, y2, z2, x3, y3, z3, u0, v0, u1, v1, u2, v2, u3, v3];
    for (i, v) in vals.iter().enumerate() {
        wr::<u32>(e, 4 * i as isize, *v);
    }
    wr::<u8>(e, 0x54, ty as u8);
    wr::<u8>(e, 0x55, r as u8);
    wr::<u8>(e, 0x56, gr as u8);
    wr::<u8>(e, 0x57, b as u8);
    let ds = crate::common::rd::<f32>(e, 0x50);
    if !crate::common::jp(h, X87::from_f32(ds), 5) {
        let m = X87::from_f32(ds).fsub(h);
        let m = m.fdiv(h).fsubr_m(f32::from_bits(0x3F80_0000));
        for o in [0x55isize, 0x56, 0x57] {
            let c = X87::from_i32(i32::from(crate::common::rd::<u8>(e, o)));
            wr::<u8>(e, o, crate::common::ftol32(c.fmul(m)) as u8);
        }
    }
    g::<i32>(0xC7C6F8).write(n + 1);
}

fn kf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `CWindModifiers::FindWindModifier(CVector pos, float* outX, float* outY)` (0x72C950, cdecl, bool in
/// AL; pos by value). Over the `[0xC81450]` modifiers (0x14 bytes from 0xC81458: x, y, z, int type,
/// strength) of type 1: `a = |pos.z + 15 - m.z|` (x87, `fchs` when below 0) must be < 40 (ordered);
/// `f1 = 1 - a * 0.025`; `dx`, `dy` (f32), `d = sqrt((dz*dz + dy*dy) + dx*dx)` (stored f32) < 50;
/// `f2 = 1` for d < 20 else `1 - (d - 20) * (1/30)`; `S = f1 * (f2 * strength) * 0.2`; the last such
/// modifier sets `wx = dx * (1/d) * S`, `wy = (1/d) * dy * S` (f32). Any match: `R = ((rand() & 0x1F) -
/// 16) * 0.0005 + 1`, `*outX += wx * R`, `*outY += R * wy`, 1; else 0.
#[hook(addr = 0x72C950, conv = cdecl, name = "CWindModifiers::FindWindModifier", expect = "83 EC 18 A1 50 14 C8 00",
       route = "L2", hazards = "x87-transcendental,ax-al-return", recs = "grove-walk",
       note = "one rand() call when a modifier matches; fsqrt")]
pub fn find_wind_modifier(x: u32, y: u32, z: u32, out_x: GPtr<f32>, out_y: GPtr<f32>) -> u8 {
    let (x, y, z) = (kf(x), kf(y), kf(z));
    let n = g::<i32>(0xC81450).read();
    let mut wx = 0f32;
    let mut wy = 0f32;
    let mut found = false;
    let mut i = 0;
    while i < n {
        let m = g::<u8>(0xC81458 + (i as usize) * 0x14);
        i += 1;
        if crate::common::rd::<i32>(m, 0xC) != 1 {
            continue;
        }
        let mz = crate::common::rd::<f32>(m, 8);
        let mut a = X87::from_f32(z).fadd_m(kf(0x4170_0000)).fsub_m(mz);
        if !crate::common::jp(a, X87::zero(), 5) {
            a = a.fchs();
        }
        if crate::common::jp(a, X87::from_f32(kf(0x4220_0000)), 5) {
            continue;
        }
        let f1 = a.fmul_m(kf(0x3CCC_CCCD)).fsubr_m(kf(0x3F80_0000));
        let dx = X87::from_f32(x).fsub_m(crate::common::rd::<f32>(m, 0)).to_f32();
        let dy = X87::from_f32(y).fsub_m(crate::common::rd::<f32>(m, 4)).to_f32();
        let dz = X87::from_f32(z).fsub_m(mz);
        let dv = dz.fmul(dz).fadd(X87::from_f32(dy).fmul_m(dy)).fadd(X87::from_f32(dx).fmul_m(dx)).fsqrt();
        let d = dv.to_f32();
        if crate::common::jp(dv, X87::from_f32(kf(0x4248_0000)), 5) {
            continue;
        }
        let f2 = if !crate::common::jp(X87::from_f32(d), X87::from_f32(kf(0x41A0_0000)), 5) {
            X87::from_f32(kf(0x3F80_0000))
        } else {
            X87::from_f32(d).fsub_m(kf(0x41A0_0000)).fmul_m(kf(0x3D08_8889)).fsubr_m(kf(0x3F80_0000))
        };
        let sv = f1.fmul(f2.fmul_m(crate::common::rd::<f32>(m, 0x10))).fmul_m(kf(0x3E4C_CCCD));
        found = true;
        let inv = X87::from_f32(kf(0x3F80_0000)).fdiv_m(d);
        wx = X87::from_f32(dx).fmul(inv).fmul(sv).to_f32();
        wy = inv.fmul_m(dy).fmul(sv).to_f32();
    }
    if !found {
        return 0;
    }
    let r = sa_core::orig::at_site(0x72CA7B, sa_core::crt::rand);
    let rv = X87::from_i32((r & 0x1F) - 0x10).fmul_m(kf(0x3A03_126F)).fadd_m(kf(0x3F80_0000));
    out_x.write(X87::from_f32(wx).fmul(rv).fadd_m(out_x.read()).to_f32());
    out_y.write(rv.fmul_m(wy).fadd_m(out_y.read()).to_f32());
    1
}

orig! {
    /// `RwRasterCreate(w, h, depth, flags)` (0x7FB230)
    pub fn rw_raster_create(w: i32, h: i32, depth: i32, flags: i32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
    /// `RwTextureCreate(RwRaster*)` (0x7F37C0)
    pub fn rw_texture_create(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F37C0 => unported;
    /// `RwTextureSetName(RwTexture*, const char*)` (0x7F38A0)
    pub fn rw_texture_set_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F38A0 => unported;
    /// CRT `strncpy` (0x821F40)
    pub fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CCustomRoadsignMgr::RoadsignGenerateTextRaster(text, numLetters, charset, a, raster)` (0x6FEB70)
    pub fn roadsign_generate_text_raster(text: GPtr<u8>, n: i32, charset: u32, a: u32, raster: GPtr<u8>) -> u8 = cdecl 0x6FEB70 => unported;
}

/// `CCustomRoadsignMgr::CreateRoadsignTexture(const char* text, int numLetters)` (0x6FECA0, cdecl):
/// `raster = RwRasterCreate(numLetters * 8, 16, 32, 0x504)`; null → 0. `charset = *[0xC3EF84]`; 0 →
/// raster destroyed, 0. `RoadsignGenerateTextRaster(text, numLetters, charset, 0, raster)` false, or a
/// null `RwTextureCreate(raster)` → raster destroyed, 0. Otherwise the texture is named after the first
/// 10 characters (`strncpy` into an 11-byte zeroed local), filter byte +0x50 = 2; returns it.
#[hook(addr = 0x6FECA0, conv = cdecl, name = "CCustomRoadsignMgr::CreateRoadsignTexture", expect = "83 EC 0C 56 57",
       unverified = "mvpc-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "creates a raster and a texture (RW heap)")]
pub fn roadsign_create_texture(text: GPtr<u8>, n: i32) -> GPtr<u8> {
    let raster = rw_raster_create(n.wrapping_mul(8), 0x10, 0x20, 0x504);
    if raster.is_null() {
        return GPtr::NULL;
    }
    let charset = g::<GPtr<u32>>(0xC3EF84).read().read();
    if charset == 0 {
        rw_raster_destroy(raster);
        return GPtr::NULL;
    }
    if roadsign_generate_text_raster(text, n, charset, 0, raster) == 0 {
        rw_raster_destroy(raster);
        return GPtr::NULL;
    }
    let tex = rw_texture_create(raster);
    if tex.is_null() {
        rw_raster_destroy(raster);
        return GPtr::NULL;
    }
    let mut name = [0u8; 11];
    let np = GPtr::from_raw(name.as_mut_ptr());
    strncpy(np, text, 10);
    rw_texture_set_name(tex, np);
    wr::<u8>(tex, 0x50, 2);
    tex
}

orig! {
    /// `CFileMgr::OpenFile(path, mode)` (0x538900)
    pub fn cfm_open_file(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::CloseFile(file)` (0x5389D0)
    pub fn cfm_close_file(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// `CFileLoader::LoadLine(FILESTREAM)` (0x536F80)
    pub fn file_loader_load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// `ProcSurfaceInfo_c::Init(const char* surface, const char* model, float spacing, float minDist,
    /// int minRot, int maxRot, float minScl, maxScl, minSclZ, maxSclZ, zOffMin, zOffMax, int align,
    /// int useGrid)` (0x5A2EB0)
    pub fn proc_surface_info_init(this: GPtr<u8>, surface: GPtr<u8>, model: GPtr<u8>, a3: u32, a4: u32, a5: u32,
        a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32) = thiscall 0x5A2EB0 => unported;
}

/// `ProcObjectMan_c::LoadDataFile()` (0x5A3140, thiscall): `OpenFile("data\\procobj.dat", "r")`; for each
/// `CFileLoader::LoadLine` line not starting with `#` or NUL: `sscanf(line, "%s %s %f %f %d %d %f %f %f %f
/// %f %f %d %d")` into locals that persist across lines (a short line keeps the previous values),
/// `ProcSurfaceInfo_c::Init(surface, model, the 12 values)` on entry `this + 8 + 0x48 * count`, count
/// (+4) + 1. `CloseFile`.
#[hook(addr = 0x5A3140, conv = thiscall, name = "ProcObjectMan_c::LoadDataFile", expect = "81 EC B0 00 00 00",
       route = "split", hazards = "heap-writes,uninit-locals", recs = "grove-walk",
       note = "loads procobj.dat (file effects)")]
pub fn proc_object_man_load_data_file(this: GPtr<u8>) {
    let f = cfm_open_file(g(0x867014), g(0x85A53C));
    let mut surface = [0u8; 0x40];
    let mut model = [0u8; 0x40];
    let mut v = [0u32; 12];
    loop {
        let line = file_loader_load_line(f);
        if line.is_null() {
            break;
        }
        let c = line.read();
        if c == b'#' || c == 0 {
            continue;
        }
        let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
        let mut outs = [GPtr::<u8>::NULL; 14];
        outs[0] = GPtr::from_raw(surface.as_mut_ptr());
        outs[1] = GPtr::from_raw(model.as_mut_ptr());
        for (j, o) in outs.iter_mut().skip(2).enumerate() {
            *o = vp.byte_add(4 * j as isize);
        }
        sa_stream::modelinfo::vehicle::sscanf_n(line, g(0x866FE8), &outs);
        let n = crate::common::rd::<i32>(this, 4);
        let e = this.byte_add(8 + 0x48 * n as isize);
        proc_surface_info_init(e, outs[0], outs[1], v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9], v[10], v[11]);
        wr::<i32>(this, 4, crate::common::rd::<i32>(this, 4) + 1);
    }
    cfm_close_file(f);
}

orig! {
    /// `_rpAtomicResyncInterpolatedSphere(RpAtomic*)` (0x7491F0)
    pub fn rp_atomic_resync_interpolated_sphere(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x7491F0 => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    pub fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `RwV3dTransformPoints(out, in, n, m)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `RpClumpGetNumAtomics(RpClump*)` (0x7498E0)
    pub fn rp_clump_get_num_atomics(c: GPtr<u8>) -> i32 = cdecl 0x7498E0 => unported;
    /// `RpClumpForAllAtomics(RpClump*, cb, data)` (0x749B70)
    pub fn rp_clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RwMatrixInvert(out, in)` (0x7F2070)
    pub fn rw_matrix_invert(out: GPtr<u8>, inp: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F2070 => unported;
}

/// `RpClumpGetBoundingSphere`'s "use the LTM" flag (byte 0x8D60BC).
const SPHERE_USE_LTM: usize = 0x8D60BC;

/// The atomic's bounding-sphere centre (+0x1C) in the clump frame's space (`[atomic+0x3C]` + 4): with
/// the LTM flag through `RwFrameGetLTM`, else the frame's modelling matrix (+0x10). An interpolated
/// sphere (+0x4C & 2) is resynced first.
fn atomic_centre(atomic: GPtr<u8>, out: GPtr<u8>) {
    let clump = crate::common::rd::<GPtr<u8>>(atomic, 0x3C);
    let use_ltm = g::<u8>(SPHERE_USE_LTM).read() != 0;
    if crate::common::rd::<u8>(atomic, 0x4C) & 2 != 0 {
        rp_atomic_resync_interpolated_sphere(atomic);
    }
    let frame = crate::common::rd::<GPtr<u8>>(clump, 4);
    let m = if use_ltm { rw_frame_get_ltm(frame) } else { frame.byte_add(0x10) };
    rw_v3d_transform_points(out, atomic.byte_add(0x1C), 1, m);
}

/// `(missed by ghidra: data-code-immediate)` (0x734970, cdecl): `RpClumpGetBoundingSphere`'s first
/// atomic callback: `*sum += centre(atomic)` (f32 per component); returns the atomic.
#[hook(addr = 0x734970, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "A0 BC 60 8D 00",
       unverified = "mvpc-render-1", route = "L2", hazards = "none", recs = "grove-walk",
       note = "RpClumpForAllAtomics callback of RpClumpGetBoundingSphere")]
pub fn clump_sphere_sum_cb(atomic: GPtr<u8>, sum: GPtr<f32>) -> GPtr<u8> {
    let mut c = [0f32; 3];
    atomic_centre(atomic, GPtr::from_raw(c.as_mut_ptr()).cast());
    for (j, v) in c.iter().enumerate() {
        let s = sum.wrapping_index(j as i32);
        s.write(X87::from_f32(*v).fadd_m(s.read()).to_f32());
    }
    atomic
}

/// `(missed by ghidra: data-code-immediate)` (0x734ED0, cdecl): the radius callback: an interpolated
/// sphere is resynced (then again inside the centre transform); `r = |centre(atomic) - s.centre|`
/// (x87 `fsqrt` of `(dz*dz + dy*dy) + dx*dx`) + the atomic's radius (+0x28); larger than s.radius
/// (ordered) → stored (f32). Returns the atomic.
#[hook(addr = 0x734ED0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "83 EC 18 55 56",
       unverified = "mvpc-render-1", route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "RpClumpForAllAtomics callback of RpClumpGetBoundingSphere")]
pub fn clump_sphere_radius_cb(atomic: GPtr<u8>, s: GPtr<f32>) -> GPtr<u8> {
    if crate::common::rd::<u8>(atomic, 0x4C) & 2 != 0 {
        rp_atomic_resync_interpolated_sphere(atomic);
    }
    let mut p = [0f32; 3];
    atomic_centre(atomic, GPtr::from_raw(p.as_mut_ptr()).cast());
    let c = [s.read(), s.wrapping_index(1).read(), s.wrapping_index(2).read()];
    let dx = X87::from_f32(p[0]).fsub_m(c[0]);
    let dy = X87::from_f32(p[1]).fsub_m(c[1]);
    let dz = X87::from_f32(p[2]).fsub_m(c[2]);
    let r = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().fadd_m(crate::common::rd::<f32>(atomic, 0x28));
    let rad = s.wrapping_index(3);
    if !crate::common::jne(r, X87::from_f32(rad.read()), 0x41) {
        rad.write(r.to_f32());
    }
    atomic
}

/// `RpClumpGetBoundingSphere(RpClump*, RwSphere*, bool useLTM)` (0x734FC0, cdecl): flag → 0x8D60BC; a
/// null clump or sphere → 0. The sphere is zeroed; `n = RpClumpGetNumAtomics` (as f32, kept in the
/// original's own `useLTM` slot); `n < 1` → 0. The centre is the mean of the atomic centres (callback
/// 0x734970; each sum component `* (1 / n)`, f32), the radius the largest callback 0x734ED0 finds
/// around it (from 0); the centre is then brought back with the inverse of the frame's LTM (or modelling
/// matrix) via `RwV3dTransformPoints`. Returns the clump.
#[hook(addr = 0x734FC0, conv = cdecl, name = "RpClumpGetBoundingSphere", expect = "8A 44 24 0C 83 EC 5C",
       route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "keeps the atomic count (f32) in its own third argument slot (no footprint recorded)")]
pub fn rp_clump_get_bounding_sphere(clump: GPtr<u8>, sphere: GPtr<u8>, use_ltm: u32) -> GPtr<u8> {
    g::<u8>(SPHERE_USE_LTM).write(use_ltm as u8);
    if clump.is_null() || sphere.is_null() {
        return GPtr::NULL;
    }
    for o in [0, 4, 8, 0xC] {
        wr::<u32>(sphere, o, 0);
    }
    let nf = X87::from_i32(rp_clump_get_num_atomics(clump)).to_f32();
    // `fcomp 1.0`, `test ah, 5`, `jnp`: exits when the count is below 1
    if !X87::from_f32(nf).fcom(X87::from_f32(f32::from_bits(0x3F80_0000))).test_parity(5) {
        return GPtr::NULL;
    }
    let mut sum = [0f32; 3];
    rp_clump_for_all_atomics(clump, 0x734970, GPtr::from_raw(sum.as_mut_ptr()).cast());
    let inv = X87::from_f32(f32::from_bits(0x3F80_0000)).fdiv_m(nf);
    let mut s2 = [0f32; 4];
    for j in 0..3 {
        sum[j] = X87::from_f32(sum[j]).fmul(inv).to_f32();
        s2[j] = sum[j];
    }
    let s2p = GPtr::from_raw(s2.as_mut_ptr()).cast::<u8>();
    rp_clump_for_all_atomics(clump, 0x734ED0, s2p);
    let mut m = [0u32; 16];
    let mp = GPtr::from_raw(m.as_mut_ptr()).cast::<u8>();
    let frame = crate::common::rd::<GPtr<u8>>(clump, 4);
    if g::<u8>(SPHERE_USE_LTM).read() != 0 {
        rw_matrix_invert(mp, rw_frame_get_ltm(frame));
    } else {
        rw_matrix_invert(mp, frame.byte_add(0x10));
    }
    rw_v3d_transform_points(s2p, s2p, 1, mp);
    wr::<f32>(sphere, 0xC, s2[3]);
    wr::<f32>(sphere, 0, s2[0]);
    wr::<f32>(sphere, 4, s2[1]);
    wr::<f32>(sphere, 8, s2[2]);
    clump
}

orig! {
    /// `CPed::GetBonePosition(RwV3d* out, uint bone, bool updateSkin)` (0x5E4280)
    pub fn ped_get_bone_position(ped: GPtr<u8>, out: GPtr<u8>, bone: u32, update: u32) = thiscall 0x5E4280 => unported;
    /// `CWorld::FindPlayerPed(int)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CCamera::IsSphereVisible(const CVector*, float)` (0x420D40)
    pub fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<u8>, r: u32) -> u8 = thiscall 0x420D40 => unported;
    /// `CShadows::StoreShadowToBeRendered(type, RwTexture*, CVector* pos, fx, fy, sx, sy, short r, g, b, a,
    /// float zDist, bool drawOnWater, float scale, CRealTimeShadow*, bool drawOnBuildings)` (0x707390)
    pub fn store_shadow_to_be_rendered(ty: u32, tex: u32, pos: GPtr<u8>, fx: u32, fy: u32, sx: u32, sy: u32,
        r: u32, g: u32, b: u32, a: u32, zdist: u32, water: u32, scale: u32, rt: u32, buildings: u32) = cdecl 0x707390 => unported;
}

/// `CShadows::StoreShadowForPedObject(CEntity* ped, float dispX, float dispY, float frontX, float frontY,
/// float sideX, float sideY)` (0x707B40, cdecl): `pos = ped->GetBonePosition(0, false)`; `d2 = (pos.y -
/// cam.y)^2 + (pos.x - cam.x)^2` (stored f32) must be below `[0xC4B6B0]` (ordered); a ped other than the
/// player must pass `TheCamera.IsSphereVisible(ped pos, 2.0)`. `dist = sqrt(d2)`, `half = [0x8D5240] *
/// 0.5`: below half the intensity is the dword at 0xB7C4E8, else `_ftol((1 - (dist - half) / half) *
/// (short)[0xB7C4E8])`. `StoreShadowToBeRendered(1, [0xC403E4], &(pos.x + dispX, pos.y + dispY, pos.z),
/// frontX, frontY, sideX, sideY, i, i, i, i, 4.0, false, 1.0, null, isPlayer)`.
#[hook(addr = 0x707B40, conv = cdecl, name = "CShadows::StoreShadowForPedObject", expect = "83 EC 14 56 57",
       unverified = "mvpc-render-1", route = "L2", hazards = "x87-transcendental,uninit-locals", recs = "grove-walk",
       note = "the isPlayer bool is pushed as a dword whose upper 3 bytes are uninitialised stack (zero here)")]
pub fn shadows_store_shadow_for_ped_object(ped: GPtr<u8>, disp_x: u32, disp_y: u32, fx: u32, fy: u32, sx: u32, sy: u32) {
    let mut pos = [0f32; 3];
    let pp = GPtr::from_raw(pos.as_mut_ptr()).cast::<u8>();
    ped_get_bone_position(ped, pp, 0, 0);
    let c = camera_pos();
    let dy = X87::from_f32(crate::common::rd::<f32>(pp, 4)).fsub_m(c.wrapping_index(1).read());
    let dx = X87::from_f32(crate::common::rd::<f32>(pp, 0)).fsub_m(c.read());
    let d2 = dy.fmul(dy).fadd(dx.fmul(dx));
    let d2f = d2.to_f32();
    if crate::common::jp(d2, X87::from_f32(g::<f32>(0xC4B6B0).read()), 5) {
        return;
    }
    let is_player = find_player_ped(-1) == ped;
    if !is_player {
        let m = crate::common::rd::<GPtr<u8>>(ped, 0x14);
        let at = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        if camera_is_sphere_visible(g(0xB6F028), at, 0x4000_0000) == 0 {
            return;
        }
    }
    let dist = X87::from_f32(d2f).fsqrt();
    let half = X87::from_f32(g::<f32>(0x8D5240).read()).fmul_m(f32::from_bits(0x3F00_0000));
    let i = if !crate::common::jp(dist, half, 5) {
        g::<u32>(0xB7C4E8).read()
    } else {
        let w = X87::from_i32(i32::from(g::<i16>(0xB7C4E8).read()));
        crate::common::ftol32(dist.fsub(half).fdiv(half).fsubr_m(f32::from_bits(0x3F80_0000)).fmul(w)) as u32
    };
    let px = X87::from_f32(crate::common::rd::<f32>(pp, 0)).fadd_m(f32::from_bits(disp_x)).to_f32();
    wr::<f32>(pp, 0, px);
    let py = X87::from_f32(crate::common::rd::<f32>(pp, 4)).fadd_m(f32::from_bits(disp_y)).to_f32();
    wr::<f32>(pp, 4, py);
    let tex = g::<u32>(0xC403E4).read();
    store_shadow_to_be_rendered(1, tex, pp, fx, fy, sx, sy, i, i, i, i, 0x4080_0000, 0, 0x3F80_0000, 0, u32::from(is_player));
}

orig! {
    /// `FUN_00420800(float a, float b)` (0x420800, cdecl, ST0): the larger
    pub fn float_max(a: u32, b: u32) -> f64 = cdecl 0x420800 => unported;
    /// `AddAnExtraDirectionalLight(RpWorld*, float dx, dy, dz, r, g, b)` (0x735840)
    pub fn add_extra_directional_light(world: u32, dx: u32, dy: u32, dz: u32, r: u32, g: u32, b: u32) -> u8 = cdecl 0x735840 => unported;
}

/// `CPointLights::GenerateLightsAffectingObject(const CVector* pos, float* totalLighting, CEntity*
/// entity)` (0x6FFBB0, cdecl, float in ST0). Over the `[0xC3F0D0]` point lights (0x30 bytes from
/// 0xC3F0E0: pos, dir, radius +0x18, colour +0x1C..+0x24, entity +0x28, type +0x2C), types 3 and 4
/// skipped: `d = L - pos` (f32 per axis) must lie strictly inside (-r, r) per axis (x87 compares,
/// NaN fails) and `|d|` (f32, `fsqrt`) below r; `q = |d| / r`. Type 2 (shadow) multiplies the result
/// (from 1.0) by q. Others: with `totalLighting`, it gains `(1 - q) * colour / 3` (extended running sum,
/// stored f32); `f = 1` for q < 0.5 else `1 - (q - 0.5) * 2`; for `|d| != 0` (NaN passes) d is
/// normalised (f32); type 1 lights not attached to `entity` scale f by `max(0, (-(d . dir) - 0.5) * 2)`;
/// `f > 0` adds a directional light `([0xC17038], d, f * colour)`. Returns the result.
#[hook(addr = 0x6FFBB0, conv = cdecl, name = "CPointLights::GenerateLightsAffectingObject", expect = "83 EC 1C A1 D0 F0 C3 00",
       unverified = "mvpc-render-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "adds RpWorld extra directional lights")]
pub fn point_lights_generate_lights_affecting_object(pos: GPtr<f32>, total: GPtr<f32>, entity: u32) -> f32 {
    let mut result = f32::from_bits(0x3F80_0000);
    let kf = |b: u32| f32::from_bits(b);
    let mut i: i16 = 0;
    while i32::from(i) < g::<i32>(0xC3F0D0).read() {
        let l = g::<u8>(0xC3F0E0).byte_add(isize::from(i) * 0x30);
        i = i.wrapping_add(1);
        let lf = |o: isize| crate::common::rd::<f32>(l, o);
        let ty = crate::common::rd::<u8>(l, 0x2C);
        if ty == 3 || ty == 4 {
            continue;
        }
        let r = lf(0x18);
        let nr = X87::from_f32(r).fchs().to_f32();
        let outside = |v: f32| {
            crate::common::jne(X87::from_f32(v), X87::from_f32(nr), 0x41) || crate::common::jp(X87::from_f32(v), X87::from_f32(r), 5)
        };
        let dx = X87::from_f32(lf(0)).fsub_m(pos.read()).to_f32();
        if outside(dx) {
            continue;
        }
        let dy = X87::from_f32(lf(4)).fsub_m(pos.wrapping_index(1).read()).to_f32();
        if outside(dy) {
            continue;
        }
        let dz = X87::from_f32(lf(8)).fsub_m(pos.wrapping_index(2).read()).to_f32();
        if outside(dz) {
            continue;
        }
        let dv = X87::from_f32(dz).fmul_m(dz).fadd(X87::from_f32(dy).fmul_m(dy)).fadd(X87::from_f32(dx).fmul_m(dx)).fsqrt();
        let d = dv.to_f32();
        if crate::common::jp(dv, X87::from_f32(r), 5) {
            continue;
        }
        let q = X87::from_f32(d).fdiv_m(r);
        if ty == 2 {
            result = q.fmul_m(result).to_f32();
            continue;
        }
        if !total.is_null() {
            let t = X87::from_f32(kf(0x3F80_0000)).fsub(q);
            let third = kf(0x3EAA_AAAB);
            let a = t.fmul_m(lf(0x1C)).fmul_m(third).fadd_m(total.read());
            let b = t.fmul_m(lf(0x20)).fmul_m(third).fadd(a);
            let c = t.fmul_m(lf(0x24)).fmul_m(third).fadd(b);
            total.write(c.to_f32());
        }
        let mut f = if !crate::common::jp(q, X87::from_f32(kf(0x3F00_0000)), 5) {
            kf(0x3F80_0000)
        } else {
            let h = q.fsub_m(kf(0x3F00_0000));
            h.fadd(h).fsubr_m(kf(0x3F80_0000)).to_f32()
        };
        if !X87::from_f32(d).fcom(X87::zero()).test_parity(0x44) {
            continue;
        }
        let inv = X87::from_f32(kf(0x3F80_0000)).fdiv_m(d);
        let nx = X87::from_f32(dx).fmul(inv).to_f32();
        let ny = X87::from_f32(dy).fmul(inv).to_f32();
        let nz = inv.fmul_m(dz).to_f32();
        if ty == 1 && crate::common::rd::<u32>(l, 0x28) != entity {
            let dot = X87::from_f32(nz).fmul_m(lf(0x14)).fadd(X87::from_f32(ny).fmul_m(lf(0x10))).fadd(X87::from_f32(nx).fmul_m(lf(0xC)));
            let v = dot.fchs().fsub_m(kf(0x3F00_0000));
            let v = v.fadd(v).to_f32();
            let m = X87::from_f64_exact(float_max(0, v.to_bits()));
            f = m.fmul_m(f).to_f32();
        }
        if !crate::common::jne(X87::from_f32(f), X87::zero(), 0x41) {
            let fx = X87::from_f32(f);
            let cb = fx.fmul_m(lf(0x24)).to_f32();
            let cg = fx.fmul_m(lf(0x20)).to_f32();
            let cr = fx.fmul_m(lf(0x1C)).to_f32();
            add_extra_directional_light(g::<u32>(0xC17038).read(), nx.to_bits(), ny.to_bits(), nz.to_bits(), cr.to_bits(), cg.to_bits(), cb.to_bits());
        }
    }
    result
}

orig! {
    /// `RwEngineInstance->dOpenDevice` slot +0x28 (`fpIm2DRenderIndexedPrimitive(type, verts, numVerts,
    /// indices, numIndices)`)
    pub fn im2d_render_indexed_primitive(a: u32, b: u32, c: u32, d: u32, e: u32) -> i32 = cdecl table 0xC97B24 0x28 => unported;
}

/// `FUN_00734ec0` (0x734EC0, cdecl): `RwIm2DRenderIndexedPrimitive`, a tail jump through the device
/// table (`[RwEngineInstance] + 0x28`) with the caller's arguments (five forwarded; the callee reads
/// what it needs).
#[hook(addr = 0x734EC0, conv = cdecl, name = "FUN_00734ec0", expect = "8B 0D 24 7B C9 00",
       unverified = "mvpc-render-1", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "pure forwarder into the D3D9 driver (draws)")]
pub fn im2d_render_indexed_primitive_thunk(a: u32, b: u32, c: u32, d: u32, e: u32) -> i32 {
    im2d_render_indexed_primitive(a, b, c, d, e)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_IM2D_RENDER_INDEXED_PRIMITIVE_THUNK,
    HOOK_POINT_LIGHTS_GENERATE_LIGHTS_AFFECTING_OBJECT,
    HOOK_SHADOWS_STORE_SHADOW_FOR_PED_OBJECT,
    HOOK_CLUMP_SPHERE_SUM_CB,
    HOOK_CLUMP_SPHERE_RADIUS_CB,
    HOOK_RP_CLUMP_GET_BOUNDING_SPHERE,
    HOOK_PROC_OBJECT_MAN_LOAD_DATA_FILE,
    HOOK_ROADSIGN_CREATE_TEXTURE,
    HOOK_FIND_WIND_MODIFIER,
    HOOK_SHINY_TEXTS_REGISTER_ONE,
    HOOK_BRIGHT_LIGHTS_REGISTER_ONE,
    HOOK_SHADOWS_INIT,
    HOOK_SPECIAL_FX_INIT,
    HOOK_DRAW_SET_FOV,
    HOOK_INFORM_FRIENDS_EVENT_CTOR,
    HOOK_SCRIPTED_2D_USER_LIST_CTOR,
    HOOK_SCRIPTED_2D_EFFECT_PAIRS_CTOR,
    HOOK_EFFECT_PAIRS_RESET,
    HOOK_PLANT_MGR_RELOAD_CONFIG,
];
