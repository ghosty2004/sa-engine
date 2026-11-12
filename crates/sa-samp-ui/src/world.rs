//! UI anchored in the world: nametags with health / armour bars (samp.dll 0x1006d0a0) and 3D text labels.
//! World positions are projected by the engine ([`WorldView`]; natively `CSprite::CalcScreenCoors`).

use sa_samp_api::ui::{Nametag, NametagAnchor, TextLabel3d};
use sa_samp_api::{PedRef, Vec3};

use crate::colour::strip_codes;
use crate::draw::{DrawList, OUTLINE_4};
use crate::font::{FontId, Measure};
use crate::text::wrap_pixels;

/// Anchor raise above the head: `0.0475·d + 0.2` metres (d = camera distance).
pub const RAISE_PER_METRE: f32 = 0.0475;
/// Anchor raise base.
pub const RAISE_BASE: f32 = 0.2;
/// Head height above the ped's root (the bone samp reads is the head; the root is at the pelvis).
pub const HEAD_ABOVE_ROOT: f32 = 0.7;
/// Nametag draw distance (server default `nametag_draw_distance`).
pub const NAMETAG_DISTANCE: f32 = 70.0;
/// Bar width per health point (100 → 38.46 px).
pub const BAR_SCALE: f32 = 0.384_615_4;
/// Bar colours.
pub const BORDER: u32 = 0xFF00_0000;
/// Health background.
pub const HEALTH_BG: u32 = 0xFF4B_0B14;
/// Health fill.
pub const HEALTH_FILL: u32 = 0xFFB9_2228;
/// Armour background.
pub const ARMOUR_BG: u32 = 0xFF28_2828;
/// Armour fill.
pub const ARMOUR_FILL: u32 = 0xFFC8_C8C8;
/// Widest label line.
pub const LABEL_WRAP_W: f32 = 400.0;

/// What the engine tells the UI about the world this frame.
pub trait WorldView {
    /// Screen position of a world point, `None` when it is behind the camera or off the far plane.
    fn project(&self, p: Vec3) -> Option<(f32, f32)>;
    /// Camera position.
    fn camera_pos(&self) -> Vec3;
    /// A ped's root position, `None` for a stale handle.
    fn ped_position(&self, ped: PedRef) -> Option<Vec3>;
    /// Whether nothing solid (buildings, objects) is between `a` and `b` (samp's nametag and label test,
    /// opcode 06BD with buildings and objects). Default: clear.
    fn line_clear(&self, a: Vec3, b: Vec3) -> bool {
        let _ = (a, b);
        true
    }
}

/// Nametag rules from the server and the client (`UiOptions`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NametagRules {
    /// Shown at all.
    pub show: bool,
    /// Draw distance.
    pub distance: f32,
    /// Hidden behind buildings and objects.
    pub los: bool,
}

impl Default for NametagRules {
    fn default() -> Self {
        Self {
            show: true,
            distance: NAMETAG_DISTANCE,
            los: true,
        }
    }
}

