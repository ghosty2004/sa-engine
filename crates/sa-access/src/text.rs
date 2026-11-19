//! Text entries added to the game's text (`CText::Get`'s keys): a host registers a key with its text, and the
//! game's lookups of that key find it before the GXT tables (the game's menus, help and mission texts show
//! labels, not strings). The host makes the game's lookups ask [`lookup`] (by observing `CText::Get`, 0x6A0050)
//! while [`active`] says entries exist.
//!
//! The game keeps the pointer a lookup returns (a message on screen holds it for its whole time), so a key's text
//! lives in a buffer of its own that is never freed: changing the text overwrites the buffer, removing the key
//! leaves it to the next text of the same key.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// The longest key: the game's labels are 8-byte buffers with their NUL.
pub const MAX_KEY: usize = 7;
/// The longest text, in bytes (without its NUL).
pub const MAX_TEXT: usize = 511;
/// The most keys the session's entries may use, over all hosts' callers (their buffers stay allocated).
pub const MAX_KEYS: usize = 4096;

/// A key as the game compares it: upper case (`CKeyGen::GetUppercaseKey`), NUL-padded to 8 bytes.
pub type Key = [u8; MAX_KEY + 1];

/// A key from its text: 1 ..= [`MAX_KEY`] bytes of `A-Z a-z 0-9 _`, upper-cased; `None` for anything else.
pub fn key(text: &[u8]) -> Option<Key> {
    if text.is_empty() || text.len() > MAX_KEY || !text.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_') {
        return None;
    }
    let mut k = [0u8; MAX_KEY + 1];
    for (o, c) in k.iter_mut().zip(text) {
        *o = c.to_ascii_uppercase();
    }
    Some(k)
}

struct Entry {
    /// The NUL-terminated text, at a fixed address.
    text: Box<[u8; MAX_TEXT + 1]>,
    /// Who registered the key (`None`: not registered; the buffer waits for its next text).
    owner: Option<String>,
}

thread_local! {
    static ENTRIES: RefCell<HashMap<Key, Entry>> = RefCell::new(HashMap::new());
    /// The registered keys (checked on every lookup the game makes).
    static LIVE: Cell<usize> = const { Cell::new(0) };
}

/// Why an entry was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The text is longer than [`MAX_TEXT`] or holds a NUL.
    BadText,
    /// [`MAX_KEYS`] keys are in use.
    Full,
    /// Another owner holds the key.
    Taken,
}

/// Registers `key` with `text` (the game font's code page, its `~x~` codes) for `owner`, replacing the text of a key
/// the owner holds.
pub fn set(key: Key, text: &[u8], owner: &str) -> Result<(), Refused> {
    if text.len() > MAX_TEXT || text.contains(&0) {
        return Err(Refused::BadText);
    }
    ENTRIES.with_borrow_mut(|m| {
        if !m.contains_key(&key) && m.len() >= MAX_KEYS {
            return Err(Refused::Full);
        }
        let e = m.entry(key).or_insert_with(|| Entry { text: Box::new([0; MAX_TEXT + 1]), owner: None });
        match e.owner.as_deref() {
            Some(o) if o != owner => return Err(Refused::Taken),
            Some(_) => {}
            None => {
                e.owner = Some(owner.to_string());
                LIVE.set(LIVE.get() + 1);
            }
        }
        e.text[..text.len()].copy_from_slice(text);
        e.text[text.len()] = 0;
        Ok(())
    })
}

/// Removes `key` when `owner` holds it (`None`: whoever holds it); the game's own text of that key (if any) shows
/// again.
pub fn remove(key: Key, owner: Option<&str>) -> bool {
    let removed = ENTRIES.with_borrow_mut(|m| {
        m.get_mut(&key).filter(|e| owner.is_none_or(|o| e.owner.as_deref() == Some(o))).is_some_and(|e| e.owner.take().is_some())
    });
    if removed {
        LIVE.set(LIVE.get() - 1);
    }
    removed
}

/// Whether any key is registered (the host then routes the game's lookups here).
pub fn active() -> bool {
    LIVE.get() > 0
}

/// The address of the text registered for the NUL-terminated key `name` (a key as the game passes it), if any.
pub fn lookup(name: &[u8]) -> Option<u32> {
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let k = key(&name[..end])?;
    ENTRIES.with_borrow(|m| m.get(&k).filter(|e| e.owner.is_some()).map(|e| e.text.as_ptr() as usize as u32))
}

/// [`lookup`] of the key at game address `at` (the argument of a `CText::Get` call).
pub fn lookup_at(at: u32) -> Option<u32> {
    if at == 0 || !active() {
        return None;
    }
    let mut name = [0u8; MAX_KEY + 1];
    for (i, b) in name.iter_mut().enumerate() {
        *b = sa_core::mem::GPtr::<u8>::from_addr(at as usize + i).read();
        if *b == 0 {
            break;
        }
    }
    // a longer key is none of ours
    if name[MAX_KEY] != 0 {
        return None;
    }
    lookup(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_texts_and_their_buffers() {
        assert_eq!(key(b"samp000"), Some(*b"SAMP000\0"));
        assert_eq!(key(b"TOO_LONG"), None);
        assert_eq!(key(b"A B"), None);
        let k = key(b"Mod1").unwrap();
        assert!(lookup(b"MOD1\0").is_none());
        set(k, b"First ~r~text", "a").unwrap();
        let at = lookup(b"mod1\0").unwrap();
        set(k, b"Second", "a").unwrap();
        assert_eq!(lookup(b"MOD1"), Some(at));
        assert_eq!(set(k, b"Other", "b"), Err(Refused::Taken));
        assert!(!remove(k, Some("b")));
        assert!(active());
        assert!(remove(k, Some("a")));
        assert!(!remove(k, Some("a")));
        assert!(lookup(b"MOD1").is_none() && !active());
        set(k, b"Third", "b").unwrap();
        assert_eq!(lookup(b"MOD1"), Some(at));
        assert_eq!(set(k, &[b'x'; MAX_TEXT + 1], "b"), Err(Refused::BadText));
        assert_eq!(set(k, b"a\0b", "b"), Err(Refused::BadText));
        remove(k, Some("b"));
    }
}
