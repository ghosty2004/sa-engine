//! The native engine: wasmtime. On x86_64 Cranelift compiles the mod to machine code; on i686 (the game's native
//! build), where Cranelift has no backend, it compiles to Pulley bytecode that wasmtime's interpreter runs. Budgets:
//! wasmtime fuel per call, the memory cap through the store's resource limiter. Signal-based traps are off (the
//! game owns the process's signal handlers; bounds are checked explicitly) and memories are not reserved up front
//! (the i686 address space is shared with the game's arena). Sockets are real UDP sockets; storage is a folder
//! per mod under the user's data directory.

use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use sa_mod_api::{EXPORT_MEMORY, EXPORTS, IMPORT_MODULE, IMPORTS, Ty, Val};
use wasmtime::{Config, Engine as WtEngine, FuncType, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, Trap, ValType};

use crate::ModSource;
use crate::api::{CallCtx, SliceMem, dispatch};
use crate::engine::{CallError, Engine, Instance};
use crate::manifest::{INIT_FUEL_FACTOR, Limits};
use crate::net::{DatagramTransport, Network};
use crate::storage::{Storage, StorageProvider};

/// wasmtime.
pub struct Wasmtime {
    engine: WtEngine,
}

impl Default for Wasmtime {
    fn default() -> Wasmtime {
        Wasmtime::new()
    }
}

impl Wasmtime {
    /// The engine with the host's configuration.
    pub fn new() -> Wasmtime {
        let mut cfg = Config::new();
        cfg.consume_fuel(true);
        if cfg!(target_pointer_width = "32") {
            // no Cranelift backend for x86-32: Pulley bytecode, interpreted
            let _ = cfg.target("pulley32");
        }
        cfg.signals_based_traps(false);
        cfg.memory_reservation(0).memory_guard_size(0).memory_reservation_for_growth(1 << 20);
        // the defaults (or a failure to apply the target) leave a working engine; an error here is a bug
        let engine = WtEngine::new(&cfg).unwrap_or_else(|_| WtEngine::default());
        Wasmtime { engine }
    }
}

fn val_type(t: Ty) -> ValType {
    match t {
        Ty::I32 => ValType::I32,
        Ty::F32 => ValType::F32,
    }
}

fn to_wt(v: &Val) -> wasmtime::Val {
    match *v {
        Val::I32(x) => wasmtime::Val::I32(x),
        Val::F32(x) => wasmtime::Val::F32(x.to_bits()),
    }
}

/// A store's data: the context of the call in progress and the memory limiter.
struct State {
    ctx: CallCtx,
    limits: StoreLimits,
}

struct WtInstance {
    store: Store<State>,
    funcs: Vec<(&'static str, wasmtime::Func)>,
}

fn classify(e: wasmtime::Error) -> CallError {
    match e.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => CallError::OutOfFuel,
        Some(t) => CallError::Trap(t.to_string()),
        // a host trap or another error: its first line (wasmtime appends the wasm backtrace)
        None => CallError::Trap(format!("{e}").lines().next().unwrap_or("").to_string()),
    }
}

impl Engine for Wasmtime {
    fn name(&self) -> &'static str {
        if cfg!(target_pointer_width = "32") { "wasmtime (Pulley interpreter)" } else { "wasmtime (Cranelift)" }
    }

    fn instantiate(&self, wasm: &[u8], limits: &Limits) -> Result<Box<dyn Instance>, String> {
        let module = Module::new(&self.engine, wasm).map_err(|e| format!("compile: {e:#}"))?;
        let state = State {
            ctx: CallCtx::default(),
            limits: StoreLimitsBuilder::new()
                .memory_size(limits.memory_bytes as usize)
                .memories(1)
                .instances(1)
                .tables(4)
                .table_elements(100_000)
                .build(),
        };
        let mut store = Store::new(&self.engine, state);
        store.limiter(|s| &mut s.limits);
        let mut linker: Linker<State> = Linker::new(&self.engine);
        for (index, def) in IMPORTS.iter().enumerate() {
            let ty = FuncType::new(&self.engine, def.params.iter().map(|p| val_type(p.1)), [val_type(def.result)]);
            linker
                .func_new(IMPORT_MODULE, def.name, ty, move |mut caller, params, results| {
                    let mut args = [Val::I32(0); 8];
                    for (a, p) in args.iter_mut().zip(params) {
                        *a = match p {
                            wasmtime::Val::I32(x) => Val::I32(*x),
                            wasmtime::Val::F32(x) => Val::F32(f32::from_bits(*x)),
                            _ => Val::I32(0),
                        };
                    }
                    let mem = caller.get_export(EXPORT_MEMORY).and_then(|e| e.into_memory());
                    let mut empty = [0u8; 0];
                    let (data, state): (&mut [u8], &mut State) = match mem {
                        Some(m) => m.data_and_store_mut(&mut caller),
                        None => (&mut empty[..], caller.data_mut()),
                    };
                    let r = dispatch(&mut state.ctx, &mut SliceMem(data), index, &args[..params.len().min(8)])
                        .map_err(wasmtime::Error::msg)?;
                    results[0] = wasmtime::Val::I32(r);
                    Ok(())
                })
                .map_err(|e| format!("link {}: {e:#}", def.name))?;
        }
        store.set_fuel(limits.fuel.saturating_mul(INIT_FUEL_FACTOR)).map_err(|e| e.to_string())?;
        let inst = linker.instantiate(&mut store, &module).map_err(|e| format!("instantiate: {}", classify(e)))?;
        let funcs = EXPORTS
            .iter()
            .filter_map(|d| Some((d.name, inst.get_func(&mut store, d.name)?)))
            .collect();
        Ok(Box::new(WtInstance { store, funcs }))
    }
}

