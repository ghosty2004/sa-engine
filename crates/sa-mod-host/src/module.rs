//! The checks every module passes before either engine instantiates it (one implementation for both backends):
//! valid wasm; imports only host functions of [`sa_mod_api::IMPORTS`] with their exact signatures; the entry points
//! of [`sa_mod_api::EXPORTS`] with their signatures; one exported, non-shared 32-bit memory whose initial size fits
//! the mod's memory cap.

use sa_mod_api::{EXPORT_MEMORY, EXPORTS, IMPORT_MODULE, IMPORTS, Ty};
use wasmparser::{CompositeInnerType, ExternalKind, FuncType, Parser, Payload, TypeRef, ValType};

/// The wasm page size.
pub const PAGE: u64 = 65_536;

fn ty(t: Ty) -> ValType {
    match t {
        Ty::I32 => ValType::I32,
        Ty::F32 => ValType::F32,
    }
}

fn same(f: &FuncType, params: &[(&str, Ty)], result: Ty) -> bool {
    f.params().iter().copied().eq(params.iter().map(|p| ty(p.1))) && f.results() == [ty(result)]
}

/// Checks `wasm` against the interface and the memory cap `memory_bytes`.
pub fn check(wasm: &[u8], memory_bytes: u64) -> Result<(), String> {
    wasmparser::Validator::new().validate_all(wasm).map_err(|e| format!("invalid wasm: {e}"))?;
    let mut types: Vec<Option<FuncType>> = Vec::new();
    let mut funcs: Vec<u32> = Vec::new();
    let mut memories = Vec::new();
    let mut exports: Vec<(String, ExternalKind, u32)> = Vec::new();
    for p in Parser::new(0).parse_all(wasm) {
        match p.map_err(|e| e.to_string())? {
            Payload::TypeSection(r) => {
                for g in r {
                    for t in g.map_err(|e| e.to_string())?.into_types() {
                        types.push(match &t.composite_type.inner {
                            CompositeInnerType::Func(f) => Some(f.clone()),
                            _ => None,
                        });
                    }
                }
            }
            Payload::ImportSection(r) => {
                for i in r.into_imports() {
                    let i = i.map_err(|e| e.to_string())?;
                    let TypeRef::Func(t) = i.ty else {
                        return Err(format!("import {}.{}: only host functions can be imported", i.module, i.name));
                    };
                    let def = IMPORTS
                        .iter()
                        .find(|d| i.module == IMPORT_MODULE && d.name == i.name)
                        .ok_or_else(|| format!("import {}.{}: not a host function of API 1.0", i.module, i.name))?;
                    match types.get(t as usize) {
                        Some(Some(f)) if same(f, def.params, def.result) => {}
                        _ => return Err(format!("import {}.{}: wrong signature", i.module, i.name)),
                    }
                    funcs.push(t);
                }
            }
            Payload::FunctionSection(r) => {
                for f in r {
                    funcs.push(f.map_err(|e| e.to_string())?);
                }
            }
            Payload::MemorySection(r) => {
                for m in r {
                    memories.push(m.map_err(|e| e.to_string())?);
                }
            }
            Payload::ExportSection(r) => {
                for e in r {
                    let e = e.map_err(|e| e.to_string())?;
                    exports.push((e.name.to_string(), e.kind, e.index));
                }
            }
            _ => {}
        }
    }
    let [mem] = memories[..] else { return Err(format!("{} memories: a mod defines exactly one", memories.len())) };
    if mem.memory64 || mem.shared || mem.page_size_log2.is_some() {
        return Err("the memory must be a plain 32-bit, non-shared memory".into());
    }
    if mem.initial * PAGE > memory_bytes {
        return Err(format!("initial memory {} KiB is above the cap of {} KiB", (mem.initial * PAGE) >> 10, memory_bytes >> 10));
    }
    if !exports.iter().any(|(n, k, _)| n == EXPORT_MEMORY && *k == ExternalKind::Memory) {
        return Err(format!("no memory exported as {EXPORT_MEMORY:?}"));
    }
    for d in EXPORTS {
        let Some((_, _, i)) = exports.iter().find(|(n, k, _)| n == d.name && *k == ExternalKind::Func) else {
            return Err(format!("no function exported as {:?}", d.name));
        };
        match funcs.get(*i as usize).and_then(|t| types.get(*t as usize)) {
            Some(Some(f)) if same(f, d.params, d.result) => {}
            _ => return Err(format!("export {:?}: wrong signature", d.name)),
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A minimal mod: `overlay` is the body of `sa_overlay` (param 0: the frame).
    pub fn wat_mod(extra_imports: &str, memory: &str, overlay: &str) -> Vec<u8> {
        wat::parse_str(format!(
            r#"(module
              (import "sa" "overlay_rect" (func $rect (param f32 f32 f32 f32 i32) (result i32)))
              {extra_imports}
              (memory (export "memory") {memory})
              (func (export "sa_api_version") (result i32) i32.const 0x10000)
              (func (export "sa_init") (result i32) i32.const 0)
              (func (export "sa_overlay") (param i32) (result i32) {overlay}))"#
        ))
        .unwrap()
    }

    #[test]
    fn accepts_and_rejects() {
        let ok = wat_mod("", "1", "i32.const 0");
        assert_eq!(check(&ok, 1 << 20), Ok(()));
        assert!(check(&ok, 1000).unwrap_err().contains("cap"));
        let bad = wat_mod(r#"(import "env" "x" (func))"#, "1", "i32.const 0");
        assert!(check(&bad, 1 << 20).unwrap_err().contains("env.x"));
        let bad = wat_mod(r#"(import "sa" "log" (func (param i32)))"#, "1", "i32.const 0");
        assert!(check(&bad, 1 << 20).unwrap_err().contains("signature"));
        let bad = wat::parse_str(r#"(module (memory (export "memory") 1))"#).unwrap();
        assert!(check(&bad, 1 << 20).unwrap_err().contains("sa_api_version"));
        assert!(check(b"\0asm junk", 1 << 20).is_err());
    }
}
