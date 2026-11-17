//! The browser engine (wasm32): the page's own WebAssembly, through the worker's glue `web/mods.js`
//! (`self.saGameMods`). The glue knows no API function: it builds the mod's import object from the names the host
//! passes ([`sa_mod_api::IMPORTS`] order), and every import calls back into [`sa_mod_host_import`], which runs the
//! shared [`dispatch`]. Fuel: the module is instrumented at load time ([`super::meter`]); the glue sets the counter
//! before each call and reports an overrun. The memory cap is the instrumented memory's maximum. Instantiation is
//! synchronous (allowed in a worker).

use std::cell::RefCell;
use std::collections::BTreeMap;

use sa_mod_api::{IMPORTS, Val};
use wasm_bindgen::prelude::*;

use super::meter;
use crate::ModSource;
use crate::api::{CallCtx, Mem, dispatch};
use crate::engine::{CallError, Engine, Instance};
use crate::manifest::{INIT_FUEL_FACTOR, Limits};

#[wasm_bindgen]
extern "C" {
    /// `saGameMods.fetch(url)`: the file's bytes (synchronous request), `undefined` when it is missing.
    #[wasm_bindgen(js_namespace = saGameMods, js_name = fetch)]
    fn js_fetch(url: &str) -> Option<Vec<u8>>;
    /// `saGameMods.instantiate(bytes, names)`: compiles and instantiates a mod whose imports are `names`
    /// (comma-separated, the `IMPORTS` order); returns its handle.
    #[wasm_bindgen(js_namespace = saGameMods, js_name = instantiate, catch)]
    fn js_instantiate(bytes: &[u8], names: &str) -> Result<u32, JsValue>;
    /// `saGameMods.call(handle, name, args, fuel)`: the export's `i32` result; throws `"fuel"` on a fuel overrun,
    /// the trap's message otherwise.
    #[wasm_bindgen(js_namespace = saGameMods, js_name = call, catch)]
    fn js_call(handle: u32, name: &str, args: &[f64], fuel: i32) -> Result<f64, JsValue>;
    /// `saGameMods.read(handle, ptr, len)`: a copy of the mod's memory range, `undefined` when out of bounds.
    #[wasm_bindgen(js_namespace = saGameMods, js_name = read)]
    fn js_read(handle: u32, ptr: u32, len: u32) -> Option<Vec<u8>>;
    /// `saGameMods.write(handle, ptr, bytes)`: copies `bytes` into the mod's memory; false when out of bounds.
    #[wasm_bindgen(js_namespace = saGameMods, js_name = write)]
    fn js_write(handle: u32, ptr: u32, bytes: &[u8]) -> bool;
}

/// The memory of mod `handle`, through the glue.
struct JsMem(u32);

impl Mem for JsMem {
    fn read(&self, ptr: u32, len: u32) -> Option<Vec<u8>> {
        js_read(self.0, ptr, len)
    }

    fn write(&mut self, ptr: u32, data: &[u8]) -> bool {
        js_write(self.0, ptr, data)
    }
}

thread_local! {
    /// The call in progress: the mod's handle and its context (the game thread makes one call at a time).
    static CURRENT: RefCell<Option<(u32, CallCtx)>> = const { RefCell::new(None) };
}

/// The host functions' entry from the glue: import `index` of the mod being called, its arguments as numbers.
/// `None` (the glue then throws, trapping the mod) on a host trap.
#[wasm_bindgen]
pub fn sa_mod_host_import(index: u32, args: Vec<f64>) -> Option<f64> {
    let def = IMPORTS.get(index as usize)?;
    if def.params.len() != args.len() {
        return None;
    }
    let vals: Vec<Val> = def
        .params
        .iter()
        .zip(&args)
        .map(|(p, &a)| match p.1 {
            sa_mod_api::Ty::I32 => Val::I32(a as i32),
            sa_mod_api::Ty::F32 => Val::F32(a as f32),
        })
        .collect();
    CURRENT.with_borrow_mut(|c| {
        let (handle, ctx) = c.as_mut()?;
        let h = *handle;
        dispatch(ctx, &mut JsMem(h), index as usize, &vals).ok().map(f64::from)
    })
}

