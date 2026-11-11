//! Installs `orig!`'s phase B address registry (`sa_core::orig::registry`): every `#[hook]` port
//! of every engine crate (`sa_ports::ALL`) and every `#[native]` port (`sa_ports::NATIVES`), keyed
//! by its original address, plus the CRT replacements ([`crate::crt`]), the native COM objects and
//! the temporary RW driver stand-ins ([`crate::rw_standin`], only where no port exists). An `orig!` declaration whose callee address has an entry then
//! calls the port natively; any other callee fails loudly with its name and address.

use sa_core::orig::registry::{self, Entry};

/// What [`install`] registered.
pub struct Summary {
    pub ports: usize,
    pub crt: usize,
    pub imports: usize,
    /// Addresses with more than one `#[hook]` (the first in `sa_ports::ALL` order wins).
    pub duplicates: Vec<(u32, &'static str, &'static str)>,
}

pub fn install() -> Summary {
    let mut v: Vec<(u32, usize, &'static str)> = Vec::with_capacity(sa_ports::ALL.len() + 16);
    let mut duplicates = Vec::new();
    let mut seen = std::collections::BTreeMap::new();
    for h in sa_ports::ALL {
        if let Some(first) = seen.get(&h.addr) {
            duplicates.push((h.addr, *first, h.name));
            continue;
        }
        seen.insert(h.addr, h.name);
        v.push((h.addr, (h.entry)(), h.name));
    }
    // `#[native]` ports (not hookable in phase A)
    for n in sa_ports::natives() {
        if let Some(first) = seen.get(&n.addr) {
            duplicates.push((n.addr, *first, n.name));
            continue;
        }
        seen.insert(n.addr, n.name);
        v.push((n.addr, (n.entry)(), n.name));
    }
    let ports = v.len();
    // exploration stubs (`SA_GAME_STUB`), only where nothing else is registered
    for (a, f) in stubs() {
        if let std::collections::btree_map::Entry::Vacant(e) = seen.entry(a) {
            eprintln!("sa-game: SA_GAME_STUB: {a:#x} returns a constant (exploration only)");
            e.insert("SA_GAME_STUB");
            v.push((a, f, "SA_GAME_STUB (exploration stub)"));
        }
    }
    let mut crt = 0;
    let others = crate::crt::entries()
        .into_iter()
        .chain(crate::rw_standin::entries())
        .chain(crate::d3d9_shader::entries())
        .chain(crate::com::thunks());
    for (a, name, f) in others {
        if !seen.contains_key(&a) {
            v.push((a, f, name));
            crt += 1;
        }
    }
    // native COM objects: implemented methods at their traps, every trap named
    let mut names = crate::com::names();
    for (a, name, f) in crate::com::entries().into_iter().chain(crate::dx::entries()).chain(crate::d3d9::entries()).chain(crate::dsound::entries()) {
        v.push((a, f, name));
    }
    names.extend(crate::dx::names());
    names.extend(crate::d3d9::names());
    names.extend(crate::dsound::names());
    names.extend(crate::d3d9_shader::names());
    // imports: the native implementation at the import's trap address; every trap named
    let natives: std::collections::HashMap<&str, usize> =
        crate::imports::table().into_iter().collect();
    let mut imports = 0;
    for imp in crate::arena::imports() {
        names.push((imp.trap, imp.name));
        if let Some(&f) = natives.get(imp.name) {
            v.push((imp.trap, f, imp.name));
            imports += 1;
        }
    }
    names.sort_by_key(|e| e.0);
    registry::install_names(Box::leak(names.into_boxed_slice()));
    v.sort_by_key(|e| e.0);
    let table: Vec<Entry> = v.into_iter().map(|(a, f, n)| Entry::new(a, f, n)).collect();
    registry::install(Box::leak(table.into_boxed_slice()));
    Summary {
        ports,
        crt,
        imports,
        duplicates,
    }
}

/// Every engine crate's `globals!` lists (the crates of `sa_ports::CRATES`, same order).
const GLOBAL_LISTS: &[&[&[sa_core::mem::GlobalDesc]]] = &[
    sa_core::hooks::GLOBALS,
    sa_stream::hooks::GLOBALS,
    sa_world::hooks::GLOBALS,
    sa_pop::hooks::GLOBALS,
    sa_script::hooks::GLOBALS,
    sa_weapon::hooks::GLOBALS,
    sa_ped::hooks::GLOBALS,
    sa_input::hooks::GLOBALS,
    sa_anim::hooks::GLOBALS,
    sa_veh::hooks::GLOBALS,
    sa_hud::hooks::GLOBALS,
    sa_cam::hooks::GLOBALS,
    sa_ai::hooks::GLOBALS,
    sa_audio::hooks::GLOBALS,
    sa_rwcore::hooks::GLOBALS,
    sa_render::hooks::GLOBALS,
];


/// Checks that every `globals!` declaration inside the arena starts with the exe image's value
/// (the arena now holds the one copy). Returns `(checked, mismatches)`.
pub fn check_globals() -> (usize, Vec<String>) {
    let (mut n, mut bad) = (0, Vec::new());
    for list in GLOBAL_LISTS.iter().flat_map(|l| l.iter()) {
        for g in list.iter() {
            if !registry::arena_contains(g.addr as usize, g.size) {
                continue;
            }
            n += 1;
            let have: Vec<u8> = (0..g.size)
                .map(|i| sa_core::mem::GPtr::<u8>::from_addr(g.addr as usize + i).read())
                .collect();
            if have != g.init {
                bad.push(format!(
                    "{} @ {:#x}: exe {:02x?}, declared {:02x?}",
                    g.name, g.addr, &have[..have.len().min(8)], &g.init[..g.init.len().min(8)]
                ));
            }
        }
    }
    (n, bad)
}

/// `(name, address, calls)` of the entries the registry dispatched to, most called first.
pub fn dispatched() -> Vec<(&'static str, u32, u32)> {
    let mut v: Vec<_> = registry::table()
        .iter()
        .filter_map(|e| {
            let n = e.calls.load(std::sync::atomic::Ordering::Relaxed);
            (n > 0).then_some((e.name, e.addr, n))
        })
        .collect();
    v.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.cmp(&b.1)));
    v
}

