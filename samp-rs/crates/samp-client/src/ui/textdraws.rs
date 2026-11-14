//! The original exe's side of the textdraw pass (Windows only; the pass itself is `crate::ui::render::textdraws`):
//! the `CFont` calls of a text textdraw in samp's order, and the sprite textures (font 4, `txd:name`, loaded
//! like samp by `samp_ui::textdraw::sprite_source`) drawn with `CSprite2d::Draw`.
//! Spec: docs/re/subsystems/ui-rendering.md, "14. Textdraws"; addresses in docs/re/addresses/ui-textdraws.md.

use sa_samp_api::ui::TextdrawFont;

use crate::patch::{Memory, ProcessMemory};

/// `CFont` (gta_sa.exe 1.0 US).
mod cfont {
    pub const SET_SCALE: usize = 0x719380;
    pub const SET_COLOR: usize = 0x719430;
    pub const SET_JUSTIFY: usize = 0x719600;
    pub const SET_ORIENTATION: usize = 0x719610;
    pub const SET_WRAPX: usize = 0x7194D0;
    pub const SET_CENTRE_SIZE: usize = 0x7194E0;
    pub const SET_BACKGROUND: usize = 0x7195C0;
    pub const SET_BACKGROUND_COLOR: usize = 0x7195E0;
    pub const SET_PROPORTIONAL: usize = 0x7195B0;
    pub const SET_DROP_COLOR: usize = 0x719510;
    pub const SET_EDGE: usize = 0x719590;
    pub const SET_DROP_SHADOW_POSITION: usize = 0x719570;
    pub const SET_FONT_STYLE: usize = 0x719490;
    pub const PRINT_STRING: usize = 0x71A700;
    /// `CMessages::InsertNumberInString` / `InsertPlayerControlKeysInString`.
    pub const INSERT_NUMBERS: usize = 0x69DE90;
    pub const INSERT_KEYS: usize = 0x69E160;
}

/// Textures of sprite textdraws by textdraw id: (text it was loaded for, RwTexture*).
static SPRITES: std::sync::Mutex<Vec<(u16, Vec<u8>, u32)>> = std::sync::Mutex::new(Vec::new());

mod txd {
    pub const FIND_SLOT: usize = 0x731850;
    pub const ADD_SLOT: usize = 0x731C80;
    pub const LOAD: usize = 0x7320B0;
    pub const ADD_REF: usize = 0x731A00;
    pub const PUSH_CURRENT: usize = 0x7316A0;
    pub const POP_CURRENT: usize = 0x7316B0;
    pub const SET_CURRENT: usize = 0x7319C0;
    pub const SPRITE_SET_TEXTURE: usize = 0x727270;
    pub const SPRITE_DRAW: usize = 0x728350;
    pub const SPRITE_DELETE: usize = 0x7281E0;
    /// `RwEngineInstance`.
    pub const RW_ENGINE: u32 = 0xC97B24;
    /// Offset of the `RwRenderStateSet` function pointer in the engine instance.
    pub const RW_ENGINE_RENDER_STATE_SET: u32 = 0x20;
}

