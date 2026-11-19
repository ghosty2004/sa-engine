//! A ped's last damage, text entries, the game's script menus and mission trains (API 1.12).

use crate::{Error, check, ffi, ptr_len};
use sa_mod_api::sa_types::{MenuRef, PedRef, Vec3, VehicleRef};

/// A ped's last damage.
pub mod damage {
    use super::*;
    pub use sa_mod_api::sa_types::ped::{DamageSource, PedDamage};
    use sa_mod_api::state::DamageInfo;

    /// What last damaged `ped` (the weapon type and its source); `None` when the handle names no live ped.
    pub fn last(ped: PedRef) -> Option<PedDamage> {
        // SAFETY: the host writes at most `n` bytes at `p`, inside the record's buffer
        crate::state::record::<DamageInfo>(|p, n| unsafe { ffi::ped_damage(ped.0 as i32, p, n) }).map(PedDamage::from)
    }
}

/// Text entries: labels the game's text lookup finds before its own texts (permission `state = "write"`).
pub mod gxt {
    use super::*;

    /// Registers `key` (1 ..= 7 bytes of `A-Z a-z 0-9 _`) with `text` (the game font's code page, its `~x~` codes),
    /// replacing the text of a key the mod holds.
    pub fn set(key: &str, text: &[u8]) -> Result<(), Error> {
        let (k, kl) = ptr_len(key.as_bytes());
        let (t, tl) = ptr_len(text);
        // SAFETY: the host reads both ranges through bounds-checked copies
        check(unsafe { ffi::set_text_entry(k, kl, t, tl) }).map(drop)
    }

    /// Removes the mod's entry `key`.
    pub fn remove(key: &str) -> Result<(), Error> {
        let (k, kl) = ptr_len(key.as_bytes());
        // SAFETY: as in `set`
        check(unsafe { ffi::remove_text_entry(k, kl) }).map(drop)
    }
}

/// The game's script menus (permission `entities = true`): their texts are labels ([`gxt`]).
pub mod menus {
    use super::*;
    pub use sa_mod_api::menu::{ALIGN_CENTRE, ALIGN_LEFT, ALIGN_RIGHT, BACKGROUND, INTERACTIVE, NO_TEXT};

    // SAFETY (every function below): a host function call with plain values, or a range the host reads through
    // bounds-checked copies.

    /// Creates a menu with the title label `title` at (`x`, `y`), column width `width` (640 x 448 menu units),
    /// `columns` columns, `sa_mod_api::menu` `flags` and alignment `align`; shown from the next frame.
    #[allow(clippy::too_many_arguments)]
    pub fn create(title: &str, x: f32, y: f32, width: f32, columns: u8, flags: i32, align: i32) -> Result<MenuRef, Error> {
        let (t, tl) = ptr_len(title.as_bytes());
        check(unsafe { ffi::create_menu(t, tl, x, y, width, i32::from(columns), flags, align) }).map(|h| MenuRef(h as u32))
    }
    /// Deletes a menu the mod created.
    pub fn destroy(m: MenuRef) -> Result<(), Error> {
        check(unsafe { ffi::destroy_menu(m.0 as i32) }).map(drop)
    }
    /// Sets column `column`: its header label and up to 12 row labels.
    pub fn set_column(m: MenuRef, column: u8, header: &str, rows: &[&str]) -> Result<(), Error> {
        let mut labels = Vec::new();
        for l in std::iter::once(header).chain(rows.iter().copied()) {
            labels.extend_from_slice(l.as_bytes());
            labels.push(0);
        }
        let (p, n) = ptr_len(&labels);
        check(unsafe { ffi::set_menu_column(m.0 as i32, i32::from(column), p, n) }).map(drop)
    }
    /// Sets the width of column `column` (640-wide menu units).
    pub fn set_column_width(m: MenuRef, column: u8, width: u16) -> Result<(), Error> {
        check(unsafe { ffi::set_menu_column_width(m.0 as i32, i32::from(column), i32::from(width)) }).map(drop)
    }
    /// Lets row `row` be selected or not.
    pub fn set_row_enabled(m: MenuRef, row: u8, on: bool) -> Result<(), Error> {
        check(unsafe { ffi::set_menu_row_enabled(m.0 as i32, i32::from(row), i32::from(on)) }).map(drop)
    }
    /// The selected row.
    pub fn selected_row(m: MenuRef) -> Result<i32, Error> {
        check(unsafe { ffi::menu_selected_row(m.0 as i32) })
    }
}

/// Mission trains (permission `entities = true`).
pub mod trains {
    use super::*;

    /// Creates a train of type `kind` on the track nearest to `pos`, running clockwise or not; returns its first
    /// carriage.
    pub fn create(kind: i32, pos: Vec3, clockwise: bool) -> Result<VehicleRef, Error> {
        // SAFETY: a host function call with plain values
        check(unsafe { ffi::create_train(kind, pos.x, pos.y, pos.z, i32::from(clockwise)) }).map(|h| VehicleRef(h as u32))
    }
    /// Deletes a train the mod created, every carriage.
    pub fn destroy(first: VehicleRef) -> Result<(), Error> {
        // SAFETY: as in `create`
        check(unsafe { ffi::destroy_train(first.0 as i32) }).map(drop)
    }
    /// The carriages of the train whose first carriage is `first`, `first` first.
    pub fn carriages(first: VehicleRef) -> Result<Vec<VehicleRef>, Error> {
        let mut buf = [0u8; 4 * sa_mod_api::train::MAX_CARRIAGES];
        let (p, n) = (buf.as_mut_ptr() as usize as i32, buf.len() as i32);
        // SAFETY: the host writes at most `n` bytes at `p`, inside the buffer
        let count = check(unsafe { ffi::train_carriages(first.0 as i32, p, n) })? as usize;
        Ok(buf.as_chunks::<4>().0.iter().take(count).map(|c| VehicleRef(u32::from_le_bytes(*c))).collect())
    }
}
