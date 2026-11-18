//! The mod host (`docs/arch/ADR-004-mods.md`): loads WebAssembly mods and calls them at the game's fixed points.
//! Linked into sa-game only with its feature `mods`.
//!
//! * [`manifest`]: `mod.toml` (identity, `api_version`, permissions, limits);
//! * [`module`]: the checks every module passes (the interface of `sa-mod-api`, the memory cap);
//! * [`api`]: the host functions, implemented once for both engines;
//! * [`net`], [`storage`]: the mods' sockets and persistent storage, with a backend per platform;
//! * [`engine`]: the engine seam, with the backends wasmtime (native) and the browser's WebAssembly (wasm32) in
//!   `backend`;
//! * [`Host`]: loading, the load order, the call points (the settings phase before the game's D3D device exists,
//!   the overlay phase of every frame, the resize event, the game events at frame boundaries, the hooks on registry
//!   functions), budgets and disabling: a mod that traps or overruns its fuel is disabled for the session and
//!   reported (its sockets closed); the game and the other mods go on.
//!
//! A mod reads and changes the game through the engine's access layer `sa-access` ([`api::Game`]), with the
//! permissions its manifest grants, and only between the game's frames: a change asked for inside a hook waits for
//! the next frame boundary, entities and script commands are refused there. Entities a mod created are deleted
//! when it is disabled. The host learns about hooked calls from the engine's hook registry
//! ([`api::HookRegistry`]) and hands back what the hooks decided ([`api::HookDecision`]).

pub mod api;
mod backend;
pub mod engine;
pub mod events;
pub mod manifest;
pub mod module;
pub mod net;
pub mod storage;

use std::rc::Rc;

use sa_mod_api::{
    API_MAJOR, API_MINOR, EXPORT_API_VERSION, EXPORT_EVENT, EXPORT_HOOK, EXPORT_INIT, EXPORT_OVERLAY, EXPORT_RESIZE,
    EXPORT_SETTINGS, Val, event, hook, level, unpack_version,
};
use sa_platform::gpu::OverlayRect;

use sa_types::EntityRef;

use crate::api::{CallCtx, Display, Files, Game, HookCall, HookDecision, Registry, VideoRequest};
use crate::engine::{Engine, Instance};
use crate::manifest::{INIT_FUEL_FACTOR, Manifest, NetRule};
use crate::net::{Net, Network, Sockets};
use crate::storage::{StorageProvider, Store};

pub use backend::{network as platform_network, sources, storage as platform_storage};

/// Where a mod's files come from (a folder natively, the page's server on the web).
pub trait ModSource {
    /// The bytes of `file` (`mod.toml`, the entry `.wasm`, the mod's own files) in the mod's folder.
    fn read(&self, file: &str) -> Result<Vec<u8>, String>;
    /// Writes (replaces) the mod's own file `file` (`files = "own"`; the name is checked by the host).
    fn write(&self, file: &str, data: &[u8]) -> Result<(), String>;
    /// The folder, for messages.
    fn location(&self) -> String;
}

/// A loaded mod.
pub struct LoadedMod {
    /// Its manifest.
    pub manifest: Manifest,
    instance: Box<dyn Instance>,
    /// The entry points it exports.
    exports: Vec<&'static str>,
    env: ModEnv,
    /// Its open sockets.
    sockets: Sockets,
    /// Why it was disabled (`None`: running).
    pub disabled: Option<String>,
    /// Overlay calls so far.
    pub frames: u32,
    /// Time spent in its calls (host functions included), in microseconds.
    pub micros: u64,
    /// The events it subscribed to (bits `1 << event kind`).
    pub events: i32,
    /// Its hooks: (registry id, phases); a hook's id is its index.
    pub hooks: Vec<(u32, i32)>,
    /// Events delivered so far.
    pub events_sent: u64,
    /// Hook calls delivered so far.
    pub hook_calls: u64,
    /// Hook calls dropped by the per-frame limit ([`hook::MAX_CALLS_PER_FRAME`]).
    pub hook_dropped: u64,
    hook_calls_frame: u32,
    /// The entities it created and has not deleted.
    pub owned: Vec<EntityRef>,
    /// Changes it asked for inside hooks, for the next frame boundary.
    deferred: Vec<sa_mod_api::Call>,
}

/// What a mod's calls may reach besides the game.
struct ModEnv {
    /// Its folder (for its own files).
    source: Rc<dyn ModSource>,
    /// The functions it may hook.
    hook_names: Rc<[String]>,
    /// The targets it may open sockets to.
    net_rules: Rc<[NetRule]>,
    /// Its storage area (`storage = true`).
    storage: Option<Store>,
}

/// A `net` rule as the manifest writes it.
fn net_rule_text(r: &NetRule) -> String {
    let host = r.host.as_deref().unwrap_or("*");
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    format!("udp:{host}:{}", r.port.map_or("*".to_string(), |p| p.to_string()))
}