fn cstr(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

fn find_slot(name: &[u8]) -> i32 {
    // SAFETY: GTA's cdecl CTxdStore::FindTxdSlot(const char*).
    let f: extern "C" fn(*const u8) -> i32 = unsafe { std::mem::transmute(txd::FIND_SLOT) };
    f(cstr(name).as_ptr())
}

/// samp's "ensure loaded": add + load + AddRef when not present. Mirrors samp.dll 0x100B2BD0.
fn ensure_txd(key: &[u8], path: &[u8]) -> bool {
    if find_slot(key) != -1 {
        return true;
    }
    let exists = std::path::Path::new(&String::from_utf8_lossy(path).replace('\\', "/")).exists();
    // SAFETY: GTA's cdecl CTxdStore functions on the main thread.
    unsafe {
        let add: extern "C" fn(*const u8) -> i32 = std::mem::transmute(txd::ADD_SLOT);
        let slot = add(cstr(key).as_ptr());
        if !exists {
            return false;
        }
        let load: extern "C" fn(i32, *const u8) -> u8 = std::mem::transmute(txd::LOAD);
        load(slot, cstr(path).as_ptr());
        let addref: extern "C" fn(i32) = std::mem::transmute(txd::ADD_REF);
        addref(slot);
    }
    true
}

/// Texture of `txd:name`, 0 when unavailable. Mirrors samp.dll 0x100B2D10.
fn load_sprite(text: &[u8]) -> u32 {
    let Some(src) = samp_ui::textdraw::sprite_source(text) else { return 0 };
    if let Some(path) = src.path()
        && !ensure_txd(&src.slot, &path)
    {
        return 0;
    }
    let slot = find_slot(&src.slot);
    if slot == -1 {
        return 0;
    }
    let mut sprite: u32 = 0;
    // SAFETY: GTA's TXD stack and CSprite2d::SetTexture (thiscall on a one-pointer sprite).
    unsafe {
        let push: extern "C" fn() = std::mem::transmute(txd::PUSH_CURRENT);
        let set: extern "C" fn(i32) = std::mem::transmute(txd::SET_CURRENT);
        let pop: extern "C" fn() = std::mem::transmute(txd::POP_CURRENT);
        let tex: extern "thiscall" fn(*mut u32, *const u8) = std::mem::transmute(txd::SPRITE_SET_TEXTURE);
        push();
        set(slot);
        tex(&mut sprite, cstr(&src.texture).as_ptr());
        pop();
    }
    sprite
}

fn free_sprite(tex: u32) {
    if tex != 0 {
        let mut s = tex;
        // SAFETY: CSprite2d destructor on a temporary holding the texture.
        let f: extern "thiscall" fn(*mut u32) = unsafe { std::mem::transmute(txd::SPRITE_DELETE) };
        f(&mut s);
    }
}

/// Drops the textures of textdraws that are gone or changed (`live`: id and text of the sprite textdraws).
pub fn sync_sprites(live: &[(u16, &[u8])]) {
    let mut g = SPRITES.lock().unwrap_or_else(|e| e.into_inner());
    g.retain(|(id, text, tex)| {
        let keep = live.iter().any(|(i, t)| i == id && *t == text.as_slice());
        if !keep {
            free_sprite(*tex);
        }
        keep
    });
}

/// Number of sprite textdraw textures held (they share samp's 200 texture slots with previews).
pub fn sprite_count() -> usize {
    SPRITES.lock().unwrap_or_else(|e| e.into_inner()).len()
}

/// The texture of sprite textdraw `id`, loaded on first use.
pub fn sprite_texture(id: u16, text: &[u8]) -> u32 {
    let mut g = SPRITES.lock().unwrap_or_else(|e| e.into_inner());
    match g.iter().find(|(i, _, _)| *i == id) {
        Some((_, _, tex)) => *tex,
        None => {
            let tex = load_sprite(text);
            g.push((id, text.to_vec(), tex));
            tex
        }
    }
}

/// The sprite path's textured quad (`r` = left, top, right, bottom). Mirrors samp.dll 0x100B30D0.
pub fn draw_texture(tex: u32, r: [f32; 4], colour: u32) {
    // CRect as samp builds it: left, bottom, right, top.
    let rect = [r[0], r[3], r[2], r[1]];
    let mut sprite = tex;
    let eng = ProcessMemory.read_u32(txd::RW_ENGINE).unwrap_or(0);
    if eng != 0 {
        let f = ProcessMemory.read_u32(eng + txd::RW_ENGINE_RENDER_STATE_SET).unwrap_or(0);
        // SAFETY: the engine's cdecl `RwRenderStateSet(state, value)`.
        let rs: extern "C" fn(u32, u32) -> i32 = unsafe { std::mem::transmute(f as usize) };
        rs(9, 2);
    }
    // SAFETY: CSprite2d::Draw (thiscall) on a sprite holding a live RwTexture, main thread.
    let draw: extern "thiscall" fn(*mut u32, *const [f32; 4], *const u32) =
        unsafe { std::mem::transmute(txd::SPRITE_DRAW) };
    draw(&mut sprite, &rect, &colour);
}

/// samp's work buffer is 0x321+ bytes inside the textdraw; GTA's insertions grow the
/// text, so ours is larger.
const WORK: usize = 2048;

fn call1(f: usize, a: u32) {
    // SAFETY: GTA's cdecl one-argument CFont setters (by-value 32-bit argument).
    let f: extern "C" fn(u32) = unsafe { std::mem::transmute(f) };
    f(a);
}

fn call_f(f: usize, a: f32) {
    // SAFETY: GTA's cdecl float setters.
    let f: extern "C" fn(f32) = unsafe { std::mem::transmute(f) };
    f(a);
}

/// The CFont calls of one text textdraw, in samp's order. Mirrors samp.dll 0x100B31E0.
pub fn print(p: &TextdrawFont, text: &[u8], has_keys: bool) {
    let mut buf = Box::new([0u8; WORK]);
    let n = text.len().min(WORK / 2);
    buf[..n].copy_from_slice(&text[..n]);
    // SAFETY: GTA functions on the main thread; `buf` is NUL-terminated with room to grow.
    unsafe {
        let scale: extern "C" fn(f32, f32) = std::mem::transmute(cfont::SET_SCALE);
        scale(p.scale.0, p.scale.1);
        call1(cfont::SET_COLOR, p.colour);
        call1(cfont::SET_JUSTIFY, 0);
        call1(cfont::SET_ORIENTATION, u32::from(p.orientation));
        call_f(cfont::SET_WRAPX, p.wrap_x);
        call_f(cfont::SET_CENTRE_SIZE, p.centre_size);
        let bg: extern "C" fn(u32, u32) = std::mem::transmute(cfont::SET_BACKGROUND);
        bg(u32::from(p.background), 0);
        call1(cfont::SET_BACKGROUND_COLOR, p.background_colour);
        call1(cfont::SET_PROPORTIONAL, u32::from(p.proportional));
        call1(cfont::SET_DROP_COLOR, p.drop_colour);
        if p.outline != 0 {
            call1(cfont::SET_EDGE, u32::from(p.outline));
        } else {
            call1(cfont::SET_DROP_SHADOW_POSITION, u32::from(p.shadow));
        }
        call1(cfont::SET_FONT_STYLE, u32::from(p.font));
        let ins: extern "C" fn(*mut u8, i32, i32, i32, i32, i32, i32, *mut u8) =
            std::mem::transmute(cfont::INSERT_NUMBERS);
        let b = buf.as_mut_ptr();
        ins(b, -1, -1, -1, -1, -1, -1, b);
        if has_keys {
            let k: extern "C" fn(*mut u8) = std::mem::transmute(cfont::INSERT_KEYS);
            k(b);
        }
        let ps: extern "C" fn(f32, f32, *const u8) = std::mem::transmute(cfont::PRINT_STRING);
        ps(p.x, p.y, b);
        call1(cfont::SET_EDGE, 0);
    }
}
