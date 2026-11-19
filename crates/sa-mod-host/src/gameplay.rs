//! The host functions of a ped's last damage, text entries, the game's script menus and mission trains (API 1.12).

use sa_access::text;
use sa_mod_api::state::DamageInfo;
use sa_mod_api::{Call, gxt, menu, status, train};
use sa_types::{MenuRef, PedRef, Vec3, VehicleRef};

use crate::api::{CallCtx, Mem, Owned, create_marker, destroy, guarded, write_handles, write_record};

/// The labels of `set_menu_column`: NUL-terminated, each a valid key or [`menu::NO_TEXT`]; the header and at most
/// [`menu::MAX_ROWS`] rows.
fn menu_labels(raw: &[u8]) -> Option<Vec<Vec<u8>>> {
    let body = raw.strip_suffix(&[0])?;
    let labels: Vec<Vec<u8>> = body.split(|&c| c == 0).map(<[u8]>::to_vec).collect();
    let ok = (1..=menu::MAX_ROWS + 1).contains(&labels.len()) && labels.iter().all(|l| gxt::valid_key(l));
    ok.then_some(labels)
}

/// The calls of this module: `None` for any other call.
pub fn calls(ctx: &mut CallCtx, mem: &mut dyn Mem, call: Call) -> Option<i32> {
    let ours = matches!(
        call,
        Call::ped_damage { .. }
            | Call::set_text_entry { .. }
            | Call::remove_text_entry { .. }
            | Call::create_menu { .. }
            | Call::destroy_menu { .. }
            | Call::set_menu_column { .. }
            | Call::set_menu_column_width { .. }
            | Call::set_menu_row_enabled { .. }
            | Call::menu_selected_row { .. }
            | Call::create_train { .. }
            | Call::destroy_train { .. }
            | Call::train_carriages { .. }
    );
    if !ours {
        return None;
    }
    let reads = matches!(call, Call::ped_damage { .. } | Call::train_carriages { .. });
    let entries = matches!(call, Call::set_text_entry { .. } | Call::remove_text_entry { .. });
    // the menus and trains run script commands, which the game runs between frames only
    if !reads && !entries && ctx.in_hook() {
        return Some(status::WRONG_PHASE);
    }
    let game = ctx.game.clone();
    let owns_menu = |ctx: &CallCtx, h: i32| ctx.owned.contains(&Owned::Menu(MenuRef(h as u32)));
    let done = |r: Result<(), i32>| r.map_or_else(|s| s, |()| status::OK);
    Some(match call {
        Call::ped_damage { handle, buf_ptr, buf_len } => {
            let Some(g) = game else { return Some(status::NOT_FOUND) };
            match g.0.borrow().ped_damage(PedRef(handle as u32)) {
                Ok(d) => write_record(mem, buf_ptr, buf_len, &DamageInfo::from(&d)),
                Err(_) => status::NOT_FOUND,
            }
        }
        Call::set_text_entry { key_ptr, key_len, text_ptr, text_len } => {
            let Some(key) = entry_key(mem, key_ptr, key_len) else { return Some(status::BAD_ARG) };
            if text_len < 0 || text_len as usize > gxt::MAX_TEXT || text_ptr < 0 {
                return Some(status::BAD_ARG);
            }
            let Some(t) = mem.read(text_ptr as u32, text_len as u32) else { return Some(status::BAD_ARG) };
            let held = ctx.owned.contains(&Owned::Text(key));
            let count = ctx.owned.iter().filter(|o| matches!(o, Owned::Text(_))).count();
            if !held && count >= gxt::MAX_ENTRIES {
                return Some(status::FAILED);
            }
            match text::set(key, &t, &ctx.mod_id.0) {
                Ok(()) => {
                    if !held {
                        ctx.owned.push(Owned::Text(key));
                    }
                    status::OK
                }
                Err(text::Refused::Taken) => status::DENIED,
                Err(text::Refused::BadText) => status::BAD_ARG,
                Err(text::Refused::Full) => status::FAILED,
            }
        }
        Call::remove_text_entry { key_ptr, key_len } => {
            let Some(key) = entry_key(mem, key_ptr, key_len) else { return Some(status::BAD_ARG) };
            let Some(i) = ctx.owned.iter().position(|o| *o == Owned::Text(key)) else { return Some(status::NOT_FOUND) };
            ctx.owned.remove(i);
            text::remove(key, Some(&ctx.mod_id.0));
            status::OK
        }
        Call::create_menu { title_ptr, title_len, x, y, width, columns, flags, align } => {
            if title_len < 0 || title_len as usize > gxt::MAX_KEY || title_ptr < 0 {
                return Some(status::BAD_ARG);
            }
            let Some(title) = mem.read(title_ptr as u32, title_len as u32).filter(|t| gxt::valid_key(t)) else {
                return Some(status::BAD_ARG);
            };
            let valid = (1..=menu::MAX_COLUMNS).contains(&columns)
                && flags & !(menu::INTERACTIVE | menu::BACKGROUND) == 0
                && (menu::ALIGN_CENTRE..=menu::ALIGN_RIGHT).contains(&align)
                && [x, y, width].iter().all(|v| v.is_finite());
            if !valid {
                return Some(status::BAD_ARG);
            }
            let layout = sa_access::MenuLayout {
                title: &title,
                x,
                y,
                width,
                columns: columns as u8,
                interactive: flags & menu::INTERACTIVE != 0,
                background: flags & menu::BACKGROUND != 0,
                alignment: align as u8,
            };
            create_marker(ctx, game, "create_menu", |g| g.create_menu(&layout).map(Owned::Menu))
        }
        Call::destroy_menu { handle } => destroy(ctx, game, Owned::Menu(MenuRef(handle as u32))),
        Call::set_menu_column { handle, column, labels_ptr, labels_len } => {
            if !owns_menu(ctx, handle) {
                return Some(status::DENIED);
            }
            let max = (menu::MAX_ROWS + 1) * (gxt::MAX_KEY + 1);
            if labels_ptr < 0 || labels_len < 0 || labels_len as usize > max || !(0..menu::MAX_COLUMNS).contains(&column) {
                return Some(status::BAD_ARG);
            }
            let Some(labels) = mem.read(labels_ptr as u32, labels_len as u32).and_then(|r| menu_labels(&r)) else {
                return Some(status::BAD_ARG);
            };
            let rows: Vec<&[u8]> = labels[1..].iter().map(Vec::as_slice).collect();
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            let mut g = g.0.borrow_mut();
            done(guarded(&mut ctx.logs, "set_menu_column", || {
                g.set_menu_column(MenuRef(handle as u32), column as u8, &labels[0], &rows)
            }))
        }
        Call::set_menu_column_width { handle, column, width } => {
            if !owns_menu(ctx, handle) {
                return Some(status::DENIED);
            }
            if !(0..menu::MAX_COLUMNS).contains(&column) || !(0..=640).contains(&width) {
                return Some(status::BAD_ARG);
            }
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            let mut g = g.0.borrow_mut();
            done(guarded(&mut ctx.logs, "set_menu_column_width", || {
                g.set_menu_column_width(MenuRef(handle as u32), column as u8, width as u16)
            }))
        }
        Call::set_menu_row_enabled { handle, row, on } => {
            if !owns_menu(ctx, handle) {
                return Some(status::DENIED);
            }
            if !(0..menu::MAX_ROWS as i32).contains(&row) {
                return Some(status::BAD_ARG);
            }
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            let mut g = g.0.borrow_mut();
            done(guarded(&mut ctx.logs, "set_menu_row_enabled", || {
                g.set_menu_row_enabled(MenuRef(handle as u32), row as u8, on != 0)
            }))
        }
        Call::menu_selected_row { handle } => {
            if !owns_menu(ctx, handle) {
                return Some(status::DENIED);
            }
            let Some(g) = game else { return Some(status::WRONG_PHASE) };
            let mut g = g.0.borrow_mut();
            let r = guarded(&mut ctx.logs, "menu_selected_row", || g.menu_selected_row(MenuRef(handle as u32)));
            r.unwrap_or_else(|s| s)
        }
        Call::create_train { kind, x, y, z, clockwise } => {
            if !(0..train::TYPES).contains(&kind) || ![x, y, z].iter().all(|v| v.is_finite()) {
                return Some(status::BAD_ARG);
            }
            create_marker(ctx, game, "create_train", |g| {
                g.create_train(kind as u32, Vec3::new(x, y, z), clockwise != 0).map(Owned::Train)
            })
        }
        Call::destroy_train { handle } => destroy(ctx, game, Owned::Train(VehicleRef(handle as u32))),
        Call::train_carriages { handle, buf_ptr, buf_len } => {
            let Some(g) = game else { return Some(status::NOT_FOUND) };
            let g = g.0.borrow();
            let v = VehicleRef(handle as u32);
            if g.vehicle_addr(v).is_err() {
                return Some(status::NOT_FOUND);
            }
            match g.train_carriages(v) {
                Ok(c) => write_handles(mem, buf_ptr, buf_len, c.into_iter().map(|v| v.0)),
                Err(_) => status::BAD_ARG,
            }
        }
        _ => return None,
    })
}