/// The mod host.
pub struct Host {
    engine: Box<dyn Engine>,
    mods: Vec<LoadedMod>,
    log: Box<dyn Fn(&str)>,
    display: Rc<Display>,
    /// A video mode requested outside the settings phase, not yet taken by the game.
    video: Option<VideoRequest>,
    game: Option<Game>,
    registry: Option<Registry>,
    /// The state the last frame's events were derived from.
    tracked: events::Tracked,
    /// Main-loop iterations since the start ([`Host::frame_begin`]).
    frame: u32,
    /// The hooked functions whose calls hooks may change ([`hook::CONTROLLABLE`]).
    controllable: Vec<u32>,
    /// Entities of disabled mods, deleted at the next frame boundary.
    orphans: Vec<EntityRef>,
    /// The start of the host (`clock`).
    epoch: web_time::Instant,
    /// Where the mods' sockets go.
    network: Option<Net>,
    /// Where the mods' storage areas are.
    storages: Option<Rc<dyn StorageProvider>>,
}

impl Host {
    /// A host on the platform's engine; `log` receives its messages and the mods' log lines.
    pub fn new(log: Box<dyn Fn(&str)>) -> Host {
        Host::with_engine(Box::new(backend::DefaultEngine::new()), log)
    }

    /// A host on `engine`.
    pub fn with_engine(engine: Box<dyn Engine>, log: Box<dyn Fn(&str)>) -> Host {
        Host {
            engine,
            mods: Vec::new(),
            log,
            display: Rc::default(),
            video: None,
            game: None,
            registry: None,
            tracked: events::Tracked::default(),
            frame: 0,
            controllable: Vec::new(),
            orphans: Vec::new(),
            epoch: web_time::Instant::now(),
            network: None,
            storages: None,
        }
    }

    /// The network the mods' sockets go to (`platform_network`); without one `net_open` fails.
    pub fn set_network(&mut self, network: Rc<dyn Network>) {
        self.network = Some(Net(network));
    }

    /// Where the mods' storage areas are (`platform_storage`); without it a mod asking for storage is not loaded.
    pub fn set_storage(&mut self, storages: Rc<dyn StorageProvider>) {
        self.storages = Some(storages);
    }

    /// The display the platform reports to the mods (`display_mode_*`).
    pub fn set_display(&mut self, display: Display) {
        self.display = Rc::new(display);
    }

    /// The game the state queries and events read.
    pub fn set_game(&mut self, game: Game) {
        self.game = Some(game);
    }

    /// The registry of the functions mods can hook.
    pub fn set_registry(&mut self, registry: Registry) {
        self.registry = Some(registry);
    }

    /// The loaded mods, in load order.
    pub fn mods(&self) -> &[LoadedMod] {
        &self.mods
    }

    fn flush_logs(&self, id: &str, ctx: &mut CallCtx) {
        for (lv, s) in ctx.logs.drain(..) {
            let tag = match lv {
                level::WARN => "warning: ",
                level::ERROR => "error: ",
                _ => "",
            };
            for line in s.lines() {
                (self.log)(&format!("mod {id}: {tag}{line}"));
            }
        }
    }

