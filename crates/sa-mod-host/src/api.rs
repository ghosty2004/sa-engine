//! The host functions (the mod's imports, `sa_mod_api::IMPORTS`), implemented once for both engine backends: a
//! backend decodes nothing itself, it hands [`dispatch`] the import index, the raw values and an accessor of the
//! mod's linear memory ([`Mem`]).

use std::rc::Rc;

use sa_mod_api::{Call, IMPORTS, MAX_FILE, Permission, VIDEO_MODE_MAX, VIDEO_MODE_MIN, Val, status, valid_file_name};
use sa_platform::gpu::OverlayRect;

use crate::ModSource;
use crate::manifest::Permissions;

/// The longest text a host function reads, in bytes.
pub const MAX_TEXT: usize = 4096;
/// Overlay rectangles one mod may emit per frame (text counts one per horizontal run of a glyph row).
pub const MAX_RECTS: usize = 65_536;
/// Log lines one call may write.
pub const MAX_LOG_LINES: usize = 64;

/// The display the platform reports (`display_mode_*`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Display {
    /// The desktop (natively the monitor's current mode; in the browser the canvas in device pixels).
    pub desktop: (u32, u32),
    /// The display modes, `(width, height)`.
    pub modes: Vec<(u32, u32)>,
}

/// A video mode a mod requested (`request_video_mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VideoRequest {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Full screen (natively), else a window.
    pub fullscreen: bool,
}

/// The mod's own folder, for `file_read` / `file_write`.
#[derive(Clone)]
pub struct Files(pub Rc<dyn ModSource>);

impl std::fmt::Debug for Files {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Files({})", self.0.location())
    }
}

/// What a call may do and what it produced.
#[derive(Debug, Default)]
pub struct CallCtx {
    /// The mod's granted permissions.
    pub permissions: Permissions,
    /// The overlay's size in pixels (the game's back buffer; 0 x 0 before the game's device exists).
    pub screen: (u32, u32),
    /// True while the overlay phase runs (overlay drawing is valid).
    pub overlay: bool,
    /// The display the platform reports.
    pub display: Rc<Display>,
    /// The mod's own folder (`None`: no file access).
    pub files: Option<Files>,
    /// The overlay rectangles of this call.
    pub rects: Vec<OverlayRect>,
    /// Log lines of this call: (level, text).
    pub logs: Vec<(i32, String)>,
    /// The video mode the call requested (the last request).
    pub video: Option<VideoRequest>,
}

/// The mod's linear memory, as the host functions see it.
pub trait Mem {
    /// A copy of `len` bytes at `ptr`; `None` when the range is outside the memory.
    fn read(&self, ptr: u32, len: u32) -> Option<Vec<u8>>;
    /// Writes `data` at `ptr`; false (nothing written) when the range is outside the memory.
    fn write(&mut self, ptr: u32, data: &[u8]) -> bool;
}

/// A memory as a byte slice (the native engine's view, and tests).
pub struct SliceMem<'a>(pub &'a mut [u8]);

impl Mem for SliceMem<'_> {
    fn read(&self, ptr: u32, len: u32) -> Option<Vec<u8>> {
        self.0.get(ptr as usize..(ptr as usize).checked_add(len as usize)?).map(<[u8]>::to_vec)
    }

    fn write(&mut self, ptr: u32, data: &[u8]) -> bool {
        let Some(end) = (ptr as usize).checked_add(data.len()) else { return false };
        match self.0.get_mut(ptr as usize..end) {
            Some(d) => {
                d.copy_from_slice(data);
                true
            }
            None => false,
        }
    }
}

/// Reads UTF-8 text of at most [`MAX_TEXT`] bytes.
fn text(mem: &dyn Mem, ptr: i32, len: i32) -> Option<String> {
    if ptr < 0 || len < 0 || len as usize > MAX_TEXT {
        return None;
    }
    String::from_utf8(mem.read(ptr as u32, len as u32)?).ok()
}

/// Reads a file name `file_read` / `file_write` accept.
fn file_name(mem: &dyn Mem, ptr: i32, len: i32) -> Option<String> {
    text(mem, ptr, len).filter(|n| valid_file_name(n))
}

