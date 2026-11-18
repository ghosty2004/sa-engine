//! `mod.toml` (ADR-004 §3.3): identity, API version, entry file, permissions and limits.
//!
//! ```toml
//! [mod]
//! id = "hello"               # [a-z0-9_-], unique
//! name = "Hello overlay"
//! version = "0.1.0"
//! api_version = "1.0"        # the sa-mod-api version the mod was built for
//! entry = "hello.wasm"       # optional, default "<id>.wasm", next to mod.toml
//!
//! [permissions]
//! render = true              # overlays, display modes, the video mode
//! files = "own"              # the mod's own files (API 1.1)
//! state = "read"             # game state queries and game events (API 1.2)
//! input = "read"             # key state and key events (API 1.2)
//! hooks = ["CPed::Render"]   # before / after hooks on these game functions (API 1.2); the other keys of
//!                            # ADR-004 §3.3 are known but not granted yet
//!
//! [limits]                   # optional; clamped to the host's maxima
//! memory_mib = 16            # linear memory cap
//! fuel = 10000000            # per call (~ wasm instructions)
//! ```

use toml::{Table, Value};

/// What the host grants a mod.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Permissions {
    /// `render = true`: the overlay, display and video-mode functions.
    pub render: bool,
    /// `files = "own"`: the mod's own files.
    pub files: bool,
    /// `state = "read"`: game state queries and game events.
    pub state: bool,
    /// `input = "read"`: key state and key events.
    pub input: bool,
    /// `hooks = [...]` names at least one function ([`Manifest::hooks`]).
    pub hooks: bool,
}

/// A mod's budgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Linear memory cap in bytes.
    pub memory_bytes: u64,
    /// Fuel per call (`sa_init` gets [`INIT_FUEL_FACTOR`] times this).
    pub fuel: u64,
}

/// `sa_init`'s budget relative to a frame call's.
pub const INIT_FUEL_FACTOR: u64 = 10;
/// Defaults and host maxima.
pub const DEFAULT_MEMORY_MIB: u64 = 16;
/// The host's memory cap per mod (the i686 build shares a 4 GiB address space with the game's arena).
pub const MAX_MEMORY_MIB: u64 = 64;
/// Default fuel per call.
pub const DEFAULT_FUEL: u64 = 10_000_000;
/// The host's fuel cap per call (the web backend counts fuel in an `i32`).
pub const MAX_FUEL: u64 = 200_000_000;

impl Default for Limits {
    fn default() -> Limits {
        Limits { memory_bytes: DEFAULT_MEMORY_MIB << 20, fuel: DEFAULT_FUEL }
    }
}

/// A parsed manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// Unique id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// The mod's own version.
    pub version: String,
    /// The API version it was built for.
    pub api: (u16, u16),
    /// The `.wasm` file next to the manifest.
    pub entry: String,
    /// What it asked for and was granted.
    pub permissions: Permissions,
    /// The game functions it may hook (`hooks = [...]`), by their names in the hook registry.
    pub hooks: Vec<String>,
    /// Its budgets.
    pub limits: Limits,
}

fn str_field(t: &Table, k: &str) -> Result<Option<String>, String> {
    match t.get(k) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(v) => Err(format!("{k}: expected a string, got {}", v.type_str())),
    }
}

fn int_field(t: &Table, k: &str) -> Result<Option<u64>, String> {
    match t.get(k) {
        None => Ok(None),
        Some(Value::Integer(i)) if *i > 0 => Ok(Some(*i as u64)),
        Some(v) => Err(format!("{k}: expected a positive integer, got {v:?}")),
    }
}