    /// Loads the mod in `src`: manifest, API version, module checks, instantiation, `sa_api_version`, `sa_init`.
    /// An error leaves the host unchanged.
    pub fn load(&mut self, src: Box<dyn ModSource>) -> Result<(), String> {
        let src: Rc<dyn ModSource> = Rc::from(src);
        let at = src.location();
        let text = src.read("mod.toml")?;
        let text = String::from_utf8(text).map_err(|_| format!("{at}/mod.toml: not UTF-8"))?;
        let (manifest, warnings) = manifest::parse(&text).map_err(|e| format!("{at}: {e}"))?;
        let id = manifest.id.clone();
        for w in warnings {
            (self.log)(&format!("mod {id}: {w}"));
        }
        if self.mods.iter().any(|m| m.manifest.id == id) {
            return Err(format!("mod {id} ({at}): a mod with this id is already loaded"));
        }
        if !sa_mod_api::compatible((API_MAJOR, API_MINOR), manifest.api) {
            return Err(format!(
                "mod {id}: built for API {}.{}, this host has API {API_MAJOR}.{API_MINOR}",
                manifest.api.0, manifest.api.1
            ));
        }
        let wasm = src.read(&manifest.entry)?;
        let exports =
            module::check(&wasm, manifest.limits.memory_bytes).map_err(|e| format!("mod {id}: {}: {e}", manifest.entry))?;
        let mut instance =
            self.engine.instantiate(&wasm, &manifest.limits).map_err(|e| format!("mod {id}: {e}"))?;
        let storage = if manifest.permissions.storage {
            let Some(st) = &self.storages else { return Err(format!("mod {id}: no storage in this host")) };
            Some(Store(st.open(&id).map_err(|e| format!("mod {id}: storage: {e}"))?))
        } else {
            None
        };
        let env = ModEnv {
            source: src,
            hook_names: manifest.hooks.clone().into(),
            net_rules: manifest.net.clone().into(),
            storage,
        };
        let mut ctx = self.ctx(&manifest, &env, 0);
        let init_fuel = manifest.limits.fuel.saturating_mul(INIT_FUEL_FACTOR);
        let v = instance.call(EXPORT_API_VERSION, &[], init_fuel, &mut ctx).map_err(|e| format!("mod {id}: {e}"))?;
        // a mod rebuilt with a newer SDK may keep the older minor version its manifest asks for
        let built = unpack_version(v);
        if !sa_mod_api::compatible((API_MAJOR, API_MINOR), built) || !sa_mod_api::compatible(built, manifest.api) {
            return Err(format!(
                "mod {id}: the module reports API {}.{}, its manifest {}.{}, this host has API {API_MAJOR}.{API_MINOR}",
                built.0, built.1, manifest.api.0, manifest.api.1
            ));
        }
        let r = instance.call(EXPORT_INIT, &[], init_fuel, &mut ctx);
        self.flush_logs(&id, &mut ctx);
        match r {
            Ok(0) => {}
            Ok(c) => return Err(format!("mod {id}: sa_init refused with {c}")),
            Err(e) => return Err(format!("mod {id}: sa_init: {e}")),
        }
        let p = manifest.permissions;
        (self.log)(&format!(
            "mod {id} loaded: {} {} from {at} ({} bytes, {}; render {}, files {}, state {}, input {}, hooks {:?}, \
             entities {}, scripts {}, audio {}, net {:?}, storage {}, memory cap {} MiB, fuel {} per call)",
            manifest.name,
            manifest.version,
            wasm.len(),
            self.engine.name(),
            p.render,
            p.files,
            if p.write { "write" } else if p.state { "read" } else { "none" },
            p.input,
            manifest.hooks,
            p.entities,
            p.scripts,
            p.audio,
            manifest.net.iter().map(net_rule_text).collect::<Vec<_>>(),
            env.storage.as_ref().map_or("none".to_string(), |s| s.0.location()),
            manifest.limits.memory_bytes >> 20,
            manifest.limits.fuel
        ));
        self.mods.push(LoadedMod {
            manifest,
            instance,
            exports,
            env,
            sockets: std::mem::take(&mut ctx.sockets),
            disabled: None,
            frames: 0,
            micros: 0,
            events: 0,
            hooks: Vec::new(),
            events_sent: 0,
            hook_calls: 0,
            hook_dropped: 0,
            hook_calls_frame: 0,
            owned: std::mem::take(&mut ctx.owned),
            deferred: std::mem::take(&mut ctx.deferred),
        });
        let i = self.mods.len() - 1;
        self.take_subscriptions(i, &mut ctx);
        Ok(())
    }

    /// A call context for the mod with `manifest` and `env`, `hooks` hooks so far.
    fn ctx(&self, manifest: &Manifest, env: &ModEnv, hooks: usize) -> CallCtx {
        CallCtx {
            permissions: manifest.permissions,
            display: self.display.clone(),
            files: manifest.permissions.files.then(|| Files(env.source.clone())),
            game: self.game.clone(),
            registry: self.registry.clone(),
            hook_names: env.hook_names.clone(),
            hook_base: hooks,
            epoch: Some(self.epoch),
            network: if manifest.permissions.net { self.network.clone() } else { None },
            net_rules: env.net_rules.clone(),
            storage: env.storage.clone(),
            ..Default::default()
        }
    }

    /// Keeps the events and hooks mod `i` subscribed to in a call, and starts reporting the hooked functions.
    fn take_subscriptions(&mut self, i: usize, ctx: &mut CallCtx) {
        let m = &mut self.mods[i];
        if let Some(mask) = ctx.events.take() {
            m.events = mask;
        }
        for (target, phases, controllable) in ctx.subscribed.drain(..) {
            m.hooks.push((target, phases));
            if controllable && !self.controllable.contains(&target) {
                self.controllable.push(target);
            }
            if let Some(r) = &self.registry {
                r.0.watch(target, true);
            }
        }
    }

    /// Disables mod `i` for `why`; the functions only it hooked are no longer reported.
    fn disable(&mut self, i: usize, why: String) {
        let id = self.mods[i].manifest.id.clone();
        self.mods[i].disabled = Some(why.clone());
        self.mods[i].deferred.clear();
        self.mods[i].sockets = Sockets::default();
        let owned = std::mem::take(&mut self.mods[i].owned);
        let n = owned.len();
        self.orphans.extend(owned);
        (self.log)(&format!(
            "mod {id} disabled: {why}; the game continues{}",
            if n > 0 { format!(" ({n} entities it created are deleted at the next frame)") } else { String::new() }
        ));
        if let Some(r) = &self.registry {
            for &(t, _) in &self.mods[i].hooks {
                let used = self.mods.iter().any(|m| m.disabled.is_none() && m.hooks.iter().any(|h| h.0 == t));
                if !used {
                    r.0.watch(t, false);
                }
            }
        }
    }

