//! The host functions (the mod's imports, `sa_mod_api::IMPORTS`), implemented once for both engine backends: a
//! backend decodes nothing itself, it hands [`dispatch`] the import index, the raw values and an accessor of the
//! mod's linear memory ([`Mem`]).

use std::cell::RefCell;
use std::rc::Rc;

use sa_access::View;
use sa_mod_api::state::{CameraInfo, MatrixInfo, PadInfo, PedInfo, PlayerInfo, VehicleInfo, Words, WorldInfo, to_bytes};
use sa_mod_api::{
    Call, IMPORTS, MAX_FILE, Permission, VIDEO_MODE_MAX, VIDEO_MODE_MIN, Val, blip, checkpoint, event, hook, http,
    image, net, pickup, radar, script, status, storage, valid_file_name,
};
use sa_platform::gpu::OverlayQuad;
use sa_types::script::{Opcode, ScriptArg, ScriptLocals};
use sa_types::{
    BlipRef, CheckpointRef, EntityRef, GameError, GameOption, GameResult, ModelId, ObjectRef, PedRef, PickupRef, Vec3,
    VehicleRef, WeaponType,
};

use crate::ModSource;
use crate::approval::Approvals;
use crate::http::{Http, Outcome, Requests};
use crate::images::{self, CreateError, Images};
use crate::manifest::{NetRule, Permissions, split_target};
use crate::net::{LinkState, Net, Sockets};
use crate::storage::Store;

/// The longest text a host function reads, in bytes.
pub const MAX_TEXT: usize = 4096;
/// Overlay quads one mod may emit per frame (text counts one per horizontal run of a glyph row).
pub const MAX_QUADS: usize = 65_536;
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

/// The game as the engine's access layer (`sa-access`) reaches it: the engine's memory in the game, a fake in tests.
pub type GameAccess = sa_access::Game<Box<dyn View>>;

/// The game the host functions read and change.
#[derive(Clone)]
pub struct Game(pub Rc<RefCell<GameAccess>>);

impl Game {
    /// The game over `game`'s view.
    pub fn new<V: View + 'static>(game: sa_access::Game<V>) -> Game {
        Game(Rc::new(RefCell::new(game.into_dyn())))
    }
}

impl std::fmt::Debug for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Game")
    }
}

/// What a mod queued for the game's 2D phase.
#[derive(Clone, Debug, PartialEq)]
pub enum Game2d {
    /// `game_text`.
    Text(sa_types::text::FontText, Vec<u8>),
    /// `game_sprite`: a texture of [`sa_access::load_texture`], `[left, top, right, bottom]`, `0xRRGGBBAA`.
    Sprite {
        /// The texture.
        texture: u32,
        /// The rectangle.
        rect: [f32; 4],
        /// The tint.
        rgba: u32,
    },
}

impl Game2d {
    /// Draws it into the game's frame ([`sa_access::print_text`], [`sa_access::draw_sprite`]); only while the game
    /// draws its 2D phase.
    pub fn draw(&self) {
        match self {
            Game2d::Text(style, text) => sa_access::print_text(style, text),
            Game2d::Sprite { texture, rect, rgba } => sa_access::draw_sprite(*texture, *rect, *rgba),
        }
    }
}

/// What a mod created and owns: deleted when the mod deletes it or is disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owned {
    /// A ped, vehicle or object.
    Entity(EntityRef),
    /// A pickup.
    Pickup(PickupRef),
    /// A checkpoint marker.
    Checkpoint(CheckpointRef),
    /// A radar blip.
    Blip(BlipRef),
}

/// A radar area a mod shows (`set_radar_area`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadarArea {
    /// The world rectangle's lower corner.
    pub min: (f32, f32),
    /// Its upper corner.
    pub max: (f32, f32),
    /// Its colour, `0xRRGGBBAA`.
    pub rgba: u32,
    /// The colour it alternates with and the period in milliseconds (`flash_radar_area`).
    pub flash: Option<(u32, u32)>,
}

impl RadarArea {
    /// The colour shown `ms` milliseconds after the host started: the second colour in every other period.
    pub fn colour_at(&self, ms: u64) -> u32 {
        match self.flash {
            Some((rgba, period)) if period > 0 && (ms / u64::from(period)) % 2 == 1 => rgba,
            _ => self.rgba,
        }
    }
}

/// A mod's radar areas, by id (drawn in id order).
pub type RadarAreas = std::collections::BTreeMap<u16, RadarArea>;

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

/// What the hooks of one call decided (`hook_set_arg`, `hook_skip`, `hook_set_result`), for the engine to apply
/// with the call's declared types.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HookDecision {
    /// Replaced arguments: (index, new value).
    pub args: Vec<(usize, hook::Value)>,
    /// Skip the call; the caller gets this result word.
    pub skip: Option<i32>,
    /// Replace the result with this word.
    pub result: Option<i32>,
}

impl HookDecision {
    /// Whether anything was decided.
    pub fn is_empty(&self) -> bool {
        self.args.is_empty() && self.skip.is_none() && self.result.is_none()
    }

    /// Takes over what `later` (a later mod) decided.
    pub fn merge(&mut self, later: HookDecision) {
        for (i, v) in later.args {
            self.args.retain(|a| a.0 != i);
            self.args.push((i, v));
        }
        self.skip = later.skip.or(self.skip);
        self.result = later.result.or(self.result);
    }
}

