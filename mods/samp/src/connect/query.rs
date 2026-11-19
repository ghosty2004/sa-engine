//! Live server details over SA:MP's query protocol ([`samp_proto::query`]): information and ping for the servers on
//! screen, rules and players for the selected one, refreshed every few seconds.
//!
//! The manifest's `net = ["udp:user-approved"]` makes the host ask the user before the mod reaches a server for the
//! first time. So only servers the user already allowed are queried by themselves (the mod keeps them in its storage,
//! `query_allowed`); any other server is queried only when the user asks for it ([`Querier::ask`]).

use std::collections::{BTreeMap, BTreeSet};

use sa_mod::Error;
use sa_mod::net::Socket;
use samp_proto::query::{Info, Reply, address_bytes, op, parse_reply, request};

/// Seconds between two queries of a server.
const REFRESH_US: u64 = 5_000_000;
/// How long a query waits for its answers.
const WAIT_US: u64 = 2_500_000;
/// Sockets the queries hold at once (the mod has 8; the client needs one).
const MAX_ACTIVE: usize = 5;
/// The storage key of the servers the user allowed.
const ALLOWED_KEY: &str = "query_allowed";

/// Whether the mod may reach a server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// The user has not been asked.
    Unknown,
    /// The host is asking the user.
    Asking,
    Allowed,
    Denied,
}

/// What the queries learnt about a server.
#[derive(Clone, Debug, Default)]
pub struct Live {
    pub info: Option<Info>,
    /// Round-trip time in milliseconds.
    pub ping: Option<u32>,
    pub rules: Vec<(Vec<u8>, Vec<u8>)>,
    pub clients: Option<Vec<(Vec<u8>, i32)>>,
    /// The last query got no answer.
    pub silent: bool,
    /// A query is running.
    pub busy: bool,
    /// When the last query started (host microseconds).
    started: Option<u64>,
}

struct Active {
    target: String,
    socket: Socket,
    started: u64,
    payload: [u8; 4],
    details: bool,
    sent: bool,
    /// The answers so far: information, ping, rules, players.
    got: [bool; 4],
}

/// The queries.
#[derive(Default)]
pub struct Querier {
    live: BTreeMap<String, Live>,
    active: Vec<Active>,
    allowed: BTreeSet<String>,
    denied: BTreeSet<String>,
    asking: Option<String>,
    counter: u32,
}

fn now() -> u64 {
    sa_mod::time::now().monotonic_us
}

impl Querier {
    /// The queries with the servers the user allowed before.
    pub fn load() -> Querier {
        let allowed = crate::setting(ALLOWED_KEY)
            .map(|t| t.lines().map(|l| l.trim().to_string()).collect())
            .unwrap_or_default();
        Querier {
            allowed,
            ..Querier::default()
        }
    }

    pub fn live(&self, target: &str) -> Option<&Live> {
        self.live.get(target)
    }

    pub fn access(&self, target: &str) -> Access {
        if self.allowed.contains(target) {
            Access::Allowed
        } else if self.denied.contains(target) {
            Access::Denied
        } else if self.asking.as_deref() == Some(target) {
            Access::Asking
        } else {
            Access::Unknown
        }
    }

    /// The user asked for `target`'s details: the host asks the user once, if it has not.
    pub fn ask(&mut self, target: &str) {
        if self.access(target) == Access::Unknown && self.asking.is_none() {
            self.asking = Some(target.to_string());
        }
    }

    /// The user connected to `target` through the host's own question: it is allowed from now on.
    pub fn allow(&mut self, target: &str) {
        self.denied.remove(target);
        if self.allowed.insert(target.to_string()) {
            let list: Vec<&str> = self.allowed.iter().map(String::as_str).collect();
            let _ = sa_mod::storage::write(ALLOWED_KEY, list.join("\n").as_bytes());
        }
    }

    /// Queries the allowed servers of `targets` again at once.
    pub fn refresh(&mut self, targets: &[String]) {
        for t in targets {
            if let Some(l) = self.live.get_mut(t) {
                l.started = None;
            }
        }
    }

