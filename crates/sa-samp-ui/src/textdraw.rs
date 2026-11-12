//! Textdraws (RPC 134 / 135): drawn in the game's own fonts through `CFont`, as samp does, in samp's
//! 640×448 canvas scaled to the screen (samp-rs `samp-ui/textdraw.rs::font_params`, samp.dll 0x100b33f5 for
//! the hit rectangles of `SelectTextDraw`).

use std::collections::BTreeMap;

use sa_samp_api::hud::{FontStyle, TextAlign};
use sa_samp_api::ui::{TdAlign, TextDrawSpec};
use sa_samp_api::{Point2, Rgba};

use crate::draw::{Cmd, DrawList};

/// Textdraw ids: 2048 global + 256 per-player.
pub const MAX_ID: u16 = 2048 + 256;

/// The `CFont` state and position of one textdraw on a `w × h` screen. Letter height is halved (the game
/// font's cell is 2:1), positions keep samp's float operation order.
pub fn font_params(t: &TextDrawSpec, hover: Option<Rgba>, w: f32, h: f32) -> (FontStyle, f32, f32) {
    let (ux, uy) = (1.0 / 640.0, 1.0 / 448.0);
    let style = FontStyle {
        scale: Point2 {
            x: w * t.letter_w * ux,
            y: h * t.letter_h * uy * 0.5,
        },
        colour: hover.unwrap_or(t.colour),
        font: t.font.min(3),
        wrap_x: w * t.text_w * ux,
        centre_size: w * t.text_h * ux,
        drop_colour: t.background_colour,
        shadow: if t.outline == 0 { t.shadow as i8 } else { 0 },
        edge: t.outline as i8,
        proportional: t.proportional,
        background: t.use_box,
        background_colour: t.box_colour,
        align: match t.align {
            TdAlign::Left => TextAlign::Left,
            TdAlign::Centre => TextAlign::Centre,
            TdAlign::Right => TextAlign::Right,
        },
    };
    let x = w - w * ux * (640.0 - t.x);
    let y = h - h * uy * (448.0 - t.y);
    (style, x, y)
}

/// Hit rectangle (`[l, t, r, b]`) after drawing, per alignment (samp.dll 0x100b33f5).
pub fn hit_rect(s: &FontStyle, x: f32, y: f32) -> [f32; 4] {
    match s.align {
        TextAlign::Right => [
            (x - (s.wrap_x - x)).trunc(),
            y.trunc(),
            x.trunc(),
            (y + s.centre_size).trunc(),
        ],
        TextAlign::Centre => [
            (x - s.centre_size * 0.5).trunc(),
            y.trunc(),
            (x + s.centre_size * 0.5).trunc(),
            (y + s.wrap_x).trunc(),
        ],
        TextAlign::Left => [
            x.trunc(),
            y.trunc(),
            s.wrap_x.trunc(),
            (y + s.centre_size).trunc(),
        ],
    }
}

/// Font 4: a texture from a TXD (`txd:name`).
pub const FONT_SPRITE: u8 = 4;

/// Where a sprite textdraw's texture comes from (samp.dll 0x100B2D10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteSource {
    /// TXD slot name (`CTxdStore::FindTxdSlot`).
    pub slot: String,
    /// File to load into a new slot when none has that name (`models/txd/NAME.txd`, or samp's own
    /// `SAMP/samaps.txd` / `SAMP/vehicles128.txd`, with backslashes); `None` = the slot must exist (`hud`).
    pub file: Option<String>,
    /// Texture name.
    pub texture: String,
}

/// Parses `txd:name` as samp does: at most 63 characters, no path separators; `hud` is the game's HUD
/// dictionary, `samaps` / `vehicleprev` come from samp's folder, anything else from `models/txd` with the
/// slot name cut at 10 characters.
pub fn sprite_source(text: &str) -> Option<SpriteSource> {
    if text.len() > 0x3F || text.contains(['\\', '/']) {
        return None;
    }
    let (t, name) = text.split_once(':')?;
    let (slot, file) = match t {
        "hud" => ("hud".to_string(), None),
        "samaps" => ("samaps".to_string(), Some("SAMP\\samaps.txd".to_string())),
        "vehicleprev" => (
            "vehicleprev".to_string(),
            Some("SAMP\\vehicles128.txd".to_string()),
        ),
        _ => (
            t.chars().take(10).collect(),
            Some(format!("models\\txd\\{t}.txd")),
        ),
    };
    Some(SpriteSource {
        slot,
        file,
        texture: name.to_string(),
    })
}