    /// Calls `export` with `args` on mod `i` (when it runs and exports it), with `prep` applied to its context. A mod
    /// that fails is disabled and reported. Returns the context of a successful call.
    fn call_mod(
        &mut self,
        i: usize,
        export: &'static str,
        args: &[Val],
        screen: (u32, u32),
        prep: impl FnOnce(&mut CallCtx),
    ) -> Option<CallCtx> {
        let m = &self.mods[i];
        if m.disabled.is_some() || !m.exports.contains(&export) {
            return None;
        }
        let mut ctx = self.ctx(&m.manifest, &m.env, m.hooks.len());
        ctx.screen = screen;
        ctx.overlay = export == EXPORT_OVERLAY;
        prep(&mut ctx);
        let m = &mut self.mods[i];
        ctx.owned = std::mem::take(&mut m.owned);
        ctx.sockets = std::mem::take(&mut m.sockets);
        let t = web_time::Instant::now();
        let r = m.instance.call(export, args, m.manifest.limits.fuel, &mut ctx);
        m.micros += t.elapsed().as_micros() as u64;
        m.owned = std::mem::take(&mut ctx.owned);
        m.sockets = std::mem::take(&mut ctx.sockets);
        m.deferred.append(&mut ctx.deferred);
        let frame = m.frames;
        if export == EXPORT_OVERLAY {
            m.frames += 1;
        }
        let id = m.manifest.id.clone();
        // the mod's own lines first (a panicking Rust mod logs its message before it traps)
        self.flush_logs(&id, &mut ctx);
        match r {
            Ok(_) => {
                self.take_subscriptions(i, &mut ctx);
                Some(ctx)
            }
            Err(e) => {
                let msg = if export == EXPORT_OVERLAY {
                    format!("{e} in {export} (frame {frame})")
                } else {
                    format!("{e} in {export}")
                };
                self.disable(i, msg);
                None
            }
        }
    }

    /// Calls `export` with `args` (built per mod) on every running mod that exports it, in load order; `each` gets
    /// every successful call's context. Returns the last video-mode request.
    fn call_all(
        &mut self,
        export: &'static str,
        screen: (u32, u32),
        args: impl Fn(&LoadedMod) -> Vec<Val>,
        mut each: impl FnMut(&mut CallCtx),
    ) -> Option<VideoRequest> {
        let mut video = None;
        for i in 0..self.mods.len() {
            let a = args(&self.mods[i]);
            if let Some(mut ctx) = self.call_mod(i, export, &a, screen, |_| {}) {
                video = ctx.video.or(video);
                each(&mut ctx);
            }
        }
        video
    }

    /// The settings phase (once, before the game selects its video mode and creates its D3D device): calls every
    /// running mod's `sa_settings`. Returns the video mode requested (the last request in load order).
    pub fn settings(&mut self) -> Option<VideoRequest> {
        self.video = None;
        self.call_all(EXPORT_SETTINGS, (0, 0), |_| Vec::new(), |_| {})
    }

    /// The resize event: the drawable the game is shown in is now `width` x `height`. A video mode a mod requests
    /// in response is kept for [`Host::take_video_request`].
    pub fn resize(&mut self, width: u32, height: u32, screen: (u32, u32)) {
        let args = [Val::I32(width.min(i32::MAX as u32) as i32), Val::I32(height.min(i32::MAX as u32) as i32)];
        if let Some(v) = self.call_all(EXPORT_RESIZE, screen, |_| args.to_vec(), |_| {}) {
            self.video = Some(v);
        }
    }

    /// The video mode requested since the last call (outside the settings phase), for the game to apply at the next
    /// frame boundary.
    pub fn take_video_request(&mut self) -> Option<VideoRequest> {
        self.video.take()
    }

    /// The overlay phase of a frame: calls every running mod's `sa_overlay` in load order and appends what they
    /// drew to `out`. A mod that fails is disabled (its drawing of this frame is dropped) and reported.
    pub fn overlay(&mut self, screen: (u32, u32), out: &mut Vec<OverlayRect>) {
        let v = self.call_all(EXPORT_OVERLAY, screen, |m| vec![Val::I32(m.frames as i32)], |ctx| {
            out.append(&mut ctx.rects)
        });
        if v.is_some() {
            self.video = v;
        }
    }

    /// The events the running mods subscribed to, together.
    fn wanted_events(&self) -> i32 {
        self.mods
            .iter()
            .filter(|m| m.disabled.is_none() && m.exports.contains(&EXPORT_EVENT))
            .fold(0, |w, m| w | m.events)
    }

    /// Delivers `events` (kind, a, b) to the mods subscribed to them, in load order.
    fn deliver(&mut self, events: &[(i32, i32, i32)]) {
        for i in 0..self.mods.len() {
            for &(kind, a, b) in events {
                if self.mods[i].events & (1 << kind) == 0 {
                    continue;
                }
                if self.call_mod(i, EXPORT_EVENT, &[Val::I32(kind), Val::I32(a), Val::I32(b)], (0, 0), |_| {}).is_some() {
                    self.mods[i].events_sent += 1;
                }
            }
        }
    }

