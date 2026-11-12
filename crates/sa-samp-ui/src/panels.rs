//! The kill list (5 rows at the right), the scoreboard (TAB) and the mouse cursor.

use std::collections::VecDeque;

use sa_samp_api::ui::ScoreRow;

use crate::draw::{DrawList, OUTLINE_4};
use crate::font::{FontId, Measure};

/// Kill-list rows.
pub const KILL_ROWS: usize = 5;
/// The widest name samp reserves room for.
pub const NICK_SAMPLE: &str = "LONGESTNICKNICK_NICKNICK";
/// Reason codes for connect / disconnect.
pub const REASON_CONNECT: u8 = 200;
/// Disconnect.
pub const REASON_DISCONNECT: u8 = 201;

/// One kill-list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KillRow {
    /// Killer (empty = none: the victim is shown on the left).
    pub killer: String,
    /// `0xAARRGGBB`.
    pub killer_colour: u32,
    /// Victim.
    pub victim: String,
    /// `0xAARRGGBB`.
    pub victim_colour: u32,
    /// Weapon id or 200 / 201.
    pub reason: u8,
}

/// Short weapon labels standing in for samp's GTAWEAPON3 icon glyphs (the icon font is not drawn yet).
pub fn reason_label(reason: u8) -> String {
    match reason {
        0 => "FIST".into(),
        REASON_CONNECT => "+".into(),
        REASON_DISCONNECT => "-".into(),
        49 => "CAR".into(),
        50 => "HELI".into(),
        51 => "BOOM".into(),
        53 => "H2O".into(),
        54 => "FALL".into(),
        r => format!("#{r}"),
    }
}

/// The kill list.
#[derive(Clone, Debug, Default)]
pub struct KillList {
    rows: VecDeque<KillRow>,
}

impl KillList {
    /// Adds a row; the oldest of five scrolls out.
    pub fn add(&mut self, row: KillRow) {
        if self.rows.len() == KILL_ROWS {
            self.rows.pop_front();
        }
        self.rows.push_back(row);
    }

    /// Rows, oldest first.
    pub fn rows(&self) -> impl Iterator<Item = &KillRow> {
        self.rows.iter()
    }


    /// Icon height: samp measures a GTAWEAPON3 glyph of its backdrop font (aux height + 12 px; the small
    /// font's height stands in for aux).
    pub fn icon_h(m: &dyn Measure) -> f32 {
        m.line_height(FontId::Small) + 12.0
    }

    /// Icon width (the glyph's measured width; 31 px at 24 px in the reference screenshots).
    pub fn icon_w(m: &dyn Measure) -> f32 {
        (1.3 * Self::icon_h(m)).round()
    }

    /// Origin `(trunc(0.75·W), trunc(0.3·H))`, pulled left so two names and the icon fit.
    pub fn origin(w: f32, h: f32, nick_w: f32, icon_w: f32) -> (f32, f32) {
        let mut x = (0.75 * w).trunc();
        if x + nick_w * 2.0 + icon_w > w {
            x = w - (nick_w * 2.0 + icon_w);
        }
        (x, (0.3 * h).trunc())
    }

    /// Draws the five slots (samp.dll 0x1006a2e0), newest at the bottom, empty slots keeping their space:
    /// the killer right-aligned to end at `x + nickW`, the icon box at `x + nickW + 3`, the victim after the
    /// icon. Without killer the victim takes the killer's place. The weapon glyphs (samp's GTAWEAPON3 font)
    /// are stood in for by short labels.
    pub fn draw(&self, m: &dyn Measure, w: f32, h: f32, d: &mut DrawList) {
        let nick_w = m.width(FontId::Small, NICK_SAMPLE).trunc();
        let (icon_w, icon_h) = (Self::icon_w(m), Self::icon_h(m));
        let (x, y0) = Self::origin(w, h, nick_w, icon_w);
        let step = icon_h + 5.0;
        let empty = KILL_ROWS - self.rows.len().min(KILL_ROWS);
        for (i, r) in self.rows.iter().enumerate() {
            let y = y0 + (empty + i) as f32 * step;
            let icon_l = x + nick_w + 3.0;
            let left = if r.killer.is_empty() {
                (&r.victim, r.victim_colour)
            } else {
                (&r.killer, r.killer_colour)
            };
            let lw = m.width(FontId::Small, left.0);
            d.coloured(m, x + nick_w - lw, y, FontId::Small, left.0, left.1, &OUTLINE_4);
            // The backdrop glyph `G`: a black square of about the font's x-height.
            let side = icon_h - 4.0;
            let (bl, bt) = (icon_l + 3.0, y - 3.0);
            d.rect(bl, bt, bl + side, bt + side, 0xFF00_0000);
            let label = reason_label(r.reason);
            let colour = match r.reason {
                REASON_CONNECT => 0xFF11_11AA,
                REASON_DISCONNECT => 0xFFAA_1111,
                _ => 0xFFFF_FFFF,
            };
            let tw = m.width(FontId::Small, &label);
            d.text(
                (bl + (side - tw) / 2.0).round(),
                (bt + (side - m.line_height(FontId::Small)) / 2.0).round(),
                FontId::Small,
                colour,
                &label,
            );
            if !r.killer.is_empty() {
                d.coloured(
                    m,
                    icon_l + icon_w,
                    y,
                    FontId::Small,
                    &r.victim,
                    r.victim_colour,
                    &OUTLINE_4,
                );
            }
        }
    }
}

