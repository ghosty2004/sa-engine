//! The host functions (the mod's imports, `sa_mod_api::IMPORTS`), implemented once for both engine backends: a
//! backend decodes nothing itself, it hands [`dispatch`] the import index, the raw values and an accessor of the
//! mod's linear memory ([`Mem`]).

use std::rc::Rc;

use sa_mod_api::state::{CameraInfo, PedInfo, PlayerInfo, VehicleInfo, Words, WorldInfo, to_bytes};
use sa_mod_api::{Call, IMPORTS, MAX_FILE, Permission, VIDEO_MODE_MAX, VIDEO_MODE_MIN, Val, event, hook, status, valid_file_name};
use sa_platform::gpu::OverlayRect;
use sa_samp_api::{GameAdapter, PedRef, VehicleRef};

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

/// The game, read through the SA:MP game adapter (the engine's `sa-samp` backend; a fake in tests).
#[derive(Clone)]
pub struct Game(pub Rc<dyn GameAdapter>);

impl std::fmt::Debug for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Game")
    }
}

/// The game functions mods can hook: the engine's hook registry.
pub trait HookRegistry {
    /// The function named `name` (as the registry names it), as an id the host passes back in
    /// [`HookRegistry::watch`] and receives in `Host::hook`.
    fn resolve(&self, name: &str) -> Option<u32>;
    /// Starts or stops reporting the calls of `target`.
    fn watch(&self, target: u32, on: bool);
}

/// The engine's hook registry, as the host holds it.
#[derive(Clone)]
pub struct Registry(pub Rc<dyn HookRegistry>);

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Registry")
    }
}

/// The hooked call `sa_hook` runs for.
#[derive(Clone, Debug, PartialEq)]
pub struct HookCall {
    /// Its arguments (at most [`hook::MAX_ARGS`]).
    pub args: Vec<hook::Value>,
    /// Its result (after the call, of a function that returns one).
    pub result: Option<hook::Value>,
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
    /// The game (`None`: no state queries, e.g. before the game exists).
    pub game: Option<Game>,
    /// The hook registry (`None`: nothing can be hooked).
    pub registry: Option<Registry>,
    /// The functions the manifest lets the mod hook.
    pub hook_names: Rc<[String]>,
    /// The id the mod's next hook gets (its number of hooks so far).
    pub hook_base: usize,
    /// The hooks the call subscribed to: (registry id, phases).
    pub subscribed: Vec<(u32, i32)>,
    /// The event mask the call subscribed to.
    pub events: Option<i32>,
    /// The hooked call of an `sa_hook` call.
    pub hook_call: Option<HookCall>,
}

impl CallCtx {
    /// Whether the permissions grant `p`.
    fn grants(&self, p: Permission) -> bool {
        match p {
            Permission::None => true,
            Permission::Render => self.permissions.render,
            Permission::Files => self.permissions.files && self.files.is_some(),
            Permission::State => self.permissions.state,
            Permission::Input => self.permissions.input,
            Permission::Hooks => self.permissions.hooks,
        }
    }

    /// The events the permissions allow, as a mask.
    pub fn allowed_events(&self) -> i32 {
        (0..event::COUNT).filter(|&k| self.grants(event::permission(k))).fold(0, |m, k| m | 1 << k)
    }
}

/// Writes `record` into the buffer (`ptr`, `len` bytes): its size, or `BAD_ARG`.
fn write_record<T: Words>(mem: &mut dyn Mem, ptr: i32, len: i32, record: &T) -> i32 {
    let bytes = to_bytes(record);
    if ptr < 0 || len < 0 || (len as usize) < bytes.len() || !mem.write(ptr as u32, &bytes) {
        return status::BAD_ARG;
    }
    bytes.len() as i32
}

/// Writes the handles into the buffer, as many as fit: their number, or `BAD_ARG`.
fn write_handles(mem: &mut dyn Mem, ptr: i32, len: i32, handles: impl ExactSizeIterator<Item = u32>) -> i32 {
    if ptr < 0 || len < 0 {
        return status::BAD_ARG;
    }
    let n = handles.len();
    let bytes: Vec<u8> = handles.take(len as usize / 4).flat_map(u32::to_le_bytes).collect();
    if !mem.write(ptr as u32, &bytes) {
        return status::BAD_ARG;
    }
    n.min(i32::MAX as usize) as i32
}

