//! The development environment variables (`SA_GAME_STUB`, `SA_GAME_WALK`, ...): the process environment natively;
//! on wasm32 a table the page fills from its URL query (`?SA_GAME_STUB=...`).

#[cfg(target_arch = "wasm32")]
static VARS: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

/// The value of `name`, as `std::env::var` gives it.
pub fn var(name: &str) -> Result<String, std::env::VarError> {
    #[cfg(target_arch = "wasm32")]
    {
        VARS.lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .ok_or(std::env::VarError::NotPresent)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(name)
    }
}

/// wasm32: sets `name` (the page's URL query).
#[cfg(target_arch = "wasm32")]
pub fn set(name: &str, value: &str) {
    let mut v = VARS.lock().unwrap_or_else(|e| e.into_inner());
    v.retain(|(k, _)| k != name);
    v.push((name.to_owned(), value.to_owned()));
}

/// `std::env::var_os`, through [`var`].
pub fn var_os(name: &str) -> Option<String> {
    var(name).ok()
}