impl Instance for WtInstance {
    fn call(&mut self, name: &str, args: &[Val], fuel: u64, ctx: &mut CallCtx) -> Result<i32, CallError> {
        let f = self
            .funcs
            .iter()
            .find(|(n, _)| *n == name)
            .map(|p| p.1)
            .ok_or_else(|| CallError::Trap(format!("no export {name}")))?;
        let params: Vec<wasmtime::Val> = args.iter().map(to_wt).collect();
        let mut res = [wasmtime::Val::I32(0)];
        std::mem::swap(&mut self.store.data_mut().ctx, ctx);
        let r = self.store.set_fuel(fuel).and_then(|_| f.call(&mut self.store, &params, &mut res));
        std::mem::swap(&mut self.store.data_mut().ctx, ctx);
        r.map_err(classify)?;
        match res[0] {
            wasmtime::Val::I32(v) => Ok(v),
            _ => Err(CallError::Trap(format!("{name} returned no i32"))),
        }
    }
}

/// A mod folder on disk.
struct DirSource(PathBuf);

impl ModSource for DirSource {
    fn read(&self, file: &str) -> Result<Vec<u8>, String> {
        let p = self.0.join(file);
        std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))
    }

    fn write(&self, file: &str, data: &[u8]) -> Result<(), String> {
        write_replacing(&self.0, file, data)
    }

    fn location(&self) -> String {
        self.0.display().to_string()
    }
}

/// Writes `dir/file` through a temporary file in `dir` renamed over it (a crash leaves the old file).
fn write_replacing(dir: &Path, file: &str, data: &[u8]) -> Result<(), String> {
    let p = dir.join(file);
    let tmp = dir.join(format!(".{file}.tmp"));
    std::fs::write(&tmp, data).and_then(|()| std::fs::rename(&tmp, &p)).map_err(|e| format!("{}: {e}", p.display()))
}

/// Real UDP sockets (sa-net's, as SA:MP's client opens them).
struct Udp;

impl Network for Udp {
    fn open(&self, target: &str) -> io::Result<Box<dyn DatagramTransport>> {
        use sa_net::Connector;
        Ok(Box::new(sa_net::UdpConnector.connect(target)?))
    }

    fn describe(&self) -> String {
        "UDP".into()
    }
}

/// The platform's network: UDP sockets (`_proxy`, the browser's relay, does not apply natively).
pub fn network(_proxy: Option<&str>) -> Rc<dyn Network> {
    Rc::new(Udp)
}

/// The storage areas under one folder: one sub-folder per mod id, one file per key.
struct DirStorages(PathBuf);

impl StorageProvider for DirStorages {
    fn open(&self, id: &str) -> Result<Rc<dyn Storage>, String> {
        let dir = self.0.join(id);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Rc::new(DirStorage(dir)))
    }
}

struct DirStorage(PathBuf);

impl Storage for DirStorage {
    fn read(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let p = self.0.join(key);
        match std::fs::read(&p) {
            Ok(d) => Ok(Some(d)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", p.display())),
        }
    }

    fn write(&self, key: &str, data: &[u8]) -> Result<(), String> {
        write_replacing(&self.0, key, data)
    }

    fn delete(&self, key: &str) -> Result<bool, String> {
        let p = self.0.join(key);
        match std::fs::remove_file(&p) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("{}: {e}", p.display())),
        }
    }

    fn list(&self) -> Result<Vec<(String, usize)>, String> {
        let rd = std::fs::read_dir(&self.0).map_err(|e| format!("{}: {e}", self.0.display()))?;
        let mut keys: Vec<(String, usize)> = rd
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                let size = e.metadata().ok().filter(|m| m.is_file())?.len() as usize;
                sa_mod_api::storage::valid_key(&name).then_some((name, size))
            })
            .collect();
        keys.sort();
        Ok(keys)
    }

    fn location(&self) -> String {
        self.0.display().to_string()
    }
}

/// The mods' storage under `root`, by default `$SA_MOD_DATA`, else the user's data directory
/// (`$XDG_DATA_HOME`, `~/.local/share`, `%APPDATA%`) + `sa-engine/mods`.
pub fn storage(root: Option<&str>) -> Result<Rc<dyn StorageProvider>, String> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let root = match root {
        Some(r) => PathBuf::from(r),
        None => var("SA_MOD_DATA")
            .or_else(|| var("XDG_DATA_HOME").map(|d| d.join("sa-engine/mods")))
            .or_else(|| var("HOME").map(|d| d.join(".local/share/sa-engine/mods")))
            .or_else(|| var("APPDATA").map(|d| d.join("sa-engine").join("mods")))
            .ok_or("mod storage: no data directory (set SA_MOD_DATA)")?,
    };
    Ok(Rc::new(DirStorages(root)))
}

/// The mods of `--mods DIR[,DIR...]`, in that order (the load order): each DIR itself when it holds a `mod.toml`,
/// else every sub-folder of it that holds one, in name order.
pub fn sources(spec: &str) -> Result<Vec<Box<dyn ModSource>>, String> {
    let mut all = Vec::new();
    for dir in spec.split(',').filter(|d| !d.is_empty()) {
        all.extend(folder_sources(dir)?);
    }
    Ok(all)
}

fn folder_sources(spec: &str) -> Result<Vec<Box<dyn ModSource>>, String> {
    let dir = PathBuf::from(spec);
    if dir.join("mod.toml").is_file() {
        return Ok(vec![Box::new(DirSource(dir))]);
    }
    let rd = std::fs::read_dir(&dir).map_err(|e| format!("--mods {spec}: {e}"))?;
    let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.join("mod.toml").is_file()).collect();
    dirs.sort();
    Ok(dirs.into_iter().map(|d| Box::new(DirSource(d)) as Box<dyn ModSource>).collect())
}