/// Return values of the exploration stubs, by stub index.
#[cfg(target_arch = "x86")]
static STUB_RET: [std::sync::atomic::AtomicU32; 16] = [const { std::sync::atomic::AtomicU32::new(0) }; 16];

#[cfg(target_arch = "x86")]
/// Stub `i` with `n` callee-cleaned stack dwords (`n = 0`: cdecl, also fine for a thiscall
/// without stack arguments).
macro_rules! stub_fns {
    ($($i:literal),*) => {
        [$([
            {
                extern "C" fn f() -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(_a: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(_a: u32, _b: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(_a: u32, _b: u32, _c: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(_a: u32, _b: u32, _c: u32, _d: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
        ]),*]
    };
}

/// `SA_GAME_STUB="ADDR=RET[/N],..."` (development aid, never a port): registers up to 16
/// addresses as functions that return RET without doing anything, so a boot run can look past a
/// blocker another lane is porting. `N` (0..=4, default 0) is the number of stack dwords the
/// callee pops (stdcall / thiscall arguments after `this`); 0 suits cdecl.
#[cfg(target_arch = "x86")]
fn stubs() -> Vec<(u32, usize)> {
    let Ok(spec) = std::env::var("SA_GAME_STUB") else {
        return Vec::new();
    };
    let fns: [[usize; 5]; 16] = stub_fns!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
    let num = |t: &str| {
        let t = t.trim();
        match t.strip_prefix("0x") {
            Some(h) => u32::from_str_radix(h, 16).ok(),
            None => t.parse().ok(),
        }
    };
    let mut out = Vec::new();
    for (i, item) in spec.split(',').filter(|s| !s.trim().is_empty()).take(16).enumerate() {
        let (a, r) = item.split_once('=').unwrap_or((item, "0"));
        let (r, n) = r.split_once('/').unwrap_or((r, "0"));
        if let (Some(a), Some(r), Some(n)) = (num(a), num(r), num(n))
            && n <= 4
        {
            STUB_RET[i].store(r, std::sync::atomic::Ordering::Relaxed);
            out.push((a, fns[i][n as usize]));
        }
    }
    out
}
#[cfg(not(target_arch = "x86"))]
fn stubs() -> Vec<(u32, usize)> {
    Vec::new()
}

/// `SA_GAME_SCRIPT_SKIP=1` (exploration only, never a port): an unported script opcode is logged
/// ("script: skipped opcode XXXX") and its parameters skipped (`CollectParameters(n)`, n from
/// `catalog/script-params.csv`; opcodes with a variable or unknown count stop the run as before), so
/// one run lists the opcodes a script path needs. The skipped opcode's effect is missing.
pub fn install_script_skip() {
    if std::env::var_os("SA_GAME_SCRIPT_SKIP").is_none() {
        return;
    }
    fn handler(s: sa_script::vm::Scr, command: u32) -> Option<i8> {
        static PARAMS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog/script-params.csv"));
        let key = format!("{command:04X},");
        let n: u32 = PARAMS.lines().find(|l| l.starts_with(&key))?.split(',').nth(4)?.trim().parse().ok()?;
        eprintln!("script: skipped opcode {command:04X} ({n} params)");
        if n > 0 {
            sa_script::cmdlib::collect(s, n);
        }
        Some(0)
    }
    let f: fn(sa_script::vm::Scr, u32) -> Option<i8> = handler;
    sa_script::cmdlib::UNPORTED_OPCODE_HANDLER.store(f as usize, std::sync::atomic::Ordering::Relaxed);
    println!("sa-game: SA_GAME_SCRIPT_SKIP: unported script opcodes are skipped (exploration only)");
}