/// The key of `set_text_entry` / `remove_text_entry`.
fn entry_key(mem: &dyn Mem, ptr: i32, len: i32) -> Option<text::Key> {
    if ptr < 0 || len < 0 || len as usize > gxt::MAX_KEY {
        return None;
    }
    text::key(&mem.read(ptr as u32, len as u32)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Game, SliceMem, dispatch};
    use crate::manifest::Permissions;
    use sa_access::fake;
    use sa_mod_api::{Val, import_index};

    #[test]
    fn permissions_ownership_and_arguments() {
        let mut g = fake::game();
        // menu slot 0 in use by the game's own script
        fake::put(&mut g, 0xBA82E0, 1u8);
        fake::put(&mut g, 0xBA82E3, 1u8);
        let game = Game::new(g);
        let mut data = vec![0u8; 256];
        data[..7].copy_from_slice(b"MODTEXT");
        data[16..21].copy_from_slice(b"Hello");
        data[32..40].copy_from_slice(b"HDR\0ROW\0");
        let m = &mut SliceMem(&mut data);
        let mut ctx = CallCtx { game: Some(game), mod_id: ("t".into(), "t".into()), ..Default::default() };
        let i = |n| import_index(n).unwrap();
        let entry = [Val::I32(0), Val::I32(7), Val::I32(16), Val::I32(5)];
        assert_eq!(dispatch(&mut ctx, m, i("set_text_entry"), &entry), Ok(status::DENIED));
        ctx.permissions = Permissions { state: true, write: true, entities: true, ..Default::default() };
        assert_eq!(dispatch(&mut ctx, m, i("set_text_entry"), &entry), Ok(status::OK));
        assert!(text::lookup(b"modtext").is_some());
        assert_eq!(dispatch(&mut ctx, m, i("set_text_entry"), &[Val::I32(0), Val::I32(8), Val::I32(16), Val::I32(5)]), Ok(status::BAD_ARG));
        // the game's menu is not the mod's, a column of it is refused before any command
        let column = [Val::I32(0), Val::I32(0), Val::I32(32), Val::I32(8)];
        assert_eq!(dispatch(&mut ctx, m, i("set_menu_column"), &column), Ok(status::DENIED));
        assert_eq!(dispatch(&mut ctx, m, i("destroy_menu"), &[Val::I32(0)]), Ok(status::DENIED));
        let menu = [Val::I32(0), Val::I32(7), Val::F32(1.0), Val::F32(1.0), Val::F32(1.0), Val::I32(5), Val::I32(0), Val::I32(0)];
        assert_eq!(dispatch(&mut ctx, m, i("create_menu"), &menu), Ok(status::BAD_ARG));
        // a train type out of range, a vehicle that is no train
        let train = [Val::I32(16), Val::F32(0.0), Val::F32(0.0), Val::F32(0.0), Val::I32(0)];
        assert_eq!(dispatch(&mut ctx, m, i("create_train"), &train), Ok(status::BAD_ARG));
        assert_eq!(dispatch(&mut ctx, m, i("destroy_train"), &[Val::I32(0x301)]), Ok(status::DENIED));
        assert_eq!(dispatch(&mut ctx, m, i("train_carriages"), &[Val::I32(0x301), Val::I32(64), Val::I32(64)]), Ok(status::NOT_FOUND));
        // the local player's last damage
        let n = dispatch(&mut ctx, m, i("ped_damage"), &[Val::I32(0x205), Val::I32(64), Val::I32(64)]).unwrap();
        assert_eq!(n as usize, 4 * <DamageInfo as sa_mod_api::state::Words>::WORDS);
        assert_eq!(dispatch(&mut ctx, m, i("remove_text_entry"), &[Val::I32(0), Val::I32(7)]), Ok(status::OK));
        assert!(text::lookup(b"MODTEXT").is_none() && ctx.owned.is_empty());
    }

    #[test]
    fn menu_label_lists() {
        assert_eq!(menu_labels(b"SAMPRW1\0SAMP000\0DUMMY\0").map(|l| l.len()), Some(3));
        assert!(menu_labels(b"SAMPRW1").is_none());
        assert!(menu_labels(b"\0").is_none());
        assert!(menu_labels(b"TOOLONGKEY\0").is_none());
        let thirteen: Vec<u8> = (0..13).flat_map(|_| b"DUMMY\0".to_vec()).collect();
        assert!(menu_labels(&thirteen).is_some());
        let fourteen: Vec<u8> = (0..14).flat_map(|_| b"DUMMY\0".to_vec()).collect();
        assert!(menu_labels(&fourteen).is_none());
    }
}