/// Changes queued from inside a hook, applied at the next frame boundary: at most this many per mod and frame.
pub const MAX_DEFERRED: usize = 256;

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
    /// The overlay quads of this call.
    pub quads: Vec<OverlayQuad>,
    /// The overlay's clip rectangle (`overlay_clip`): `[x, y, w, h]`.
    pub clip: Option<[f32; 4]>,
    /// The mod's overlay images.
    pub images: Images,
    /// The game's 2D drawing this call queued (`game_text`, `game_sprite`).
    pub game_2d: Vec<Game2d>,
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
    /// The hooks the call subscribed to: (registry id, phases, controllable).
    pub subscribed: Vec<(u32, i32, bool)>,
    /// The event mask the call subscribed to.
    pub events: Option<i32>,
    /// The hooked call of an `sa_hook` call.
    pub hook_call: Option<HookCall>,
    /// The phase of the hooked call ([`hook::BEFORE`] / [`hook::AFTER`]; 0 outside `sa_hook`).
    pub hook_phase: i32,
    /// The hooked function is one of [`hook::CONTROLLABLE`].
    pub hook_control: bool,
    /// What the call decided about the hooked call.
    pub decision: HookDecision,
    /// Changes asked for inside a hook, for the next frame boundary.
    pub deferred: Vec<Call>,
    /// What the mod created and has not deleted.
    pub owned: Vec<Owned>,
    /// The mod's radar areas.
    pub radar_areas: RadarAreas,
    /// When the host started (`clock`'s monotonic time; `None`: zero).
    pub epoch: Option<web_time::Instant>,
    /// The network the mod's sockets go to (`None`: none).
    pub network: Option<Net>,
    /// The targets the manifest lets the mod open sockets to.
    pub net_rules: Rc<[NetRule]>,
    /// The mod's open sockets.
    pub sockets: Sockets,
    /// The mod's storage area (`None`: no storage).
    pub storage: Option<Store>,
    /// What the mod takes from the game (`input_capture`): keyboard, mouse, the platform's cursor.
    pub capture: (bool, bool, bool),
    /// The game options the mod switched off (turned back on when the host disables it).
    pub options_off: Vec<GameOption>,
    /// The value the mod's entry point returned.
    pub result: i32,
    /// The mod's id and name.
    pub mod_id: (String, String),
    /// `net = ["udp:user-approved"]`: targets the user approves are allowed.
    pub user_approved: bool,
    /// The user's answers (the host's).
    pub approvals: Option<Rc<RefCell<Approvals>>>,
    /// The HTTP client the mod's requests go to (`None`: none).
    pub http: Option<Http>,
    /// The URL prefixes the manifest lets the mod GET.
    pub http_prefixes: Rc<[String]>,
    /// The mod's requests.
    pub requests: Requests,
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
            Permission::Write => self.permissions.write,
            Permission::HookControl => self.permissions.hooks && self.permissions.write,
            Permission::Entities => self.permissions.entities,
            Permission::Scripts => self.permissions.scripts,
            Permission::Audio => self.permissions.audio,
            Permission::Net => self.permissions.net,
            Permission::Storage => self.permissions.storage,
            Permission::Http => self.permissions.http,
            Permission::Capture => self.permissions.capture,
        }
    }

    fn log_error(&mut self, text: String) {
        if self.logs.len() < MAX_LOG_LINES {
            self.logs.push((sa_mod_api::level::ERROR, text));
        }
    }

    /// True inside `sa_hook`, where the game is in the middle of its frame.
    pub fn in_hook(&self) -> bool {
        self.hook_call.is_some()
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
pub fn player_info(game: &GameAccess) -> PlayerInfo {
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
    if is_change(&call) {
        if ctx.in_hook() {
            if ctx.deferred.len() >= MAX_DEFERRED {
                return Ok(status::FAILED);
            }
            ctx.deferred.push(call);
            return Ok(status::OK);
        }
        let Some(g) = ctx.game.clone() else { return Ok(status::WRONG_PHASE) };
        return Ok(apply(&mut g.0.borrow_mut(), call, &mut ctx.logs));
    }
    if let Some(r) = entities_and_scripts(ctx, mem, call) {
        return Ok(r);
    }
    if let Some(r) = markers(ctx, call) {
        return Ok(r);
    }
    if let Some(r) = services(ctx, mem, call) {
        return Ok(r);
    }
    let game_rc = ctx.game.clone();
    let game_ref = game_rc.as_ref().map(|g| g.0.borrow());
    let game = game_ref.as_deref();
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
            if !ctx.overlay || ![x, y, w, h].iter().all(|v| v.is_finite()) || ctx.quads.len() >= MAX_QUADS {
                return Ok(status::BAD_ARG);
            }
            ctx.quads.extend(images::clip(OverlayQuad::rect(x, y, w, h, rgba as u32), ctx.clip));
            status::OK
        }
        Call::overlay_text { x, y, size, rgba, text_ptr, text_len } => {
            if !ctx.overlay || ![x, y, size].iter().all(|v| v.is_finite()) || !(0.0..=1024.0).contains(&size) {
                return Ok(status::BAD_ARG);
            }
            let Some(s) = text(mem, text_ptr, text_len) else { return Ok(status::BAD_ARG) };
            let mut quads = Vec::new();
            let width = text_rects(x, y, size, rgba as u32, &s, &mut quads);
            if ctx.quads.len() + quads.len() > MAX_QUADS {
                return Ok(status::BAD_ARG);
            }
            let clip = ctx.clip;
            ctx.quads.extend(quads.into_iter().filter_map(|q| images::clip(q, clip)));
            width.min(i32::MAX as f32) as i32
        }
        Call::image_create { width, height, flags, data_ptr, data_len } => {
            if width <= 0 || height <= 0 || (width as u64) * (height as u64) * 4 != data_len as u64 {
                return Ok(status::BAD_ARG);
            }
            if data_ptr < 0 {
                return Ok(status::BAD_ARG);
            }
            let Some(data) = mem.read(data_ptr as u32, data_len as u32) else { return Ok(status::BAD_ARG) };
            match ctx.images.create(width as u32, height as u32, flags & image::SMOOTH != 0, &data) {
                Ok(h) => h as i32,
                Err(CreateError::BadSize) => status::BAD_ARG,
                Err(CreateError::Budget) => {
                    ctx.logs.push((
                        sa_mod_api::level::WARN,
                        format!(
                            "image_create: over the budget ({} images, {} MiB)",
                            image::MAX_IMAGES,
                            image::MAX_BYTES >> 20
                        ),
                    ));
                    status::FAILED
                }
            }
        }
        Call::image_destroy { handle } => {
            if ctx.images.destroy(handle) { status::OK } else { status::NOT_FOUND }
        }
        Call::overlay_quads { handle, quads_ptr, count } => {
            if !ctx.overlay || !(0..=image::MAX_QUADS_PER_CALL as i32).contains(&count) {
                return Ok(status::BAD_ARG);
            }
            let id = match handle {
                image::NONE => 0,
                h => match ctx.images.id(h) {
                    Some(id) => id,
                    None => return Ok(status::NOT_FOUND),
                },
            };
            let words = image::QUAD_WORDS;
            let Some(data) = mem.read(quads_ptr as u32, count as u32 * (words * 4) as u32) else {
                return Ok(status::BAD_ARG);
            };
            if ctx.quads.len() + count as usize > MAX_QUADS {
                return Ok(status::BAD_ARG);
            }
            let mut parsed = Vec::with_capacity(count as usize);
            for q in data.chunks_exact(words * 4) {
                let w = |i: usize| u32::from_le_bytes(q[i * 4..i * 4 + 4].try_into().expect("4 bytes"));
                let f: [f32; 8] = std::array::from_fn(|i| f32::from_bits(w(i)));
                if !f.iter().all(|v| v.is_finite()) {
                    return Ok(status::BAD_ARG);
                }
                parsed.push(OverlayQuad { x: f[0], y: f[1], w: f[2], h: f[3], uv: [f[4], f[5], f[6], f[7]], rgba: w(8), image: id });
            }
            let before = ctx.quads.len();
            let clip = ctx.clip;
            ctx.quads.extend(parsed.into_iter().filter_map(|q| images::clip(q, clip)));
            (ctx.quads.len() - before) as i32
        }
        Call::game_text { style_ptr, text_ptr, text_len } => {
            if !ctx.overlay || ctx.game_2d.len() >= sa_mod_api::text::MAX_PER_FRAME || style_ptr < 0 || text_ptr < 0 {
                return Ok(status::BAD_ARG);
            }
            if !(0..=sa_mod_api::text::MAX_TEXT as i32).contains(&text_len) {
                return Ok(status::BAD_ARG);
            }
            let words = sa_mod_api::text::WORDS;
            let Some(raw) = mem.read(style_ptr as u32, (words * 4) as u32) else { return Ok(status::BAD_ARG) };
            let w: [u32; sa_mod_api::text::WORDS] =
                std::array::from_fn(|i| u32::from_le_bytes(raw[i * 4..i * 4 + 4].try_into().expect("4 bytes")));
            let Some(style) = sa_mod_api::text::decode(&w) else { return Ok(status::BAD_ARG) };
            let Some(bytes) = mem.read(text_ptr as u32, text_len as u32) else { return Ok(status::BAD_ARG) };
            ctx.game_2d.push(Game2d::Text(style, bytes));
            status::OK
        }
        Call::world_to_screen { x, y, z, out_ptr } => {
            if game.is_none() || ![x, y, z].iter().all(|v| v.is_finite()) || out_ptr < 0 {
                return Ok(status::BAD_ARG);
            }
            match sa_access::world_to_screen(Vec3::new(x, y, z)) {
                Some((sx, sy, depth)) => {
                    let bytes: Vec<u8> = [sx, sy, depth].iter().flat_map(|f| f.to_le_bytes()).collect();
                    if !mem.write(out_ptr as u32, &bytes) {
                        return Ok(status::BAD_ARG);
                    }
                    1
                }
                None => 0,
            }
        }
        Call::game_texture { dict_ptr, dict_len, file_ptr, file_len, name_ptr, name_len } => {
            if game.is_none() {
                return Ok(status::BAD_ARG);
            }
            if ctx.hook_phase != 0 {
                return Ok(status::WRONG_PHASE);
            }
            let (Some(dict), Some(file), Some(name)) =
                (text(mem, dict_ptr, dict_len), text(mem, file_ptr, file_len), text(mem, name_ptr, name_len))
            else {
                return Ok(status::BAD_ARG);
            };
            if dict.is_empty() || name.is_empty() || file.split(['/', '\\']).any(|c| c == "..") || file.contains(':') {
                return Ok(status::BAD_ARG);
            }
            let file = (!file.is_empty()).then(|| file.replace('/', "\\"));
            let t = crate::images::GameTexture { dict, file, name, found: None };
            let known = ctx.images.textures().iter().any(|x| x.dict == t.dict && x.file == t.file && x.name == t.name);
            if !known && ctx.images.textures().len() >= sa_mod_api::text::MAX_TEXTURES {
                return Ok(status::FAILED);
            }
            let h = ctx.images.texture(t);
            match ctx.images.textures()[h].found {
                None => status::AGAIN,
                Some(None) => status::NOT_FOUND,
                Some(Some(_)) => h as i32,
            }
        }
        Call::game_sprite { texture, x, y, w, h, rgba } => {
            if !ctx.overlay || ctx.game_2d.len() >= sa_mod_api::text::MAX_PER_FRAME {
                return Ok(status::BAD_ARG);
            }
            if ![x, y, w, h].iter().all(|v| v.is_finite()) {
                return Ok(status::BAD_ARG);
            }
            let found = usize::try_from(texture).ok().and_then(|i| ctx.images.textures().get(i)).and_then(|t| t.found);
            let Some(Some(t)) = found else {
                return Ok(status::NOT_FOUND);
            };
            ctx.game_2d.push(Game2d::Sprite { texture: t, rect: [x, y, x + w, y + h], rgba: rgba as u32 });
            status::OK
        }
        Call::overlay_clip { x, y, w, h } => {
            if ![x, y, w, h].iter().all(|v| v.is_finite()) {
                return Ok(status::BAD_ARG);
            }
            ctx.clip = (w >= 0.0 && h >= 0.0).then_some([x, y, w, h]);
            status::OK
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
        Call::menu_active {} => game.is_some_and(|g| g.menu_active()) as i32,
        Call::pad_state { buf_ptr, buf_len } => {
            let Some(g) = game else { return Ok(status::NOT_FOUND) };
            write_record(mem, buf_ptr, buf_len, &PadInfo::from(&g.pad().0))
        }
        Call::ped_matrix { handle, buf_ptr, buf_len } => {
            let Some(s) = game.and_then(|g| g.ped_snapshot(PedRef(handle as u32)).ok()) else {
                return Ok(status::NOT_FOUND);
            };
            write_record(mem, buf_ptr, buf_len, &MatrixInfo::from(&s.matrix))
        }
        Call::vehicle_matrix { handle, buf_ptr, buf_len } => {
            let Some(s) = game.and_then(|g| g.vehicle_snapshot(VehicleRef(handle as u32)).ok()) else {
                return Ok(status::NOT_FOUND);
            };
            write_record(mem, buf_ptr, buf_len, &MatrixInfo::from(&s.matrix))
        }
        Call::set_game_option { option, on } => {
            let Some(o) = sa_mod_api::option::from_number(option) else { return Ok(status::BAD_ARG) };
            drop(game_ref);
            let Some(g) = game_rc else { return Ok(status::WRONG_PHASE) };
            g.0.borrow_mut().set_option(o, on != 0);
            ctx.options_off.retain(|x| *x != o);
            if on == 0 {
                ctx.options_off.push(o);
            }
            return Ok(status::OK);
        }
        Call::input_capture { keyboard, mouse, cursor } => {
            ctx.capture = (keyboard != 0, mouse != 0, cursor != 0);
            status::OK
        }
        Call::big_message { text_ptr, text_len, time_ms, style } => {
            let Some(s) = text(mem, text_ptr, text_len) else { return Ok(status::BAD_ARG) };
            if ctx.in_hook() {
                return Ok(status::WRONG_PHASE);
            }
            if !(0..=6).contains(&style) || time_ms < 0 {
                return Ok(status::BAD_ARG);
            }
            drop(game_ref);
            let Some(g) = game_rc else { return Ok(status::WRONG_PHASE) };
            let r = guarded(&mut ctx.logs, "big_message", || g.0.borrow_mut().big_message(&s, time_ms as u32, style as u16));
            return Ok(r.map_or_else(|s| s, |()| status::OK));
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
            ctx.subscribed.push((target, phases, hook::controllable(&name)));
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
        Call::hook_set_arg { index, word } => {
            let Some(c) = &ctx.hook_call else { return Ok(status::BAD_ARG) };
            if !ctx.hook_control {
                return Ok(status::DENIED);
            }
            let Some(old) = usize::try_from(index).ok().and_then(|i| c.args.get(i)) else {
                return Ok(status::NOT_FOUND);
            };
            if ctx.hook_phase != hook::BEFORE {
                return Ok(status::BAD_ARG);
            }
            let v = match *old {
                hook::Value::Int(_) => hook::Value::Int(word),
                hook::Value::Float(_) => hook::Value::Float(f32::from_bits(word as u32)),
                hook::Value::Ped(_) => hook::Value::Ped(PedRef(word as u32)),
                hook::Value::Vehicle(_) => hook::Value::Vehicle(VehicleRef(word as u32)),
                hook::Value::Object(_) => hook::Value::Object(ObjectRef(word as u32)),
                hook::Value::Pointer => return Ok(status::BAD_ARG),
            };
            if let Some(e) = entity_of(v)
                && game.is_none_or(|g| g.entity_addr(e).is_err())
            {
                return Ok(status::NOT_FOUND);
            }
            ctx.decision.args.retain(|a| a.0 != index as usize);
            ctx.decision.args.push((index as usize, v));
            status::OK
        }
        Call::hook_skip { result } => {
            if ctx.hook_call.is_none() || ctx.hook_phase != hook::BEFORE {
                return Ok(status::BAD_ARG);
            }
            if !ctx.hook_control {
                return Ok(status::DENIED);
            }
            ctx.decision.skip = Some(result);
            status::OK
        }
        Call::hook_set_result { result } => {
            if ctx.hook_call.is_none() || ctx.hook_phase != hook::AFTER {
                return Ok(status::BAD_ARG);
            }
            if !ctx.hook_control {
                return Ok(status::DENIED);
            }
            ctx.decision.result = Some(result);
            status::OK
        }
        c => unreachable!("{c:?} is served before"),
    })
}