    /// The start of a main-loop iteration (before the game's logic): resets the per-frame hook budgets and delivers
    /// the frame-begin event and the game events derived from the state since the last frame ([`events`]).
    pub fn frame_begin(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        for m in &mut self.mods {
            m.hook_calls_frame = 0;
            m.sockets.new_frame();
        }
        self.apply_deferred();
        let wanted = self.wanted_events();
        if wanted == 0 {
            return;
        }
        let mut out = vec![(event::FRAME_BEGIN, self.frame as i32, 0)];
        if let Some(g) = &self.game {
            self.tracked.update(&g.0.borrow(), wanted, &mut out);
        }
        out.retain(|e| wanted & (1 << e.0) != 0);
        self.deliver(&out);
    }

    /// Applies what the mods asked for inside hooks and deletes the entities of disabled mods (at the frame
    /// boundary: the game is between two frames).
    fn apply_deferred(&mut self) {
        let Some(g) = self.game.clone() else { return };
        let mut g = g.0.borrow_mut();
        for e in std::mem::take(&mut self.orphans) {
            let mut logs = Vec::new();
            api::destroy_entity(&mut g, e, &mut logs);
            for (_, l) in logs {
                (self.log)(&format!("mods: {l}"));
            }
        }
        for m in &mut self.mods {
            if m.deferred.is_empty() || m.disabled.is_some() {
                m.deferred.clear();
                continue;
            }
            let mut logs = Vec::new();
            for call in std::mem::take(&mut m.deferred) {
                api::apply(&mut g, call, &mut logs);
            }
            for (_, l) in logs {
                (self.log)(&format!("mod {}: error: {l}", m.manifest.id));
            }
        }
    }

    /// The end of a frame (after the overlay phase, before the present): the frame-end event.
    pub fn frame_end(&mut self) {
        if self.wanted_events() & (1 << event::FRAME_END) != 0 {
            self.deliver(&[(event::FRAME_END, self.frame as i32, 0)]);
        }
    }

    /// A call of the registry function `target` in `phase` ([`hook::BEFORE`] / [`hook::AFTER`]): calls `sa_hook`
    /// of every running mod hooked there, in load order, up to each mod's per-frame limit. Returns what the hooks
    /// decided about the call (a later mod's decision wins).
    pub fn hook(&mut self, target: u32, phase: i32, call: &HookCall) -> HookDecision {
        let control = self.controllable.contains(&target);
        let mut decision = HookDecision::default();
        for i in 0..self.mods.len() {
            let m = &self.mods[i];
            if m.disabled.is_some() {
                continue;
            }
            let ids: Vec<usize> =
                m.hooks.iter().enumerate().filter(|(_, h)| h.0 == target && h.1 & phase != 0).map(|(id, _)| id).collect();
            for id in ids {
                let m = &mut self.mods[i];
                if m.hook_calls_frame >= hook::MAX_CALLS_PER_FRAME {
                    m.hook_dropped += 1;
                    continue;
                }
                m.hook_calls_frame += 1;
                let args = [Val::I32(id as i32), Val::I32(phase)];
                let prep = |ctx: &mut CallCtx| {
                    ctx.hook_call = Some(call.clone());
                    ctx.hook_phase = phase;
                    ctx.hook_control = control;
                };
                if let Some(ctx) = self.call_mod(i, EXPORT_HOOK, &args, (0, 0), prep) {
                    self.mods[i].hook_calls += 1;
                    decision.merge(ctx.decision);
                }
            }
        }
        decision
    }

    /// One line per mod: state, frames, time per frame, events and hook calls.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        for m in &self.mods {
            let per = if m.frames > 0 { m.micros as f64 / m.frames as f64 } else { 0.0 };
            s += &format!(
                "mod {}: {}, {} overlay calls, {:.1} us per frame, {} events, {} hook calls ({} dropped)\n",
                m.manifest.id,
                m.disabled.as_deref().map_or("running".to_string(), |d| format!("disabled ({d})")),
                m.frames,
                per,
                m.events_sent,
                m.hook_calls,
                m.hook_dropped
            );
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::tests::wat_mod;
    use std::cell::RefCell;