/// The scoreboard panel.
#[derive(Clone, Debug, Default)]
pub struct Scoreboard {
    /// Server name.
    pub server: String,
    /// Rows.
    pub rows: Vec<ScoreRow>,
}

/// Scoreboard background (its GUI dialog, samp.dll 0x1006ea30).
pub const SB_BACKGROUND: u32 = 0x960A_0A0A;
/// Server name and player count colour.
pub const SB_TEXT: u32 = 0xFFBE_BEBE;
/// Column header colour.
pub const SB_HEADER: u32 = 0xFF95_B0D0;
/// The player count text samp measures to place the count.
pub const PLAYERS_SAMPLE: &str = "Players: 10000";
/// Height of the area above the list box.
pub const SB_TOP: f32 = 60.0;

impl Scoreboard {
    /// Panel rectangle (samp.dll 0x1006eab0): 640 × 480, or 800 × 600 above 800 px, centred on the video
    /// mode.
    pub fn rect(w: f32, h: f32) -> [f32; 4] {
        let (pw, ph) = if w > 800.0 { (800.0, 600.0) } else { (640.0, 480.0) };
        let (x, y) = ((w / 2.0).trunc() - pw / 2.0, (h / 2.0).trunc() - ph / 2.0);
        [x, y, x + pw, y + ph]
    }

    /// The list box.
    fn list_rect(r: &[f32; 4]) -> [f32; 4] {
        [r[0], r[1] + SB_TOP, r[2], r[3]]
    }

    /// Column widths: id, name, score (ping takes the rest).
    pub fn columns(r: &[f32; 4]) -> [f32; 3] {
        let sw = r[2] - r[0];
        [(0.085 * sw).round(), (0.4375 * sw).round(), (0.265625 * sw).round()]
    }

    /// The row under `(x, y)`.
    fn row_at(&self, m: &dyn Measure, r: &[f32; 4], x: f32, y: f32) -> Option<usize> {
        let sel = crate::skin::list_selection_rect(Self::list_rect(r));
        if x < sel[0] || x >= sel[2] || y < sel[1] || y >= sel[3] {
            return None;
        }
        let i = ((y - sel[1]) / m.line_height(FontId::Gui1)) as usize;
        (i < self.rows.len()).then_some(i)
    }

    /// Draws the panel (samp.dll 0x1006f0b0): the GUI dialog with its list box (rows in the player colours,
    /// the row under the cursor selected), then the server name, the player count and the column headers in
    /// the small font with the outline.
    pub fn draw(&self, m: &dyn Measure, w: f32, h: f32, cursor: Option<(f32, f32)>, d: &mut DrawList) {
        let r = Self::rect(w, h);
        d.dialog_background(r, SB_BACKGROUND);
        let cols = Self::columns(&r);
        let rows: Vec<(u32, Vec<String>)> = self
            .rows
            .iter()
            .map(|row| {
                (
                    row.colour.to_argb_u32() | 0xFF00_0000,
                    vec![
                        row.id.to_string(),
                        row.name.clone(),
                        row.score.to_string(),
                        row.ping.to_string(),
                    ],
                )
            })
            .collect();
        // DXUT keeps the list selection (row 0 when the scoreboard opens); the row under the cursor here.
        let selected = cursor.and_then(|(x, y)| self.row_at(m, &r, x, y)).or(Some(0));
        d.list_box(m, Self::list_rect(&r), &rows, 4, &cols, selected, 0);
        d.coloured(m, r[0] + 5.0, r[1] + 5.0, FontId::Small, &self.server, SB_TEXT, &OUTLINE_4);
        let count = format!("Players: {}", self.rows.len());
        d.coloured(
            m,
            r[2] - m.width(FontId::Small, PLAYERS_SAMPLE),
            r[1] + 5.0,
            FontId::Small,
            &count,
            SB_TEXT,
            &OUTLINE_4,
        );
        let top = r[1] + SB_TOP - m.line_height(FontId::Small) - 3.0;
        let mut x = r[0] + 10.0;
        for (i, t) in ["id", "name", "score", "ping"].iter().enumerate() {
            d.coloured(m, x, top, FontId::Small, t, SB_HEADER, &OUTLINE_4);
            x += cols.get(i).copied().unwrap_or(0.0);
        }
    }