/// The clock, the sockets and the storage: `None` for any other call.
fn services(ctx: &mut CallCtx, mem: &mut dyn Mem, call: Call) -> Option<i32> {
    Some(match call {
        Call::clock { out_ptr } => {
            let monotonic = ctx.epoch.map_or(0, |e| e.elapsed().as_micros() as u64);
            let wall = web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .map_or(0, |d| d.as_micros() as u64);
            let mut out = [0u8; 16];
            out[..8].copy_from_slice(&monotonic.to_le_bytes());
            out[8..].copy_from_slice(&wall.to_le_bytes());
            if out_ptr < 0 || !mem.write(out_ptr as u32, &out) {
                return Some(status::BAD_ARG);
            }
            status::OK
        }
        Call::net_open { target_ptr, target_len } => {
            let Some(target) = text(mem, target_ptr, target_len) else { return Some(status::BAD_ARG) };
            let Some((host, port)) = split_target(&target) else { return Some(status::BAD_ARG) };
            if !ctx.net_rules.iter().any(|r| r.allows(host, port)) {
                let asked = ctx.approvals.as_ref().filter(|_| ctx.user_approved);
                match asked.map(|a| a.borrow_mut().check(&ctx.mod_id.0, &ctx.mod_id.1, &target)) {
                    Some(Some(true)) => {}
                    Some(None) => return Some(status::AGAIN),
                    Some(Some(false)) | None => return Some(status::DENIED),
                }
            }
            if !ctx.sockets.has_room() {
                ctx.log_error(format!("net_open {target}: the mod holds {} sockets already", net::MAX_SOCKETS));
                return Some(status::FAILED);
            }
            let Some(n) = ctx.network.clone() else {
                ctx.log_error(format!("net_open {target}: no network in this host"));
                return Some(status::FAILED);
            };
            match n.0.open(&target) {
                Ok(socket) => ctx.sockets.add(socket).map_or(status::FAILED, |h| h as i32),
                Err(e) => {
                    ctx.log_error(format!("net_open {target}: {e}"));
                    status::FAILED
                }
            }
        }
        Call::net_state { handle } => match ctx.sockets.get(handle).map(|s| s.state()) {
            None => status::NOT_FOUND,
            Some(LinkState::Connecting) => net::CONNECTING,
            Some(LinkState::Open) => net::OPEN,
            Some(LinkState::Closed(_)) => net::CLOSED,
        },
        Call::net_send { handle, data_ptr, data_len } => {
            if data_ptr < 0 || data_len < 0 || data_len as usize > net::MAX_DATAGRAM {
                return Some(status::BAD_ARG);
            }
            let Some(data) = mem.read(data_ptr as u32, data_len as u32) else { return Some(status::BAD_ARG) };
            if ctx.sockets.get(handle).is_none() {
                return Some(status::NOT_FOUND);
            }
            if !ctx.sockets.charge(data.len()) {
                return Some(status::AGAIN);
            }
            let socket = ctx.sockets.get(handle)?;
            match socket.send(&data) {
                Ok(n) => n as i32,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => status::AGAIN,
                Err(e) => {
                    ctx.log_error(format!("net_send: {e}"));
                    status::FAILED
                }
            }
        }
        Call::net_recv { handle, buf_ptr, buf_len } => {
            if buf_ptr < 0 || buf_len < 0 {
                return Some(status::BAD_ARG);
            }
            let Some(socket) = ctx.sockets.get(handle) else { return Some(status::NOT_FOUND) };
            let mut buf = vec![0u8; (buf_len as usize).min(net::MAX_DATAGRAM)];
            match socket.recv(&mut buf) {
                Ok(n) => {
                    if !mem.write(buf_ptr as u32, &buf[..n]) {
                        return Some(status::BAD_ARG);
                    }
                    n as i32
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => status::AGAIN,
                Err(e) => {
                    ctx.log_error(format!("net_recv: {e}"));
                    status::FAILED
                }
            }
        }
        Call::net_close { handle } => {
            if ctx.sockets.close(handle) {
                status::OK
            } else {
                status::NOT_FOUND
            }
        }
        Call::http_get { url_ptr, url_len } => {
            if url_len < 0 || url_len as usize > http::MAX_URL {
                return Some(status::BAD_ARG);
            }
            let Some(url) = text(mem, url_ptr, url_len) else { return Some(status::BAD_ARG) };
            if !http::valid_url(&url) {
                return Some(status::BAD_ARG);
            }
            if !http::allows(&ctx.http_prefixes, &url) {
                return Some(status::DENIED);
            }
            if !ctx.requests.has_room() {
                ctx.log_error(format!("http_get {url}: the mod holds {} requests already", http::MAX_REQUESTS));
                return Some(status::FAILED);
            }
            let Some(client) = ctx.http.clone() else {
                ctx.log_error(format!("http_get {url}: no HTTP client in this host"));
                return Some(status::FAILED);
            };
            ctx.requests.add(client.0.get(&url)).map_or(status::FAILED, |h| h as i32)
        }
        Call::http_status { handle } | Call::http_body { handle, .. } => {
            let Some((outcome, why)) = ctx.requests.poll(handle) else { return Some(status::NOT_FOUND) };
            let r = match (outcome, call) {
                (Outcome::Running, _) => status::AGAIN,
                (Outcome::Failed, _) => status::FAILED,
                (Outcome::Done(code, _), Call::http_status { .. }) => i32::from(code),
                (Outcome::Done(_, body), Call::http_body { buf_ptr, buf_len, .. }) => {
                    if buf_ptr < 0 || buf_len < 0 {
                        return Some(status::BAD_ARG);
                    }
                    let n = body.len().min(buf_len as usize);
                    if !mem.write(buf_ptr as u32, &body[..n]) {
                        return Some(status::BAD_ARG);
                    }
                    body.len() as i32
                }
                (Outcome::Done(..), _) => unreachable!("only http_status and http_body poll"),
            };
            if let Some(why) = why {
                ctx.log_error(format!("http request {handle}: {why}"));
            }
            r
        }
        Call::http_close { handle } => {
            if ctx.requests.close(handle) {
                status::OK
            } else {
                status::NOT_FOUND
            }
        }
        Call::storage_read { key_ptr, key_len, buf_ptr, buf_len } => {
            let Some(key) = storage_key(mem, key_ptr, key_len) else { return Some(status::BAD_ARG) };
            if buf_ptr < 0 || buf_len < 0 {
                return Some(status::BAD_ARG);
            }
            let Some(Store(st)) = ctx.storage.clone() else { return Some(status::DENIED) };
            match st.read(&key) {
                Ok(Some(data)) => {
                    let n = data.len().min(buf_len as usize);
                    if !mem.write(buf_ptr as u32, &data[..n]) {
                        return Some(status::BAD_ARG);
                    }
                    data.len().min(i32::MAX as usize) as i32
                }
                Ok(None) => status::NOT_FOUND,
                Err(e) => {
                    ctx.log_error(format!("storage_read {key}: {e}"));
                    status::FAILED
                }
            }
        }
        Call::storage_write { key_ptr, key_len, data_ptr, data_len } => {
            let Some(key) = storage_key(mem, key_ptr, key_len) else { return Some(status::BAD_ARG) };
            if data_ptr < 0 || data_len < 0 || data_len as usize > storage::MAX_VALUE {
                return Some(status::BAD_ARG);
            }
            let Some(data) = mem.read(data_ptr as u32, data_len as u32) else { return Some(status::BAD_ARG) };
            let Some(Store(st)) = ctx.storage.clone() else { return Some(status::DENIED) };
            let r = st.list().and_then(|keys| {
                let others: Vec<_> = keys.iter().filter(|k| k.0 != key).collect();
                let total: usize = others.iter().map(|k| k.1).sum::<usize>() + data.len();
                if others.len() >= storage::MAX_KEYS {
                    return Err(format!("the mod keeps {} keys already", storage::MAX_KEYS));
                }
                if total > storage::MAX_TOTAL {
                    return Err(format!("{total} bytes in all, over the mod's {} bytes", storage::MAX_TOTAL));
                }
                st.write(&key, &data)
            });
            match r {
                Ok(()) => status::OK,
                Err(e) => {
                    ctx.log_error(format!("storage_write {key}: {e}"));
                    status::FAILED
                }
            }
        }
        Call::storage_delete { key_ptr, key_len } => {
            let Some(key) = storage_key(mem, key_ptr, key_len) else { return Some(status::BAD_ARG) };
            let Some(Store(st)) = ctx.storage.clone() else { return Some(status::DENIED) };
            match st.delete(&key) {
                Ok(true) => status::OK,
                Ok(false) => status::NOT_FOUND,
                Err(e) => {
                    ctx.log_error(format!("storage_delete {key}: {e}"));
                    status::FAILED
                }
            }
        }
        Call::storage_list { buf_ptr, buf_len } => {
            if buf_ptr < 0 || buf_len < 0 {
                return Some(status::BAD_ARG);
            }
            let Some(Store(st)) = ctx.storage.clone() else { return Some(status::DENIED) };
            let keys = match st.list() {
                Ok(k) => k,
                Err(e) => {
                    ctx.log_error(format!("storage_list: {e}"));
                    return Some(status::FAILED);
                }
            };
            let list: String = keys.iter().map(|k| format!("{}\n", k.0)).collect();
            let n = list.len().min(buf_len as usize);
            if !mem.write(buf_ptr as u32, &list.as_bytes()[..n]) {
                return Some(status::BAD_ARG);
            }
            list.len().min(i32::MAX as usize) as i32
        }
        _ => return None,
    })
}

/// Reads a key the storage functions accept.
fn storage_key(mem: &dyn Mem, ptr: i32, len: i32) -> Option<String> {
    text(mem, ptr, len).filter(|k| storage::valid_key(k))
}

/// The calls that create or delete entities or run a script command: `None` for any other call.
fn entities_and_scripts(ctx: &mut CallCtx, mem: &mut dyn Mem, call: Call) -> Option<i32> {
    let game = ctx.game.clone();
    Some(match call {
        Call::create_vehicle { model, x, y, z, degrees } => {
            create(ctx, game, |g| {
                let v = g.create_car(ModelId(model), Vec3::new(x, y, z))?;
                g.set_vehicle_heading(v, degrees)?;
                Ok(Owned::Entity(EntityRef::Vehicle(v)))
            }, ModelId(model), [x, y, z, degrees])
        }
        Call::create_ped { model, x, y, z } => {
            create(ctx, game, |g| g.create_ped(ModelId(model), Vec3::new(x, y, z)).map(|p| Owned::Entity(EntityRef::Ped(p))), ModelId(model), [x, y, z, 0.0])
        }
        Call::create_object { model, x, y, z } => {
            create(ctx, game, |g| g.create_object(ModelId(model), Vec3::new(x, y, z)).map(|o| Owned::Entity(EntityRef::Object(o))), ModelId(model), [x, y, z, 0.0])
        }
        Call::destroy_vehicle { handle } => destroy(ctx, game, Owned::Entity(EntityRef::Vehicle(VehicleRef(handle as u32)))),
        Call::destroy_ped { handle } => destroy(ctx, game, Owned::Entity(EntityRef::Ped(PedRef(handle as u32)))),
        Call::destroy_object { handle } => destroy(ctx, game, Owned::Entity(EntityRef::Object(ObjectRef(handle as u32)))),
        Call::script_command { opcode, args_ptr, args_len, out_ptr } => {
            let Some(params) = script::params(opcode) else { return Some(status::DENIED) };
            if ctx.in_hook() {
                return Some(status::WRONG_PHASE);
            }
            if args_len < 0 || args_len as usize != params.len() || args_ptr < 0 {
                return Some(status::BAD_ARG);
            }
            let Some(raw) = mem.read(args_ptr as u32, 4 * args_len as u32) else { return Some(status::BAD_ARG) };
            let words: Vec<i32> = raw.as_chunks::<4>().0.iter().map(|w| i32::from_le_bytes(*w)).collect();
            let mut texts = Vec::new();
            for (k, &w) in params.chars().zip(&words) {
                if k == 's' {
                    let Some(t) = c_string(mem, w, script::MAX_TEXT) else { return Some(status::BAD_ARG) };
                    texts.push(t);
                }
            }
            let mut texts = texts.iter();
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            let mut g = g.0.borrow_mut();
            let mut args = Vec::with_capacity(params.len());
            let mut outputs = 0u16;
            for (k, &w) in params.chars().zip(&words) {
                let entity = match k {
                    'p' => Some(EntityRef::Ped(PedRef(w as u32))),
                    'v' => Some(EntityRef::Vehicle(VehicleRef(w as u32))),
                    _ => None,
                };
                if let Some(e) = entity
                    && g.entity_addr(e).is_err()
                {
                    return Some(status::NOT_FOUND);
                }
                args.push(match k {
                    'f' => ScriptArg::Float(f32::from_bits(w as u32)),
                    's' => ScriptArg::Text(texts.next().map_or(&[][..], |t| t.as_slice())),
                    'P' if w != 0 => return Some(status::BAD_ARG),
                    'o' => {
                        outputs += 1;
                        ScriptArg::Local(outputs - 1)
                    }
                    _ => ScriptArg::Int(w),
                });
            }
            let mut locals = ScriptLocals::default();
            let r = guarded(&mut ctx.logs, "script_command", || g.run_command(Opcode(opcode as u16), &args, &mut locals));
            let cond = match r {
                Ok(c) => c,
                Err(s) => return Some(s),
            };
            if outputs > 0 {
                let out: Vec<u8> = locals.0[..usize::from(outputs)].iter().flat_map(|w| w.to_le_bytes()).collect();
                if out_ptr < 0 || !mem.write(out_ptr as u32, &out) {
                    return Some(status::BAD_ARG);
                }
            }
            i32::from(cond)
        }
        _ => return None,
    })
}

/// Pickups, checkpoint markers, blips and radar areas: `None` for any other call.
fn markers(ctx: &mut CallCtx, call: Call) -> Option<i32> {
    let game = ctx.game.clone();
    let finite = |v: &[f32]| v.iter().all(|f| f.is_finite());
    let owned = |ctx: &CallCtx, o: Owned| ctx.owned.contains(&o);
    Some(match call {
        Call::create_pickup { model, kind, x, y, z } => {
            if !(1..=pickup::MAX_TYPE).contains(&kind) {
                return Some(status::BAD_ARG);
            }
            create(ctx, game, |g| g.create_pickup(ModelId(model), kind, Vec3::new(x, y, z)).map(Owned::Pickup), ModelId(model), [x, y, z, 0.0])
        }
        Call::destroy_pickup { handle } => destroy(ctx, game, Owned::Pickup(PickupRef(handle as u32))),
        Call::pickup_collected { handle } => {
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            if ctx.in_hook() {
                return Some(status::WRONG_PHASE);
            }
            let p = PickupRef(handle as u32);
            let mut g = g.0.borrow_mut();
            if g.pickup_touched(p).is_err() {
                return Some(status::NOT_FOUND);
            }
            match guarded(&mut ctx.logs, "pickup_collected", || g.pickup_collected(p)) {
                Ok(c) => i32::from(c),
                Err(s) => s,
            }
        }
        Call::pickup_touched { handle } => {
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            match g.0.borrow().pickup_touched(PickupRef(handle as u32)) {
                Ok(t) => i32::from(t),
                Err(_) => status::NOT_FOUND,
            }
        }
        Call::create_checkpoint { kind, x, y, z, nx, ny, nz, size } => {
            if !(0..=checkpoint::MAX_TYPE).contains(&kind) || !finite(&[x, y, z, nx, ny, nz, size]) {
                return Some(status::BAD_ARG);
            }
            create_marker(ctx, game, "create_checkpoint", |g| {
                g.create_checkpoint(kind, Vec3::new(x, y, z), Vec3::new(nx, ny, nz), size).map(Owned::Checkpoint)
            })
        }
        Call::destroy_checkpoint { handle } => destroy(ctx, game, Owned::Checkpoint(CheckpointRef(handle as u32))),
        Call::create_coord_blip { x, y, z, sprite, flags } => {
            if !finite(&[x, y, z]) || !(0..=255).contains(&sprite) || flags & !(blip::GLOBAL | blip::CONTACT_POINT) != 0 {
                return Some(status::BAD_ARG);
            }
            let style = sa_access::BlipStyle { global: flags & blip::GLOBAL != 0, contact_point: flags & blip::CONTACT_POINT != 0 };
            create_marker(ctx, game, "create_coord_blip", |g| g.add_coord_blip(Vec3::new(x, y, z), sprite, style).map(Owned::Blip))
        }
        Call::create_entity_blip { kind, handle } => {
            let r = match kind {
                blip::PED => create_marker(ctx, game, "create_entity_blip", |g| g.add_ped_blip(PedRef(handle as u32)).map(Owned::Blip)),
                blip::VEHICLE => {
                    create_marker(ctx, game, "create_entity_blip", |g| g.add_vehicle_blip(VehicleRef(handle as u32)).map(Owned::Blip))
                }
                _ => status::BAD_ARG,
            };
            return Some(r);
        }
        Call::destroy_blip { handle } => destroy(ctx, game, Owned::Blip(BlipRef(handle as u32))),
        Call::set_blip_colour { handle, colour } | Call::set_blip_scale { handle, scale: colour } | Call::set_blip_display { handle, display: colour } => {
            let b = BlipRef(handle as u32);
            if !owned(ctx, Owned::Blip(b)) {
                return Some(status::DENIED);
            }
            if ctx.in_hook() {
                return Some(status::WRONG_PHASE);
            }
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            let mut g = g.0.borrow_mut();
            let r = match call {
                Call::set_blip_colour { .. } => guarded(&mut ctx.logs, "set_blip_colour", || g.set_blip_colour(b, colour as u32)),
                Call::set_blip_scale { .. } => guarded(&mut ctx.logs, "set_blip_scale", || g.set_blip_scale(b, colour)),
                _ => guarded(&mut ctx.logs, "set_blip_display", || g.set_blip_display(b, colour)),
            };
            r.map_or_else(|s| s, |()| status::OK)
        }
        Call::set_radar_area { id, min_x, min_y, max_x, max_y, rgba } => {
            let Some(id) = radar_area_id(id) else { return Some(status::BAD_ARG) };
            if !finite(&[min_x, min_y, max_x, max_y]) {
                return Some(status::BAD_ARG);
            }
            ctx.radar_areas.insert(id, RadarArea { min: (min_x, min_y), max: (max_x, max_y), rgba: rgba as u32, flash: None });
            status::OK
        }
        Call::flash_radar_area { id, rgba, period_ms } => {
            let Some(id) = radar_area_id(id) else { return Some(status::BAD_ARG) };
            let Ok(period) = u32::try_from(period_ms) else { return Some(status::BAD_ARG) };
            match ctx.radar_areas.get_mut(&id) {
                Some(a) => {
                    a.flash = (period > 0).then_some((rgba as u32, period));
                    status::OK
                }
                None => status::NOT_FOUND,
            }
        }
        Call::model_exists { model } => {
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            i32::from(g.0.borrow().model_exists(ModelId(model)))
        }
        Call::remove_radar_area { id } => match radar_area_id(id).and_then(|id| ctx.radar_areas.remove(&id)) {
            Some(_) => status::OK,
            None => status::NOT_FOUND,
        },
        _ => return None,
    })
}

/// A radar area id a mod may use.
fn radar_area_id(id: i32) -> Option<u16> {
    (0..radar::MAX_AREAS).contains(&id).then_some(id as u16)
}

/// Creates a pickup-less marker (a checkpoint or a blip) with `f`; the mod owns it.
fn create_marker(ctx: &mut CallCtx, game: Option<Game>, what: &str, f: impl FnOnce(&mut GameAccess) -> GameResult<Owned>) -> i32 {
    if ctx.in_hook() {
        return status::WRONG_PHASE;
    }
    let Some(g) = game else { return status::WRONG_PHASE };
    let r = guarded(&mut ctx.logs, what, || f(&mut g.0.borrow_mut()));
    match r {
        Ok(o) => {
            ctx.owned.push(o);
            owned_handle(o)
        }
        Err(s) => s,
    }
}

/// The handle a mod gets for what it created.
fn owned_handle(o: Owned) -> i32 {
    match o {
        Owned::Entity(EntityRef::Ped(PedRef(h)) | EntityRef::Vehicle(VehicleRef(h)) | EntityRef::Object(ObjectRef(h)))
        | Owned::Pickup(PickupRef(h))
        | Owned::Checkpoint(CheckpointRef(h))
        | Owned::Blip(BlipRef(h)) => h as i32,
        Owned::Entity(_) => status::FAILED,
    }
}

/// The NUL-terminated string of at most `max` bytes at `ptr` in the mod's memory (without the NUL).
fn c_string(mem: &dyn Mem, ptr: i32, max: usize) -> Option<Vec<u8>> {
    let ptr = u32::try_from(ptr).ok()?;
    let mut out = Vec::new();
    for i in 0..=max as u32 {
        let b = mem.read(ptr.checked_add(i)?, 1)?[0];
        if b == 0 {
            return Some(out);
        }
        out.push(b);
    }
    None
}

/// The entity a hook value names, if it names one.
fn entity_of(v: hook::Value) -> Option<EntityRef> {
    match v {
        hook::Value::Ped(p) => Some(EntityRef::Ped(p)),
        hook::Value::Vehicle(v) => Some(EntityRef::Vehicle(v)),
        hook::Value::Object(o) => Some(EntityRef::Object(o)),
        _ => None,
    }
}

/// Whether `call` changes the game state through [`apply`] (deferred to the next frame boundary inside a hook).
fn is_change(call: &Call) -> bool {
    matches!(
        call,
        Call::set_ped_position { .. }
            | Call::set_ped_heading { .. }
            | Call::set_ped_health { .. }
            | Call::set_ped_armour { .. }
            | Call::give_weapon { .. }
            | Call::set_money { .. }
            | Call::set_wanted_level { .. }
            | Call::set_clock { .. }
            | Call::set_weather { .. }
            | Call::set_game_time { .. }
            | Call::set_vehicle_position { .. }
            | Call::set_vehicle_heading { .. }
            | Call::set_vehicle_health { .. }
            | Call::set_vehicle_colours { .. }
            | Call::play_frontend_sound { .. }
            | Call::play_sound { .. }
            | Call::set_ped_model { .. }
            | Call::set_hud_visible { .. }
    )
}

/// Runs `f` (a call into the game's code); a panic (game code that is not ported yet) becomes `FAILED` with a log
/// line instead of ending the game.
fn guarded<R>(logs: &mut Vec<(i32, String)>, what: &str, f: impl FnOnce() -> GameResult<R>) -> Result<R, i32> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(r)) => Ok(r),
        Ok(Err(GameError::InvalidHandle)) => Err(status::NOT_FOUND),
        Ok(Err(e)) => {
            if logs.len() < MAX_LOG_LINES {
                logs.push((sa_mod_api::level::ERROR, format!("{what}: {e}")));
            }
            Err(status::FAILED)
        }
        Err(p) => {
            let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()));
            if logs.len() < MAX_LOG_LINES {
                logs.push((sa_mod_api::level::ERROR, format!("{what}: the game failed: {}", msg.unwrap_or_default())));
            }
            Err(status::FAILED)
        }
    }
}

