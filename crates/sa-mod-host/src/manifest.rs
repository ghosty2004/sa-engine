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
//! state = "write"            # "read": game state queries and game events (API 1.2); "write": also changes
//!                            # (API 1.3)
//! input = "read"             # key state and key events (API 1.2)
//! hooks = ["CPed::Render"]   # before / after hooks on these game functions (API 1.2); with state = "write",
//!                            # decisions on the controllable ones (API 1.3)
//! entities = true            # create / delete peds, vehicles, objects (API 1.3)
//! scripts = true             # the allowed script commands (API 1.3)
//! audio = true               # game sounds (API 1.3)
//! net = ["udp:127.0.0.1:7777", "udp:*:7777"]
//!                            # UDP sockets to these targets (API 1.4): `udp:HOST:PORT`, `*` for any host or
//!                            # port; the host compares the target text the mod opens, before resolving it
//! storage = true             # the mod's persistent key / value storage (API 1.4); `ui` is known but not granted
//! http = ["https://api.example.org/"]
//!                            # GET requests to URLs under these prefixes (API 1.7): `http://` or `https://`, a
//!                            # host and a path starting with `/`
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
    /// `state = "write"`: changes of the game state (and `state = "read"`'s queries).
    pub write: bool,
    /// `entities = true`: creating and deleting peds, vehicles and objects.
    pub entities: bool,
    /// `scripts = true`: the allowed script commands.
    pub scripts: bool,
    /// `audio = true`: game sounds.
    pub audio: bool,
    /// `net = [...]` names at least one rule ([`Manifest::net`]).
    pub net: bool,
    /// `storage = true`: the mod's persistent key / value storage.
    pub storage: bool,
    /// `http = [...]` names at least one prefix ([`Manifest::http`]).
    pub http: bool,
    /// `input = "capture"`: input events before the game, keyboard and mouse taken from the game (with `input`).
    pub capture: bool,
}

/// The `net` rule for the servers the user approves ([`crate::approval`]).
pub const USER_APPROVED: &str = "udp:user-approved";

/// One rule of `net = [...]`: `udp:HOST:PORT`, `*` for any host or any port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetRule {
    /// The host as the mod names it (compared without case), `None`: any.
    pub host: Option<String>,
    /// The port, `None`: any.
    pub port: Option<u16>,
}

impl NetRule {
    /// Parses `udp:HOST:PORT`.
    pub fn parse(rule: &str) -> Result<NetRule, String> {
        let bad = || format!("permissions.net: {rule:?}: expected \"udp:HOST:PORT\" (\"*\" for any host or port)");
        let rest = rule.strip_prefix("udp:").ok_or_else(bad)?;
        let (host, port) = rest.rsplit_once(':').ok_or_else(bad)?;
        let host = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host);
        if host.is_empty() {
            return Err(bad());
        }
        let port = match port {
            "*" => None,
            p => Some(p.parse::<u16>().ok().filter(|&p| p != 0).ok_or_else(bad)?),
        };
        Ok(NetRule { host: (host != "*").then(|| host.to_ascii_lowercase()), port })
    }

    /// Whether the rule allows `host`:`port`.
    pub fn allows(&self, host: &str, port: u16) -> bool {
        self.host.as_deref().is_none_or(|h| h.eq_ignore_ascii_case(host)) && self.port.is_none_or(|p| p == port)
    }
}