/// A sprite textdraw's rectangle: position and `TextSize` scaled from 640x448.
pub fn sprite_rect(t: &TextDrawSpec, w: f32, h: f32) -> [f32; 4] {
    let (sx, sy) = (w / 640.0, h / 448.0);
    [t.x * sx, t.y * sy, (t.x + t.text_w) * sx, (t.y + t.text_h) * sy]
}

/// The textdraw pool and `SelectTextDraw` state.
#[derive(Clone, Debug, Default)]
pub struct TextDraws {
    shown: BTreeMap<u16, TextDrawSpec>,
    hits: BTreeMap<u16, [f32; 4]>,
    /// `SelectTextDraw` hover colour while selecting.
    pub selecting: Option<Rgba>,
}

impl TextDraws {
    /// Shows / replaces or hides a textdraw.
    pub fn set(&mut self, id: u16, t: Option<&TextDrawSpec>) {
        if id >= MAX_ID {
            return;
        }
        match t {
            Some(t) => {
                self.shown.insert(id, t.clone());
            }
            None => {
                self.shown.remove(&id);
                self.hits.remove(&id);
            }
        }
    }

    /// Shown textdraws.
    pub fn len(&self) -> usize {
        self.shown.len()
    }

    /// Whether none is shown.
    pub fn is_empty(&self) -> bool {
        self.shown.is_empty()
    }

    /// Draws every shown textdraw in id order (text in the game fonts, font 4 as a TXD sprite; font 5, model
    /// previews, is not drawn yet) and records the hit rectangles. `cursor` hovers selectable ones while
    /// selecting.
    pub fn draw(&mut self, w: f32, h: f32, cursor: Option<(f32, f32)>, d: &mut DrawList) {
        self.hits.clear();
        for (&id, t) in &self.shown {
            if t.font == FONT_SPRITE {
                let r = sprite_rect(t, w, h);
                let hit = r.map(f32::trunc);
                let hovered = self.selecting.is_some()
                    && t.selectable
                    && cursor
                        .is_some_and(|(cx, cy)| cx >= hit[0] && cx < hit[2] && cy >= hit[1] && cy < hit[3]);
                if let Some(source) = sprite_source(&t.text) {
                    let c = if hovered {
                        self.selecting.unwrap_or(t.colour)
                    } else {
                        t.colour
                    };
                    d.cmds.push(Cmd::Sprite {
                        source,
                        rect: r,
                        colour: c.to_argb_u32(),
                    });
                }
                if t.selectable {
                    self.hits.insert(id, hit);
                }
                continue;
            }
            if t.font > 3 || t.text.is_empty() {
                continue;
            }
            let (s, x, y) = font_params(t, None, w, h);
            let r = hit_rect(&s, x, y);
            let hovered = self.selecting.is_some()
                && t.selectable
                && cursor.is_some_and(|(cx, cy)| cx >= r[0] && cx < r[2] && cy >= r[1] && cy < r[3]);
            let (s, x, y) = font_params(t, if hovered { self.selecting } else { None }, w, h);
            d.cmds.push(Cmd::GameFont {
                style: s,
                x,
                y,
                text: t.text.clone(),
            });
            if t.selectable {
                self.hits.insert(id, r);
            }
        }
    }