/// Applies a change ([`is_change`]) to the game: its status.
pub fn apply(g: &mut GameAccess, call: Call, logs: &mut Vec<(i32, String)>) -> i32 {
    let ped = |h: i32| PedRef(h as u32);
    let veh = |h: i32| VehicleRef(h as u32);
    let byte = |v: i32, max: i32| u8::try_from(v).ok().filter(|&b| i32::from(b) <= max);
    let r = match call {
        Call::set_ped_position { handle, x, y, z } => guarded(logs, "set_ped_position", || g.set_ped_position(ped(handle), Vec3::new(x, y, z))),
        Call::set_ped_heading { handle, degrees } => guarded(logs, "set_ped_heading", || g.set_ped_heading(ped(handle), degrees)),
        Call::set_ped_health { handle, health } => guarded(logs, "set_ped_health", || g.set_ped_health(ped(handle), health)),
        Call::set_ped_armour { handle, armour } => guarded(logs, "set_ped_armour", || g.set_ped_armour(ped(handle), armour)),
        Call::give_weapon { handle, weapon, ammo } => {
            let (Some(w), Ok(ammo)) = (byte(weapon, 46), u32::try_from(ammo)) else { return status::BAD_ARG };
            guarded(logs, "give_weapon", || g.give_weapon(ped(handle), WeaponType(w), ammo))
        }
        Call::set_money { money } => guarded(logs, "set_money", || g.set_player_money(money)),
        Call::set_wanted_level { level } => {
            let Some(l) = byte(level, 6) else { return status::BAD_ARG };
            guarded(logs, "set_wanted_level", || g.set_wanted_level(l))
        }
        Call::set_clock { hour, minute } => {
            let (Some(h), Some(m)) = (byte(hour, 23), byte(minute, 59)) else { return status::BAD_ARG };
            guarded(logs, "set_clock", || g.set_clock(h, m))
        }
        Call::set_weather { weather } => {
            let Some(w) = byte(weather, 22) else { return status::BAD_ARG };
            guarded(logs, "set_weather", || g.set_weather(w))
        }
        Call::set_game_time { ms } => {
            if !(0..=0x3FFF_FFFF).contains(&ms) {
                return status::BAD_ARG;
            }
            guarded(logs, "set_game_time", || g.set_game_time(ms as u32))
        }
        Call::set_vehicle_position { handle, x, y, z } => guarded(logs, "set_vehicle_position", || g.set_vehicle_position(veh(handle), Vec3::new(x, y, z))),
        Call::set_vehicle_heading { handle, degrees } => guarded(logs, "set_vehicle_heading", || g.set_vehicle_heading(veh(handle), degrees)),
        Call::set_vehicle_health { handle, health } => guarded(logs, "set_vehicle_health", || g.set_vehicle_health(veh(handle), health)),
        Call::set_vehicle_colours { handle, primary, secondary } => {
            let (Some(a), Some(b)) = (byte(primary, 255), byte(secondary, 255)) else { return status::BAD_ARG };
            guarded(logs, "set_vehicle_colours", || g.set_vehicle_colours(veh(handle), [a, b]))
        }
        Call::play_frontend_sound { event, speed } => {
            let Ok(e) = u32::try_from(event) else { return status::BAD_ARG };
            guarded(logs, "play_frontend_sound", || g.frontend_sound(e, speed))
        }
        Call::play_sound { sound, x, y, z } => guarded(logs, "play_sound", || g.one_off_sound(Vec3::new(x, y, z), sound)),
        Call::set_ped_model { handle, model } => guarded(logs, "set_ped_model", || g.set_ped_model(ped(handle), ModelId(model))),
        Call::set_hud_visible { on } => {
            g.set_hud_visible(on != 0);
            Ok(())
        }
        _ => return status::BAD_ARG,
    };
    r.map_or_else(|s| s, |()| status::OK)
}

