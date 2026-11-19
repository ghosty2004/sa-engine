//! The server's menus on GTA's script menus, for every host: the UI core's show / hide requests (RPC 77 / 78) and
//! samp's per-frame selection poll (RPC 132 / 140) at the frame entry (docs/re/subsystems/ui-chat-audio.md, "76
//! PlayerInitMenu" to "132 OnPlayerSelectedMenuRow"). The host builds and reads the GTA menu ([`MenuGame`]).

use sa_samp_api::hud::MenuSpec;
use sa_samp_api::{GameAdapter, MenuRef, Point2};
use samp_ui::menu::{Menu, MenuBuild, NO_ROW, PollAction};

use super::core::{MenuRequest, Outgoing, UiCore};

/// The GTA menu as a host builds and reads it.
pub trait MenuGame {
    /// Builds the GTA menu of `build` (the script commands of samp.dll 0x100A7780), its texts `menu`'s; its handle.
    fn build_menu(&mut self, build: &MenuBuild, menu: Option<&Menu>) -> Option<i32>;
    /// `DELETE_MENU`.
    fn delete_menu(&mut self, handle: i32);
    /// `GET_MENU_ITEM_SELECTED`.
    fn selected_row(&mut self, handle: i32) -> u8;
}

/// The GTA menu of the current server menu and the pad buttons of the last frame.
#[derive(Debug, Default)]
pub struct MenuWork {
    handle: Option<i32>,
    pad: (bool, bool),
}

impl MenuWork {
    /// The frame entry's menu step: the requests, then the poll with pad 0's cross and triangle this frame.
    /// Mirrors samp.dll 0x10008200.
    pub fn frame(&mut self, core: &mut UiCore, game: &mut dyn MenuGame, (cross, triangle): (bool, bool)) {
        for req in std::mem::take(&mut core.requests.menus) {
            match req {
                MenuRequest::Show { delete_previous, build } => {
                    if delete_previous && let Some(h) = self.handle.take() {
                        game.delete_menu(h);
                    }
                    self.handle = game.build_menu(&build, core.menus.current_menu());
                }
                MenuRequest::Delete => {
                    if let Some(h) = self.handle.take() {
                        game.delete_menu(h);
                    }
                }
            }
        }
        let (old_c, old_t) = std::mem::replace(&mut self.pad, (cross, triangle));
        let handle = self.handle;
        let action = core.menus.poll(cross && !old_c, triangle && !old_t, || match handle {
            Some(h) => game.selected_row(h),
            None => NO_ROW,
        });
        match action {
            Some(PollAction::Selected(row)) => core.push_outgoing(Outgoing::MenuRow(row)),
            Some(PollAction::Exited) => core.push_outgoing(Outgoing::MenuExit),
            Some(PollAction::HideLocal) => {
                if let Some(h) = self.handle.take() {
                    game.delete_menu(h);
                }
            }
            None => {}
        }
    }
}

/// The GTA menu through the game adapter's menus, its texts registered as the game's text entries.
pub struct AdapterMenus<'a> {
    /// The game.
    pub game: &'a mut dyn GameAdapter,
    /// The keys the current menu registered.
    pub keys: &'a mut Vec<&'static str>,
}

impl MenuGame for AdapterMenus<'_> {
    fn build_menu(&mut self, b: &MenuBuild, menu: Option<&Menu>) -> Option<i32> {
        // The empty texts' keys are never asked for: the build names them `DUMMY`.
        let texts: Vec<(&'static str, &[u8])> =
            menu.map(Menu::texts).unwrap_or_default().into_iter().filter(|(_, t)| !t.is_empty()).collect();
        for k in std::mem::take(self.keys) {
            if !texts.iter().any(|(t, _)| *t == k) {
                let _ = self.game.remove_text_entry(k);
            }
        }
        for (k, t) in &texts {
            if self.game.set_text_entry(k, t).is_ok() {
                self.keys.push(k);
            }
        }
        let spec = MenuSpec {
            title: b.title,
            pos: Point2 { x: b.x, y: b.y },
            width: b.width,
            columns: b.columns as u8,
            interactive: b.interactive != 0,
            background: true,
            alignment: 1,
        };
        let m = self.game.create_menu(&spec).ok()?;
        for c in &b.column_builds {
            let _ = self.game.set_menu_column(m, c.column, c.header, &c.rows);
            let _ = self.game.set_menu_column_width(m, c.column, c.width.clamp(0, i32::from(u16::MAX)) as u16);
        }
        for (row, &on) in b.rows_enabled.iter().enumerate() {
            let _ = self.game.set_menu_row_enabled(m, row as u8, on != 0);
        }
        Some(m.0 as i32)
    }

    fn delete_menu(&mut self, handle: i32) {
        let _ = self.game.delete_menu(MenuRef(handle as u32));
    }

    fn selected_row(&mut self, handle: i32) -> u8 {
        self.game.menu_selection(MenuRef(handle as u32)).map_or(NO_ROW, |r| r as u8)
    }
}
