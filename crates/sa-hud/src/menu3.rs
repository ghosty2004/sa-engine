//! `CMenuSystem`, third pass (PORT-HUD-3): the grid menus (`InputGridMenu`, `DisplayGridMenu`,
//! `FillGridWithCarColours`) and `DisplayStandardMenu`. Ported from the 1.0 US bytes; gta-reversed
//! `MenuSystem.cpp` as reading aid (the bytes win). Panel layout: see `menu2.rs`.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};
use sa_math::x87::X87;
use crate::menu2::{alt, get_pad, just, pad_analogue_down, pad_analogue_up, panel};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `CVehicleModelInfo::ms_vehicleColourTable` (`CRGBA[128]`)
const CAR_COLOURS: usize = 0xB4E480;
/// `FrontEndMenuManager`
const FRONTEND: usize = 0xBA6748;
/// `RsGlobal.maximumWidth`, `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;

/// f32 constants of the exe (bits): 1 (0x858624), 3 (0x858B3C), 1/640 (0x859520), 1/448 (0x859524)
const K_1: u32 = 0x3F80_0000;
const K_3: u32 = 0x4040_0000;
const K_1_640: u32 = 0x3ACC_CCCD;
const K_1_448: u32 = 0x3B12_4925;

fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

orig! {
    /// `CPad::GetAnaloguePadLeft()` (0x5409B0, AL)
    fn pad_analogue_left(pad: GPtr<u8>) -> u8 = thiscall 0x5409B0 => unported;
    /// `CPad::GetAnaloguePadRight()` (0x5409E0, AL)
    fn pad_analogue_right(pad: GPtr<u8>) -> u8 = thiscall 0x5409E0 => unported;
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0)
    fn rgba_ctor(this: GPtr<u8>, r: u32, gg: u32, bb: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite_draw_rect(rect: GPtr<u8>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CMenuManager::DrawWindow(const CRect&, const char*, uint8, CRGBA, bool, bool)` (0x573EE0)
    fn draw_window_call(this: GPtr<u8>, rect: GPtr<u8>, key: GPtr<u8>, colour: u32, back: u32, unused: u32, bg: u32) = thiscall 0x573EE0 => crate::menu_mgr::draw_window;
    /// MSVC `eh vector constructor iterator(void*, size, count, ctor, dtor)` (0x821D30, stdcall)
    fn eh_vector_ctor(p: GPtr<u8>, size: u32, count: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// MSVC `eh vector destructor iterator(void*, size, count, dtor)` (0x821E02, stdcall)
    fn eh_vector_dtor(p: GPtr<u8>, size: u32, count: u32, dtor: u32) = stdcall 0x821E02 => unported;
}

/// The original's stack temporaries as game pointers.
fn local<T, const N: usize>(buf: &mut [T; N]) -> GPtr<T> {
    GPtr::from_raw(buf.as_mut_ptr())
}

/// A CRGBA built by the original constructor in `buf`; returns `buf`.
fn rgba_in(buf: &mut [u8; 4], r: u32, gg: u32, bb: u32, a: u32) -> GPtr<u8> {
    rgba_ctor(local(buf), r, gg, bb, a)
}

/// `CMenuSystem::InputGridMenu(uint8 menu)` (0x580BD0): accept (cross just pressed, or the alternative
/// input) takes the active cell when its +0x3D6 flag is set; the accepted cell is cleared (-99) when it
/// is not the active one; then up / down move by a row (`NumColumns`) inside `[0, NumRows)`, left / right
/// by one cell inside `[0, NumRows - 1]` (the analogue pad, or the d-pad when +0x3F9 is set). No sounds
/// (unlike `InputStandardMenu`).
#[hook(addr = 0x580BD0, conv = cdecl, name = "CMenuSystem::InputGridMenu", expect = "53 56 57 6A 00")]
pub fn input_grid_menu(menu: u32) {
    if just(0x20, 0x50) || alt(0xB733EA, 0xB7317A) {
        let p = panel(menu);
        let r = rd::<u8>(p, 0x415);
        if rd::<u8>(p, 0x3D6 + isize::from(r as i8)) != 0 {
            wr::<u8>(p, 0x416, r);
        }
    }
    let p = panel(menu);
    if rd::<u8>(p, 0x416) != rd::<u8>(p, 0x415) {
        wr::<u8>(p, 0x416, 0x9D);
    }
    let dpad = |n: isize, o: isize| rd::<u8>(panel(menu), 0x3F9) != 0 && just(n, o);
    if pad_analogue_up(get_pad(0)) != 0 || dpad(0x10, 0x40) {
        let p = panel(menu);
        let a = rd::<u8>(p, 0x415);
        let cols = rd::<u8>(p, 0x3F7);
        if i32::from(a as i8) >= i32::from(cols) {
            wr::<u8>(p, 0x415, a.wrapping_sub(cols));
        }
    }
    if pad_analogue_down(get_pad(0)) != 0 || dpad(0x12, 0x42) {
        let p = panel(menu);
        let cols = rd::<u8>(p, 0x3F7);
        let rows = rd::<u8>(p, 0x3F6);
        let a = rd::<u8>(p, 0x415);
        if i32::from(a as i8) < i32::from(rows) - i32::from(cols) {
            wr::<u8>(p, 0x415, cols.wrapping_add(a));
        }
    }
    if pad_analogue_left(get_pad(0)) != 0 || dpad(0x14, 0x44) {
        let p = panel(menu);
        let a = rd::<u8>(p, 0x415);
        if (a as i8) > 0 {
            wr::<u8>(p, 0x415, a - 1);
        }
    }
    if pad_analogue_right(get_pad(0)) != 0 || dpad(0x16, 0x46) {
        let p = panel(menu);
        let a = rd::<u8>(p, 0x415);
        let rows = rd::<u8>(p, 0x3F6);
        if i32::from(a as i8) < i32::from(rows) - 1 {
            wr::<u8>(p, 0x415, a.wrapping_add(1));
        }
    }
}

/// `CMenuSystem::FillGridWithCarColours(uint8 menu)` (0x5820E0): fills the panel's 64 grid bytes with
/// car colour indices. A local `CRGBA[64]` (eh-vector constructed, every entry then set to (0, 0, 0,
/// 255)) holds the colours taken; for each slot the colour table is searched from index 0 for a colour
/// not within 15 (exclusive, per channel) of any of the first `slot` entries, i.e. of the colours taken
/// so far *and* the next, still black entry (from the second slot on; the first slot takes colour 0).
/// When index 127 is reached without a match, 127 is taken anyway. gta-reversed: not reversed.
#[hook(addr = 0x5820E0, conv = cdecl, name = "CMenuSystem::FillGridWithCarColours", expect = "6A FF 68 76 CE 83 00")]
pub fn fill_grid_with_car_colours(menu: u32) {
    let mut arr = [0u32; 64];
    eh_vector_ctor(local(&mut arr).cast(), 4, 0x40, 0x463360, 0x43CE70);
    for e in arr.iter_mut() {
        let mut t = [0u8; 4];
        *e = rgba_in(&mut t, 0, 0, 0, 0xFF).cast::<u32>().read();
    }
    let (mut c, mut n, mut slot) = (0u8, 0u8, 1u8);
    while slot < 0x41 {
        let tc = g::<u32>(CAR_COLOURS + usize::from(c) * 4).read();
        let mut t = [0u8; 4];
        // r = the table dword (low byte read), g / b = unaligned dwords from the copy (low byte read)
        let col = rgba_in(&mut t, tc, tc >> 8, tc >> 16, 0xFF).cast::<u32>().read();
        let saved = c;
        let [cr, cg, cb, _] = col.to_le_bytes().map(i32::from);
        let mut found = false;
        if slot > 1 {
            for e in arr.iter().take(usize::from(slot)) {
                let [er, eg, eb, _] = e.to_le_bytes().map(i32::from);
                if cr > er - 15 && cg > eg - 15 && cb > eb - 15 && cr < er + 15 && cg < eg + 15 && cb < eb + 15 {
                    found = true;
                }
            }
        }
        if found {
            c = c.wrapping_add(1);
            if c < 0x80 {
                continue;
            }
            c = saved;
        }
        arr[usize::from(n)] = col;
        wr::<u8>(panel(menu), isize::from(n), c);
        n += 1;
        slot += 1;
        c = 0;
    }
    eh_vector_dtor(local(&mut arr).cast(), 4, 0x40, 0x43CE70);
}

/// `CMenuSystem::DisplayGridMenu(uint8 menu, bool bright)` (0x5816E0): with a background, the window
/// `(x, y + cols * w, x + cols * w, y)` in (0, 0, 0, 190) (`DrawWindow` without a title; colour 0 when
/// bright, else 120); then for each cell (row r, column c of a cols x cols grid, cell index counting up
/// as a byte) the active cell's frame `(c * w + x, (r + 1) * w + y, (c + 1) * w + x, r * w + y)` in
/// (225, 225, 225, 255) and the car colour `ms_vehicleColourTable[grid[cell]]` inset by `3 * W/640` /
/// `3 * H/448`. x87 order and f32 stores as the bytes.
///
/// `DrawWindow`'s colour argument is a dword whose upper three bytes are an uninitialised local of the
/// original; the port passes them as 0 (no title is drawn, so `DrawWindow` never reads the argument).
#[hook(addr = 0x5816E0, conv = cdecl, name = "CMenuSystem::DisplayGridMenu", expect = "83 EC 44 8A 44 24 4C")]
pub fn display_grid_menu(menu: u32, bright: u32) {
    let fade: u32 = if bright as u8 != 0 { 0 } else { 0x78 };
    let p = panel(menu);
    if rd::<u8>(p, 0x414) != 0 {
        let cols = i32::from(rd::<u8>(p, 0x3F7));
        let cw = xi(cols).fmul_m(rd::<f32>(p, 0x3FC));
        let cw32 = f32_of(cw);
        let bottom = f32_of(cw.fadd_m(rd::<f32>(p, 0x410)));
        let right = f32_of(x(cw32).fadd_m(rd::<f32>(p, 0x40C)));
        let mut rect = [rd::<f32>(p, 0x40C), bottom, right, rd::<f32>(p, 0x410)];
        let mut t = [0u8; 4];
        let c = rgba_in(&mut t, 0, 0, 0, 0xBE).cast::<u32>().read();
        draw_window_call(g(FRONTEND), local(&mut rect).cast(), GPtr::NULL, fade, c, 0, 1);
    }
    if rd::<u8>(panel(menu), 0x3F7) == 0 {
        return;
    }
    let mut idx = 0u8;
    let mut r = 0u8;
    loop {
        let mut c = 0u8;
        if rd::<u8>(panel(menu), 0x3F7) != 0 {
            let rf = f32_of(xi(i32::from(r)));
            loop {
                let p = panel(menu);
                let gi = rd::<u8>(p, isize::from(idx));
                let col = g::<u32>(CAR_COLOURS + usize::from(gi) * 4).read();
                let (w, px, py) = (rd::<f32>(p, 0x3FC), rd::<f32>(p, 0x40C), rd::<f32>(p, 0x410));
                if i32::from(idx) == i32::from(rd::<i8>(p, 0x415)) {
                    let a = x(rf).fadd_m(k(K_1)).fmul_m(w).fadd_m(py);
                    let cf = xi(i32::from(c));
                    let cf32 = f32_of(cf);
                    let b1 = cf.fadd_m(k(K_1)).fmul_m(w).fadd_m(px);
                    let c1 = x(rf).fmul_m(w).fadd_m(py);
                    let d1 = x(cf32).fmul_m(w).fadd_m(px);
                    let mut rect = [f32_of(d1), f32_of(a), f32_of(b1), f32_of(c1)];
                    let mut t = [0u8; 4];
                    let cp = rgba_in(&mut t, 0xE1, 0xE1, 0xE1, 0xFF);
                    sprite_draw_rect(local(&mut rect).cast(), cp);
                }
                let p = panel(menu);
                let (w, px, py) = (rd::<f32>(p, 0x3FC), rd::<f32>(p, 0x40C), rd::<f32>(p, 0x410));
                let hs = f32_of(xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)).fmul_m(k(K_3)));
                let rw = f32_of(x(rf).fmul_m(w));
                let e1 = x(py).fadd_m(w).fsub_m(hs).fadd_m(rw);
                let ws = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640)).fmul_m(k(K_3)));
                let cw2 = f32_of(xi(i32::from(c)).fmul_m(w));
                let f1 = x(px).fadd_m(w).fsub_m(ws).fadd_m(cw2);
                let g1 = x(rw).fadd_m(py).fadd_m(hs);
                let h1 = x(cw2).fadd_m(px).fadd_m(ws);
                let mut rect = [f32_of(h1), f32_of(e1), f32_of(f1), f32_of(g1)];
                let mut t = [0u8; 4];
                let cp = rgba_in(&mut t, col, col >> 8, col >> 16, 0xFF);
                sprite_draw_rect(local(&mut rect).cast(), cp);
                idx = idx.wrapping_add(1);
                c = c.wrapping_add(1);
                if c >= rd::<u8>(panel(menu), 0x3F7) {
                    break;
                }
            }
        }
        r = r.wrapping_add(1);
        if r >= rd::<u8>(panel(menu), 0x3F7) {
            break;
        }
    }
}

orig! {
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    fn font_set_font_style(v: u32) = cdecl 0x719490 => crate::font::set_font_style;
    /// `CFont::SetScale(float, float)` (0x719380)
    fn font_set_scale(w: u32, h: u32) = cdecl 0x719380 => crate::font::set_scale;
    /// `CFont::SetDropShadowPosition(int8)` (0x719570)
    fn font_set_drop_shadow_position(v: u32) = cdecl 0x719570 => crate::font::set_drop_shadow_position;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    fn font_set_drop_color(c: u32) = cdecl 0x719510 => crate::font::set_drop_color;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    fn font_set_orientation(v: u32) = cdecl 0x719610 => crate::font::set_orientation;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    fn font_set_color(c: u32) = cdecl 0x719430 => crate::font::set_color;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    fn font_print_string(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => crate::font_print::print_string;
    /// `CFont::GetStringWidth(const GxtChar*, bool full, bool scriptText)` (0x71A0E0), ST0
    fn font_get_string_width_st0(text: GPtr<u8>, full: u32, script: u32) -> f64 = cdecl 0x71A0E0 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;
    /// `CMessages::InsertNumberInString(text, n1..n6, out)` (0x69DE90)
    fn insert_number_in_string(text: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, out: GPtr<u8>) = cdecl 0x69DE90 => crate::messages::insert_number_in_string;
    /// `CMessages::InsertPlayerControlKeysInString(GxtChar*)` (0x69E160)
    fn insert_player_control_keys(s: GPtr<u8>) = cdecl 0x69E160 => crate::messages::insert_player_control_keys_in_string;
}

/// `TheText`
const THE_TEXT: usize = 0xC1B340;
/// `HudColour` entries (CRGBA, `CHudColours` at 0xBAB22C): 1, 3, 8 and 12
const HUD_COLOUR_1: usize = 0xBAB230;
const HUD_COLOUR_3: usize = 0xBAB238;
const HUD_COLOUR_8: usize = 0xBAB24C;
const HUD_COLOUR_12: usize = 0xBAB25C;
/// f32 constants (bits): 20 (0x858BA4), 40 (0x858A10), 16 (0x8599D0), 10 (0x85862C), 0.5 (0x858B8C),
/// 0.52 (0x8665BC and the immediate 0x3F051EB8), 0.05 (0x858C28)
const K_20: u32 = 0x41A0_0000;
const K_40: u32 = 0x4220_0000;
const K_16: u32 = 0x4180_0000;
const K_10: u32 = 0x4120_0000;
const K_HALF: u32 = 0x3F00_0000;
const K_052: u32 = 0x3F05_1EB8;
const K_005: u32 = 0x3D4C_CCCD;

/// `_ftol(float(acc) + widths[i])` for i in 0..n, the accumulator taken as its low word each step: the
/// x offset of column n (`movzx; fild; fadd; _ftol`). Returns the last full `_ftol` result (0 for n = 0).
fn column_offset(p: GPtr<u8>, n: u8) -> u32 {
    let mut acc = 0u32;
    for i in 0..isize::from(n) {
        acc = ftol32(xi(i32::from(acc as u16)).fadd_m(rd::<f32>(p, 0x3FC + i * 4))) as u32;
    }
    acc
}

/// A HudColour entry with `fade` subtracted from r, g, b (bytes), alpha 255, built by the CRGBA
/// constructor.
fn faded(entry: usize, fade: u8) -> u32 {
    let ch = |o: usize| u32::from(g::<u8>(entry + o).read().wrapping_sub(fade));
    let mut t = [0u8; 4];
    rgba_in(&mut t, ch(0), ch(1), ch(2), 0xFF).cast::<u32>().read()
}

/// The text x of a column for orientation `o` (0 centre, 2 right, else left): `W/640 * 10` (+ the
/// column width, or half of it), `+ x`, `+ off` (integer), x87 in the original's order.
fn column_x(p: GPtr<u8>, c: u8, o: u8, off: u32) -> f32 {
    let w640 = xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640)).fmul_m(k(K_10));
    let wc = rd::<f32>(p, 0x3FC + isize::from(c) * 4);
    let v = match o {
        0 => x(wc).fmul_m(k(K_HALF)).fadd(w640),
        2 => w640.fadd_m(wc),
        _ => w640,
    };
    f32_of(v.fadd_m(rd::<f32>(p, 0x40C)).fadd(xi(off as i32)))
}

/// `CMenuSystem::DisplayStandardMenu(uint8 menu, bool bright)` (0x580E00): the standard script menu.
/// Fade = 0 when bright, else 120. With a background: the window from `(x, y)` to `x + W/640 * 20 + the
/// summed column widths` and `y + rows * (H/448 * 16) + head * H/448` (head 70 when any column has a
/// header, else 50) in (0, 0, 0, 190) with the title (`DrawWindow`); a header moves the first row to
/// `_ftol(H/448 * 40)` (else `_ftol(H/448 * 20)`). Headers: font 1, scale (W/640 * 0.52, H/448), drop
/// shadow 2, black drop colour; each header in grey `0xE1 - fade`, aligned by its flag byte (+0x3F2 when
/// not negative, else the column orientation). Rows (column-major texts `[c][r]`): numbers inserted
/// (`InsertNumberInString` with the row's two numbers), control keys inserted, scale shrunk from 0.52 by
/// 0.05 until the text fits the column width (`GetStringWidth(full)`), colour by state: interactive
/// active row HudColour 3, highlighted 1, active 12, else 8; non-interactive highlighted 1, active 3,
/// else 12 (each minus fade). Rows step by `_ftol(H/448 * 16 + y)`. Column x offsets accumulate through
/// `_ftol` with 16-bit truncation, as the bytes. x87 order and f32 stores as the bytes; gta-reversed
/// differs in the rounding steps.
///
/// Arguments passed with uninitialised upper bytes in the original (the fade byte to `DrawWindow`, the
/// orientation byte to `SetOrientation`): the callees read the low byte; the port passes the byte
/// zero-extended (only the outgoing argument slots differ).
#[hook(addr = 0x580E00, conv = cdecl, name = "CMenuSystem::DisplayStandardMenu", expect = "81 EC D8 01 00 00")]
pub fn display_standard_menu(menu: u32, bright: u32) {
    let hs = xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448));
    let mut yoff = ftol32(x(k(K_20)).fmul(hs)) as u32;
    let fade: u8 = if bright as u8 != 0 { 0 } else { 0x78 };
    let p = panel(menu);
    if rd::<u8>(p, 0x414) != 0 {
        let cols = rd::<u8>(p, 0x3F7);
        let mut head = 0x32u8;
        let mut tw = 0u32;
        for i in 0..isize::from(cols) {
            tw = ftol32(xi(i32::from(tw as u16)).fadd_m(rd::<f32>(p, 0x3FC + i * 4))) as u32;
            if rd::<u8>(p, 0x3A4 + i * 10) != 0 {
                head = 0x46;
                yoff = ftol32(x(k(K_40)).fmul(hs)) as u32;
            }
        }
        let rows = i32::from(rd::<u8>(p, 0x3F6));
        let r16 = xi(rows).fmul(hs.fmul_m(k(K_16)));
        let bottom = f32_of(r16.fadd(xi(i32::from(head)).fmul(hs)).fadd_m(rd::<f32>(p, 0x410)));
        let right = f32_of(
            xi(g::<i32>(RS_MAX_WIDTH).read())
                .fmul_m(k(K_1_640))
                .fmul_m(k(K_20))
                .fadd(xi(i32::from(tw as u16)))
                .fadd_m(rd::<f32>(p, 0x40C)),
        );
        let mut rect = [rd::<f32>(p, 0x40C), bottom, right, rd::<f32>(p, 0x410)];
        let mut t = [0u8; 4];
        let c = rgba_in(&mut t, 0, 0, 0, 0xBE).cast::<u32>().read();
        draw_window_call(g(FRONTEND), local(&mut rect).cast(), panel(menu).byte_add(0x3CC), u32::from(fade), c, 0, 1);
    }
    font_set_font_style(1);
    let sh = f32_of(xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)));
    let sw = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640)).fmul_m(k(K_052)));
    font_set_scale(sw.to_bits(), sh.to_bits());
    font_set_drop_shadow_position(2);
    let mut t = [0u8; 4];
    font_set_drop_color(rgba_in(&mut t, 0, 0, 0, 0xFF).cast::<u32>().read());
    font_set_orientation(0);
    // column headers
    let mut c = 0u8;
    while c < rd::<u8>(panel(menu), 0x3F7) {
        let p = panel(menu);
        let ci = isize::from(c);
        if rd::<u8>(p, 0x3A4 + ci * 10) != 0 {
            let flag = rd::<u8>(p, 0x3F2 + ci);
            let o = if (flag as i8) < 0 { rd::<u8>(p, 0x3EE + ci) } else { flag };
            font_set_orientation(u32::from(o));
            let grey = u32::from(0xE1u8.wrapping_sub(fade));
            let mut t = [0u8; 4];
            font_set_color(rgba_in(&mut t, grey, grey, grey, 0xFF).cast::<u32>().read());
            let off = column_offset(panel(menu), c) as u16;
            let p = panel(menu);
            let text = text_get(g(THE_TEXT), p.byte_add(0x3A4 + ci * 10));
            let yy = f32_of(xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)).fmul_m(k(K_20)).fadd_m(rd::<f32>(p, 0x410)));
            font_print_string(column_x(p, c, o, u32::from(off)), yy, text);
        }
        c = c.wrapping_add(1);
    }
    // rows
    let mut r = 0u8;
    while r < rd::<u8>(panel(menu), 0x3F6) {
        let mut c = 0u8;
        while c < rd::<u8>(panel(menu), 0x3F7) {
            let p = panel(menu);
            let (ri, ci) = (isize::from(r), isize::from(c));
            let cell = ci * 12 + ri;
            if rd::<u8>(p, 0x41 + cell * 10) != 0 {
                let hi = rd::<u8>(p, 0x3E2 + ri) != 0;
                let act = rd::<u8>(p, 0x3D6 + ri) != 0;
                let entry = if rd::<u8>(p, 0x3F8) != 0 {
                    if i32::from(r) == i32::from(rd::<i8>(p, 0x415)) && act {
                        HUD_COLOUR_3
                    } else if hi {
                        HUD_COLOUR_1
                    } else if act {
                        HUD_COLOUR_12
                    } else {
                        HUD_COLOUR_8
                    }
                } else if hi {
                    HUD_COLOUR_1
                } else if act {
                    HUD_COLOUR_3
                } else {
                    HUD_COLOUR_12
                };
                font_set_color(faded(entry, fade));
                let p = panel(menu);
                if rd::<u8>(p, 0x41 + cell * 10) != 0 {
                    row_text(menu, p, c, cell, yoff);
                }
            }
            c = c.wrapping_add(1);
        }
        let v = xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)).fmul_m(k(K_16)).fadd(xi(i32::from(yoff as u16)));
        yoff = ftol32(v) as u32;
        r = r.wrapping_add(1);
    }
}