/// The local player as `player_info` reports it.
pub fn player_info(game: &dyn GameAdapter) -> PlayerInfo {
    let handle = game.local_player();
    PlayerInfo {
        handle,
        money: game.player_money(),
        wanted: game.wanted_level().unwrap_or(0),
        ped: handle.and_then(|p| game.ped_snapshot(p).ok()).map(|s| PedInfo::from(&s)).unwrap_or_default(),
    }
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
    if !ctx.grants(IMPORTS[index].permission) {
        return Ok(status::DENIED);
    }
    let game = ctx.game.clone();
    let game = game.as_ref().map(|g| &*g.0);
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
        Call::events_subscribe { mask } => {
            let granted = mask & ctx.allowed_events();
            ctx.events = Some(granted);
            granted
        }
        Call::game_state {} => game.map_or(status::NOT_FOUND, |g| g.game_state().raw()),
        Call::player_info { buf_ptr, buf_len } => {
            let Some(g) = game else { return Ok(status::NOT_FOUND) };
            write_record(mem, buf_ptr, buf_len, &player_info(g))
        }
        Call::ped_info { handle, buf_ptr, buf_len } => {
            let Some(s) = game.and_then(|g| g.ped_snapshot(PedRef(handle as u32)).ok()) else {
                return Ok(status::NOT_FOUND);
            };
            write_record(mem, buf_ptr, buf_len, &PedInfo::from(&s))
        }
        Call::vehicle_info { handle, buf_ptr, buf_len } => {
            let Some(s) = game.and_then(|g| g.vehicle_snapshot(VehicleRef(handle as u32)).ok()) else {
                return Ok(status::NOT_FOUND);
            };
            write_record(mem, buf_ptr, buf_len, &VehicleInfo::from(&s))
        }
        Call::world_info { buf_ptr, buf_len } => {
            let Some(g) = game else { return Ok(status::NOT_FOUND) };
            let w = WorldInfo::new(
                g.game_state(),
                g.game_time_ms(),
                g.time_step(),
                g.clock(),
                g.weather(),
                g.area_raw(),
                g.gravity(),
            );
            write_record(mem, buf_ptr, buf_len, &w)
        }
        Call::camera_info { buf_ptr, buf_len } => {
            let Some(g) = game else { return Ok(status::NOT_FOUND) };
            write_record(mem, buf_ptr, buf_len, &CameraInfo::from(&g.camera()))
        }
        Call::ped_list { buf_ptr, buf_len } => {
            let peds = game.map(|g| g.peds()).unwrap_or_default();
            write_handles(mem, buf_ptr, buf_len, peds.into_iter().map(|p| p.0))
        }
        Call::vehicle_list { buf_ptr, buf_len } => {
            let vehicles = game.map(|g| g.vehicles()).unwrap_or_default();
            write_handles(mem, buf_ptr, buf_len, vehicles.into_iter().map(|v| v.0))
        }
        Call::key_down { vk } => match u16::try_from(vk) {
            Ok(vk) if vk <= 0xFF => game.is_some_and(|g| g.key_held(vk)) as i32,
            _ => status::BAD_ARG,
        },
        Call::hook_subscribe { name_ptr, name_len, phases } => {
            let Some(name) = text(mem, name_ptr, name_len) else { return Ok(status::BAD_ARG) };
            if !ctx.hook_names.contains(&name) {
                return Ok(status::DENIED);
            }
            let phases = phases & (hook::BEFORE | hook::AFTER);
            if phases == 0 {
                return Ok(status::BAD_ARG);
            }
            let Some(target) = ctx.registry.as_ref().and_then(|r| r.0.resolve(&name)) else {
                return Ok(status::NOT_FOUND);
            };
            let id = ctx.hook_base + ctx.subscribed.len();
            ctx.subscribed.push((target, phases));
            id.min(i32::MAX as usize) as i32
        }
        Call::hook_arg_count {} => ctx.hook_call.as_ref().map_or(status::BAD_ARG, |c| c.args.len() as i32),
        Call::hook_value { index, out_ptr } => {
            let Some(c) = &ctx.hook_call else { return Ok(status::BAD_ARG) };
            let v = if index == -1 { c.result } else { usize::try_from(index).ok().and_then(|i| c.args.get(i).copied()) };
            let Some(v) = v else { return Ok(status::NOT_FOUND) };
            let (kind, word) = v.encode();
            if out_ptr < 0 || !mem.write(out_ptr as u32, &word.to_le_bytes()) {
                return Ok(status::BAD_ARG);
            }
            kind
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
            permissions: Permissions { render: true, ..Default::default() },
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
        ctx.permissions = Permissions { render: true, files: true, ..Default::default() };
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

    #[test]
    fn state_queries() {
        use sa_mod_api::state::from_bytes;
        use sa_samp::fake;
        let mut g = fake::game();
        fake::set_game_state(&mut g, 9);
        fake::set_local_player(&mut g, 75.0, [10.0, -20.0, 5.5]);
        let mut data = vec![0u8; 512];
        let m = &mut SliceMem(&mut data);
        let mut ctx = CallCtx { game: Some(Game(Rc::new(g))), ..Default::default() };
        let i = |n| import_index(n).unwrap();
        let buf = |len| [Val::I32(64), Val::I32(len)];
        assert_eq!(dispatch(&mut ctx, m, i("player_info"), &buf(400)), Ok(status::DENIED));
        assert_eq!(dispatch(&mut ctx, m, i("key_down"), &[Val::I32(0x41)]), Ok(status::DENIED));
        ctx.permissions.state = true;
        assert_eq!(dispatch(&mut ctx, m, i("game_state"), &[]), Ok(9));
        let n = 4 * PlayerInfo::WORDS as i32;
        assert_eq!(dispatch(&mut ctx, m, i("player_info"), &buf(n - 1)), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, m, i("player_info"), &buf(400)), Ok(n));
        let p: PlayerInfo = from_bytes(&m.0[64..]).unwrap();
        assert_eq!(p.handle, Some(PedRef(0x205)));
        assert_eq!((p.ped.health, p.ped.pos.x, p.ped.pos.z), (75.0, 10.0, 5.5));
        let ped = [Val::I32(0x205), Val::I32(64), Val::I32(400)];
        assert_eq!(dispatch(&mut ctx, m, i("ped_info"), &ped), Ok(4 * PedInfo::WORDS as i32));
        let stale = [Val::I32(0x206), Val::I32(64), Val::I32(400)];
        assert_eq!(dispatch(&mut ctx, m, i("ped_info"), &stale), Ok(status::NOT_FOUND));
        assert_eq!(dispatch(&mut ctx, m, i("vehicle_info"), &stale), Ok(status::NOT_FOUND));
        // one ped, a buffer for none: the count all the same
        assert_eq!(dispatch(&mut ctx, m, i("ped_list"), &buf(0)), Ok(1));
        assert_eq!(dispatch(&mut ctx, m, i("vehicle_list"), &buf(64)), Ok(0));
        assert_eq!(dispatch(&mut ctx, m, i("world_info"), &buf(400)), Ok(4 * WorldInfo::WORDS as i32));
        assert_eq!(dispatch(&mut ctx, m, i("camera_info"), &buf(400)), Ok(4 * CameraInfo::WORDS as i32));
        ctx.permissions.input = true;
        assert_eq!(dispatch(&mut ctx, m, i("key_down"), &[Val::I32(0x41)]), Ok(0));
        assert_eq!(dispatch(&mut ctx, m, i("key_down"), &[Val::I32(256)]), Ok(status::BAD_ARG));
        // events: only those the permissions allow
        let granted = dispatch(&mut ctx, m, i("events_subscribe"), &[Val::I32(-1)]).unwrap();
        assert_eq!(granted, (1 << event::COUNT) - 1);
        ctx.permissions = Permissions::default();
        let granted = dispatch(&mut ctx, m, i("events_subscribe"), &[Val::I32(-1)]).unwrap();
        assert_eq!(granted, (1 << event::FRAME_BEGIN) | (1 << event::FRAME_END));
        assert_eq!(ctx.events, Some(granted));
        // hook values only inside sa_hook
        ctx.permissions.hooks = true;
        assert_eq!(dispatch(&mut ctx, m, i("hook_arg_count"), &[]), Ok(status::BAD_ARG));
        ctx.hook_call = Some(HookCall { args: vec![hook::Value::Float(1.5)], result: Some(hook::Value::Int(-4)) });
        assert_eq!(dispatch(&mut ctx, m, i("hook_arg_count"), &[]), Ok(1));
        assert_eq!(dispatch(&mut ctx, m, i("hook_value"), &[Val::I32(0), Val::I32(8)]), Ok(1));
        assert_eq!(&m.0[8..12], &1.5f32.to_le_bytes());
        assert_eq!(dispatch(&mut ctx, m, i("hook_value"), &[Val::I32(-1), Val::I32(8)]), Ok(0));
        assert_eq!(&m.0[8..12], &(-4i32).to_le_bytes());
        assert_eq!(dispatch(&mut ctx, m, i("hook_value"), &[Val::I32(1), Val::I32(8)]), Ok(status::NOT_FOUND));
    }
}