/// Creates an entity with `f` after loading `model` (`at`: position and heading, which must be finite); the mod
/// owns it.
fn create(
    ctx: &mut CallCtx,
    game: Option<Game>,
    f: impl FnOnce(&mut GameAccess) -> GameResult<Owned>,
    model: ModelId,
    at: [f32; 4],
) -> i32 {
    if ctx.in_hook() {
        return status::WRONG_PHASE;
    }
    if !at.iter().all(|v| v.is_finite()) || !(0..20_000).contains(&model.0) {
        return status::BAD_ARG;
    }
    let Some(g) = game else { return status::WRONG_PHASE };
    let mut g = g.0.borrow_mut();
    let r = guarded(&mut ctx.logs, "create", || {
        g.load_model_now(model)?;
        f(&mut g)
    });
    match r {
        Ok(o) => {
            ctx.owned.push(o);
            owned_handle(o)
        }
        Err(s) => s,
    }
}

/// Deletes `e`, which the mod must own.
fn destroy(ctx: &mut CallCtx, game: Option<Game>, e: Owned) -> i32 {
    if ctx.in_hook() {
        return status::WRONG_PHASE;
    }
    let Some(i) = ctx.owned.iter().position(|o| *o == e) else { return status::DENIED };
    let Some(g) = game else { return status::WRONG_PHASE };
    let r = destroy_owned(&mut g.0.borrow_mut(), e, &mut ctx.logs);
    if r != status::FAILED {
        ctx.owned.remove(i);
    }
    r
}

