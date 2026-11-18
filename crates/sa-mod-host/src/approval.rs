//! The servers the user approved (`net = ["udp:user-approved"]`): a mod with that rule may open a socket to any
//! target the user allowed for it. The first time it opens one the user has not answered for, the host asks in the
//! game (its own prompt, drawn over the mods' overlay, keys taken from the game and the mods until the answer) and
//! the open returns AGAIN until then. The answers are kept per mod id in the host's own storage area.

use std::collections::HashMap;
use std::rc::Rc;

use crate::storage::Storage;

/// The host's storage area (its keys `net.<mod id>`); no mod has this id.
pub const AREA: &str = "sa-engine-host";

/// A question waiting for the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    /// The mod's id.
    pub id: String,
    /// The mod's name, as the prompt shows it.
    pub name: String,
    /// `HOST:PORT` as the mod asked for it.
    pub target: String,
}

/// The answers so far and the open question.
#[derive(Default)]
pub struct Approvals {
    store: Option<Rc<dyn Storage>>,
    /// Per mod id: target (lower case) → allowed.
    answers: HashMap<String, HashMap<String, bool>>,
    /// The question on screen.
    pub pending: Option<Question>,
}

impl std::fmt::Debug for Approvals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Approvals").field("pending", &self.pending).finish()
    }
}

fn key(id: &str) -> String {
    format!("net.{id}")
}

impl Approvals {
    /// Answers kept in `store` (`None`: for the session only).
    pub fn new(store: Option<Rc<dyn Storage>>) -> Approvals {
        Approvals { store, ..Approvals::default() }
    }

    fn answers(&mut self, id: &str) -> &mut HashMap<String, bool> {
        let store = self.store.clone();
        self.answers.entry(id.to_string()).or_insert_with(|| {
            let text = store.and_then(|s| s.read(&key(id)).ok().flatten()).unwrap_or_default();
            String::from_utf8_lossy(&text)
                .lines()
                .filter_map(|l| match l.split_once(' ') {
                    Some(("allow", t)) => Some((t.to_ascii_lowercase(), true)),
                    Some(("deny", t)) => Some((t.to_ascii_lowercase(), false)),
                    _ => None,
                })
                .collect()
        })
    }

    /// The user's answer for mod `id` and `target`; without one, asks (unless another question is open) and
    /// returns `None`.
    pub fn check(&mut self, id: &str, name: &str, target: &str) -> Option<bool> {
        if let Some(&a) = self.answers(id).get(&target.to_ascii_lowercase()) {
            return Some(a);
        }
        if self.pending.is_none() {
            self.pending = Some(Question { id: id.into(), name: name.into(), target: target.into() });
        }
        None
    }

    /// The user answered the open question.
    pub fn answer(&mut self, allow: bool) -> Option<Question> {
        let q = self.pending.take()?;
        self.answers(&q.id).insert(q.target.to_ascii_lowercase(), allow);
        let mut list: Vec<_> = self.answers(&q.id).iter().map(|(t, a)| (t.clone(), *a)).collect();
        list.sort();
        let text: String =
            list.iter().map(|(t, a)| format!("{} {t}\n", if *a { "allow" } else { "deny" })).collect();
        if let Some(s) = &self.store {
            let _ = s.write(&key(&q.id), text.as_bytes());
        }
        Some(q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_once_and_remembers() {
        let mut a = Approvals::new(None);
        assert_eq!(a.check("samp", "SA:MP", "127.0.0.1:7777"), None);
        assert_eq!(a.check("other", "Other", "10.0.0.1:1"), None);
        assert_eq!(a.pending.as_ref().map(|q| q.id.as_str()), Some("samp"));
        assert_eq!(a.answer(true).map(|q| q.target), Some("127.0.0.1:7777".into()));
        assert_eq!(a.check("samp", "SA:MP", "127.0.0.1:7777"), Some(true));
        assert_eq!(a.check("samp", "SA:MP", "127.0.0.1:7778"), None);
        a.answer(false);
        assert_eq!(a.check("samp", "SA:MP", "127.0.0.1:7778"), Some(false));
    }
}
