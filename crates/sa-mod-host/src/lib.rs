//! The mod host (MOD-2, `docs/arch/ADR-004-mods.md`): loads WebAssembly mods and calls them at the game's fixed
//! frame points. Linked into sa-game only with its feature `mods`.
//!
//! * [`manifest`]: `mod.toml` (identity, `api_version`, permissions, limits);
//! * [`module`]: the checks every module passes (the interface of `sa-mod-api`, the memory cap);
//! * [`api`]: the host functions, implemented once for both engines;
//! * [`engine`]: the engine seam, with the backends wasmtime (native) and the browser's WebAssembly (wasm32) in
//!   `backend`;
//! * [`Host`]: loading, the load order, the call points (the settings phase before the game's D3D device exists,
//!   the overlay phase of every frame, the resize event), budgets and disabling: a mod that traps or overruns its
//!   fuel is disabled for the session and reported; the game and the other mods go on.
//!
//! The host never touches game state: what a mod produces is overlay rectangles for the render layer, video-mode
//! requests (which the game applies through the original's own video-mode state), its own files and log lines.

pub mod api;
mod backend;
pub mod engine;
pub mod manifest;
pub mod module;

use std::rc::Rc;

use sa_mod_api::{
    API_MAJOR, API_MINOR, EXPORT_API_VERSION, EXPORT_INIT, EXPORT_OVERLAY, EXPORT_RESIZE, EXPORT_SETTINGS, Val, level,
    unpack_version,
};
use sa_platform::gpu::OverlayRect;

use crate::api::{CallCtx, Display, Files, VideoRequest};
use crate::engine::{Engine, Instance};
use crate::manifest::{INIT_FUEL_FACTOR, Manifest};

pub use backend::sources;

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
    /// Its folder (for its own files).
    source: Rc<dyn ModSource>,
    /// Why it was disabled (`None`: running).
    pub disabled: Option<String>,
    /// Overlay calls so far.
    pub frames: u32,
    /// Time spent in its calls (host functions included), in microseconds.
    pub micros: u64,
}

/// The mod host.
pub struct Host {
    engine: Box<dyn Engine>,
    mods: Vec<LoadedMod>,
    log: Box<dyn Fn(&str)>,
    display: Rc<Display>,
    /// A video mode requested outside the settings phase, not yet taken by the game.
    video: Option<VideoRequest>,
}

impl Host {
    /// A host on the platform's engine; `log` receives its messages and the mods' log lines.
    pub fn new(log: Box<dyn Fn(&str)>) -> Host {
        Host::with_engine(Box::new(backend::DefaultEngine::new()), log)
    }

    /// A host on `engine`.
    pub fn with_engine(engine: Box<dyn Engine>, log: Box<dyn Fn(&str)>) -> Host {
        Host { engine, mods: Vec::new(), log, display: Rc::default(), video: None }
    }

    /// The display the platform reports to the mods (`display_mode_*`).
    pub fn set_display(&mut self, display: Display) {
        self.display = Rc::new(display);
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
        let mut ctx = self.ctx(&manifest, &src);
        let init_fuel = manifest.limits.fuel.saturating_mul(INIT_FUEL_FACTOR);
        let v = instance.call(EXPORT_API_VERSION, &[], init_fuel, &mut ctx).map_err(|e| format!("mod {id}: {e}"))?;
        if unpack_version(v) != manifest.api {
            let (a, b) = unpack_version(v);
            return Err(format!(
                "mod {id}: the module reports API {a}.{b}, its manifest {}.{}",
                manifest.api.0, manifest.api.1
            ));
        }
        let r = instance.call(EXPORT_INIT, &[], init_fuel, &mut ctx);
        self.flush_logs(&id, &mut ctx);
        match r {
            Ok(0) => {}
            Ok(c) => return Err(format!("mod {id}: sa_init refused with {c}")),
            Err(e) => return Err(format!("mod {id}: sa_init: {e}")),
        }
        (self.log)(&format!(
            "mod {id} loaded: {} {} from {at} ({} bytes, {}; render {}, files {}, memory cap {} MiB, fuel {} per call)",
            manifest.name,
            manifest.version,
            wasm.len(),
            self.engine.name(),
            manifest.permissions.render,
            manifest.permissions.files,
            manifest.limits.memory_bytes >> 20,
            manifest.limits.fuel
        ));
        self.mods.push(LoadedMod { manifest, instance, exports, source: src, disabled: None, frames: 0, micros: 0 });
        Ok(())
    }

    /// A call context for the mod with `manifest` and folder `src`.
    fn ctx(&self, manifest: &manifest::Manifest, src: &Rc<dyn ModSource>) -> CallCtx {
        CallCtx {
            permissions: manifest.permissions,
            display: self.display.clone(),
            files: manifest.permissions.files.then(|| Files(src.clone())),
            ..Default::default()
        }
    }

    /// Calls `export` with `args` (built per mod) on every running mod that exports it, in load order; `each` gets
    /// every successful call's context. A mod that fails is disabled (what it produced in that call is dropped)
    /// and reported. Returns the last video-mode request.
    fn call_all(
        &mut self,
        export: &'static str,
        screen: (u32, u32),
        args: impl Fn(&LoadedMod) -> Vec<Val>,
        mut each: impl FnMut(&mut CallCtx),
    ) -> Option<VideoRequest> {
        let mut video = None;
        for i in 0..self.mods.len() {
            let m = &self.mods[i];
            if m.disabled.is_some() || !m.exports.contains(&export) {
                continue;
            }
            let mut ctx = self.ctx(&m.manifest, &m.source);
            ctx.screen = screen;
            ctx.overlay = export == EXPORT_OVERLAY;
            let a = args(m);
            let m = &mut self.mods[i];
            let t = web_time::Instant::now();
            let r = m.instance.call(export, &a, m.manifest.limits.fuel, &mut ctx);
            m.micros += t.elapsed().as_micros() as u64;
            let frame = m.frames;
            if export == EXPORT_OVERLAY {
                m.frames += 1;
            }
            let id = m.manifest.id.clone();
            // the mod's own lines first (a panicking Rust mod logs its message before it traps)
            self.flush_logs(&id, &mut ctx);
            match r {
                Ok(_) => {
                    video = ctx.video.or(video);
                    each(&mut ctx);
                }
                Err(e) => {
                    let msg = if export == EXPORT_OVERLAY {
                        format!("{e} in {export} (frame {frame})")
                    } else {
                        format!("{e} in {export}")
                    };
                    self.mods[i].disabled = Some(msg.clone());
                    (self.log)(&format!("mod {id} disabled: {msg}; the game continues"));
                }
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

    /// One line per mod: state, frames, time per frame.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        for m in &self.mods {
            let per = if m.frames > 0 { m.micros as f64 / m.frames as f64 } else { 0.0 };
            s += &format!(
                "mod {}: {}, {} overlay calls, {:.1} us per call\n",
                m.manifest.id,
                m.disabled.as_deref().map_or("running".to_string(), |d| format!("disabled ({d})")),
                m.frames,
                per
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
    use std::rc::Rc;

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
        s.0[0].1 = b"[mod]\nid = \"new\"\napi_version = \"1.2\"\n".to_vec();
        assert!(h.load(s).unwrap_err().contains("API 1.2"));
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
}