/// One row cell of [`display_standard_menu`]: numbers and keys inserted into a local buffer (0x190
/// bytes in the original; larger here), orientation, the shrinking scale, then the text at
/// `(column x, y + yoff)` (`yoff` taken as its low word).
fn row_text(menu: u32, p: GPtr<u8>, c: u8, cell: isize, yoff: u32) {
    let mut buf = [0u8; 0x400];
    let bp = local(&mut buf);
    let n1 = rd::<i32>(p, 0x224 + cell * 4);
    let n2 = rd::<i32>(p, 0x2E4 + cell * 4);
    let text = text_get(g(THE_TEXT), p.byte_add(0x41 + cell * 10));
    insert_number_in_string(text, n1, n2, -1, -1, -1, -1, bp);
    let ci = isize::from(c);
    font_set_orientation(u32::from(rd::<u8>(panel(menu), 0x3EE + ci)));
    insert_player_control_keys(bp);
    let mut sc = k(K_052);
    let sh = f32_of(xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)));
    let sw = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640)).fmul_m(k(K_052)));
    font_set_scale(sw.to_bits(), sh.to_bits());
    let fits = |bp: GPtr<u8>| {
        let w = X87::from_f64_exact(font_get_string_width_st0(bp, 1, 0));
        w.fcom(x(rd::<f32>(panel(menu), 0x3FC + ci * 4))).test_any(0x41)
    };
    if !fits(bp) {
        loop {
            sc = f32_of(x(sc).fsub_m(k(K_005)));
            let sh = f32_of(xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)));
            let sw = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640)).fmul_m(sc));
            font_set_scale(sw.to_bits(), sh.to_bits());
            if fits(bp) {
                break;
            }
        }
    }
    let off = column_offset(panel(menu), c) as u16;
    let p = panel(menu);
    let o = rd::<u8>(p, 0x3EE + ci);
    let yy = f32_of(xi(i32::from(yoff as u16)).fadd_m(rd::<f32>(p, 0x410)));
    font_print_string(column_x(p, c, o, u32::from(off)), yy, bp);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INPUT_GRID_MENU,
    HOOK_FILL_GRID_WITH_CAR_COLOURS,
    HOOK_DISPLAY_GRID_MENU,
    HOOK_DISPLAY_STANDARD_MENU,
];