/// `"MAJOR.MINOR"`.
fn parse_version(s: &str) -> Option<(u16, u16)> {
    let (a, b) = s.split_once('.')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// Parses `mod.toml`. Returns the manifest and warnings (unknown keys, permissions this host does not grant).
pub fn parse(text: &str) -> Result<(Manifest, Vec<String>), String> {
    let root: Table = text.parse().map_err(|e| format!("mod.toml: {e}"))?;
    let mut warnings = Vec::new();
    let m = match root.get("mod") {
        Some(Value::Table(t)) => t,
        _ => return Err("mod.toml: no [mod] table".into()),
    };
    let id = str_field(m, "id")?.ok_or("mod.toml: [mod] id is missing")?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_') {
        return Err(format!("mod.toml: id {id:?}: use a-z, 0-9, '-' and '_'"));
    }
    let api_s = str_field(m, "api_version")?.ok_or("mod.toml: [mod] api_version is missing")?;
    let api = parse_version(&api_s).ok_or(format!("mod.toml: api_version {api_s:?}: expected \"MAJOR.MINOR\""))?;
    let entry = str_field(m, "entry")?.unwrap_or(format!("{id}.wasm"));
    if entry.contains(['/', '\\']) || entry.starts_with('.') || !entry.ends_with(".wasm") {
        return Err(format!("mod.toml: entry {entry:?}: a .wasm file name next to mod.toml"));
    }
    for k in m.keys() {
        if !["id", "name", "version", "api_version", "entry"].contains(&k.as_str()) {
            warnings.push(format!("[mod] {k}: unknown key, ignored"));
        }
    }
    let mut permissions = Permissions::default();
    let mut hooks = Vec::new();
    if let Some(p) = root.get("permissions") {
        let Value::Table(p) = p else { return Err("mod.toml: [permissions] is not a table".into()) };
        for (k, v) in p {
            // what asks for nothing: false, "none", []
            let nothing = matches!(v, Value::Boolean(false))
                || matches!(v, Value::String(s) if s == "none")
                || matches!(v, Value::Array(a) if a.is_empty());
            match k.as_str() {
                "render" => match v {
                    Value::Boolean(b) => permissions.render = *b,
                    _ => return Err(format!("mod.toml: permissions.render: expected true or false, got {v:?}")),
                },
                "files" if matches!(v, Value::String(s) if s == "own") => permissions.files = true,
                "state" | "input" if !nothing => {
                    let Value::String(level) = v else {
                        return Err(format!("mod.toml: permissions.{k}: expected a string, got {v:?}"));
                    };
                    let granted = if k == "state" { &mut permissions.state } else { &mut permissions.input };
                    match level.as_str() {
                        "read" => *granted = true,
                        "write" | "capture" => {
                            *granted = true;
                            warnings.push(format!("permission {k} = {level:?}: only \"read\" is available in API 1.2"));
                        }
                        _ => return Err(format!("mod.toml: permissions.{k}: expected \"none\" or \"read\", got {level:?}")),
                    }
                }
                "hooks" => {
                    let Value::Array(names) = v else {
                        return Err(format!("mod.toml: permissions.hooks: expected a list of function names, got {v:?}"));
                    };
                    for n in names {
                        let Value::String(n) = n else {
                            return Err(format!("mod.toml: permissions.hooks: expected function names, got {n:?}"));
                        };
                        hooks.push(n.clone());
                    }
                    permissions.hooks = !hooks.is_empty();
                }
                "state" | "input" | "ui" | "entities" | "net" | "files" => {
                    if !nothing {
                        warnings.push(format!("permission {k} = {v:?}: not available in API 1.2, not granted"));
                    }
                }
                _ => warnings.push(format!("[permissions] {k}: unknown permission, ignored")),
            }
        }
    }
    let mut limits = Limits::default();
    if let Some(l) = root.get("limits") {
        let Value::Table(l) = l else { return Err("mod.toml: [limits] is not a table".into()) };
        if let Some(mib) = int_field(l, "memory_mib")? {
            if mib > MAX_MEMORY_MIB {
                warnings.push(format!("limits.memory_mib = {mib}: capped at {MAX_MEMORY_MIB}"));
            }
            limits.memory_bytes = mib.min(MAX_MEMORY_MIB) << 20;
        }
        if let Some(f) = int_field(l, "fuel")? {
            if f > MAX_FUEL {
                warnings.push(format!("limits.fuel = {f}: capped at {MAX_FUEL}"));
            }
            limits.fuel = f.min(MAX_FUEL);
        }
        for k in l.keys() {
            if !["memory_mib", "fuel"].contains(&k.as_str()) {
                warnings.push(format!("[limits] {k}: unknown key, ignored"));
            }
        }
    }
    for k in root.keys() {
        if !["mod", "permissions", "limits"].contains(&k.as_str()) {
            warnings.push(format!("[{k}]: unknown table, ignored"));
        }
    }
    Ok((
        Manifest {
            name: str_field(m, "name")?.unwrap_or(id.clone()),
            version: str_field(m, "version")?.unwrap_or_default(),
            id,
            api,
            entry,
            permissions,
            hooks,
            limits,
        },
        warnings,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_example() {
        let (m, w) = parse(
            "[mod]\nid = \"hello\"\nname = \"Hello\"\nversion = \"0.1.0\"\napi_version = \"1.0\"\n\
             [permissions]\nrender = true\ninput = \"none\"\nnet = [\"1.2.3.4:7777\"]\n[limits]\nmemory_mib = 512\n",
        )
        .unwrap();
        assert_eq!(m.id, "hello");
        assert_eq!(m.entry, "hello.wasm");
        assert_eq!(m.api, (1, 0));
        assert!(m.permissions.render);
        assert_eq!(m.limits.memory_bytes, MAX_MEMORY_MIB << 20);
        assert_eq!(w.len(), 2, "{w:?}");
    }

    #[test]
    fn level_two_permissions() {
        let (m, w) = parse(
            "[mod]\nid = \"t\"\napi_version = \"1.2\"\n\
             [permissions]\nstate = \"read\"\ninput = \"capture\"\nhooks = [\"CPed::Render\"]\n",
        )
        .unwrap();
        assert!(m.permissions.state && m.permissions.input && m.permissions.hooks);
        assert_eq!(m.hooks, ["CPed::Render"]);
        assert_eq!(w.len(), 1, "{w:?}");
        let (m, _) = parse("[mod]\nid = \"t\"\napi_version = \"1.2\"\n[permissions]\nstate = \"none\"\nhooks = []\n").unwrap();
        assert_eq!(m.permissions, Permissions::default());
        assert!(parse("[mod]\nid = \"t\"\napi_version = \"1.2\"\n[permissions]\nstate = true\n").is_err());
        assert!(parse("[mod]\nid = \"t\"\napi_version = \"1.2\"\n[permissions]\nhooks = [1]\n").is_err());
    }

    #[test]
    fn rejects_bad_manifests() {
        assert!(parse("[mod]\nid = \"Bad Id\"\napi_version = \"1.0\"\n").is_err());
        assert!(parse("[mod]\nid = \"a\"\napi_version = \"1\"\n").is_err());
        assert!(parse("[mod]\nid = \"a\"\napi_version = \"1.0\"\nentry = \"../x.wasm\"\n").is_err());
        assert!(parse("id = \"a\"\n").is_err());
    }
}