/// Runs host function `index` with `args`. `Err` is a host-side trap (an import call that does not match the
/// interface: the module checks make it unreachable); everything a mod can get wrong is a negative status.
pub fn dispatch(ctx: &mut CallCtx, mem: &mut dyn Mem, index: usize, args: &[Val]) -> Result<i32, String> {
    let call = Call::decode(index, args).ok_or_else(|| format!("import {index} called with {args:?}"))?;
    let granted = match IMPORTS[index].permission {
        Permission::None => true,
        Permission::Render => ctx.permissions.render,
        Permission::Files => ctx.permissions.files && ctx.files.is_some(),
    };
    if !granted {
        return Ok(status::DENIED);
    }
    let mode = |ctx: &CallCtx, index: i32| -> Option<(u32, u32)> {
        match index {
            -1 => Some(ctx.display.desktop),
            i if i >= 0 => ctx.display.modes.get(i as usize).copied(),
            _ => None,
        }
    };
    Ok(match call {
        Call::log { level, text_ptr, text_len } => {
            let Some(s) = text(mem, text_ptr, text_len) else { return Ok(status::BAD_ARG) };
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
            let Some(s) = text(mem, text_ptr, text_len) else { return Ok(status::BAD_ARG) };
            let mut rects = Vec::new();
            let width = text_rects(x, y, size, rgba as u32, &s, &mut rects);
            if ctx.rects.len() + rects.len() > MAX_RECTS {
                return Ok(status::BAD_ARG);
            }
            ctx.rects.extend(rects);
            width.min(i32::MAX as f32) as i32
        }
        Call::display_mode_count {} => ctx.display.modes.len().min(i32::MAX as usize) as i32,
        Call::display_mode_width { index } => mode(ctx, index).map_or(status::BAD_ARG, |m| m.0 as i32),
        Call::display_mode_height { index } => mode(ctx, index).map_or(status::BAD_ARG, |m| m.1 as i32),
        Call::request_video_mode { width, height, fullscreen } => {
            if !(VIDEO_MODE_MIN.0..=VIDEO_MODE_MAX).contains(&width)
                || !(VIDEO_MODE_MIN.1..=VIDEO_MODE_MAX).contains(&height)
            {
                return Ok(status::BAD_ARG);
            }
            ctx.video = Some(VideoRequest { width: width as u32, height: height as u32, fullscreen: fullscreen != 0 });
            status::OK
        }
        Call::file_read { name_ptr, name_len, buf_ptr, buf_len } => {
            let Some(name) = file_name(mem, name_ptr, name_len) else { return Ok(status::BAD_ARG) };
            if buf_ptr < 0 || buf_len < 0 {
                return Ok(status::BAD_ARG);
            }
            let Some(Files(src)) = &ctx.files else { return Ok(status::DENIED) };
            let Ok(data) = src.read(&name) else { return Ok(status::NOT_FOUND) };
            let n = data.len().min(buf_len as usize);
            if !mem.write(buf_ptr as u32, &data[..n]) {
                return Ok(status::BAD_ARG);
            }
            data.len().min(i32::MAX as usize) as i32
        }
        Call::file_write { name_ptr, name_len, data_ptr, data_len } => {
            let Some(name) = file_name(mem, name_ptr, name_len) else { return Ok(status::BAD_ARG) };
            if data_ptr < 0 || data_len < 0 || data_len as usize > MAX_FILE {
                return Ok(status::BAD_ARG);
            }
            let Some(data) = mem.read(data_ptr as u32, data_len as u32) else { return Ok(status::BAD_ARG) };
            let Some(Files(src)) = &ctx.files else { return Ok(status::DENIED) };
            match src.write(&name, &data) {
                Ok(()) => status::OK,
                Err(e) => {
                    if ctx.logs.len() < MAX_LOG_LINES {
                        ctx.logs.push((sa_mod_api::level::ERROR, format!("file_write {name}: {e}")));
                    }
                    status::BAD_ARG
                }
            }
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

    #[test]
    fn overlay_calls() {
        let mut data = *b"hi\xFF";
        let m = &mut SliceMem(&mut data);
        let mut ctx = CallCtx {
            permissions: Permissions { render: true, files: false },
            screen: (640, 480),
            overlay: true,
            ..Default::default()
        };
        let text = import_index("overlay_text").unwrap();
        let args = |p: i32, n: i32| [Val::F32(0.0), Val::F32(0.0), Val::F32(16.0), Val::I32(-1), Val::I32(p), Val::I32(n)];
        assert_eq!(dispatch(&mut ctx, m, text, &args(0, 2)), Ok(32));
        assert!(!ctx.rects.is_empty());
        // out of bounds, not UTF-8
        assert_eq!(dispatch(&mut ctx, m, text, &args(2, 9)), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, m, text, &args(2, 1)), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, m, import_index("screen_height").unwrap(), &[]), Ok(480));
        // a wrong argument list is a host trap
        assert!(dispatch(&mut ctx, m, text, &[]).is_err());
        // no permission, outside the overlay phase
        ctx.permissions.render = false;
        assert_eq!(dispatch(&mut ctx, m, text, &args(0, 2)), Ok(status::DENIED));
        ctx.permissions.render = true;
        ctx.overlay = false;
        let rect = [Val::F32(0.0), Val::F32(0.0), Val::F32(1.0), Val::F32(1.0), Val::I32(0)];
        assert_eq!(dispatch(&mut ctx, m, import_index("overlay_rect").unwrap(), &rect), Ok(status::BAD_ARG));
    }

    /// A folder in memory.
    #[derive(Default)]
    struct MemFolder(std::cell::RefCell<std::collections::BTreeMap<String, Vec<u8>>>);
    impl ModSource for MemFolder {
        fn read(&self, file: &str) -> Result<Vec<u8>, String> {
            self.0.borrow().get(file).cloned().ok_or("missing".into())
        }
        fn write(&self, file: &str, data: &[u8]) -> Result<(), String> {
            self.0.borrow_mut().insert(file.into(), data.to_vec());
            Ok(())
        }
        fn location(&self) -> String {
            "memory".into()
        }
    }

    #[test]
    fn display_video_mode_and_files() {
        let mut data = vec![0u8; 64];
        data[..11].copy_from_slice(b"config.toml");
        data[16..21].copy_from_slice(b"w=800");
        let m = &mut SliceMem(&mut data);
        let display = Rc::new(Display { desktop: (1920, 1080), modes: vec![(800, 600), (1920, 1080)] });
        let mut ctx = CallCtx { display, ..Default::default() };
        let i = |n| import_index(n).unwrap();
        // no permissions
        assert_eq!(dispatch(&mut ctx, m, i("display_mode_count"), &[]), Ok(status::DENIED));
        ctx.permissions = Permissions { render: true, files: true };
        assert_eq!(dispatch(&mut ctx, m, i("display_mode_count"), &[]), Ok(2));
        assert_eq!(dispatch(&mut ctx, m, i("display_mode_width"), &[Val::I32(-1)]), Ok(1920));
        assert_eq!(dispatch(&mut ctx, m, i("display_mode_height"), &[Val::I32(0)]), Ok(600));
        assert_eq!(dispatch(&mut ctx, m, i("display_mode_height"), &[Val::I32(2)]), Ok(status::BAD_ARG));
        let req = |w, h| [Val::I32(w), Val::I32(h), Val::I32(1)];
        assert_eq!(dispatch(&mut ctx, m, i("request_video_mode"), &req(100, 100)), Ok(status::BAD_ARG));
        assert_eq!(ctx.video, None);
        assert_eq!(dispatch(&mut ctx, m, i("request_video_mode"), &req(1920, 1080)), Ok(0));
        assert_eq!(ctx.video, Some(VideoRequest { width: 1920, height: 1080, fullscreen: true }));
        // files: none without a folder; then write, read back (truncated to the buffer), missing
        let name = [Val::I32(0), Val::I32(11)];
        let rd = |len| [name[0], name[1], Val::I32(32), Val::I32(len)];
        assert_eq!(dispatch(&mut ctx, m, i("file_read"), &rd(8)), Ok(status::DENIED));
        ctx.files = Some(Files(Rc::new(MemFolder::default())));
        assert_eq!(dispatch(&mut ctx, m, i("file_read"), &rd(8)), Ok(status::NOT_FOUND));
        let wr = [name[0], name[1], Val::I32(16), Val::I32(5)];
        assert_eq!(dispatch(&mut ctx, m, i("file_write"), &wr), Ok(0));
        assert_eq!(dispatch(&mut ctx, m, i("file_read"), &rd(3)), Ok(5));
        assert_eq!(&m.0[32..36], b"w=8\0");
        // a bad name, a buffer outside the memory
        let bad = [Val::I32(16), Val::I32(5), Val::I32(32), Val::I32(8)];
        assert_eq!(dispatch(&mut ctx, m, i("file_read"), &bad), Ok(status::BAD_ARG));
        let outside = [name[0], name[1], Val::I32(60), Val::I32(8)];
        assert_eq!(dispatch(&mut ctx, m, i("file_read"), &outside), Ok(status::BAD_ARG));
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