fn dist(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// One bar: border, background, fill (samp's three fans).
fn bar(d: &mut DrawList, x: f32, y: f32, value: f32, bg: u32, fill: u32) {
    d.rect(x - 20.0, y + 18.0, x + 21.0, y + 24.0, BORDER);
    d.rect(x - 19.0, y + 19.0, x + 19.0, y + 23.0, bg);
    d.rect(x - 19.0, y + 19.0, x - 19.0 + value * BAR_SCALE, y + 23.0, fill);
}

/// Bars at the truncated anchor `(x, y)`: health 8 px lower when armour shows; nothing for health < 0.
pub fn bars(d: &mut DrawList, x: i32, y: i32, health: f32, armour: f32) {
    if health < 0.0 {
        return;
    }
    let (fx, fy) = (x as f32, y as f32);
    let hp = health.min(100.0);
    let ar = armour.min(100.0);
    if ar > 0.0 {
        bar(d, fx, fy + 8.0, hp, HEALTH_BG, HEALTH_FILL);
        bar(d, fx, fy, ar, ARMOUR_BG, ARMOUR_FILL);
    } else {
        bar(d, fx, fy, hp, HEALTH_BG, HEALTH_FILL);
    }
}

/// Draws the nametags in range.
pub fn draw_nametags(
    tags: &[Nametag],
    rules: NametagRules,
    w: &dyn WorldView,
    m: &dyn Measure,
    d: &mut DrawList,
) {
    if !rules.show {
        return;
    }
    let cam = w.camera_pos();
    for t in tags {
        let root = match t.anchor {
            NametagAnchor::Ped(p) => match w.ped_position(p) {
                Some(p) => p,
                None => continue,
            },
            NametagAnchor::World(p) => p,
        };
        let head = Vec3::new(root.x, root.y, root.z + HEAD_ABOVE_ROOT);
        let dd = dist(head, cam);
        if dd >= rules.distance || (rules.los && !w.line_clear(head, cam)) {
            continue;
        }
        let anchor = Vec3::new(head.x, head.y, head.z + RAISE_PER_METRE * dd + RAISE_BASE);
        let Some((sx, sy)) = w.project(anchor) else {
            continue;
        };
        let (x, y) = (sx as i32, sy as i32);
        let name = format!("{} ({})", t.name, t.player_id);
        let tw = m.width(FontId::Small, &name);
        let th = m.line_height(FontId::Small);
        let colour = t.colour.to_argb_u32() | 0xFF00_0000;
        d.coloured(
            m,
            (x as f32 - tw / 2.0).round(),
            y as f32 + 17.0 - th,
            FontId::Small,
            &name,
            colour,
            &OUTLINE_4,
        );
        bars(d, x, y, t.health, t.armour);
        if t.afk {
            d.coloured(
                m,
                x as f32 + tw / 2.0 + 4.0,
                y as f32 + 17.0 - th,
                FontId::Small,
                "AFK",
                0xFFE3_E1E3,
                &OUTLINE_4,
            );
        }
    }
}

/// Draws the 3D text labels in range, lines centred on the projected point.
pub fn draw_labels<'a>(
    labels: impl Iterator<Item = &'a TextLabel3d>,
    w: &dyn WorldView,
    m: &dyn Measure,
    d: &mut DrawList,
) {
    let cam = w.camera_pos();
    for l in labels {
        let pos = match l.attached_ped {
            Some(p) => match w.ped_position(p) {
                Some(r) => Vec3::new(r.x + l.pos.x, r.y + l.pos.y, r.z + l.pos.z),
                None => continue,
            },
            None => l.pos,
        };
        if dist(pos, cam) > l.draw_distance || (l.test_los && !w.line_clear(pos, cam)) {
            continue;
        }
        let Some((sx, sy)) = w.project(pos) else { continue };
        let lines = wrap_pixels(m, FontId::Small, &l.text, LABEL_WRAP_W);
        let lh = m.line_height(FontId::Small);
        let top = (sy - lines.len() as f32 * lh / 2.0).round();
        let base = l.colour.to_argb_u32();
        for (i, line) in lines.iter().enumerate() {
            let lw = m.width(FontId::Small, &strip_codes(line));
            d.coloured(
                m,
                (sx - lw / 2.0).round(),
                top + i as f32 * lh,
                FontId::Small,
                line,
                base,
                &OUTLINE_4,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Cmd;
    use crate::font::MonoMeasure;
    use sa_samp_api::Rgba;

    /// Camera at the origin looking along +y; x → screen x, z → screen y (1 px per cm at 1 m).
    struct Flat;
    impl WorldView for Flat {
        fn project(&self, p: Vec3) -> Option<(f32, f32)> {
            (p.y > 0.1).then(|| (640.0 + 100.0 * p.x / p.y, 360.0 - 100.0 * p.z / p.y))
        }
        fn camera_pos(&self) -> Vec3 {
            Vec3::default()
        }
        fn ped_position(&self, ped: PedRef) -> Option<Vec3> {
            (ped.0 == 1).then_some(Vec3::new(0.0, 10.0, 0.0))
        }
    }

    const M: MonoMeasure = MonoMeasure {
        char_w: 7.0,
        line_h: 14.0,
    };

    #[test]
    fn bar_geometry_matches_samp_measurements() {
        let mut d = DrawList::default();
        bars(&mut d, 826, 278, 56.0, 100.0);
        assert_eq!(d.cmds.len(), 6);
        assert_eq!(
            d.cmds[0],
            Cmd::Rect {
                rect: [806.0, 304.0, 847.0, 310.0],
                colour: BORDER
            }
        );
        let Cmd::Rect { rect, .. } = d.cmds[2] else {
            panic!()
        };
        assert!((rect[2] - 828.538).abs() < 0.01);
        let Cmd::Rect { rect, colour } = d.cmds[4] else {
            panic!()
        };
        assert_eq!((rect[1], colour), (297.0, ARMOUR_BG));
        let mut d = DrawList::default();
        bars(&mut d, 0, 0, -1.0, 0.0);
        assert!(d.cmds.is_empty());
    }

    #[test]
    fn nametags_project_raise_and_cull() {
        let tag = |anchor| Nametag {
            player_id: 3,
            anchor,
            name: "CJ".into(),
            colour: Rgba::from_argb_u32(0x00FF_8000),
            health: 100.0,
            armour: 0.0,
            afk: false,
        };
        let mut d = DrawList::default();
        draw_nametags(
            &[
                tag(NametagAnchor::Ped(PedRef(1))),
                tag(NametagAnchor::Ped(PedRef(2))),
                tag(NametagAnchor::World(Vec3::new(0.0, 100.0, 0.0))),
            ],
            NametagRules::default(),
            &Flat,
            &M,
            &mut d,
        );
        // Only the first: the second is stale, the third out of range.
        let names: Vec<&str> = d.texts();
        assert_eq!(names.iter().filter(|t| **t == "CJ (3)").count(), 1);
        // Raised anchor: head 0.7 + 0.0475·d + 0.2 at d ≈ 10.02 → z ≈ 1.376 → y ≈ 346.
        let Some(Cmd::Text { y, colour, .. }) = d.cmds.first() else {
            panic!()
        };
        assert_eq!((*y, *colour), (346.0 + 17.0 - 14.0, 0xFFFF_8000));
    }

    /// Everything with x < 0 is behind a wall.
    struct Walled;
    impl WorldView for Walled {
        fn project(&self, p: Vec3) -> Option<(f32, f32)> {
            Flat.project(p)
        }
        fn camera_pos(&self) -> Vec3 {
            Vec3::default()
        }
        fn ped_position(&self, _ped: PedRef) -> Option<Vec3> {
            None
        }
        fn line_clear(&self, a: Vec3, _b: Vec3) -> bool {
            a.x >= 0.0
        }
    }

    #[test]
    fn nametag_rules_hide_by_switch_distance_and_line_of_sight() {
        let tag = |x: f32| Nametag {
            player_id: 1,
            anchor: NametagAnchor::World(Vec3::new(x, 10.0, 0.0)),
            name: "P".into(),
            colour: Rgba::default(),
            health: 50.0,
            armour: 0.0,
            afk: false,
        };
        let count = |rules: NametagRules| {
            let mut d = DrawList::default();
            draw_nametags(&[tag(1.0), tag(-1.0)], rules, &Walled, &M, &mut d);
            d.texts().len()
        };
        assert_eq!(count(NametagRules::default()), 1);
        assert_eq!(
            count(NametagRules {
                los: false,
                ..NametagRules::default()
            }),
            2
        );
        assert_eq!(
            count(NametagRules {
                distance: 5.0,
                ..NametagRules::default()
            }),
            0
        );
        assert_eq!(
            count(NametagRules {
                show: false,
                ..NametagRules::default()
            }),
            0
        );
    }

    #[test]
    fn labels_centre_lines_and_respect_distance() {
        let l = TextLabel3d {
            text: "{FFFF00}Shop\nOpen".into(),
            colour: Rgba::from_argb_u32(0xFFFF_FFFF),
            pos: Vec3::new(0.0, 5.0, 0.0),
            draw_distance: 20.0,
            test_los: false,
            attached_ped: None,
        };
        let far = TextLabel3d {
            pos: Vec3::new(0.0, 50.0, 0.0),
            ..l.clone()
        };
        let mut d = DrawList::default();
        draw_labels([&l, &far].into_iter(), &Flat, &M, &mut d);
        let coloured: Vec<(f32, f32, u32, &str)> = d
            .cmds
            .iter()
            .filter_map(|c| match c {
                Cmd::Text {
                    x, y, colour, text, ..
                } if *colour != 0xFF00_0000 => Some((*x, *y, *colour, text.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(
            coloured,
            vec![
                (626.0, 346.0, 0xFFFF_FF00, "Shop"),
                (626.0, 360.0, 0xFFFF_FF00, "Open")
            ]
        );
    }
}
