//! The game's script menus (`CMenuSystem`, the panels missions show: shops, the garage, the save screen), built and
//! read through their script commands. Their texts are labels the game looks up with `CText::Get` ([`crate::text`]).
//! The handlers trust the menu number, the column and the row; the checks refuse what they would misuse
//! (`crate::checks`).

use sa_types::script::ScriptArg::{Float as F, Int as I, Local as L, Text as T};
use sa_types::{GameError, GameResult, MenuRef};

use crate::script::op;
use crate::{Game, View};

/// `CMenuSystem::MenuInUse[2]`.
const MENU_IN_USE: u32 = 0xBA82E0;
/// `CMenuSystem::num_menus_in_use`: the slot `CreateNewMenu` fills next, without a test.
const NUM_MENUS_IN_USE: u32 = 0xBA82E3;
/// The panels `CMenuSystem` holds.
pub const SLOTS: u8 = 2;
/// The columns of a standard panel.
pub const COLUMNS: u8 = 4;
/// The rows of a panel.
pub const ROWS: u8 = 12;
/// The label the menu commands take for "no text".
pub const NO_TEXT: &[u8] = b"DUMMY";

/// A menu to create (`CREATE_MENU`): its title label and place, in the game's 640 x 448 menu units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuLayout<'a> {
    /// The title's label ([`NO_TEXT`]: none).
    pub title: &'a [u8],
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Column width.
    pub width: f32,
    /// Columns (1 ..= [`COLUMNS`]).
    pub columns: u8,
    /// Rows can be selected.
    pub interactive: bool,
    /// A background box.
    pub background: bool,
    /// Text alignment (`eFontAlignment`: 0 centre, 1 left, 2 right).
    pub alignment: u8,
}

impl<V: View> Game<V> {
    /// The menu number of `m` when its slot holds a panel.
    pub(crate) fn menu_slot(&self, m: MenuRef) -> GameResult<i32> {
        if m.0 < u32::from(SLOTS) && self.view.read_u8(MENU_IN_USE + m.0) != 0 {
            Ok(m.0 as i32)
        } else {
            Err(GameError::InvalidHandle)
        }
    }

    /// Whether `CreateNewMenu` has a free slot to fill.
    pub(crate) fn menu_slot_free(&self) -> bool {
        let slot = self.view.read_u8(NUM_MENUS_IN_USE);
        slot < SLOTS && self.view.read_u8(MENU_IN_USE + u32::from(slot)) == 0
    }

    /// Creates a menu (`CREATE_MENU` 08D4: shown from the next frame, the first row selected).
    pub fn create_menu(&mut self, l: &MenuLayout<'_>) -> GameResult<MenuRef> {
        let b = |v: bool| I(i32::from(v));
        let args = [
            T(l.title),
            F(l.x),
            F(l.y),
            F(l.width),
            I(i32::from(l.columns)),
            b(l.interactive),
            b(l.background),
            I(i32::from(l.alignment)),
            L(0),
        ];
        let m = self.run_op_out(op::CREATE_MENU, &args)?;
        self.menu_slot(MenuRef(m)).map(|_| MenuRef(m)).map_err(|_| GameError::NotLoaded)
    }

    /// Sets column `column` of the menu: its header and up to [`ROWS`] row labels (`SET_MENU_COLUMN` 08DB; missing
    /// rows are empty).
    pub fn set_menu_column(&mut self, m: MenuRef, column: u8, header: &[u8], rows: &[&[u8]]) -> GameResult<()> {
        let h = self.menu_slot(m)?;
        if rows.len() > usize::from(ROWS) {
            return Err(GameError::Other(format!("{} menu rows", rows.len())));
        }
        let mut args = vec![I(h), I(i32::from(column)), T(header)];
        args.extend((0..usize::from(ROWS)).map(|i| T(rows.get(i).copied().unwrap_or(NO_TEXT))));
        self.run_op(op::SET_MENU_COLUMN, &args).map(drop)
    }

    /// Sets the width of column `column` (`SET_MENU_COLUMN_WIDTH` 09DB, in 640-wide menu units).
    pub fn set_menu_column_width(&mut self, m: MenuRef, column: u8, width: u16) -> GameResult<()> {
        let h = self.menu_slot(m)?;
        self.run_op(op::SET_MENU_COLUMN_WIDTH, &[I(h), I(i32::from(column)), I(i32::from(width))]).map(drop)
    }

    /// Lets row `row` be selected or not (`ACTIVATE_MENU_ITEM` 08D9).
    pub fn set_menu_row_enabled(&mut self, m: MenuRef, row: u8, on: bool) -> GameResult<()> {
        let h = self.menu_slot(m)?;
        self.run_op(op::ACTIVATE_MENU_ITEM, &[I(h), I(i32::from(row)), I(i32::from(on))]).map(drop)
    }

    /// The selected row (`GET_MENU_ITEM_SELECTED` 08D7).
    pub fn menu_selected_row(&mut self, m: MenuRef) -> GameResult<i32> {
        let h = self.menu_slot(m)?;
        Ok(self.run_op_out(op::GET_MENU_ITEM_SELECTED, &[I(h), L(0)])? as i32)
    }

    /// Deletes the menu (`DELETE_MENU` 08DA).
    pub fn delete_menu(&mut self, m: MenuRef) -> GameResult<()> {
        let h = self.menu_slot(m)?;
        self.run_op(op::DELETE_MENU, &[I(h)]).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{self, put};

    fn layout(columns: u8) -> MenuLayout<'static> {
        MenuLayout { title: b"SAMPHED", x: 200.0, y: 150.0, width: 150.0, columns, interactive: true, background: true, alignment: 1 }
    }

    #[test]
    fn slots_and_arguments() {
        let mut g = fake::game();
        // the fake executor runs nothing: the slot stays unused, so the menu is not there
        assert_eq!(g.create_menu(&layout(1)), Err(GameError::NotLoaded));
        put(&mut g, MENU_IN_USE, 1u8);
        put(&mut g, NUM_MENUS_IN_USE, 1u8);
        let m = MenuRef(0);
        assert!(g.create_menu(&layout(0)).is_err());
        assert!(g.set_menu_column(m, 0, b"SAMPRW1", &[b"SAMP000", b"SAMP001"]).is_ok());
        assert!(g.set_menu_column(m, 4, b"SAMPRW1", &[]).is_err());
        assert!(g.set_menu_column(m, 0, b"TOOLONG1", &[]).is_err());
        assert!(g.set_menu_row_enabled(m, 12, true).is_err());
        assert!(g.delete_menu(MenuRef(1)).is_err());
        assert!(g.delete_menu(MenuRef(0x100)).is_err());
        put(&mut g, MENU_IN_USE + 1, 1u8);
        put(&mut g, NUM_MENUS_IN_USE, 2u8);
        assert!(matches!(g.create_menu(&layout(1)), Err(GameError::Other(_))));
    }
}
