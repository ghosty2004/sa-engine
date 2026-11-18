//! The mods' persistent key / value storage: one area per mod id, natively a folder under the user's data
//! directory, in the browser the page's IndexedDB (`backend`). Keys are checked (`sa_mod_api::storage::valid_key`)
//! and quotas enforced by the host before a backend sees a call.

use std::rc::Rc;

/// One mod's storage area.
pub trait Storage {
    /// The value of `key`, `None` when there is none.
    fn read(&self, key: &str) -> Result<Option<Vec<u8>>, String>;
    /// Writes (replaces) the value of `key`.
    fn write(&self, key: &str, data: &[u8]) -> Result<(), String>;
    /// Deletes `key`; false when there was none.
    fn delete(&self, key: &str) -> Result<bool, String>;
    /// Every key with its value's size, sorted by key.
    fn list(&self) -> Result<Vec<(String, usize)>, String>;
    /// Where it is, for the log.
    fn location(&self) -> String;
}

/// Opens the storage area of a mod.
pub trait StorageProvider {
    /// The area of mod `id` (created when needed).
    fn open(&self, id: &str) -> Result<Rc<dyn Storage>, String>;
}

/// A mod's storage area, as the host holds it.
#[derive(Clone)]
pub struct Store(pub Rc<dyn Storage>);

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Store({})", self.0.location())
    }
}

/// A storage area in memory, for tests.
#[cfg(test)]
#[derive(Default)]
pub struct MemoryStorage(std::cell::RefCell<std::collections::BTreeMap<String, Vec<u8>>>);

#[cfg(test)]
impl Storage for MemoryStorage {
    fn read(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(self.0.borrow().get(key).cloned())
    }

    fn write(&self, key: &str, data: &[u8]) -> Result<(), String> {
        self.0.borrow_mut().insert(key.to_string(), data.to_vec());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<bool, String> {
        Ok(self.0.borrow_mut().remove(key).is_some())
    }

    fn list(&self) -> Result<Vec<(String, usize)>, String> {
        Ok(self.0.borrow().iter().map(|(k, v)| (k.clone(), v.len())).collect())
    }

    fn location(&self) -> String {
        "memory".into()
    }
}