    /// The player id of the row under `(x, y)`.
    pub fn hit(&self, m: &dyn Measure, w: f32, h: f32, x: f32, y: f32) -> Option<u16> {
        let r = Self::rect(w, h);
        self.row_at(m, &r, x, y).map(|i| self.rows[i].id)
    }
}

/// The arrow cursor at `(x, y)`: a black arrow with a white inner arrow (two triangle pairs).
pub fn draw_cursor(x: f32, y: f32, d: &mut DrawList) {
    use crate::draw::Cmd;
    let arrow = |s: f32, o: f32, colour: u32, d: &mut DrawList| {
        let p = |dx: f32, dy: f32| (x + o + dx * s, y + o * 2.0 + dy * s);
        d.cmds.push(Cmd::Triangle {
            pts: [p(0.0, 0.0), p(0.0, 16.0), p(11.0, 11.0)],
            colour,
        });
        d.cmds.push(Cmd::Triangle {
            pts: [p(3.0, 11.0), p(5.5, 9.0), p(8.0, 18.0)],
            colour,
        });
    };
    arrow(1.15, -1.0, 0xFF00_0000, d);
    arrow(1.0, 0.0, 0xFFFF_FFFF, d);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::MonoMeasure;
    use sa_samp_api::Rgba;

    const M: MonoMeasure = MonoMeasure {
        char_w: 7.0,
        line_h: 14.0,
    };

    #[test]
    fn kill_list_keeps_five_and_pulls_origin_left() {
        let mut k = KillList::default();
        for i in 0..7u8 {
            k.add(KillRow {
                killer: format!("k{i}"),
                killer_colour: 0,
                victim: "v".into(),
                victim_colour: 0,
                reason: i,
            });
        }
        let r: Vec<&str> = k.rows().map(|r| r.killer.as_str()).collect();
        assert_eq!(r, ["k2", "k3", "k4", "k5", "k6"]);
        assert_eq!(KillList::origin(1280.0, 720.0, 168.0, 31.0), (1280.0 - 367.0, 216.0));
        assert_eq!(KillList::origin(1920.0, 1080.0, 100.0, 31.0), (1440.0, 324.0));
        assert_eq!(reason_label(REASON_CONNECT), "+");
        assert_eq!(reason_label(24), "#24");
    }

    #[test]
    fn scoreboard_hit_maps_rows_to_ids() {
        let sb = Scoreboard {
            server: "srv".into(),
            rows: (0..3)
                .map(|i| ScoreRow {
                    id: 10 + i,
                    name: format!("p{i}"),
                    score: 0,
                    ping: 30,
                    colour: Rgba::default(),
                })
                .collect(),
        };
        let r = Scoreboard::rect(1280.0, 720.0);
        assert_eq!(r, [240.0, 60.0, 1040.0, 660.0]);
        assert_eq!(Scoreboard::rect(800.0, 600.0), [80.0, 60.0, 720.0, 540.0]);
        assert_eq!(Scoreboard::columns(&r), [68.0, 350.0, 213.0]);
        // Rows start at the list box's border (top + 60 + 6), one line each.
        let y1 = r[1] + SB_TOP + 6.0 + 14.0 + 3.0;
        assert_eq!(sb.hit(&M, 1280.0, 720.0, 400.0, y1), Some(11));
        assert_eq!(sb.hit(&M, 1280.0, 720.0, 100.0, y1), None);
        let mut d = DrawList::default();
        sb.draw(&M, 1280.0, 720.0, None, &mut d);
        assert!(d.texts().contains(&"Players: 3"));
    }
}
