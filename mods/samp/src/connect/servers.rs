//! The servers the screen lists: the Internet list (a JSON array in the open.mp server list's format, fetched over
//! HTTP), the user's favourites and recent servers, their filters and orders.

use super::paint::latin1;
use samp_proto::query::Info;

/// The open.mp server list.
pub const DEFAULT_LIST_URL: &str = "https://api.open.mp/servers/";
/// Recent servers kept.
pub const MAX_RECENT: usize = 20;

/// A server as a list shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Server {
    /// `HOST:PORT`.
    pub address: String,
    /// The server's name, in the game's code page.
    pub hostname: Vec<u8>,
    pub players: u16,
    pub max_players: u16,
    pub gamemode: Vec<u8>,
    pub language: Vec<u8>,
    pub password: bool,
    /// The server's version (`vn`), when the list has it.
    pub version: Vec<u8>,
    /// An open.mp server.
    pub omp: bool,
}

impl Server {
    /// A server known only by its address.
    pub fn unknown(address: &str) -> Server {
        Server {
            address: address.to_string(),
            ..Server::default()
        }
    }

    /// Takes over what a query answered.
    pub fn apply(&mut self, i: &Info) {
        self.hostname.clone_from(&i.hostname);
        self.players = i.players;
        self.max_players = i.max_players;
        self.gamemode.clone_from(&i.gamemode);
        self.language.clone_from(&i.language);
        self.password = i.password;
    }

    /// Whether the list knows more than the address.
    pub fn known(&self) -> bool {
        !self.hostname.is_empty()
    }
}

/// Parses the server list: a JSON array of objects with `ip` (`HOST:PORT`), `hn` (hostname), `pc` / `pm` (players,
/// slots), `gm` (gamemode), `la` (language), `pa` (password), `vn` (version), `omp`. Rows without a valid address are
/// skipped.
pub fn parse_list(json: &[u8]) -> Result<Vec<Server>, String> {
    let v: serde_json::Value = serde_json::from_slice(json).map_err(|e| format!("the server list: {e}"))?;
    let rows = v.as_array().ok_or("the server list is not a JSON array")?;
    let text =
        |r: &serde_json::Value, k: &str| r.get(k).and_then(|v| v.as_str()).map(latin1).unwrap_or_default();
    let num = |r: &serde_json::Value, k: &str| {
        r.get(k)
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(u16::MAX.into()) as u16
    };
    let flag = |r: &serde_json::Value, k: &str| r.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    Ok(rows
        .iter()
        .filter_map(|r| {
            let address = r.get("ip")?.as_str()?.trim().to_string();
            valid_server(&address).then(|| Server {
                hostname: text(r, "hn"),
                players: num(r, "pc"),
                max_players: num(r, "pm"),
                gamemode: text(r, "gm"),
                language: text(r, "la"),
                password: flag(r, "pa"),
                version: text(r, "vn"),
                omp: flag(r, "omp"),
                address,
            })
        })
        .collect())
}

/// Whether `s` is `HOST:PORT`.
pub fn valid_server(s: &str) -> bool {
    s.rsplit_once(':')
        .is_some_and(|(h, p)| !h.is_empty() && !h.contains(' ') && p.parse::<u16>().is_ok_and(|p| p != 0))
}

/// The list orders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Players,
    Ping,
    Name,
}

impl Sort {
    pub fn next(self) -> Sort {
        match self {
            Sort::Players => Sort::Ping,
            Sort::Ping => Sort::Name,
            Sort::Name => Sort::Players,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Sort::Players => "Players",
            Sort::Ping => "Ping",
            Sort::Name => "Name",
        }
    }

    /// The label where room is short.
    pub fn short_label(self) -> &'static str {
        match self {
            Sort::Players => "By players",
            Sort::Ping => "By ping",
            Sort::Name => "By name",
        }
    }
}

/// What the Internet list hides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    pub hide_full: bool,
    pub hide_empty: bool,
    pub hide_locked: bool,
}

/// Whether `haystack` contains `needle` (both Latin-1, `needle` lower case), case-insensitively.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || haystack
            .windows(needle.len())
            .any(|w| w.iter().zip(needle).all(|(a, b)| a.to_ascii_lowercase() == *b))
}

