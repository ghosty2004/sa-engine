//! STD-5: `CTheScripts::DrawScriptSpritesAndRectangles(bool)` (0x464980), the draw pass over the
//! script rectangles / sprites table, ported from the 1.0 US bytes (gta-reversed `TheScripts.cpp` as a
//! reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// `CMenuManager::DrawWindowedText(float x, float y, float wrap, const char* title, const char* text, eFontAlignment)` (0x578F50)
    fn draw_windowed_text(this: GPtr<u8>, x: u32, y: u32, wrap: u32, title: GPtr<u8>, text: GPtr<u8>, align: u32) = thiscall 0x578F50 => unported;
    /// `CMenuManager::DrawWindow(const CRect&, const char* key, uint8 fadeState, CRGBA titleColour, bool, bool drawBox)` (0x573EE0)
    fn draw_window(this: GPtr<u8>, rect: GPtr<u8>, key: GPtr<u8>, fade: u32, colour: u32, unused: u32, draw_box: u32) = thiscall 0x573EE0 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite2d_draw_rect(rect: GPtr<u8>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350)
    fn sprite2d_draw(this: GPtr<u8>, rect: GPtr<u8>, colour: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CSprite2d::Draw(float x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA&)` (0x728520)
    fn sprite2d_draw_quad(this: GPtr<u8>, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32, colour: GPtr<u8>) = thiscall 0x728520 => unported;
}

/// `CTheScripts::IntroRectangles` (0xA92D68): 0x80 entries of 0x3C bytes.
const INTRO_RECTANGLES: usize = 0xA92D68;
/// `CTheScripts::ScriptSprites` (0xA94B68): `CSprite2d` (4 bytes) per texture id.
const SCRIPT_SPRITES: usize = 0xA94B68;
/// `FrontEndMenuManager` (0xBA6748).
const MENU_MANAGER: usize = 0xBA6748;
/// 0.5f (0x858B8C).
const HALF: usize = 0x858B8C;

fn at<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast::<T>()
}

/// `fld m32; fstp m32` of a field: a signalling NaN comes out quiet.
fn reload(p: GPtr<u8>, off: isize) -> f32 {
    X87::from_f32(at::<f32>(p, off).read()).to_f32()
}

/// The `CRect` local the cases build: left = +8, bottom = +0x14, right = +0x10, top = +0xC (each
/// through `fld`/`fstp` except `left`, copied with `mov`).
fn rect_of(e: GPtr<u8>) -> [u32; 4] {
    [
        at::<u32>(e, 8).read(),
        reload(e, 0x14).to_bits(),
        reload(e, 0x10).to_bits(),
        reload(e, 0xC).to_bits(),
    ]
}

/// `CTheScripts::DrawScriptSpritesAndRectangles(bool bBeforeFade)` (0x464980). For each of the 0x80
/// entries whose `bBeforeFade` byte (+4) equals the argument's low byte, by type (+0, 1..5; others
/// skipped):
/// 1. `FrontEndMenuManager.DrawWindowedText(+8, +0xC, +0x10, &+0x20, &+0x2A, (uint8)+0x34)`;
/// 2. `DrawWindow(rect, &+0x20, 0, CRGBA(0, 0, 0, 190), (uint8)+0x38, true)`;
/// 3. `CSprite2d::DrawRect(rect, &+0x1C)`;
/// 4. `ScriptSprites[(int16)+6].Draw(rect, &+0x1C)`;
/// 5. the rotated sprite: centre `c = ((+0x10 + +8) * 0.5, (+0x14 + +0xC) * 0.5)`, half sizes
///    `w = cx - +8`, `h = cy - +0xC`, `C = cos(+0x18)`, `S = sin(+0x18)`, the products stored as f32
///    locals, and `ScriptSprites[(int16)+6].Draw` with the four corners (x87 code below, in the
///    original's evaluation order; PC=24 arithmetic, FSIN/FCOS on the 80-bit argument).
#[hook(addr = 0x464980, conv = cdecl, name = "CTheScripts::DrawScriptSpritesAndRectangles", expect = "83 EC 48 53 8A 5C 24 50",
       unverified = "std5-script-1", route = "L3-only", hazards = "x87-transcendental", recs = "menus-hud",
       note = "draws through the menu manager / CSprite2d (render side effects)")]
pub fn draw_script_sprites_and_rectangles(before_fade: u32) {
    let bl = before_fade as u8;
    for i in 0..0x80isize {
        let e = GPtr::<u8>::from_addr(INTRO_RECTANGLES).byte_add(i * 0x3C);
        if at::<u8>(e, 4).read() != bl {
            continue;
        }
        let colour = e.byte_add(0x1C);
        let sprite = || GPtr::<u8>::from_addr(SCRIPT_SPRITES).byte_add(isize::from(at::<i16>(e, 6).read()) * 4);
        match at::<u32>(e, 0).read() {
            1 => draw_windowed_text(
                GPtr::from_addr(MENU_MANAGER),
                at::<u32>(e, 8).read(),
                at::<u32>(e, 0xC).read(),
                at::<u32>(e, 0x10).read(),
                e.byte_add(0x20),
                e.byte_add(0x2A),
                u32::from(at::<u8>(e, 0x34).read()),
            ),
            2 => {
                let mut rect = rect_of(e);
                // CRGBA(0, 0, 0, 190) built by the constructor 0x7170C0 in a local, passed by value
                draw_window(
                    GPtr::from_addr(MENU_MANAGER),
                    GPtr::from_raw(rect.as_mut_ptr()).cast(),
                    e.byte_add(0x20),
                    0,
                    0xBE00_0000,
                    u32::from(at::<u8>(e, 0x38).read()),
                    1,
                );
            }
            3 => {
                let mut rect = rect_of(e);
                sprite2d_draw_rect(GPtr::from_raw(rect.as_mut_ptr()).cast(), colour);
            }
            4 => {
                let mut rect = rect_of(e);
                sprite2d_draw(sprite(), GPtr::from_raw(rect.as_mut_ptr()).cast(), colour);
            }
            5 => draw_rotated(e, sprite(), colour),
            _ => {}
        }
    }
}

/// Type 5 of [`draw_script_sprites_and_rectangles`]: the x87 sequence 0x464A9A..0x464B7F.
fn draw_rotated(e: GPtr<u8>, sprite: GPtr<u8>, colour: GPtr<u8>) {
    let f = |off: isize| at::<f32>(e, off).read();
    let half = GPtr::<f32>::from_addr(HALF).read();
    let cx = X87::from_f32(f(0x10)).fadd_m(f(8)).fmul_m(half);
    let cy = X87::from_f32(f(0x14)).fadd_m(f(0xC)).fmul_m(half);
    let w = cx.fsub_m(f(8));
    let h = cy.fsub_m(f(0xC));
    let c = X87::from_f32(f(0x18)).fcos();
    let s = X87::from_f32(f(0x18)).fsin();
    // the f32 locals [esp+0x30..0x40] (after the 0x20 bytes of arguments)
    let t34 = s.to_f32();
    let t30 = c.fmul(h).to_f32();
    let t38 = X87::from_f32(t34).fmul(w).to_f32();
    let t3c = c.fmul(w).to_f32();
    let t34 = X87::from_f32(t34).fmul(h).to_f32();
    let neg_sw = X87::from_f32(t38).fchs();
    let t40 = X87::from_f32(t3c).fchs().to_f32();
    let y4 = X87::from_f32(t38).fadd_m(t30).fadd(cy).to_f32();
    let x4 = X87::from_f32(t3c).fsub_m(t34).fadd(cx).to_f32();
    let y3 = X87::from_f32(t30).fadd(neg_sw).fadd(cy).to_f32();
    let x3 = X87::from_f32(t40).fsub_m(t34).fadd(cx).to_f32();
    let y2 = X87::from_f32(t38).fsub_m(t30).fadd(cy).to_f32();
    let x2 = X87::from_f32(t34).fadd_m(t3c).fadd(cx).to_f32();
    let y1 = neg_sw.fsub_m(t30).fadd(cy).to_f32();
    let x1 = X87::from_f32(t40).fadd_m(t34).fadd(cx).to_f32();
    sprite2d_draw_quad(sprite, x1, y1, x2, y2, x3, y3, x4, y4, colour);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_DRAW_SCRIPT_SPRITES_AND_RECTANGLES];