    struct Mem(Vec<(&'static str, Vec<u8>)>);
    impl ModSource for Mem {
        fn read(&self, file: &str) -> Result<Vec<u8>, String> {
            self.0.iter().find(|f| f.0 == file).map(|f| f.1.clone()).ok_or(format!("{file}: missing"))
        }
        fn write(&self, _file: &str, _data: &[u8]) -> Result<(), String> {
            Err("read-only".into())
        }
        fn location(&self) -> String {
            "memory".into()
        }
    }

    fn src(id: &str, wasm: Vec<u8>) -> Box<Mem> {
        let toml = format!("[mod]\nid = \"{id}\"\napi_version = \"1.0\"\n[permissions]\nrender = true\n[limits]\nfuel = 100000\n");
        let entry: &'static str = Box::leak(format!("{id}.wasm").into_boxed_str());
        Box::new(Mem(vec![("mod.toml", toml.into_bytes()), (entry, wasm)]))
    }

    fn host() -> (Host, Rc<RefCell<Vec<String>>>) {
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        (Host::new(Box::new(move |s| l.borrow_mut().push(s.to_string()))), log)
    }

    const DRAW: &str = "f32.const 1 f32.const 2 f32.const 3 f32.const 4 i32.const -1 call $rect drop i32.const 0";

    #[test]
    fn broken_mods_are_disabled_and_the_others_go_on() {
        let (mut h, log) = host();
        h.load(src("good", wat_mod("", "1", DRAW))).unwrap();
        // traps in its third frame / loops forever from its second
        let trap = format!("local.get 0 i32.const 2 i32.eq (if (then unreachable)) {DRAW}");
        h.load(src("trap", wat_mod("", "1", &trap))).unwrap();
        let spin = format!("local.get 0 i32.const 1 i32.ge_s (if (then (loop $l (br $l)))) {DRAW}");
        h.load(src("spin", wat_mod("", "1", &spin))).unwrap();
        let mut counts = Vec::new();
        for _ in 0..4 {
            let mut out = Vec::new();
            h.overlay((640, 480), &mut out);
            counts.push(out.len());
        }
        assert_eq!(counts, [3, 2, 1, 1]);
        let ms = h.mods();
        assert!(ms[0].disabled.is_none());
        assert!(ms[1].disabled.as_deref().unwrap().contains("trap"), "{:?}", ms[1].disabled);
        assert!(ms[2].disabled.as_deref().unwrap().contains("fuel"), "{:?}", ms[2].disabled);
        assert_eq!(log.borrow().iter().filter(|l| l.contains("disabled")).count(), 2);
    }

    #[test]
    fn refusals() {
        let (mut h, _) = host();
        // newer minor API version
        let mut s = src("new", wat_mod("", "1", DRAW));
        s.0[0].1 = b"[mod]\nid = \"new\"\napi_version = \"1.4\"\n".to_vec();
        assert!(h.load(s).unwrap_err().contains("API 1.4"));
        // memory above the cap (16 MiB default: 300 pages)
        assert!(h.load(src("big", wat_mod("", "300", DRAW))).unwrap_err().contains("cap"));
        // growth beyond the cap fails inside the mod (memory.grow returns -1), the mod goes on
        let grow = "i32.const 1000 memory.grow i32.const -1 i32.ne (if (then unreachable)) i32.const 0";
        h.load(src("grow", wat_mod("", "1", grow))).unwrap();
        let mut out = Vec::new();
        h.overlay((640, 480), &mut out);
        assert!(h.mods()[0].disabled.is_none(), "{:?}", h.mods()[0].disabled);
        // duplicate id
        assert!(h.load(src("grow", wat_mod("", "1", DRAW))).is_err());
        // no render permission: DENIED, nothing drawn
        let mut s = src("norender", wat_mod("", "1", DRAW));
        s.0[0].1 = b"[mod]\nid = \"norender\"\napi_version = \"1.0\"\n".to_vec();
        h.load(s).unwrap();
        let mut out = Vec::new();
        h.overlay((640, 480), &mut out);
        assert!(out.is_empty());
    }

    /// The registry of a test: one function, `CPed::Render`, id 7.
    #[derive(Default)]
    struct FakeRegistry(RefCell<Vec<(u32, bool)>>);
    impl api::HookRegistry for FakeRegistry {
        fn resolve(&self, name: &str) -> Option<u32> {
            (name == "CPed::Render").then_some(7)
        }
        fn watch(&self, target: u32, on: bool) {
            self.0.borrow_mut().push((target, on));
        }
    }

    /// A level-2 mod: subscribes to every event and hooks `CPed::Render` in `sa_init`; logs the digit of each event
    /// kind it receives, then `h` and the digit of its hook's first argument's kind for each hook call.
    fn level_two_mod() -> Vec<u8> {
        wat::parse_str(
            r#"(module
              (import "sa" "log" (func $log (param i32 i32 i32) (result i32)))
              (import "sa" "events_subscribe" (func $events (param i32) (result i32)))
              (import "sa" "hook_subscribe" (func $hook (param i32 i32 i32) (result i32)))
              (import "sa" "hook_value" (func $value (param i32 i32) (result i32)))
              (import "sa" "player_info" (func $player (param i32 i32) (result i32)))
              (memory (export "memory") 1)
              (data (i32.const 16) "0123456789")
              (data (i32.const 32) "CPed::Render")
              (data (i32.const 48) "h")
              (func (export "sa_api_version") (result i32) i32.const 0x10002)
              (func (export "sa_init") (result i32)
                i32.const -1 call $events drop
                i32.const 32 i32.const 12 i32.const 3 call $hook drop
                i32.const 0)
              (func (export "sa_overlay") (param i32) (result i32) i32.const 0)
              (func (export "sa_event") (param i32 i32 i32) (result i32)
                ;; a spawn reads the player (its record must fit in 4 KiB at 1024)
                local.get 0 i32.const 3 i32.eq
                (if (then i32.const 1024 i32.const 4096 call $player i32.const 0 i32.le_s (if (then unreachable))))
                i32.const 0 local.get 0 i32.const 16 i32.add i32.const 1 call $log drop
                i32.const 0)
              (func (export "sa_hook") (param i32 i32) (result i32)
                i32.const 0 i32.const 48 i32.const 1 call $log drop
                i32.const 0
                i32.const 0 i32.const 0 call $value i32.const 16 i32.add
                i32.const 1 call $log drop
                i32.const 0))"#,
        )
        .unwrap()
    }

    #[test]
    fn events_and_hooks_reach_a_level_two_mod() {
        use sa_access::fake;
        let (mut h, log) = host();
        let mut g = fake::game();
        fake::set_game_state(&mut g, 9);
        fake::set_local_player(&mut g, 100.0, [1.0, 2.0, 3.0]);
        h.set_game(Game::new(g));
        let reg = Rc::new(FakeRegistry::default());
        h.set_registry(Registry(reg.clone()));
        let toml = "[mod]\nid = \"two\"\napi_version = \"1.2\"\n[permissions]\nstate = \"read\"\n\
                    hooks = [\"CPed::Render\"]\n";
        h.load(Box::new(Mem(vec![("mod.toml", toml.as_bytes().to_vec()), ("two.wasm", level_two_mod())])))
            .unwrap();
        // state events granted, key events not (no input permission)
        let all_but_keys = ((1 << event::COUNT) - 1) & !((1 << event::KEY_DOWN) | (1 << event::KEY_UP));
        assert_eq!(h.mods()[0].events, all_but_keys);
        assert_eq!(h.mods()[0].hooks, [(7, hook::BEFORE | hook::AFTER)]);
        assert_eq!(*reg.0.borrow(), [(7, true)]);
        log.borrow_mut().clear();
        h.frame_begin();
        // frame begin, game state 0 -> 9, the player spawned
        assert_eq!(*log.borrow(), ["mod two: 0", "mod two: 2", "mod two: 3"]);
        log.borrow_mut().clear();
        h.frame_begin();
        h.frame_end();
        assert_eq!(*log.borrow(), ["mod two: 0", "mod two: 1"]);
        log.borrow_mut().clear();
        let call = HookCall { args: vec![hook::Value::Ped(sa_types::PedRef(0x205))], result: None };
        h.hook(7, hook::BEFORE, &call);
        h.hook(8, hook::BEFORE, &call);
        assert_eq!(*log.borrow(), ["mod two: h", "mod two: 2"]);
        assert_eq!(h.mods()[0].hook_calls, 1);
        // the per-frame limit
        for _ in 0..hook::MAX_CALLS_PER_FRAME {
            h.hook(7, hook::AFTER, &call);
        }
        assert_eq!(h.mods()[0].hook_dropped, 1);
        h.frame_begin();
        h.hook(7, hook::AFTER, &call);
        assert_eq!(h.mods()[0].hook_dropped, 1);
        assert!(h.summary().contains("hook calls (1 dropped)"), "{}", h.summary());
    }

    /// A level-4 mod: opens a socket to `target` and sends "ping" in `sa_init`; every overlay call logs a received
    /// datagram (`AGAIN` logs nothing). Its storage: `sa_init` writes `count` = "1", reads it back and logs the key
    /// list; a write above the value size limit is BAD_ARG.
    fn level_four_mod(target: &str) -> Vec<u8> {
        wat::parse_str(format!(
            r#"(module
              (import "sa" "log" (func $log (param i32 i32 i32) (result i32)))
              (import "sa" "net_open" (func $open (param i32 i32) (result i32)))
              (import "sa" "net_send" (func $send (param i32 i32 i32) (result i32)))
              (import "sa" "net_recv" (func $recv (param i32 i32 i32) (result i32)))
              (import "sa" "storage_write" (func $put (param i32 i32 i32 i32) (result i32)))
              (import "sa" "storage_read" (func $get (param i32 i32 i32 i32) (result i32)))
              (import "sa" "storage_list" (func $list (param i32 i32) (result i32)))
              (import "sa" "clock" (func $clock (param i32) (result i32)))
              (memory (export "memory") 1)
              (global $sock (mut i32) (i32.const -1))
              (data (i32.const 16) "ping")
              (data (i32.const 32) "count1")
              (data (i32.const 64) "{target}")
              (func (export "sa_api_version") (result i32) i32.const 0x10004)
              (func (export "sa_init") (result i32) (local $n i32)
                i32.const 64 i32.const {len} call $open global.set $sock
                global.get $sock i32.const 0 i32.lt_s (if (then unreachable))
                global.get $sock i32.const 16 i32.const 4 call $send i32.const 4 i32.ne (if (then unreachable))
                i32.const 32 i32.const 5 i32.const 37 i32.const 1 call $put (if (then unreachable))
                i32.const 32 i32.const 5 i32.const 512 i32.const 16 call $get i32.const 1 i32.ne (if (then unreachable))
                i32.const 0 i32.const 512 i32.const 1 call $log drop
                i32.const 512 i32.const 64 call $list local.set $n
                i32.const 0 i32.const 512 local.get $n call $log drop
                ;; 2 MiB is over the per-value limit
                i32.const 32 i32.const 5 i32.const 0 i32.const 0x200000 call $put i32.const -2 i32.ne (if (then unreachable))
                i32.const 1024 call $clock drop
                i32.const 0)
              (func (export "sa_overlay") (param i32) (result i32) (local $n i32)
                global.get $sock i32.const 256 i32.const 64 call $recv local.set $n
                local.get $n i32.const 0 i32.gt_s
                (if (then i32.const 0 i32.const 256 local.get $n call $log drop))
                i32.const 0))"#,
            len = target.len()
        ))
        .unwrap()
    }

