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
    // exploration stubs (`SA_GAME_STUB`), only where nothing else is registered unless forced (`!ADDR`)
    for (a, f, force) in stubs() {
        if force && let Some(slot) = v.iter_mut().find(|r| r.0 == a) {
            eprintln!("sa-game: SA_GAME_STUB: {a:#x} ({}) is replaced by a constant (forced; exploration only)", slot.2);
            *slot = (a, f, "SA_GAME_STUB (forced exploration stub)");
            continue;
        }
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
#[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
static STUB_RET: [std::sync::atomic::AtomicU32; 48] = [const { std::sync::atomic::AtomicU32::new(0) }; 48];

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
            {
                extern "stdcall" fn f(_a: u32, _b: u32, _c: u32, _d: u32, _e: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(_a: u32, _b: u32, _c: u32, _d: u32, _e: u32, _f: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(_a: u32, _b: u32, _c: u32, _d: u32, _e: u32, _f: u32, _g: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(_a: u32, _b: u32, _c: u32, _d: u32, _e: u32, _f: u32, _g: u32, _h: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(_a: u32, _b: u32, _c: u32, _d: u32, _e: u32, _f: u32, _g: u32, _h: u32, _i: u32) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
            {
                extern "stdcall" fn f(
                    _a: u32, _b: u32, _c: u32, _d: u32, _e: u32, _f: u32, _g: u32, _h: u32, _i: u32, _j: u32,
                ) -> u32 {
                    STUB_RET[$i].load(std::sync::atomic::Ordering::Relaxed)
                }
                f as *const () as usize
            },
        ]),*]
    };
}

/// `SA_GAME_STUB="ADDR=RET[/N],..."` (development aid, never a port): registers up to 48
/// addresses as functions that return RET without doing anything, so a boot run can look past a
/// blocker another lane is porting. `N` (0..=10, default 0) is the number of stack dwords the
/// callee pops (stdcall / thiscall arguments after `this`); 0 suits cdecl. `!ADDR` replaces a registered port too
/// (calls through the address registry only; Rust callers that name the port directly still reach it).
/// wasm32: the stubbed addresses, by stub index (the one erased stub entry finds its value by the callee address).
#[cfg(target_arch = "wasm32")]
static STUB_ADDR: [std::sync::atomic::AtomicU32; 48] = [const { std::sync::atomic::AtomicU32::new(0) }; 48];

/// wasm32: every exploration stub (the callee pops nothing: wasm has no caller-visible stack to clean).
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
extern "C" fn stub_entry(callee: u32, _args: *const u32, ret: *mut sa_core::wabi::Ret) {
    use std::sync::atomic::Ordering;
    let v = (0..48)
        .find(|&i| STUB_ADDR[i].load(Ordering::Relaxed) == callee)
        .map_or(0, |i| STUB_RET[i].load(Ordering::Relaxed));
    // SAFETY: the dispatcher passes its `Ret` slot.
    sa_core::wabi::put_ret(v, unsafe { &mut *ret });
}

#[cfg(target_arch = "wasm32")]
macro_rules! stub_fns {
    ($($i:literal),*) => {
        [$({
            let _ = $i;
            [stub_entry as sa_core::wabi::Erased as usize; 11]
        }),*]
    };
}

#[cfg(any(target_arch = "x86", target_arch = "wasm32"))]
fn stubs() -> Vec<(u32, usize, bool)> {
    let Ok(spec) = crate::env::var("SA_GAME_STUB") else {
        return Vec::new();
    };
    let fns: [[usize; 11]; 48] = stub_fns!(
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
        31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47
    );
    let num = |t: &str| {
        let t = t.trim();
        match t.strip_prefix("0x") {
            Some(h) => u32::from_str_radix(h, 16).ok(),
            None => t.parse().ok(),
        }
    };
    let mut out = Vec::new();
    for (i, item) in spec.split(',').filter(|s| !s.trim().is_empty()).take(48).enumerate() {
        let (a, r) = item.split_once('=').unwrap_or((item, "0"));
        let (a, force) = match a.trim().strip_prefix('!') {
            Some(a) => (a, true),
            None => (a, false),
        };
        let (r, n) = r.split_once('/').unwrap_or((r, "0"));
        if let (Some(a), Some(r), Some(n)) = (num(a), num(r), num(n))
            && n <= 10
        {
            STUB_RET[i].store(r, std::sync::atomic::Ordering::Relaxed);
            #[cfg(target_arch = "wasm32")]
            STUB_ADDR[i].store(a, std::sync::atomic::Ordering::Relaxed);
            out.push((a, fns[i][n as usize], force));
        }
    }
    out
}
#[cfg(not(any(target_arch = "x86", target_arch = "wasm32")))]
fn stubs() -> Vec<(u32, usize, bool)> {
    Vec::new()
}

/// `SA_GAME_SCRIPT_TRACE=1` (exploration only): every command's script name (+8), offset in the script space and
/// command word on stderr.
static SCRIPT_TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `--start-in-world` (see [`set_start_in_world`]).
static START_IN_WORLD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `--start-in-world` (evidence runs, never a port): the new game starts without the intro mission. main.scm's MAIN
/// script runs unchanged (it creates the player at Grove Street, `CREATE_PLAYER 0 2488.56 -1666.84 13.38`, and loads
/// the scene there); only its `START_MISSION 2` (0x0417 at main.scm offset 0xEA48: the INTRO mission, the airport
/// cutscene, the ride to Grove Street and the intro's ped scenes) is skipped, and the screen that MAIN faded out just
/// before it (`DO_FADE 0 0` at 0xEA3B) is faded in as the intro would, through the game's own `CCamera::Fade(0.0,
/// FADE_IN)`. MAIN then continues as after the intro's launch (`SELECT_INTERIOR 0`, `SET_PLAYER_CONTROL on`,
/// `RELEASE_WEATHER`, its main loop). Must be set before [`install_script_skip`].
pub fn set_start_in_world() {
    START_IN_WORLD.store(true, std::sync::atomic::Ordering::Relaxed);
    println!("sa-game: --start-in-world: main.scm's START_MISSION 2 (the intro) is skipped; the screen is faded in");
}

sa_core::orig! {
    /// `CCamera::Fade(float time, int16 direction)` (0x50AC20, `ret 8`), the float as its raw dword
    fn camera_fade(cam: sa_core::mem::GPtr<u8>, time: u32, direction: u32) = thiscall 0x50AC20 => unported;
}

/// The script VM's per-command callback (installed for `SA_GAME_SCRIPT_TRACE` and `--start-in-world`), called with
/// the script and the command word before the command is dispatched.
fn before_command(s: sa_script::vm::Scr, w: u32) {
    let ip_cell = s.byte_add(0x14).cast::<u32>();
    let ip = ip_cell.read();
    if SCRIPT_TRACE.load(std::sync::atomic::Ordering::Relaxed) {
        let name: Vec<u8> = (0..8).map(|i| s.byte_add(8 + i).read()).take_while(|&c| c != 0).collect();
        eprintln!("script-trace: {} {:#x} {w:04X}", String::from_utf8_lossy(&name), ip.wrapping_sub(0xA49960));
    }
    // START_MISSION with the 8-bit immediate 2 (`17 04 04 02`)
    if START_IN_WORLD.load(std::sync::atomic::Ordering::Relaxed)
        && w == 0x0417
        && sa_core::mem::GPtr::<u16>::from_addr(ip as usize + 2).read() == 0x0204
    {
        START_IN_WORLD.store(false, std::sync::atomic::Ordering::Relaxed);
        ip_cell.write(ip + 4);
        camera_fade(sa_core::mem::GPtr::from_addr(0xB6F028), 0, 1);
        println!("sa-game: --start-in-world: skipped START_MISSION 2 at main.scm {:#x}", ip.wrapping_sub(0xA49960));
    }
}

/// `SA_GAME_SCRIPT_SKIP=1` (exploration only, never a port): an unported script opcode is logged
/// ("script: skipped opcode XXXX") and its parameters skipped (`CollectParameters(n)`, n from
/// `catalog/script-params.csv`; opcodes with a variable or unknown count stop the run as before), so
/// one run lists the opcodes a script path needs (the parameter
/// forms of the `reads` column: `T` text labels and `Cn` parameter groups). The skipped opcode's effect is missing.
pub fn install_script_skip() {
    let trace = crate::env::var_os("SA_GAME_SCRIPT_TRACE").is_some();
    SCRIPT_TRACE.store(trace, std::sync::atomic::Ordering::Relaxed);
    if trace || START_IN_WORLD.load(std::sync::atomic::Ordering::Relaxed) {
        let f: fn(sa_script::vm::Scr, u32) = before_command;
        sa_script::cmdlib::COMMAND_TRACE_HANDLER.store(f as usize, std::sync::atomic::Ordering::Relaxed);
    }
    if crate::env::var_os("SA_GAME_SCRIPT_SKIP").is_none() {
        return;
    }
    fn handler(s: sa_script::vm::Scr, command: u32) -> Option<i8> {
        static PARAMS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog/script-params.csv"));
        let key = format!("{command:04X},");
        let row = PARAMS.lines().find(|l| l.starts_with(&key))?;
        let reads = row.split(',').nth(3)?.trim();
        // the "reads" column: `T` (an 8-byte text label), `Cn` (n parameters), in order; other forms stop
        let mut steps = Vec::new();
        for t in reads.split_whitespace() {
            if t == "T" {
                steps.push(0);
            } else {
                steps.push(t.strip_prefix('C')?.parse::<u32>().ok()?);
            }
        }
        eprintln!("script: skipped opcode {command:04X} ({reads})");
        for n in steps {
            if n == 0 {
                let mut buf = [0u8; 16];
                sa_script::cmdlib::read_text_label(s, sa_core::mem::GPtr::from_raw(buf.as_mut_ptr()), 8);
            } else {
                sa_script::cmdlib::collect(s, n);
            }
        }
        Some(0)
    }
    let f: fn(sa_script::vm::Scr, u32) -> Option<i8> = handler;
    sa_script::cmdlib::UNPORTED_OPCODE_HANDLER.store(f as usize, std::sync::atomic::Ordering::Relaxed);
    println!("sa-game: SA_GAME_SCRIPT_SKIP: unported script opcodes are skipped (exploration only)");
}