    /// Stops every query (the client needs the sockets).
    pub fn stop(&mut self) {
        self.active.clear();
        self.asking = None;
        for l in self.live.values_mut() {
            l.busy = false;
        }
    }

    fn open(&mut self, target: &str, details: bool) -> Result<(), Error> {
        let socket = Socket::open(target)?;
        self.counter = self.counter.wrapping_add(1);
        let started = now();
        let l = self.live.entry(target.to_string()).or_default();
        l.started = Some(started);
        l.busy = true;
        self.active.push(Active {
            target: target.to_string(),
            socket,
            started,
            payload: self.counter.to_le_bytes(),
            details,
            sent: false,
            got: [false; 4],
        });
        Ok(())
    }

    /// One frame: answers the pending question, starts the queries that are due for `targets` (the servers on
    /// screen; `selected` also gets rules and players), reads the answers.
    pub fn frame(&mut self, targets: &[String], selected: Option<&str>) {
        if let Some(t) = self.asking.clone() {
            match self.open(&t, selected == Some(t.as_str())) {
                Ok(()) => {
                    self.asking = None;
                    self.allow(&t);
                }
                Err(Error::Again) => {}
                Err(_) => {
                    self.asking = None;
                    self.denied.insert(t);
                }
            }
        }
        let t0 = now();
        let due = |l: Option<&Live>| {
            l.and_then(|l| l.started)
                .is_none_or(|s| t0.saturating_sub(s) >= REFRESH_US)
        };
        let mut order: Vec<&String> = targets.iter().collect();
        // the selected server first: its details matter most
        order.sort_by_key(|t| Some(t.as_str()) != selected);
        let wanted: Vec<String> = order
            .into_iter()
            .filter(|t| {
                self.allowed.contains(*t)
                    && due(self.live.get(*t))
                    && !self.active.iter().any(|a| &a.target == *t)
            })
            .cloned()
            .collect();
        for t in wanted {
            if self.active.len() >= MAX_ACTIVE {
                break;
            }
            // `Again` here: the host forgot the answer and asks again; the next frame retries
            let details = selected == Some(t.as_str());
            if self.open(&t, details).is_err() {
                break;
            }
        }
        self.poll(t0);
    }

    fn poll(&mut self, t: u64) {
        let mut buf = [0u8; 4096];
        let mut done = Vec::new();
        for (i, a) in self.active.iter_mut().enumerate() {
            if !a.sent {
                let (host, port) = a.target.rsplit_once(':').unwrap_or((&a.target, "0"));
                let (addr, port) = (address_bytes(host), port.parse().unwrap_or(0));
                let mut ops = vec![op::INFO, op::PING];
                if a.details {
                    ops.extend([op::RULES, op::CLIENTS]);
                }
                a.sent = ops
                    .iter()
                    .all(|&o| a.socket.send(&request(addr, port, o, a.payload)).is_ok());
                a.started = now();
            }
            let live = self.live.entry(a.target.clone()).or_default();
            while let Ok(n) = a.socket.recv(&mut buf) {
                match parse_reply(&buf[..n]) {
                    Some(Reply::Info(info)) => {
                        live.info = Some(info);
                        a.got[0] = true;
                    }
                    Some(Reply::Ping(p)) if p == a.payload => {
                        live.ping = Some((now().saturating_sub(a.started) / 1000) as u32);
                        a.got[1] = true;
                    }
                    Some(Reply::Rules(r)) => {
                        live.rules = r;
                        a.got[2] = true;
                    }
                    Some(Reply::Clients(c)) => {
                        live.clients = Some(c);
                        a.got[3] = true;
                    }
                    _ => {}
                }
            }
            let complete = a.got[0] && a.got[1] && (!a.details || (a.got[2] && a.got[3]));
            if complete || t.saturating_sub(a.started) > WAIT_US {
                live.silent = !a.got[0];
                live.busy = false;
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.active.remove(i);
        }
    }
}