/// The indices of `servers` that pass `filters` and `search` (hostname, gamemode, address), in `sort` order;
/// `ping` gives a server's ping when it was queried.
pub fn view(
    servers: &[Server],
    filters: Filters,
    search: &str,
    sort: Sort,
    ping: impl Fn(&str) -> Option<u32>,
) -> Vec<usize> {
    let needle = latin1(&search.trim().to_ascii_lowercase());
    let mut v: Vec<usize> = (0..servers.len())
        .filter(|&i| {
            let s = &servers[i];
            !(filters.hide_full && s.max_players > 0 && s.players >= s.max_players)
                && !(filters.hide_empty && s.players == 0)
                && !(filters.hide_locked && s.password)
                && (contains(&s.hostname, &needle)
                    || contains(&s.gamemode, &needle)
                    || contains(s.address.as_bytes(), &needle))
        })
        .collect();
    let name = |i: usize| servers[i].hostname.to_ascii_lowercase();
    match sort {
        Sort::Players => v.sort_by(|&a, &b| {
            servers[b]
                .players
                .cmp(&servers[a].players)
                .then_with(|| name(a).cmp(&name(b)))
        }),
        Sort::Ping => v.sort_by_key(|&i| {
            (
                ping(&servers[i].address).unwrap_or(u32::MAX),
                u16::MAX - servers[i].players,
            )
        }),
        Sort::Name => v.sort_by_cached_key(|&i| name(i)),
    }
    v
}

/// A list of addresses as kept in storage (one `HOST:PORT` per line).
pub fn parse_addresses(text: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for l in text.lines().map(str::trim).filter(|l| valid_server(l)) {
        if !v.iter().any(|x| x == l) {
            v.push(l.to_string());
        }
    }
    v
}

/// `recent` with `server` moved to the front, at most [`MAX_RECENT`].
pub fn remember(recent: &mut Vec<String>, server: &str) {
    recent.retain(|s| s != server);
    recent.insert(0, server.to_string());
    recent.truncate(MAX_RECENT);
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"[
        {"ip":"127.0.0.1:7813","hn":"Local test server","pc":0,"pm":50,"gm":"sampcov","la":"English","pa":false,"vn":"omp 1.6.7","omp":true,"pr":false},
        {"ip":"10.0.0.2:7777","hn":"Freeroam Paradise","pc":120,"pm":500,"gm":"Freeroam","la":"EN","pa":false,"vn":"0.3.7-R2","omp":false},
        {"ip":"10.0.0.3:7777","hn":"Locked RP","pc":40,"pm":40,"gm":"Roleplay v2","la":"RO","pa":true},
        {"ip":"bad","hn":"no port"},
        {"hn":"no address"}
    ]"#;

    #[test]
    fn parses_the_list() {
        let s = parse_list(LIST.as_bytes()).unwrap();
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].hostname, b"Local test server");
        assert!(s[0].omp && !s[0].password && s[2].password);
        assert_eq!((s[1].players, s[1].max_players), (120, 500));
        assert!(parse_list(b"{}").is_err() && parse_list(b"[").is_err());
    }

    #[test]
    fn filters_and_orders() {
        let s = parse_list(LIST.as_bytes()).unwrap();
        let none = |_: &str| None;
        assert_eq!(view(&s, Filters::default(), "", Sort::Players, none), [1, 2, 0]);
        assert_eq!(view(&s, Filters::default(), "", Sort::Name, none), [1, 0, 2]);
        let f = Filters {
            hide_full: true,
            hide_empty: true,
            hide_locked: false,
        };
        assert_eq!(view(&s, f, "", Sort::Players, none), [1]);
        assert_eq!(view(&s, Filters::default(), "ROLEPLAY", Sort::Players, none), [2]);
        assert_eq!(view(&s, Filters::default(), "127.0", Sort::Players, none), [0]);
        let ping = |a: &str| (a == "127.0.0.1:7813").then_some(3);
        assert_eq!(view(&s, Filters::default(), "", Sort::Ping, ping)[0], 0);
    }

    #[test]
    fn recent_servers() {
        let mut r = parse_addresses("a:1\nb:2\nbad\na:1\n");
        assert_eq!(r, ["a:1", "b:2"]);
        remember(&mut r, "b:2");
        assert_eq!(r, ["b:2", "a:1"]);
        for i in 0..30 {
            remember(&mut r, &format!("h:{}", i + 1));
        }
        assert_eq!(r.len(), MAX_RECENT);
    }
}