/// `HOST:PORT` or `[IPV6]:PORT` as (host, port); `None` for anything else.
pub fn split_target(target: &str) -> Option<(&str, u16)> {
    let (host, port) = target.rsplit_once(':')?;
    let host = match host.strip_prefix('[') {
        Some(h) => h.strip_suffix(']')?,
        None if host.contains(':') => return None,
        None => host,
    };
    let port = port.parse::<u16>().ok().filter(|&p| p != 0)?;
    (!host.is_empty() && !host.contains(['/', '"', '\\', ' '])).then_some((host, port))
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
    /// The targets it may open sockets to (`net = [...]`).
    pub net: Vec<NetRule>,
    /// `net = ["udp:user-approved"]`: also any target the user approves for it ([`crate::approval`]).
    pub net_user_approved: bool,
    /// The URL prefixes it may GET (`http = [...]`).
    pub http: Vec<String>,
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
    let mut net = Vec::new();
    let mut net_user_approved = false;
    let mut http = Vec::new();
    if id == crate::approval::AREA {
        return Err(format!("mod.toml: [mod] id {id:?} is the host's"));
    }
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
                    match (k.as_str(), level.as_str()) {
                        (_, "read") => *granted = true,
                        ("state", "write") => {
                            *granted = true;
                            permissions.write = true;
                        }
                        ("input", "capture") => {
                            *granted = true;
                            permissions.capture = true;
                        }
                        _ => {
                            return Err(format!(
                                "mod.toml: permissions.{k}: expected \"none\", \"read\"{}, got {level:?}",
                                if k == "state" { " or \"write\"" } else { "" }
                            ));
                        }
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
                "net" => {
                    let Value::Array(rules) = v else {
                        return Err(format!("mod.toml: permissions.net: expected a list of \"udp:HOST:PORT\", got {v:?}"));
                    };
                    for r in rules {
                        let Value::String(r) = r else {
                            return Err(format!("mod.toml: permissions.net: expected \"udp:HOST:PORT\", got {r:?}"));
                        };
                        if r == USER_APPROVED {
                            net_user_approved = true;
                            continue;
                        }
                        let rule = NetRule::parse(r).map_err(|e| format!("mod.toml: {e}"))?;
                        if rule.host.is_none() {
                            warnings.push(format!("permission net {r:?}: any host"));
                        }
                        net.push(rule);
                    }
                    permissions.net = !net.is_empty() || net_user_approved;
                }
                "http" => {
                    let bad = |v: &Value| {
                        format!("mod.toml: permissions.http: expected a list of \"https://HOST/PATH\" prefixes, got {v:?}")
                    };
                    let Value::Array(prefixes) = v else { return Err(bad(v)) };
                    for p in prefixes {
                        match p {
                            Value::String(p) if sa_mod_api::http::valid_prefix(p) => http.push(p.clone()),
                            _ => return Err(bad(p)),
                        }
                    }
                    permissions.http = !http.is_empty();
                }
                "entities" | "scripts" | "audio" | "storage" => match v {
                    Value::Boolean(b) => match k.as_str() {
                        "entities" => permissions.entities = *b,
                        "scripts" => permissions.scripts = *b,
                        "storage" => permissions.storage = *b,
                        _ => permissions.audio = *b,
                    },
                    _ => return Err(format!("mod.toml: permissions.{k}: expected true or false, got {v:?}")),
                },
                "state" | "input" | "ui" | "files" => {
                    if !nothing {
                        warnings.push(format!("permission {k} = {v:?}: not available in API 1.4, not granted"));
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
            net,
            net_user_approved,
            http,
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
             [permissions]\nrender = true\ninput = \"none\"\nui = true\n[limits]\nmemory_mib = 512\n",
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
        assert!(m.permissions.state && m.permissions.input && m.permissions.capture && m.permissions.hooks);
        assert_eq!(m.hooks, ["CPed::Render"]);
        assert!(w.is_empty(), "{w:?}");
        let (m, _) = parse("[mod]\nid = \"t\"\napi_version = \"1.2\"\n[permissions]\nstate = \"none\"\nhooks = []\n").unwrap();
        assert_eq!(m.permissions, Permissions::default());
        assert!(parse("[mod]\nid = \"t\"\napi_version = \"1.2\"\n[permissions]\nstate = true\n").is_err());
        assert!(parse("[mod]\nid = \"t\"\napi_version = \"1.2\"\n[permissions]\nhooks = [1]\n").is_err());
    }

    #[test]
    fn level_three_permissions() {
        let (m, w) = parse(
            "[mod]\nid = \"t\"\napi_version = \"1.3\"\n\
             [permissions]\nstate = \"write\"\nentities = true\nscripts = true\naudio = true\n",
        )
        .unwrap();
        let p = m.permissions;
        assert!(p.state && p.write && p.entities && p.scripts && p.audio, "{p:?}");
        assert!(w.is_empty(), "{w:?}");
        assert!(parse("[mod]\nid = \"t\"\napi_version = \"1.3\"\n[permissions]\nentities = \"yes\"\n").is_err());
        assert!(parse("[mod]\nid = \"t\"\napi_version = \"1.3\"\n[permissions]\ninput = \"write\"\n").is_err());
    }

    #[test]
    fn level_four_permissions() {
        let (m, w) = parse(
            "[mod]\nid = \"t\"\napi_version = \"1.4\"\n\
             [permissions]\nnet = [\"udp:127.0.0.1:7777\", \"udp:*:8000\", \"udp:[::1]:*\"]\nstorage = true\n",
        )
        .unwrap();
        assert!(m.permissions.net && m.permissions.storage);
        assert_eq!(w.len(), 1, "{w:?}");
        let allows = |t: &str| split_target(t).is_some_and(|(h, p)| m.net.iter().any(|r| r.allows(h, p)));
        assert!(allows("127.0.0.1:7777") && allows("example.org:8000") && allows("[::1]:9"));
        assert!(!allows("127.0.0.1:7778") && !allows("localhost:7777") && !allows("::1:9"));
        for bad in ["127.0.0.1:7777", "tcp:a:1", "udp:a", "udp::1", "udp:a:0", "udp:a:x"] {
            let t = format!("[mod]\nid = \"t\"\napi_version = \"1.4\"\n[permissions]\nnet = [\"{bad}\"]\n");
            assert!(parse(&t).is_err(), "{bad}");
        }
        assert_eq!(split_target("a.b:7777"), Some(("a.b", 7777)));
        assert_eq!(split_target("a.b"), None);
    }

    #[test]
    fn level_seven_permissions() {
        let (m, w) = parse(
            "[mod]\nid = \"t\"\napi_version = \"1.7\"\n[permissions]\nhttp = [\"https://api.open.mp/\"]\n",
        )
        .unwrap();
        assert!(m.permissions.http && w.is_empty(), "{w:?}");
        assert_eq!(m.http, ["https://api.open.mp/"]);
        for bad in ["\"https://api.open.mp\"", "\"ftp://a/\"", "1"] {
            let t = format!("[mod]\nid = \"t\"\napi_version = \"1.7\"\n[permissions]\nhttp = [{bad}]\n");
            assert!(parse(&t).is_err(), "{bad}");
        }
    }

    #[test]
    fn rejects_bad_manifests() {
        assert!(parse("[mod]\nid = \"Bad Id\"\napi_version = \"1.0\"\n").is_err());
        assert!(parse("[mod]\nid = \"a\"\napi_version = \"1\"\n").is_err());
        assert!(parse("[mod]\nid = \"a\"\napi_version = \"1.0\"\nentry = \"../x.wasm\"\n").is_err());
        assert!(parse("id = \"a\"\n").is_err());
    }
}