/// Deletes `e` (`NOT_FOUND`: it is gone already).
pub fn destroy_owned(g: &mut GameAccess, e: Owned, logs: &mut Vec<(i32, String)>) -> i32 {
    let r = guarded(logs, "destroy", || match e {
        Owned::Entity(EntityRef::Ped(p)) => g.destroy_ped(p),
        Owned::Entity(EntityRef::Vehicle(v)) => g.destroy_vehicle(v),
        Owned::Entity(EntityRef::Object(o)) => g.destroy_object(o),
        Owned::Entity(_) => Err(GameError::InvalidHandle),
        Owned::Pickup(p) => g.remove_pickup(p),
        Owned::Checkpoint(c) => g.delete_checkpoint(c),
        Owned::Blip(b) => g.remove_blip(b),
    });
    r.map_or_else(|s| s, |()| status::OK)
}

/// Lays out `text` in the host's 8x8 font (`font8x8`, ASCII; other characters as `?`), `size` pixels per glyph
/// cell, as one rectangle per horizontal run of set pixels in a glyph row. Returns the width in pixels.
pub fn text_rects(x: f32, y: f32, size: f32, rgba: u32, text: &str, out: &mut Vec<OverlayQuad>) -> f32 {
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
                out.push(OverlayQuad::rect(gx + start as f32 * px, y + row as f32 * px, (col - start) as f32 * px, px, rgba));
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
        assert!(!ctx.quads.is_empty());
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

    #[test]
    fn images_quads_and_clip() {
        // 0..16: a 2 x 2 image; 16..52: one quad; 52..88: a second quad
        let mut data = vec![0u8; 88];
        data[..16].fill(0xFF);
        let quad = |x: f32, rgba: u32| -> Vec<u8> {
            [x, 10.0, 20.0, 20.0, 0.0, 0.0, 1.0, 1.0].iter().flat_map(|f| f.to_le_bytes()).chain(rgba.to_le_bytes()).collect()
        };
        data[16..52].copy_from_slice(&quad(0.0, 0x1122_33FF));
        data[52..88].copy_from_slice(&quad(f32::NAN, 0));
        let m = &mut SliceMem(&mut data);
        let mut ctx = CallCtx { permissions: Permissions { render: true, ..Default::default() }, ..Default::default() };
        let create = import_index("image_create").unwrap();
        let quads = import_index("overlay_quads").unwrap();
        let clip = import_index("overlay_clip").unwrap();
        let i = |v: i32| Val::I32(v);
        assert_eq!(dispatch(&mut ctx, m, create, &[i(2), i(2), i(1), i(0), i(15)]), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, m, create, &[i(2), i(2), i(1), i(0), i(16)]), Ok(0));
        // images are made in any phase, drawn only in the overlay phase
        assert_eq!(dispatch(&mut ctx, m, quads, &[i(0), i(16), i(1)]), Ok(status::BAD_ARG));
        ctx.overlay = true;
        assert_eq!(dispatch(&mut ctx, m, quads, &[i(5), i(16), i(1)]), Ok(status::NOT_FOUND));
        assert_eq!(dispatch(&mut ctx, m, quads, &[i(0), i(16), i(2)]), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, m, quads, &[i(0), i(16), i(1)]), Ok(1));
        let f = |v: f32| Val::F32(v);
        assert_eq!(dispatch(&mut ctx, m, clip, &[f(5.0), f(0.0), f(100.0), f(100.0)]), Ok(status::OK));
        assert_eq!(dispatch(&mut ctx, m, quads, &[i(image::NONE), i(16), i(1)]), Ok(1));
        assert_eq!(dispatch(&mut ctx, m, clip, &[f(100.0), f(0.0), f(1.0), f(1.0)]), Ok(status::OK));
        assert_eq!(dispatch(&mut ctx, m, quads, &[i(0), i(16), i(1)]), Ok(0));
        let id = ctx.images.id(0).unwrap();
        let q: Vec<(f32, f32, u32, u32)> = ctx.quads.iter().map(|q| (q.x, q.w, q.rgba, q.image)).collect();
        assert_eq!(q, [(0.0, 20.0, 0x1122_33FF, id), (5.0, 15.0, 0x1122_33FF, 0)]);
        assert_eq!(dispatch(&mut ctx, m, import_index("image_destroy").unwrap(), &[i(0)]), Ok(status::OK));
        assert_eq!(dispatch(&mut ctx, m, quads, &[i(0), i(16), i(1)]), Ok(status::NOT_FOUND));
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
        use sa_access::fake;
        let mut g = fake::game();
        fake::set_game_state(&mut g, 9);
        fake::set_local_player(&mut g, 75.0, [10.0, -20.0, 5.5]);
        let mut data = vec![0u8; 512];
        let m = &mut SliceMem(&mut data);
        let mut ctx = CallCtx { game: Some(Game::new(g)), ..Default::default() };
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

    #[test]
    fn changes_entities_and_scripts() {
        use sa_access::fake;
        let mut g = fake::game();
        fake::set_local_player(&mut g, 100.0, [1.0, 2.0, 3.0]);
        let game = Game::new(g);
        let mut data = vec![0u8; 256];
        let m = &mut SliceMem(&mut data);
        let mut ctx = CallCtx { game: Some(game.clone()), ..Default::default() };
        let i = |n| import_index(n).unwrap();
        let money = [Val::I32(1234)];
        let local = PedRef(0x205);
        let health = [Val::I32(local.0 as i32), Val::F32(55.0)];
        // permissions
        assert_eq!(dispatch(&mut ctx, m, i("set_money"), &money), Ok(status::DENIED));
        ctx.permissions = Permissions { state: true, write: true, ..Default::default() };
        // at once outside a hook
        assert_eq!(dispatch(&mut ctx, m, i("set_money"), &money), Ok(0));
        assert_eq!(game.0.borrow().player_money(), 1234);
        assert_eq!(dispatch(&mut ctx, m, i("set_ped_health"), &health), Ok(0));
        assert_eq!(game.0.borrow().ped_snapshot(local).unwrap().health, 55.0);
        let stale = [Val::I32(0x206), Val::F32(1.0)];
        assert_eq!(dispatch(&mut ctx, m, i("set_ped_health"), &stale), Ok(status::NOT_FOUND));
        assert_eq!(dispatch(&mut ctx, m, i("set_clock"), &[Val::I32(24), Val::I32(0)]), Ok(status::BAD_ARG));
        // queued inside a hook
        ctx.hook_call = Some(HookCall { args: vec![hook::Value::Ped(local)], result: None });
        ctx.hook_phase = hook::BEFORE;
        assert_eq!(dispatch(&mut ctx, m, i("set_money"), &[Val::I32(7)]), Ok(0));
        assert_eq!(game.0.borrow().player_money(), 1234);
        assert_eq!(ctx.deferred, [Call::set_money { money: 7 }]);
        // decisions only on controllable functions, with write permission
        let skip = [Val::I32(0)];
        assert_eq!(dispatch(&mut ctx, m, i("hook_skip"), &skip), Ok(status::DENIED));
        ctx.permissions.hooks = true;
        assert_eq!(dispatch(&mut ctx, m, i("hook_skip"), &skip), Ok(status::DENIED));
        ctx.hook_control = true;
        assert_eq!(dispatch(&mut ctx, m, i("hook_skip"), &skip), Ok(0));
        assert_eq!(dispatch(&mut ctx, m, i("hook_set_arg"), &[Val::I32(0), Val::I32(0x206)]), Ok(status::NOT_FOUND));
        assert_eq!(dispatch(&mut ctx, m, i("hook_set_arg"), &[Val::I32(0), Val::I32(0x205)]), Ok(0));
        assert_eq!(dispatch(&mut ctx, m, i("hook_set_result"), &skip), Ok(status::BAD_ARG));
        assert_eq!(ctx.decision, HookDecision { args: vec![(0, hook::Value::Ped(local))], skip: Some(0), result: None });
        // entities and scripts: not in a hook, only own entities
        ctx.permissions.entities = true;
        ctx.permissions.scripts = true;
        let car = [Val::I32(411), Val::F32(0.0), Val::F32(0.0), Val::F32(0.0), Val::F32(0.0)];
        assert_eq!(dispatch(&mut ctx, m, i("create_vehicle"), &car), Ok(status::WRONG_PHASE));
        ctx.hook_call = None;
        assert_eq!(dispatch(&mut ctx, m, i("destroy_ped"), &[Val::I32(0x205)]), Ok(status::DENIED));
        m.0[..8].copy_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0]);
        let cmd = |op: i32, n: i32| [Val::I32(op), Val::I32(0), Val::I32(n), Val::I32(0)];
        assert_eq!(dispatch(&mut ctx, m, i("script_command"), &cmd(0x00A5, 0)), Ok(status::DENIED));
        assert_eq!(dispatch(&mut ctx, m, i("script_command"), &cmd(0x01B4, 1)), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, m, i("script_command"), &cmd(0x01B4, 2)), Ok(0));
        let last = game.0.borrow().issued_commands().last().cloned().unwrap();
        assert_eq!(last[..2], 0x01B4u16.to_le_bytes());
        // a stale ped handle never reaches the command
        m.0[..4].copy_from_slice(&0x206u32.to_le_bytes());
        assert_eq!(dispatch(&mut ctx, m, i("script_command"), &cmd(0x0687, 1)), Ok(status::NOT_FOUND));
    }

    #[test]
    fn markers_and_radar_areas() {
        use sa_access::fake;
        let game = Game::new(fake::game());
        let mut data = vec![0u8; 16];
        let m = &mut SliceMem(&mut data);
        let mut ctx = CallCtx { game: Some(game.clone()), ..Default::default() };
        let i = |n| import_index(n).unwrap();
        let area = |id: i32| [Val::I32(id), Val::F32(-10.0), Val::F32(-20.0), Val::F32(10.0), Val::F32(20.0), Val::I32(0x11223344)];
        let blip = [Val::F32(1.0), Val::F32(2.0), Val::F32(3.0), Val::I32(0), Val::I32(blip::GLOBAL)];
        assert_eq!(dispatch(&mut ctx, m, i("set_radar_area"), &area(3)), Ok(status::DENIED));
        assert_eq!(dispatch(&mut ctx, m, i("create_coord_blip"), &blip), Ok(status::DENIED));
        ctx.permissions = Permissions { render: true, entities: true, ..Default::default() };
        // radar areas: the mod's ids, flashing every other period
        assert_eq!(dispatch(&mut ctx, m, i("set_radar_area"), &area(radar::MAX_AREAS)), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, m, i("set_radar_area"), &area(3)), Ok(0));
        assert_eq!(dispatch(&mut ctx, m, i("flash_radar_area"), &[Val::I32(4), Val::I32(1), Val::I32(500)]), Ok(status::NOT_FOUND));
        assert_eq!(dispatch(&mut ctx, m, i("flash_radar_area"), &[Val::I32(3), Val::I32(0x55), Val::I32(500)]), Ok(0));
        let a = ctx.radar_areas[&3];
        assert_eq!((a.min, a.max, a.colour_at(499), a.colour_at(500), a.colour_at(1000)), ((-10.0, -20.0), (10.0, 20.0), 0x11223344, 0x55, 0x11223344));
        assert_eq!(dispatch(&mut ctx, m, i("remove_radar_area"), &[Val::I32(3)]), Ok(0));
        assert!(ctx.radar_areas.is_empty());
        // blips: owned, through the game's command; only its own ones change
        let h = dispatch(&mut ctx, m, i("create_coord_blip"), &blip).unwrap();
        assert!(h >= 0 && ctx.owned == [Owned::Blip(BlipRef(h as u32))]);
        let last = game.0.borrow().issued_commands().last().cloned().unwrap();
        assert_eq!(last[..2], 0x02A8u16.to_le_bytes());
        assert_eq!(dispatch(&mut ctx, m, i("set_blip_colour"), &[Val::I32(h + 1), Val::I32(1)]), Ok(status::DENIED));
        assert_eq!(dispatch(&mut ctx, m, i("set_blip_colour"), &[Val::I32(h), Val::I32(1)]), Ok(0));
        assert_eq!(dispatch(&mut ctx, m, i("destroy_blip"), &[Val::I32(h)]), Ok(0));
        assert!(ctx.owned.is_empty());
        // a stale pickup handle
        ctx.permissions.state = true;
        assert_eq!(dispatch(&mut ctx, m, i("pickup_touched"), &[Val::I32(0x0001_0005)]), Ok(status::NOT_FOUND));
    }
}
