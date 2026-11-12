//! The kill list (5 rows at the right), the scoreboard (TAB) and the mouse cursor.

use std::collections::VecDeque;

use sa_samp_api::ui::ScoreRow;

use crate::draw::{DrawList, OUTLINE_4};
use crate::font::{FontId, Measure};

/// Kill-list rows.
pub const KILL_ROWS: usize = 5;
/// The widest name samp reserves room for.
pub const NICK_SAMPLE: &str = "LONGESTNICKNICK_NICKNICK";
/// Icon box width.
pub const ICON_W: f32 = 35.0;
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

    /// Origin `(round(0.75·W), round(0.3·H))`, pulled left so two names and the icon fit.
    pub fn origin(w: f32, h: f32, nick_w: f32) -> (f32, f32) {
        let mut x = (0.75 * w).round();
        if x + nick_w * 2.0 + ICON_W > w {
            x = w - (nick_w * 2.0 + ICON_W);
        }
        (x, (0.3 * h).round())
    }

    /// Draws the rows: killer right-aligned before the icon box, victim after it.
    pub fn draw(&self, m: &dyn Measure, w: f32, h: f32, d: &mut DrawList) {
        let nick_w = m.width(FontId::Small, NICK_SAMPLE);
        let (x, y0) = Self::origin(w, h, nick_w);
        let lh = m.line_height(FontId::Small);
        let row_h = lh + 9.0;
        for (i, r) in self.rows.iter().enumerate() {
            let y = y0 + i as f32 * row_h;
            let icon_l = x + nick_w + 3.0;
            let left = if r.killer.is_empty() {
                (&r.victim, r.victim_colour)
            } else {
                (&r.killer, r.killer_colour)
            };
            let lw = m.width(FontId::Small, left.0);
            d.coloured(m, x + nick_w - lw, y, FontId::Small, left.0, left.1, &OUTLINE_4);
            d.rect(icon_l, y - 3.0, icon_l + ICON_W, y + lh + 3.0, 0xFF00_0000);
            let label = reason_label(r.reason);
            let colour = match r.reason {
                REASON_CONNECT => 0xFF11_11AA,
                REASON_DISCONNECT => 0xFFAA_1111,
                _ => 0xFFFF_FFFF,
            };
            let tw = m.width(FontId::Small, &label);
            d.text(
                (icon_l + (ICON_W - tw) / 2.0).round(),
                y,
                FontId::Small,
                colour,
                &label,
            );
            if !r.killer.is_empty() {
                d.coloured(
                    m,
                    icon_l + ICON_W + 3.0,
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

/// Scoreboard background.
pub const SB_BACKGROUND: u32 = 0xC000_0000;

impl Scoreboard {
    /// Panel rectangle: centred, 60 % of the width (at most 640), 50 % of the height.
    pub fn rect(w: f32, h: f32) -> [f32; 4] {
        let pw = (0.6 * w).min(640.0).round();
        let ph = (0.5 * h).round();
        let (x, y) = (((w - pw) / 2.0).round(), ((h - ph) / 2.0).round());
        [x, y, x + pw, y + ph]
    }

    fn row_top(r: &[f32; 4], lh: f32, i: usize) -> f32 {
        r[1] + 10.0 + 2.0 * (lh + 4.0) + i as f32 * (lh + 2.0)
    }

    /// Draws the panel; `cursor` highlights the row under it.
    pub fn draw(&self, m: &dyn Measure, w: f32, h: f32, cursor: Option<(f32, f32)>, d: &mut DrawList) {
        let r = Self::rect(w, h);
        let lh = m.line_height(FontId::Small);
        d.rect(r[0], r[1], r[2], r[3], SB_BACKGROUND);
        d.frame(r[0], r[1], r[2], r[3], 0xFF80_8080);
        d.coloured(
            m,
            r[0] + 10.0,
            r[1] + 8.0,
            FontId::Small,
            &self.server,
            0xFFFF_FFFF,
            &[],
        );
        let count = format!("Players: {}", self.rows.len());
        d.text(
            r[2] - 10.0 - m.width(FontId::Small, &count),
            r[1] + 8.0,
            FontId::Small,
            0xFFFF_FFFF,
            &count,
        );
        let cols = [r[0] + 10.0, r[0] + 60.0, r[2] - 150.0, r[2] - 70.0];
        let hy = r[1] + 10.0 + lh + 4.0;
        for (x, t) in cols.iter().zip(["id", "name", "score", "ping"]) {
            d.text(*x, hy, FontId::Small, 0xFFA9_C4E4, t);
        }
        for (i, row) in self.rows.iter().enumerate() {
            let y = Self::row_top(&r, lh, i);
            if y + lh > r[3] - 5.0 {
                break;
            }
            if cursor.is_some_and(|(cx, cy)| cx >= r[0] && cx < r[2] && cy >= y && cy < y + lh + 2.0) {
                d.rect(r[0] + 4.0, y - 1.0, r[2] - 4.0, y + lh + 1.0, 0x8040_4040);
            }
            let c = row.colour.to_argb_u32() | 0xFF00_0000;
            d.text(cols[0], y, FontId::Small, c, &row.id.to_string());
            d.text(cols[1], y, FontId::Small, c, &row.name);
            d.text(cols[2], y, FontId::Small, c, &row.score.to_string());
            d.text(cols[3], y, FontId::Small, c, &row.ping.to_string());
        }
    }

    /// The player id of the row under `(x, y)`.
    pub fn hit(&self, m: &dyn Measure, w: f32, h: f32, x: f32, y: f32) -> Option<u16> {
        let r = Self::rect(w, h);
        if x < r[0] || x >= r[2] {
            return None;
        }
        let lh = m.line_height(FontId::Small);
        self.rows.iter().enumerate().find_map(|(i, row)| {
            let t = Self::row_top(&r, lh, i);
            (y >= t && y < t + lh + 2.0).then_some(row.id)
        })
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
        assert_eq!(KillList::origin(1280.0, 720.0, 168.0), (1280.0 - 371.0, 216.0));
        assert_eq!(KillList::origin(1920.0, 1080.0, 100.0), (1440.0, 324.0));
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
        assert_eq!(r, [320.0, 180.0, 960.0, 540.0]);
        let y1 = Scoreboard::row_top(&r, 14.0, 1) + 3.0;
        assert_eq!(sb.hit(&M, 1280.0, 720.0, 400.0, y1), Some(11));
        assert_eq!(sb.hit(&M, 1280.0, 720.0, 100.0, y1), None);
        let mut d = DrawList::default();
        sb.draw(&M, 1280.0, 720.0, None, &mut d);
        assert!(d.texts().contains(&"Players: 3"));
    }
}
