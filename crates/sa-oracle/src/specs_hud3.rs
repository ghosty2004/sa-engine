//! L1 specs for PORT-HUD-3: `CMenuManager::StretchX` / `StretchY` (`sa_hud::menu_mgr`), hooked with
//! `preserve = "edx"` (callers keep EDX across the call): the specs compare EDX. `RsGlobal.maximumWidth`
//! / `maximumHeight` (0xC17044 / 0xC17048) are patched with a fixed value per spec: the identity value
//! (640 / 448) and two other screen sizes; `this` is random (unused by the original).

use crate::call::Conv::*;
use crate::mspec::*;
use crate::specs::{RetKind::*, Spec};
use sa_core::abi::PRESERVE_EDX;
use sa_hud::menu_mgr;

fn stretch(key: &'static str, y: bool, size: i32) -> Spec {
    let (name, addr, global) = if y {
        ("CMenuManager::StretchY", 0x573410, 0xC17048)
    } else {
        ("CMenuManager::StretchX", 0x5733E0, 0xC17044)
    };
    b(key, name, addr, Thiscall, St0)
        .args(vec![A::I(g_rand), A::F])
        .patch(vec![(global, size.to_le_bytes().to_vec())])
        .reads(vec![(global, global + 4)])
        .preserve(PRESERVE_EDX)
        .rust(move |c| {
            let v = c.f(1);
            st0(if y {
                menu_mgr::stretch_y_with(size, v)
            } else {
                menu_mgr::stretch_x_with(size, v)
            })
        })
}

pub fn all() -> Vec<Spec> {
    vec![
        stretch("HudMenuStretchX640", false, 640),
        stretch("HudMenuStretchX1024", false, 1024),
        stretch("HudMenuStretchX1366", false, 1366),
        stretch("HudMenuStretchY448", true, 448),
        stretch("HudMenuStretchY768", true, 768),
        stretch("HudMenuStretchY1080", true, 1080),
    ]
}