    #[test]
    fn sockets_and_storage_reach_a_level_four_mod() {
        use std::net::UdpSocket;
        let echo = UdpSocket::bind("127.0.0.1:0").unwrap();
        let target = echo.local_addr().unwrap().to_string();
        let (mut h, log) = host();
        h.set_network(platform_network(None));
        struct Mem1(Rc<storage::MemoryStorage>);
        impl StorageProvider for Mem1 {
            fn open(&self, _id: &str) -> Result<Rc<dyn storage::Storage>, String> {
                Ok(self.0.clone())
            }
        }
        let st = Rc::new(storage::MemoryStorage::default());
        h.set_storage(Rc::new(Mem1(st.clone())));
        let toml = format!(
            "[mod]\nid = \"four\"\napi_version = \"1.4\"\n[permissions]\nnet = [\"udp:{target}\"]\nstorage = true\n"
        );
        h.load(Box::new(Mem(vec![("mod.toml", toml.into_bytes()), ("four.wasm", level_four_mod(&target))]))).unwrap();
        assert_eq!(storage::Storage::read(&*st, "count").unwrap(), Some(b"1".to_vec()));
        assert!(log.borrow().iter().any(|l| l == "mod four: 1"), "{:?}", log.borrow());
        assert!(log.borrow().iter().any(|l| l == "mod four: count"), "{:?}", log.borrow());
        // the echo server answers the ping
        let mut buf = [0u8; 16];
        echo.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
        let (n, peer) = echo.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"ping");
        echo.send_to(b"pong", peer).unwrap();
        let start = std::time::Instant::now();
        while !log.borrow().iter().any(|l| l == "mod four: pong") {
            assert!(start.elapsed().as_secs() < 2, "{:?}", log.borrow());
            h.frame_begin();
            h.overlay((640, 480), &mut Vec::new());
        }
        assert_eq!(h.mods()[0].sockets.open(), 1);
        // disabling closes its sockets
        h.disable(0, "test".into());
        assert_eq!(h.mods()[0].sockets.open(), 0);
    }

    #[test]
    fn sockets_need_the_manifest() {
        let (mut h, _) = host();
        h.set_network(platform_network(None));
        h.set_storage(Rc::new(DenyAll));
        struct DenyAll;
        impl StorageProvider for DenyAll {
            fn open(&self, _id: &str) -> Result<Rc<dyn storage::Storage>, String> {
                Err("no".into())
            }
        }
        // the rule names another port: net_open is DENIED and the mod traps in sa_init
        let toml = "[mod]\nid = \"four\"\napi_version = \"1.4\"\n[permissions]\nnet = [\"udp:127.0.0.1:1\"]\n";
        let e = h
            .load(Box::new(Mem(vec![("mod.toml", toml.as_bytes().to_vec()), ("four.wasm", level_four_mod("127.0.0.1:2"))])))
            .unwrap_err();
        assert!(e.contains("sa_init"), "{e}");
        // storage asked for, the host cannot open it: not loaded
        let toml = "[mod]\nid = \"four\"\napi_version = \"1.4\"\n[permissions]\nstorage = true\n";
        let e = h
            .load(Box::new(Mem(vec![("mod.toml", toml.as_bytes().to_vec()), ("four.wasm", level_four_mod("127.0.0.1:2"))])))
            .unwrap_err();
        assert!(e.contains("storage"), "{e}");
    }

    #[test]
    fn hooks_need_the_manifest() {
        let (mut h, _) = host();
        let reg = Rc::new(FakeRegistry::default());
        h.set_registry(Registry(reg.clone()));
        let toml = "[mod]\nid = \"two\"\napi_version = \"1.2\"\n";
        h.load(Box::new(Mem(vec![("mod.toml", toml.as_bytes().to_vec()), ("two.wasm", level_two_mod())])))
            .unwrap();
        assert_eq!(h.mods()[0].events, (1 << event::FRAME_BEGIN) | (1 << event::FRAME_END));
        assert!(h.mods()[0].hooks.is_empty());
        assert!(reg.0.borrow().is_empty());
    }
}