    /// The selectable textdraw under `(x, y)` (last drawn wins).
    pub fn hit(&self, x: f32, y: f32) -> Option<u16> {
        self.hits
            .iter()
            .rev()
            .find(|(_, r)| x >= r[0] && x < r[2] && y >= r[1] && y < r[3])
            .map(|(id, _)| *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn td(align: TdAlign, pos: (f32, f32), size: (f32, f32)) -> TextDrawSpec {
        TextDrawSpec {
            text: "Hello".into(),
            x: pos.0,
            y: pos.1,
            letter_w: 0.5,
            letter_h: 1.0,
            text_w: size.0,
            text_h: size.1,
            colour: Rgba::from_rgba_u32(0xFFFF_FFFF),
            use_box: false,
            box_colour: Rgba::default(),
            shadow: 1,
            outline: 0,
            background_colour: Rgba::from_rgba_u32(0x0000_00FF),
            font: 1,
            proportional: true,
            align,
            selectable: true,
        }
    }

    #[test]
    fn params_and_hit_rects_match_samp() {
        let (s, x, y) = font_params(
            &td(TdAlign::Left, (320.0, 224.0), (400.0, 20.0)),
            None,
            1280.0,
            720.0,
        );
        assert!((x - 640.0).abs() < 0.01 && (y - 360.0).abs() < 0.01);
        assert!((s.scale.x - 1.0).abs() < 1e-6 && (s.scale.y - 720.0 / 448.0 * 0.5).abs() < 1e-6);
        assert_eq!((s.shadow, s.edge, s.align), (1, 0, TextAlign::Left));
        let r = hit_rect(&s, x, y);
        assert_eq!((r[0], r[1], r[2]), (640.0, 360.0, 800.0));
        let (s, x, y) = font_params(
            &td(TdAlign::Centre, (320.0, 224.0), (20.0, 100.0)),
            None,
            1280.0,
            720.0,
        );
        let r = hit_rect(&s, x, y);
        assert_eq!((r[0], r[2]), (540.0, 740.0));
        let (s, x, y) = font_params(
            &td(TdAlign::Right, (320.0, 224.0), (200.0, 20.0)),
            None,
            1280.0,
            720.0,
        );
        let r = hit_rect(&s, x, y);
        assert_eq!((r[0], r[2]), (880.0, 640.0));
    }

    #[test]
    fn sprite_sources_and_rects() {
        assert_eq!(
            sprite_source("LD_SPAC:white"),
            Some(SpriteSource {
                slot: "LD_SPAC".into(),
                file: Some("models\\txd\\LD_SPAC.txd".into()),
                texture: "white".into()
            })
        );
        assert_eq!(sprite_source("hud:radar_burgershot").unwrap().file, None);
        assert_eq!(
            sprite_source("vehicleprev:411").unwrap().file.as_deref(),
            Some("SAMP\\vehicles128.txd")
        );
        assert_eq!(sprite_source("loadscs_longname:x").unwrap().slot, "loadscs_lo");
        assert_eq!(sprite_source("../x:y"), None);
        assert_eq!(sprite_source("nocolon"), None);
        let mut t = td(TdAlign::Left, (320.0, 224.0), (64.0, 32.0));
        t.font = FONT_SPRITE;
        t.text = "LD_SPAC:white".into();
        assert_eq!(
            sprite_rect(&t, 1280.0, 720.0),
            [640.0, 360.0, 768.0, 360.0 + 32.0 * 720.0 / 448.0]
        );
        let mut p = TextDraws::default();
        p.set(1, Some(&t));
        let mut d = DrawList::default();
        p.draw(1280.0, 720.0, None, &mut d);
        assert!(matches!(&d.cmds[0], Cmd::Sprite { source, .. } if source.texture == "white"));
        assert_eq!(p.hit(700.0, 370.0), Some(1));
    }

    #[test]
    fn pool_draw_hover_and_hit() {
        let mut p = TextDraws::default();
        p.set(5, Some(&td(TdAlign::Left, (320.0, 224.0), (400.0, 20.0))));
        p.set(MAX_ID, Some(&td(TdAlign::Left, (0.0, 0.0), (1.0, 1.0))));
        assert_eq!(p.len(), 1);
        p.selecting = Some(Rgba::from_rgba_u32(0xFF00_00FF));
        let mut d = DrawList::default();
        p.draw(1280.0, 720.0, Some((700.0, 365.0)), &mut d);
        let Cmd::GameFont { style, .. } = &d.cmds[0] else {
            panic!()
        };
        assert_eq!(style.colour, Rgba::from_rgba_u32(0xFF00_00FF));
        assert_eq!(p.hit(700.0, 365.0), Some(5));
        assert_eq!(p.hit(10.0, 10.0), None);
        p.set(5, None);
        assert!(p.is_empty() && p.hit(700.0, 365.0).is_none());
    }
}
