//! The host functions (the mod's imports, `sa_mod_api::IMPORTS`), implemented once for both engine backends: a
//! backend decodes nothing itself, it hands [`dispatch`] the import index, the raw values and a reader of the mod's
//! linear memory.

use sa_mod_api::{Call, IMPORTS, Permission, Val, status};
use sa_platform::gpu::OverlayRect;

use crate::manifest::Permissions;

/// The longest text a host function reads, in bytes.
pub const MAX_TEXT: usize = 4096;
/// Overlay rectangles one mod may emit per frame (text counts one per horizontal run of a glyph row).
pub const MAX_RECTS: usize = 65_536;
/// Log lines one call may write.
pub const MAX_LOG_LINES: usize = 64;

/// What a call may do and what it produced.
#[derive(Debug, Default)]
pub struct CallCtx {
    /// The mod's granted permissions.
    pub permissions: Permissions,
    /// The overlay's size in pixels.
    pub screen: (u32, u32),
    /// True while the overlay phase runs (overlay drawing is valid).
    pub overlay: bool,
    /// The overlay rectangles of this call.
    pub rects: Vec<OverlayRect>,
    /// Log lines of this call: (level, text).
    pub logs: Vec<(i32, String)>,
}

/// Reads `len` bytes at `ptr` of the mod's memory; `None` when the range is outside it.
pub type MemReader<'a> = &'a dyn Fn(u32, u32) -> Option<Vec<u8>>;

/// Runs host function `index` with `args`. `Err` is a host-side trap (an import call that does not match the
/// interface: the module checks make it unreachable); everything a mod can get wrong is a negative status.
pub fn dispatch(ctx: &mut CallCtx, mem: MemReader<'_>, index: usize, args: &[Val]) -> Result<i32, String> {
    let call = Call::decode(index, args).ok_or_else(|| format!("import {index} called with {args:?}"))?;
    if IMPORTS[index].permission == Permission::Render && !ctx.permissions.render {
        return Ok(status::DENIED);
    }
    let text = |ptr: i32, len: i32| -> Option<String> {
        if ptr < 0 || len < 0 || len as usize > MAX_TEXT {
            return None;
        }
        String::from_utf8(mem(ptr as u32, len as u32)?).ok()
    };
    Ok(match call {
        Call::log { level, text_ptr, text_len } => {
            let Some(s) = text(text_ptr, text_len) else { return Ok(status::BAD_ARG) };
            if ctx.logs.len() < MAX_LOG_LINES {
                ctx.logs.push((level, s));
            }
            status::OK
        }
        Call::screen_width {} => ctx.screen.0 as i32,
        Call::screen_height {} => ctx.screen.1 as i32,
        Call::overlay_rect { x, y, w, h, rgba } => {
            if !ctx.overlay || ![x, y, w, h].iter().all(|v| v.is_finite()) || ctx.rects.len() >= MAX_RECTS {
                return Ok(status::BAD_ARG);
            }
            if w > 0.0 && h > 0.0 {
                ctx.rects.push(OverlayRect { x, y, w, h, rgba: rgba as u32 });
            }
            status::OK
        }
        Call::overlay_text { x, y, size, rgba, text_ptr, text_len } => {
            if !ctx.overlay || ![x, y, size].iter().all(|v| v.is_finite()) || !(0.0..=1024.0).contains(&size) {
                return Ok(status::BAD_ARG);
            }
            let Some(s) = text(text_ptr, text_len) else { return Ok(status::BAD_ARG) };
            let mut rects = Vec::new();
            let width = text_rects(x, y, size, rgba as u32, &s, &mut rects);
            if ctx.rects.len() + rects.len() > MAX_RECTS {
                return Ok(status::BAD_ARG);
            }
            ctx.rects.extend(rects);
            width.min(i32::MAX as f32) as i32
        }
    })
}

/// Lays out `text` in the host's 8x8 font (`font8x8`, ASCII; other characters as `?`), `size` pixels per glyph
/// cell, as one rectangle per horizontal run of set pixels in a glyph row. Returns the width in pixels.
pub fn text_rects(x: f32, y: f32, size: f32, rgba: u32, text: &str, out: &mut Vec<OverlayRect>) -> f32 {
    let px = size / 8.0;
    let mut n = 0usize;
    for ch in text.chars() {
        let c = if (' '..='~').contains(&ch) { ch as usize } else { '?' as usize };
        let glyph = font8x8::legacy::BASIC_LEGACY[c];
        let gx = x + (n * 8) as f32 * px;
        for (row, bits) in glyph.iter().enumerate() {
            let mut col = 0;
            while col < 8 {
                if bits >> col & 1 == 0 {
                    col += 1;
                    continue;
                }
                let start = col;
                while col < 8 && bits >> col & 1 != 0 {
                    col += 1;
                }
                out.push(OverlayRect {
                    x: gx + start as f32 * px,
                    y: y + row as f32 * px,
                    w: (col - start) as f32 * px,
                    h: px,
                    rgba,
                });
            }
        }
        n += 1;
    }
    n as f32 * size
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_mod_api::import_index;

    fn mem(data: &'static [u8]) -> impl Fn(u32, u32) -> Option<Vec<u8>> {
        move |p, n| data.get(p as usize..(p as usize).checked_add(n as usize)?).map(|s| s.to_vec())
    }

    #[test]
    fn overlay_calls() {
        let m = mem(b"hi\xFF");
        let mut ctx = CallCtx { permissions: Permissions { render: true }, screen: (640, 480), overlay: true, ..Default::default() };
        let text = import_index("overlay_text").unwrap();
        let args = |p: i32, n: i32| [Val::F32(0.0), Val::F32(0.0), Val::F32(16.0), Val::I32(-1), Val::I32(p), Val::I32(n)];
        assert_eq!(dispatch(&mut ctx, &m, text, &args(0, 2)), Ok(32));
        assert!(!ctx.rects.is_empty());
        // out of bounds, not UTF-8
        assert_eq!(dispatch(&mut ctx, &m, text, &args(2, 9)), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, &m, text, &args(2, 1)), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, &m, import_index("screen_height").unwrap(), &[]), Ok(480));
        // a wrong argument list is a host trap
        assert!(dispatch(&mut ctx, &m, text, &[]).is_err());
        // no permission, outside the overlay phase
        ctx.permissions.render = false;
        assert_eq!(dispatch(&mut ctx, &m, text, &args(0, 2)), Ok(status::DENIED));
        ctx.permissions.render = true;
        ctx.overlay = false;
        let rect = [Val::F32(0.0), Val::F32(0.0), Val::F32(1.0), Val::F32(1.0), Val::I32(0)];
        assert_eq!(dispatch(&mut ctx, &m, import_index("overlay_rect").unwrap(), &rect), Ok(status::BAD_ARG));
    }

    #[test]
    fn glyph_runs() {
        let mut out = Vec::new();
        // '-' in font8x8 is one row with a run of 6 pixels
        assert_eq!(text_rects(0.0, 0.0, 8.0, 0, "-", &mut out), 8.0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].w, 6.0);
        out.clear();
        text_rects(0.0, 0.0, 8.0, 0, "\u{e9}", &mut out);
        let mut q = Vec::new();
        text_rects(0.0, 0.0, 8.0, 0, "?", &mut q);
        assert_eq!(out, q);
    }
}
