//! The native engine: wasmtime. On x86_64 Cranelift compiles the mod to machine code; on i686 (the game's native
//! build), where Cranelift has no backend, it compiles to Pulley bytecode that wasmtime's interpreter runs. Budgets:
//! wasmtime fuel per call, the memory cap through the store's resource limiter. Signal-based traps are off (the
//! game owns the process's signal handlers; bounds are checked explicitly) and memories are not reserved up front
//! (the i686 address space is shared with the game's arena).

use std::path::PathBuf;

use sa_mod_api::{EXPORT_MEMORY, EXPORTS, IMPORT_MODULE, IMPORTS, Ty, Val};
use wasmtime::{Config, Engine as WtEngine, FuncType, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, Trap, ValType};

use crate::ModSource;
use crate::api::{CallCtx, dispatch};
use crate::engine::{CallError, Engine, Instance};
use crate::manifest::{INIT_FUEL_FACTOR, Limits};

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
                    let (data, state): (&[u8], &mut State) = match mem {
                        Some(m) => {
                            let (d, s) = m.data_and_store_mut(&mut caller);
                            (d, s)
                        }
                        None => (&[], caller.data_mut()),
                    };
                    let read = |p: u32, n: u32| data.get(p as usize..(p as usize).checked_add(n as usize)?).map(<[u8]>::to_vec);
                    let r = dispatch(&mut state.ctx, &read, index, &args[..params.len().min(8)])
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

    fn location(&self) -> String {
        self.0.display().to_string()
    }
}

/// The mods of `--mods DIR`: DIR itself when it holds a `mod.toml`, else every sub-folder that holds one, in name
/// order (the load order).
pub fn sources(spec: &str) -> Result<Vec<Box<dyn ModSource>>, String> {
    let dir = PathBuf::from(spec);
    if dir.join("mod.toml").is_file() {
        return Ok(vec![Box::new(DirSource(dir))]);
    }
    let rd = std::fs::read_dir(&dir).map_err(|e| format!("--mods {spec}: {e}"))?;
    let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.join("mod.toml").is_file()).collect();
    dirs.sort();
    Ok(dirs.into_iter().map(|d| Box::new(DirSource(d)) as Box<dyn ModSource>).collect())
}