/// The browser's WebAssembly.
#[derive(Default)]
pub struct Browser;

impl Browser {
    /// The engine.
    pub fn new() -> Browser {
        Browser
    }
}

struct BrowserInstance {
    handle: u32,
}

fn fuel_i32(fuel: u64) -> i32 {
    fuel.min(i32::MAX as u64) as i32
}

impl Engine for Browser {
    fn name(&self) -> &'static str {
        "browser WebAssembly (instrumented fuel)"
    }

    fn instantiate(&self, wasm: &[u8], limits: &Limits) -> Result<Box<dyn Instance>, String> {
        // the counter starts with the init budget: a start function runs during instantiation
        let init = fuel_i32(limits.fuel.saturating_mul(INIT_FUEL_FACTOR));
        let wasm = meter::instrument(wasm, limits.memory_bytes, init)?;
        let names: Vec<&str> = IMPORTS.iter().map(|d| d.name).collect();
        let handle = js_instantiate(&wasm, &names.join(","))
            .map_err(|e| format!("instantiate: {}", e.as_string().unwrap_or_else(|| format!("{e:?}"))))?;
        Ok(Box::new(BrowserInstance { handle }))
    }
}

impl Instance for BrowserInstance {
    fn call(&mut self, name: &str, args: &[Val], fuel: u64, ctx: &mut CallCtx) -> Result<i32, CallError> {
        let nums: Vec<f64> = args
            .iter()
            .map(|v| match *v {
                Val::I32(x) => f64::from(x),
                Val::F32(x) => f64::from(x),
            })
            .collect();
        CURRENT.with_borrow_mut(|c| *c = Some((self.handle, std::mem::take(ctx))));
        let r = js_call(self.handle, name, &nums, fuel_i32(fuel));
        if let Some((_, back)) = CURRENT.with_borrow_mut(|c| c.take()) {
            *ctx = back;
        }
        match r {
            Ok(v) => Ok(v as i32),
            Err(e) => match e.as_string() {
                Some(s) if s == "fuel" => Err(CallError::OutOfFuel),
                Some(s) => Err(CallError::Trap(s)),
                None => Err(CallError::Trap(format!("{e:?}"))),
            },
        }
    }
}

thread_local! {
    /// The files mods wrote this session, by URL (the page keeps no files; a written file shadows the server's).
    static WRITTEN: RefCell<BTreeMap<String, Vec<u8>>> = const { RefCell::new(BTreeMap::new()) };
}

/// A mod folder on the page's server (`mods/NAME/`, relative to the worker). Files the mod writes are kept in
/// memory for the session.
struct UrlSource(String);

impl ModSource for UrlSource {
    fn read(&self, file: &str) -> Result<Vec<u8>, String> {
        let url = format!("{}{file}", self.0);
        if let Some(d) = WRITTEN.with_borrow(|w| w.get(&url).cloned()) {
            return Ok(d);
        }
        js_fetch(&url).ok_or_else(|| format!("{url}: not found"))
    }

    fn write(&self, file: &str, data: &[u8]) -> Result<(), String> {
        WRITTEN.with_borrow_mut(|w| w.insert(format!("{}{file}", self.0), data.to_vec()));
        Ok(())
    }

    fn location(&self) -> String {
        self.0.clone()
    }
}

/// The mods of the page's `?mods=a,b` (`SA_MODS`): `mods/a/`, `mods/b/`, in that order.
pub fn sources(spec: &str) -> Result<Vec<Box<dyn ModSource>>, String> {
    spec.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|name| {
            if name.contains(['/', '\\', '.']) {
                return Err(format!("mods: {name:?} is not a mod folder name"));
            }
            Ok(Box::new(UrlSource(format!("mods/{name}/"))) as Box<dyn ModSource>)
        })
        .collect()
}
