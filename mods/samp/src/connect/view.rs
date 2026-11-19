//! How the connect screen looks: one of the game's loading screens behind a darkening veil (a gradient when the
//! install has none), a header with the tabs, a glass card with the server table and the selected server's details,
//! and the key hints. Laid out on 1280 x 760 design units centred on the overlay ([`paint::scale_for`]).

use sa_mod::{Error, GameTexture, Overlay, Rgba};

use super::paint::{Icon, Painter, Rect, Size, Weight, latin1, rgb};
use super::query::Access;
use super::servers::Server;
use super::{Focus, Hit, Internet, Progress, Screen, Tab, Tone};

const TEXT: u32 = 0xF3F6FB;
const SUB: u32 = 0xA7B2C5;
const MUTED: u32 = 0x6E7A90;
const ACCENT: u32 = 0x5B8CFF;
const ACCENT_2: u32 = 0x8B5CF6;
const CARD: u32 = 0x0D1320;
const GREEN: u32 = 0x34D399;
const AMBER: u32 = 0xFBBF24;
const RED: u32 = 0xF87171;
const NIGHT: u32 = 0x05080F;

/// Design size of the screen.
const DESIGN: (f32, f32) = (1280.0, 760.0);
/// Rows of the server table.
const ROW_H: f32 = 34.0;
/// Seconds one loading screen stays before the next fades in.
const SLIDE_US: u64 = 12_000_000;
const FADE_US: u64 = 1_500_000;

/// The game's loading screens the backdrop shows (`models/txd/loadscN.txd`, texture `loadscN`).
#[derive(Default)]
pub struct Backdrop {
    found: Vec<GameTexture>,
    /// The next screen number to look up; past 14 the search is over.
    next: u32,
    looking: bool,
}

impl Backdrop {
    /// Looks up the loading screens, a few per frame (the game answers in its next 2D phase).
    fn look(&mut self) {
        if self.next == 0 {
            self.next = 1;
        }
        for _ in 0..3 {
            if self.next > 14 {
                return;
            }
            let n = self.next;
            let name = format!("loadsc{n}");
            match GameTexture::find(&name, Some(&format!("models/txd/{name}.txd")), &name) {
                Ok(t) => {
                    self.found.push(t);
                    self.next += 1;
                }
                Err(Error::Again) => {
                    self.looking = true;
                    return;
                }
                Err(_) => self.next += 1,
            }
        }
    }

    /// Whether the search is still running (the backdrop is then the gradient alone, never the world).
    fn ready(&self) -> bool {
        self.next > 14 || self.found.len() >= 4
    }
}

/// The screen's origin and scale.
#[derive(Clone, Copy)]
struct Frame {
    ox: f32,
    oy: f32,
    s: f32,
    w: f32,
    h: f32,
}

impl Frame {
    fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(self.ox + x * self.s, self.oy + y * self.s, w * self.s, h * self.s)
    }
    fn u(&self, v: f32) -> f32 {
        v * self.s
    }
}

fn frame(p: &Painter, size: (u32, u32)) -> Frame {
    let (w, h) = (size.0 as f32, size.1 as f32);
    let s = p.scale;
    Frame {
        ox: ((w - DESIGN.0 * s) / 2.0).max(0.0),
        oy: ((h - DESIGN.1 * s) / 2.0).max(0.0),
        s,
        w,
        h,
    }
}

fn now_us() -> u64 {
    sa_mod::time::now().monotonic_us
}

fn ping_colour(ms: u32) -> u32 {
    match ms {
        0..=80 => GREEN,
        81..=180 => AMBER,
        _ => RED,
    }
}

fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The backdrop over the whole overlay; `a` is the opacity of everything (1 while it covers the world).
fn backdrop(sc: &mut Screen, o: &mut Overlay, a: f32) {
    let (w, h) = o.size();
    let (w, h) = (w as f32, h as f32);
    let full = Rect::new(0.0, 0.0, w, h);
    sc.backdrop.look();
    let p = &mut sc.painter;
    let shown = sc.backdrop.ready() && !sc.backdrop.found.is_empty();
    if shown {
        // the game draws its sprites under the overlay, in its next frame: two loading screens cross-fade with a
        // slow zoom
        let t = now_us().saturating_sub(sc.shown_us);
        let n = sc.backdrop.found.len() as u64;
        let slide = t / SLIDE_US;
        let into = t % SLIDE_US;
        let cover = |zoom: f32| {
            // the loading screens are 4:3; cover the overlay and crop
            let (cw, ch) = if w / h > 4.0 / 3.0 {
                (w, w * 3.0 / 4.0)
            } else {
                (h * 4.0 / 3.0, h)
            };
            let (cw, ch) = (cw * zoom, ch * zoom);
            [(w - cw) / 2.0, (h - ch) / 2.0, cw, ch]
        };
        let zoom = |k: u64| 1.04 + 0.06 * (k as f32 / (SLIDE_US + FADE_US) as f32);
        let cur = sc.backdrop.found[(slide % n) as usize];
        let _ = o.game_sprite(
            cur,
            cover(zoom(into + FADE_US)),
            Rgba::new(255, 255, 255, (255.0 * a) as u8),
        );
        if into < FADE_US && slide > 0 {
            let prev = sc.backdrop.found[((slide - 1) % n) as usize];
            let k = 1.0 - into as f32 / FADE_US as f32;
            let _ = o.game_sprite(
                prev,
                cover(zoom(SLIDE_US + into + FADE_US)),
                Rgba::new(255, 255, 255, (255.0 * a * k) as u8),
            );
        }
        p.rect(o, full, rgb(NIGHT, 0.5 * a));
    } else {
        p.gradient(o, full, rgb(0x0B1324, a), rgb(0x1B0F2E, a));
        p.disc(o, w * 0.18, h * 0.1, w * 0.45, rgb(ACCENT, 0.10 * a));
        p.disc(o, w * 0.9, h * 1.0, w * 0.5, rgb(ACCENT_2, 0.10 * a));
    }
    p.fade(
        o,
        Rect::new(0.0, h * 0.35, w, h * 0.65),
        rgb(NIGHT, 0.85 * a),
        false,
    );
    p.fade(o, Rect::new(0.0, 0.0, w, h * 0.25), rgb(NIGHT, 0.6 * a), true);
    p.vignette(o, full, rgb(0x000000, 0.75 * a));
}

/// The logo: "SA:MP" and its line.
fn logo(p: &mut Painter, o: &mut Overlay, f: Frame, x: f32, y: f32, a: f32) {
    let r = f.r(x, y + 4.0, 6.0, 30.0);
    p.round(o, r, f.u(3.0), rgb(ACCENT, a));
    let tw = p.text(
        o,
        f.ox + f.u(x + 16.0),
        f.oy + f.u(y),
        Size::Title,
        Weight::Bold,
        rgb(TEXT, a),
        b"SA:MP",
    );
    let sub = b"San Andreas Multiplayer";
    let lh = p.line_height(Size::Title) - p.line_height(Size::Small);
    p.text(
        o,
        f.ox + f.u(x + 16.0) + tw + f.u(14.0),
        f.oy + f.u(y) + lh * 0.72,
        Size::Small,
        Weight::Regular,
        rgb(SUB, a),
        sub,
    );
}

/// A text field: its frame, the text or the placeholder, the caret when focused.
#[allow(clippy::too_many_arguments)]
fn field(
    p: &mut Painter,
    o: &mut Overlay,
    r: Rect,
    text: &str,
    placeholder: &str,
    focused: bool,
    icon: Option<Icon>,
    f: Frame,
) {
    let line = if focused {
        rgb(ACCENT, 0.9)
    } else {
        rgb(0xFFFFFF, 0.08)
    };
    p.outlined(o, r, f.u(10.0), f.u(1.5).max(1.0), line, rgb(0x0A0F1A, 0.85));
    let mut x = r.x + f.u(14.0);
    if let Some(i) = icon {
        let is = f.u(16.0);
        p.icon(o, i, x, r.y + (r.h - is) / 2.0, is, rgb(MUTED, 1.0));
        x += is + f.u(10.0);
    }
    let ty = r.y + (r.h - p.line_height(Size::Body)) / 2.0;
    let max = r.right() - x - f.u(14.0);
    let tw = if text.is_empty() {
        if !focused {
            p.text_fit(
                o,
                x,
                ty,
                Size::Body,
                Weight::Regular,
                rgb(MUTED, 1.0),
                &latin1(placeholder),
                max,
            );
        }
        0.0
    } else {
        p.text_fit(
            o,
            x,
            ty,
            Size::Body,
            Weight::Regular,
            rgb(TEXT, 1.0),
            &latin1(text),
            max,
        )
    };
    if focused && (now_us() / 530_000).is_multiple_of(2) {
        let ch = p.line_height(Size::Body) * 0.8;
        p.rect(
            o,
            Rect::new(x + tw + f.u(1.0), r.y + (r.h - ch) / 2.0, f.u(1.6).max(1.0), ch),
            rgb(ACCENT, 1.0),
        );
    }
}

/// A small rounded label.
#[allow(clippy::too_many_arguments)]
fn chip(p: &mut Painter, o: &mut Overlay, x: f32, y: f32, text: &[u8], fg: u32, bg: Rgba, f: Frame) -> Rect {
    let w = p.measure(Size::Tiny, Weight::Bold, text) + f.u(16.0);
    let r = Rect::new(x, y, w, f.u(20.0));
    p.round(o, r, f.u(6.0), bg);
    p.text(
        o,
        x + f.u(8.0),
        y + (r.h - p.line_height(Size::Tiny)) / 2.0,
        Size::Tiny,
        Weight::Bold,
        rgb(fg, 1.0),
        text,
    );
    r
}

/// The width of a [`toggle`].
fn toggle_width(p: &Painter, text: &str, on: bool, f: Frame) -> f32 {
    p.measure(Size::Small, Weight::Bold, &latin1(text)) + f.u(26.0) + if on { f.u(18.0) } else { 0.0 }
}

/// A toggle or button chip in the toolbar; returns its rectangle.
#[allow(clippy::too_many_arguments)]
fn toggle(
    p: &mut Painter,
    o: &mut Overlay,
    x: f32,
    y: f32,
    text: &str,
    on: bool,
    hover: bool,
    f: Frame,
) -> Rect {
    let t = latin1(text);
    let icon = if on { f.u(18.0) } else { 0.0 };
    let w = p.measure(Size::Small, Weight::Bold, &t) + f.u(26.0) + icon;
    let r = Rect::new(x, y, w, f.u(32.0));
    let (bg, fg) = if on {
        (rgb(ACCENT, 0.22), ACCENT)
    } else {
        (rgb(0xFFFFFF, if hover { 0.11 } else { 0.06 }), SUB)
    };
    p.round(o, r, f.u(9.0), bg);
    let mut tx = x + f.u(13.0);
    if on {
        p.icon(
            o,
            Icon::Check,
            tx - f.u(2.0),
            y + f.u(8.0),
            f.u(16.0),
            rgb(ACCENT, 1.0),
        );
        tx += icon;
    }
    p.text(
        o,
        tx,
        y + (r.h - p.line_height(Size::Small)) / 2.0,
        Size::Small,
        Weight::Bold,
        rgb(fg, 1.0),
        &t,
    );
    r
}

/// The columns of the server table: x offset and width in design units.
const COL_LOCK: (f32, f32) = (66.0, 20.0);
const COL_NAME: (f32, f32) = (92.0, 330.0);
const COL_PLAYERS: (f32, f32) = (436.0, 104.0);
const COL_PING: (f32, f32) = (548.0, 66.0);
const COL_MODE: (f32, f32) = (622.0, 140.0);
const COL_LANG: (f32, f32) = (770.0, 62.0);

/// Draws the screen.
pub fn draw(sc: &mut Screen, o: &mut Overlay) {
    let size = o.size();
    if !sc.painter.begin(size) {
        // without the fonts or images nothing but the cover can be drawn
        let _ = o.rect(0.0, 0.0, size.0 as f32, size.1 as f32, rgb(NIGHT, 1.0));
        return;
    }
    sc.hits.clear();
    backdrop(sc, o, 1.0);
    let f = frame(&sc.painter, size);
    let hover = sc.cursor;
    let p = &mut sc.painter;

    // header
    logo(p, o, f, 40.0, 26.0, 1.0);
    let refresh = f.r(1204.0, 30.0, 36.0, 36.0);
    p.round(
        o,
        refresh,
        f.u(10.0),
        rgb(0xFFFFFF, if refresh.contains(hover) { 0.12 } else { 0.06 }),
    );
    p.icon(
        o,
        Icon::Refresh,
        refresh.x + f.u(8.0),
        refresh.y + f.u(8.0),
        f.u(20.0),
        rgb(SUB, 1.0),
    );
    sc.hits.push((refresh, Hit::Refresh));
    if let Internet::Loaded(l) = &sc.internet {
        let players: u64 = l.iter().map(|s| u64::from(s.players)).sum();
        let t = latin1(&format!(
            "{} servers  /  {} players online",
            thousands(l.len() as u64),
            thousands(players)
        ));
        let tw = p.measure(Size::Small, Weight::Regular, &t);
        let ty = refresh.y + (refresh.h - p.line_height(Size::Small)) / 2.0;
        p.text(
            o,
            refresh.x - f.u(16.0) - tw,
            ty,
            Size::Small,
            Weight::Regular,
            rgb(SUB, 1.0),
            &t,
        );
        p.disc(
            o,
            refresh.x - f.u(26.0) - tw,
            refresh.y + refresh.h / 2.0,
            f.u(4.0),
            rgb(GREEN, 1.0),
        );
    }

    // tabs
    let mut x = f.u(40.0) + f.ox;
    for (tab, icon, label) in [
        (Tab::Internet, Icon::Globe, "Internet"),
        (Tab::Favourites, Icon::Star, "Favourites"),
        (Tab::Recent, Icon::Clock, "Recent"),
        (Tab::Direct, Icon::Arrow, "Direct connect"),
    ] {
        let t = latin1(label);
        let count = match tab {
            Tab::Favourites => sc.favourites.len(),
            Tab::Recent => sc.recent.len(),
            _ => 0,
        };
        let badge = (count > 0).then(|| latin1(&count.to_string()));
        let bw = badge
            .as_ref()
            .map_or(0.0, |b| p.measure(Size::Tiny, Weight::Bold, b) + f.u(18.0));
        let w = p.measure(Size::Body, Weight::Bold, &t) + f.u(52.0) + bw;
        let r = Rect::new(x, f.oy + f.u(98.0), w, f.u(40.0));
        let on = sc.tab == tab;
        let bg = if on {
            rgb(ACCENT, 1.0)
        } else {
            rgb(0xFFFFFF, if r.contains(hover) { 0.12 } else { 0.06 })
        };
        p.round(o, r, f.u(12.0), bg);
        let fg = if on { TEXT } else { SUB };
        p.icon(o, icon, x + f.u(14.0), r.y + f.u(11.0), f.u(18.0), rgb(fg, 1.0));
        let tw = p.text(
            o,
            x + f.u(40.0),
            r.y + (r.h - p.line_height(Size::Body)) / 2.0,
            Size::Body,
            Weight::Bold,
            rgb(fg, 1.0),
            &t,
        );
        if let Some(b) = badge {
            let br = Rect::new(x + f.u(48.0) + tw, r.y + f.u(11.0), bw - f.u(6.0), f.u(18.0));
            p.round(
                o,
                br,
                f.u(9.0),
                if on {
                    rgb(0xFFFFFF, 0.25)
                } else {
                    rgb(0xFFFFFF, 0.1)
                },
            );
            let bt = p.measure(Size::Tiny, Weight::Bold, &b);
            p.text(
                o,
                br.x + (br.w - bt) / 2.0,
                br.y + (br.h - p.line_height(Size::Tiny)) / 2.0,
                Size::Tiny,
                Weight::Bold,
                rgb(fg, 1.0),
                &b,
            );
        }
        sc.hits.push((r, Hit::Tab(tab)));
        x += w + f.u(10.0);
    }

    // the card
    let card = f.r(40.0, 154.0, 1200.0, 548.0);
    p.shadow(o, card, f.u(18.0), f.u(40.0), rgb(0x000000, 0.55));
    p.outlined(
        o,
        card,
        f.u(18.0),
        f.u(1.0).max(1.0),
        rgb(0xFFFFFF, 0.08),
        rgb(CARD, 0.9),
    );

    list(sc, o, f);
    details(sc, o, f);
    footer(sc, o, f);
    sc.painter.flush(o);
}

/// The toolbar and the server table.
fn list(sc: &mut Screen, o: &mut Overlay, f: Frame) {
    let hover = sc.cursor;
    let tab = sc.tab;
    let p = &mut sc.painter;
    let ty = 172.0;
    match tab {
        Tab::Internet => {
            let r = f.r(60.0, ty, 290.0, 38.0);
            field(
                p,
                o,
                r,
                &sc.search,
                "Search name, mode or address",
                sc.focus == Focus::Search,
                Some(Icon::Search),
                f,
            );
            sc.hits.push((r, Hit::Search));
            let mut x = f.ox + f.u(362.0);
            let y = f.oy + f.u(ty + 3.0);
            let sort = format!("Sort: {}", sc.sort.label());
            let long = [
                ("Hide full", sc.filters.hide_full, Hit::HideFull),
                ("Hide empty", sc.filters.hide_empty, Hit::HideEmpty),
                ("Hide locked", sc.filters.hide_locked, Hit::HideLocked),
                (sort.as_str(), false, Hit::Sort),
            ];
            let short = [
                ("Full", long[0].1, Hit::HideFull),
                ("Empty", long[1].1, Hit::HideEmpty),
                ("Locked", long[2].1, Hit::HideLocked),
                (sc.sort.short_label(), false, Hit::Sort),
            ];
            // small screens get the short labels when the long ones do not fit beside the search field
            let width = |p: &Painter, items: &[(&str, bool, Hit)]| {
                items
                    .iter()
                    .map(|(l, on, _)| toggle_width(p, l, *on, f) + f.u(8.0))
                    .sum::<f32>()
            };
            let room = f.ox + f.u(840.0) - x;
            let items = if width(p, &long) <= room { long } else { short };
            for (label, on, hit) in items {
                let probe = Rect::new(x, y, toggle_width(p, label, on, f), f.u(32.0));
                if probe.right() > f.ox + f.u(840.0) {
                    break;
                }
                let r = toggle(p, o, x, y, label, on, probe.contains(hover), f);
                sc.hits.push((r, hit));
                x = r.right() + f.u(8.0);
            }
        }
        Tab::Direct => {
            let r = f.r(60.0, ty, 520.0, 38.0);
            field(
                p,
                o,
                r,
                &sc.server,
                "HOST:PORT, e.g. 127.0.0.1:7777",
                sc.focus == Focus::Address,
                Some(Icon::Arrow),
                f,
            );
            sc.hits.push((r, Hit::Address));
        }
        Tab::Favourites | Tab::Recent => {
            let (title, hint) = if tab == Tab::Favourites {
                (
                    "Favourite servers",
                    "Insert adds the selected server, Delete removes it",
                )
            } else {
                ("Recently joined", "Newest first. Delete forgets a server")
            };
            let x = f.ox + f.u(62.0);
            let tw = p.text(
                o,
                x,
                f.oy + f.u(ty + 7.0),
                Size::Heading,
                Weight::Bold,
                rgb(TEXT, 1.0),
                &latin1(title),
            );
            let dy = p.line_height(Size::Heading) - p.line_height(Size::Small);
            p.text(
                o,
                x + tw + f.u(14.0),
                f.oy + f.u(ty + 7.0) + dy * 0.7,
                Size::Small,
                Weight::Regular,
                rgb(MUTED, 1.0),
                &latin1(hint),
            );
        }
    }

    // the table
    let head_y = 224.0;
    let hy = f.oy + f.u(head_y) + (f.u(24.0) - p.line_height(Size::Tiny)) / 2.0;
    for (col, label) in [
        (COL_NAME, "SERVER"),
        (COL_PLAYERS, "PLAYERS"),
        (COL_PING, "PING"),
        (COL_MODE, "MODE"),
        (COL_LANG, "LANG"),
    ] {
        p.text(
            o,
            f.ox + f.u(col.0),
            hy,
            Size::Tiny,
            Weight::Bold,
            rgb(MUTED, 1.0),
            &latin1(label),
        );
    }
    p.rect(o, f.r(60.0, head_y + 26.0, 780.0, 1.0), rgb(0xFFFFFF, 0.07));
    let top = head_y + 30.0;
    let area = f.r(60.0, top, 780.0, 682.0 - top);
    sc.page = ((682.0 - top) / ROW_H).floor() as usize;
    sc.hits.push((area, Hit::List));
    let rows = sc.rows();
    let t = sc.tab_index();
    let (first, sel) = (sc.scroll[t].min(rows.len().saturating_sub(1)), sc.selected[t]);
    if rows.is_empty() {
        empty(sc, o, f, area);
        return;
    }
    let p = &mut sc.painter;
    p.clip(o, Some(area));
    for (k, s) in rows.iter().enumerate().skip(first).take(sc.page + 1) {
        let y = top + (k - first) as f32 * ROW_H;
        let r = f.r(60.0, y, 774.0, ROW_H - 2.0);
        let selected = k == sel;
        if selected {
            p.round(
                o,
                r,
                f.u(8.0),
                rgb(ACCENT, if sc.focus == Focus::List { 0.24 } else { 0.14 }),
            );
            p.round(
                o,
                f.r(60.0, y + 7.0, 3.0, ROW_H - 16.0),
                f.u(1.5),
                rgb(ACCENT, 1.0),
            );
        } else if r.contains(hover) {
            p.round(o, r, f.u(8.0), rgb(0xFFFFFF, 0.05));
        } else if k % 2 == 1 {
            p.round(o, r, f.u(8.0), rgb(0xFFFFFF, 0.02));
        }
        sc.hits.push((r, Hit::Row(k)));
        row(
            p,
            o,
            f,
            y,
            s,
            sc.querier.live(&s.address),
            sc.querier.access(&s.address),
            selected,
        );
    }
    p.clip(o, None);
    // the scrollbar
    let n = rows.len();
    if n > sc.page {
        let track = f.r(844.0, top, 4.0, 682.0 - top);
        p.round(o, track, f.u(2.0), rgb(0xFFFFFF, 0.05));
        let h = (track.h * sc.page as f32 / n as f32).max(f.u(24.0));
        let y = track.y + (track.h - h) * first as f32 / (n - sc.page) as f32;
        p.round(
            o,
            Rect::new(track.x, y, track.w, h),
            f.u(2.0),
            rgb(0xFFFFFF, 0.28),
        );
    }
}

/// One row of the table.
#[allow(clippy::too_many_arguments)]
fn row(
    p: &mut Painter,
    o: &mut Overlay,
    f: Frame,
    y: f32,
    s: &Server,
    live: Option<&super::query::Live>,
    access: Access,
    selected: bool,
) {
    let mid = |p: &Painter, size: Size| f.oy + f.u(y + (ROW_H - 2.0) / 2.0) - p.line_height(size) / 2.0;
    if s.password {
        p.icon(
            o,
            Icon::Lock,
            f.ox + f.u(COL_LOCK.0),
            f.oy + f.u(y + 8.0),
            f.u(16.0),
            rgb(AMBER, 1.0),
        );
    }
    let weight = if selected { Weight::Bold } else { Weight::Regular };
    let x = f.ox + f.u(COL_NAME.0);
    if s.known() {
        p.text_fit(
            o,
            x,
            mid(p, Size::Body),
            Size::Body,
            weight,
            rgb(TEXT, 1.0),
            &s.hostname,
            f.u(COL_NAME.1),
        );
    } else {
        let w = p.text(
            o,
            x,
            mid(p, Size::Body),
            Size::Body,
            weight,
            rgb(SUB, 1.0),
            s.address.as_bytes(),
        );
        let note = match (access, live.is_some_and(|l| l.silent)) {
            (_, true) => "no answer",
            (Access::Allowed, false) => "querying...",
            (Access::Asking, _) => "waiting for your answer",
            (Access::Denied, _) => "not allowed",
            (Access::Unknown, _) => "not queried yet",
        };
        p.text(
            o,
            x + w + f.u(10.0),
            mid(p, Size::Small),
            Size::Small,
            Weight::Regular,
            rgb(MUTED, 1.0),
            &latin1(note),
        );
    }
    if s.known() {
        let t = latin1(&format!("{}/{}", s.players, s.max_players));
        let px = f.ox + f.u(COL_PLAYERS.0);
        let tw = p.text(
            o,
            px,
            mid(p, Size::Small),
            Size::Small,
            Weight::Bold,
            rgb(if s.players > 0 { TEXT } else { SUB }, 1.0),
            &t,
        );
        let bar = Rect::new(px + tw + f.u(8.0), f.oy + f.u(y + 14.0), f.u(34.0), f.u(4.0));
        if bar.right() < f.ox + f.u(COL_PLAYERS.0 + COL_PLAYERS.1) {
            p.round(o, bar, f.u(2.0), rgb(0xFFFFFF, 0.08));
            let k = if s.max_players > 0 {
                (f32::from(s.players) / f32::from(s.max_players)).min(1.0)
            } else {
                0.0
            };
            if k > 0.0 {
                p.round(
                    o,
                    Rect::new(bar.x, bar.y, (bar.w * k).max(bar.h), bar.h),
                    f.u(2.0),
                    rgb(ACCENT, 1.0),
                );
            }
        }
        p.text_fit(
            o,
            f.ox + f.u(COL_MODE.0),
            mid(p, Size::Small),
            Size::Small,
            Weight::Regular,
            rgb(SUB, 1.0),
            &s.gamemode,
            f.u(COL_MODE.1),
        );
        p.text_fit(
            o,
            f.ox + f.u(COL_LANG.0),
            mid(p, Size::Small),
            Size::Small,
            Weight::Regular,
            rgb(SUB, 1.0),
            &s.language,
            f.u(COL_LANG.1),
        );
    }
    let px = f.ox + f.u(COL_PING.0);
    match live.and_then(|l| l.ping) {
        Some(ms) => {
            p.disc(
                o,
                px + f.u(4.0),
                f.oy + f.u(y + 16.0),
                f.u(3.5),
                rgb(ping_colour(ms), 1.0),
            );
            p.text(
                o,
                px + f.u(13.0),
                mid(p, Size::Small),
                Size::Small,
                Weight::Bold,
                rgb(TEXT, 1.0),
                &latin1(&ms.to_string()),
            );
        }
        None => {
            p.text(
                o,
                px + f.u(13.0),
                mid(p, Size::Small),
                Size::Small,
                Weight::Regular,
                rgb(MUTED, 1.0),
                b"-",
            );
        }
    }
}

/// What the table shows without rows.
fn empty(sc: &mut Screen, o: &mut Overlay, f: Frame, area: Rect) {
    let (icon, title, line, tone) = match (sc.tab, &sc.internet) {
        (Tab::Internet, Internet::Loading(_) | Internet::NotLoaded) => (
            Icon::Globe,
            "Loading the server list".to_string(),
            sc.list_url.clone(),
            SUB,
        ),
        (Tab::Internet, Internet::Failed(e)) => (
            Icon::Globe,
            "The server list is not available".to_string(),
            format!("{e}. F5 tries again."),
            RED,
        ),
        (Tab::Internet, _) => (
            Icon::Search,
            "No server matches".to_string(),
            "Change the search or the filters.".to_string(),
            SUB,
        ),
        (Tab::Favourites, _) => (
            Icon::Star,
            "No favourites yet".to_string(),
            "Select a server in any tab and press Insert or click the star.".to_string(),
            SUB,
        ),
        (Tab::Recent, _) => (
            Icon::Clock,
            "No recent servers".to_string(),
            "The servers you join appear here.".to_string(),
            SUB,
        ),
        (Tab::Direct, _) => (
            Icon::Arrow,
            "Type an address".to_string(),
            "HOST:PORT, then Enter connects.".to_string(),
            SUB,
        ),
    };
    let p = &mut sc.painter;
    let cx = area.x + area.w / 2.0;
    let mut y = area.y + area.h * 0.28;
    let is = f.u(46.0);
    p.disc(o, cx, y + is / 2.0, is, rgb(0xFFFFFF, 0.04));
    let spin = matches!(sc.internet, Internet::Loading(_)) && sc.tab == Tab::Internet;
    if spin {
        spinner(p, o, cx, y + is / 2.0, f.u(20.0), 1.0);
    } else {
        p.icon(o, icon, cx - is / 2.0, y, is, rgb(MUTED, 1.0));
    }
    y += is + f.u(26.0);
    let t = latin1(&title);
    let tw = p.measure(Size::Heading, Weight::Bold, &t);
    p.text(
        o,
        cx - tw / 2.0,
        y,
        Size::Heading,
        Weight::Bold,
        rgb(TEXT, 1.0),
        &t,
    );
    y += p.line_height(Size::Heading) + f.u(6.0);
    let l = latin1(&line);
    let lw = p
        .measure(Size::Small, Weight::Regular, &l)
        .min(area.w - f.u(40.0));
    p.text_fit(
        o,
        cx - lw / 2.0,
        y,
        Size::Small,
        Weight::Regular,
        rgb(tone, 1.0),
        &l,
        area.w - f.u(40.0),
    );
}

/// Eight dots turning around (`cx`, `cy`).
fn spinner(p: &mut Painter, o: &mut Overlay, cx: f32, cy: f32, r: f32, a: f32) {
    let t = now_us() as f32 / 1_000_000.0;
    for i in 0..8 {
        let ang = std::f32::consts::TAU * i as f32 / 8.0;
        let phase = (t * 1.4 - i as f32 / 8.0).rem_euclid(1.0);
        p.disc(
            o,
            cx + r * ang.cos(),
            cy + r * ang.sin(),
            r * 0.16,
            rgb(ACCENT, a * (0.2 + 0.8 * (1.0 - phase))),
        );
    }
}

/// The selected server's panel, the nickname and Connect.
fn details(sc: &mut Screen, o: &mut Overlay, f: Frame) {
    let panel = f.r(860.0, 172.0, 362.0, 512.0);
    let hover = sc.cursor;
    let selected = sc.selected_address();
    let server = selected.as_ref().map(|a| {
        sc.rows()
            .into_iter()
            .find(|s| &s.address == a)
            .unwrap_or_else(|| Server::unknown(a))
    });
    let live = selected.as_deref().and_then(|a| sc.querier.live(a)).cloned();
    let access = selected.as_deref().map_or(Access::Unknown, |a| sc.access(a));
    let favourite = selected.as_ref().is_some_and(|a| sc.favourites.contains(a));
    let p = &mut sc.painter;
    p.outlined(
        o,
        panel,
        f.u(14.0),
        f.u(1.0).max(1.0),
        rgb(0xFFFFFF, 0.06),
        rgb(0xFFFFFF, 0.03),
    );
    let x = 880.0;
    let inner = 322.0;
    match &server {
        None => {
            let t = latin1("No server selected");
            p.text(
                o,
                f.ox + f.u(x),
                f.oy + f.u(196.0),
                Size::Heading,
                Weight::Bold,
                rgb(SUB, 1.0),
                &t,
            );
            let l = latin1("Pick one in the list, or type an address in Direct connect.");
            p.text_fit(
                o,
                f.ox + f.u(x),
                f.oy + f.u(228.0),
                Size::Small,
                Weight::Regular,
                rgb(MUTED, 1.0),
                &l,
                f.u(inner),
            );
        }
        Some(s) => {
            // badges
            let mut bx = f.ox + f.u(x);
            let by = f.oy + f.u(192.0);
            let version = live
                .as_ref()
                .and_then(|l| {
                    l.rules
                        .iter()
                        .find(|(k, _)| k == b"version")
                        .map(|(_, v)| v.clone())
                })
                .unwrap_or_else(|| s.version.clone());
            if s.omp {
                bx = chip(p, o, bx, by, b"open.mp", GREEN, rgb(GREEN, 0.14), f).right() + f.u(6.0);
            }
            if !version.is_empty() {
                bx = chip(p, o, bx, by, &version, SUB, rgb(0xFFFFFF, 0.08), f).right() + f.u(6.0);
            }
            if s.password {
                bx = chip(p, o, bx, by, b"Password", AMBER, rgb(AMBER, 0.14), f).right() + f.u(6.0);
            }
            if !s.language.is_empty() {
                chip(p, o, bx, by, &s.language, SUB, rgb(0xFFFFFF, 0.08), f);
            }
            let name: &[u8] = if s.known() {
                &s.hostname
            } else {
                s.address.as_bytes()
            };
            p.text_fit(
                o,
                f.ox + f.u(x),
                f.oy + f.u(222.0),
                Size::Heading,
                Weight::Bold,
                rgb(TEXT, 1.0),
                name,
                f.u(inner),
            );
            p.icon(
                o,
                Icon::Globe,
                f.ox + f.u(x),
                f.oy + f.u(254.0),
                f.u(14.0),
                rgb(MUTED, 1.0),
            );
            p.text(
                o,
                f.ox + f.u(x + 20.0),
                f.oy + f.u(252.0),
                Size::Small,
                Weight::Regular,
                rgb(SUB, 1.0),
                s.address.as_bytes(),
            );

            // the figures
            let tiles = [
                (
                    "PLAYERS",
                    if s.known() {
                        format!("{}/{}", s.players, s.max_players)
                    } else {
                        "-".into()
                    },
                    TEXT,
                ),
                (
                    "PING",
                    live.as_ref()
                        .and_then(|l| l.ping)
                        .map_or("-".into(), |m| format!("{m} ms")),
                    live.as_ref().and_then(|l| l.ping).map_or(SUB, ping_colour),
                ),
            ];
            for (i, (label, value, colour)) in tiles.iter().enumerate() {
                let r = f.r(x + i as f32 * 106.0, 282.0, 98.0, 58.0);
                p.round(o, r, f.u(10.0), rgb(0xFFFFFF, 0.05));
                p.text(
                    o,
                    r.x + f.u(12.0),
                    r.y + f.u(9.0),
                    Size::Tiny,
                    Weight::Bold,
                    rgb(MUTED, 1.0),
                    &latin1(label),
                );
                let v = latin1(value);
                let room = r.w - f.u(18.0);
                let size = if p.measure(Size::Heading, Weight::Bold, &v) <= room {
                    Size::Heading
                } else {
                    Size::Body
                };
                let vy = r.y + f.u(40.0) - p.line_height(size) / 2.0;
                p.text_fit(
                    o,
                    r.x + f.u(12.0),
                    vy,
                    size,
                    Weight::Bold,
                    rgb(*colour, 1.0),
                    &v,
                    room,
                );
            }
            let r = f.r(x + 212.0, 282.0, 110.0, 58.0);
            p.round(o, r, f.u(10.0), rgb(0xFFFFFF, 0.05));
            p.text(
                o,
                r.x + f.u(12.0),
                r.y + f.u(9.0),
                Size::Tiny,
                Weight::Bold,
                rgb(MUTED, 1.0),
                b"MODE",
            );
            let gm: &[u8] = if s.gamemode.is_empty() { b"-" } else { &s.gamemode };
            p.text_fit(
                o,
                r.x + f.u(12.0),
                r.y + f.u(29.0),
                Size::Small,
                Weight::Bold,
                rgb(TEXT, 1.0),
                gm,
                r.w - f.u(18.0),
            );

            // live details, or the question
            let silent = live.as_ref().is_some_and(|l| l.silent && l.info.is_none());
            if let Some(l) = live
                .as_ref()
                .filter(|l| access == Access::Allowed && l.info.is_some())
            {
                live_details(p, o, f, x, inner, l);
            } else {
                let (title, line, button) = match access {
                    Access::Asking => (
                        "Waiting for your answer",
                        "The game is asking whether SA:MP may contact this server.",
                        None,
                    ),
                    Access::Denied => (
                        "Not allowed",
                        "You chose not to let SA:MP contact this server.",
                        None,
                    ),
                    Access::Allowed if silent => (
                        "No answer",
                        "The server did not answer: it may be offline. F5 asks again.",
                        None,
                    ),
                    Access::Allowed => (
                        "Contacting the server",
                        "Asking for its players and rules...",
                        None,
                    ),
                    Access::Unknown => (
                        "Live details",
                        "Players, ping and rules need SA:MP to contact the server. The game asks you once.",
                        Some("Query server (Space)"),
                    ),
                };
                let pad = f.u(16.0);
                let lines = wrap_lines(p, f.u(inner) - 2.0 * pad, Size::Small, line);
                let lh = p.line_height(Size::Small) * 1.15;
                let head = f.u(40.0);
                let button_h = if button.is_some() {
                    f.u(34.0) + f.u(12.0)
                } else {
                    0.0
                };
                let r = Rect::new(
                    f.ox + f.u(x),
                    f.oy + f.u(356.0),
                    f.u(inner),
                    head + lines.len() as f32 * lh + button_h + pad,
                );
                p.round(o, r, f.u(12.0), rgb(ACCENT, 0.08));
                p.icon(
                    o,
                    Icon::Lock,
                    r.x + pad,
                    r.y + f.u(14.0),
                    f.u(18.0),
                    rgb(ACCENT, 1.0),
                );
                let ty = r.y + f.u(23.0) - p.line_height(Size::Body) / 2.0;
                p.text(
                    o,
                    r.x + f.u(42.0),
                    ty,
                    Size::Body,
                    Weight::Bold,
                    rgb(TEXT, 1.0),
                    &latin1(title),
                );
                for (i, l) in lines.iter().enumerate() {
                    p.text(
                        o,
                        r.x + pad,
                        r.y + head + i as f32 * lh,
                        Size::Small,
                        Weight::Regular,
                        rgb(SUB, 1.0),
                        l,
                    );
                }
                if let Some(b) = button {
                    let t = latin1(b);
                    let br = Rect::new(
                        r.x + pad,
                        r.bottom() - pad - f.u(34.0),
                        p.measure(Size::Small, Weight::Bold, &t) + f.u(28.0),
                        f.u(34.0),
                    );
                    p.round(
                        o,
                        br,
                        f.u(9.0),
                        rgb(ACCENT, if br.contains(hover) { 0.45 } else { 0.3 }),
                    );
                    p.text(
                        o,
                        br.x + f.u(14.0),
                        br.y + (br.h - p.line_height(Size::Small)) / 2.0,
                        Size::Small,
                        Weight::Bold,
                        rgb(TEXT, 1.0),
                        &t,
                    );
                    sc.hits.push((br, Hit::Query));
                }
            }
        }
    }

    // the nickname and Connect
    let p = &mut sc.painter;
    p.text(
        o,
        f.ox + f.u(x),
        f.oy + f.u(552.0),
        Size::Tiny,
        Weight::Bold,
        rgb(MUTED, 1.0),
        b"NICKNAME",
    );
    let nr = f.r(x, 570.0, inner, 40.0);
    field(
        p,
        o,
        nr,
        &sc.name,
        "Your name in the game",
        sc.focus == Focus::Name,
        Some(Icon::User),
        f,
    );
    sc.hits.push((nr, Hit::Name));
    let cr = f.r(x, 622.0, inner - 56.0, 46.0);
    let hot = cr.contains(hover);
    p.shadow(
        o,
        cr,
        f.u(12.0),
        f.u(16.0),
        rgb(ACCENT, if hot { 0.45 } else { 0.3 }),
    );
    p.button(
        o,
        cr,
        if hot {
            rgb(0xFFFFFF, 1.0)
        } else {
            rgb(0xE8ECF5, 1.0)
        },
    );
    let t = b"Connect";
    let tw = p.measure(Size::Body, Weight::Bold, t);
    let total = tw + f.u(28.0);
    let tx = cr.x + (cr.w - total) / 2.0;
    p.text(
        o,
        tx,
        cr.y + (cr.h - p.line_height(Size::Body)) / 2.0,
        Size::Body,
        Weight::Bold,
        rgb(0xFFFFFF, 1.0),
        t,
    );
    p.icon(
        o,
        Icon::Arrow,
        tx + tw + f.u(10.0),
        cr.y + f.u(14.0),
        f.u(18.0),
        rgb(0xFFFFFF, 1.0),
    );
    sc.hits.push((cr, Hit::Connect));
    let fr = f.r(x + inner - 46.0, 622.0, 46.0, 46.0);
    p.round(
        o,
        fr,
        f.u(12.0),
        rgb(
            if favourite { AMBER } else { 0xFFFFFF },
            if favourite {
                0.16
            } else if fr.contains(hover) {
                0.12
            } else {
                0.06
            },
        ),
    );
    p.icon(
        o,
        Icon::Star,
        fr.x + f.u(12.0),
        fr.y + f.u(12.0),
        f.u(22.0),
        rgb(if favourite { AMBER } else { MUTED }, 1.0),
    );
    sc.hits.push((fr, Hit::Favourite));
}

/// Rules and players of a queried server.
fn live_details(p: &mut Painter, o: &mut Overlay, f: Frame, x: f32, inner: f32, l: &super::query::Live) {
    let mut y = 356.0;
    let rules: Vec<&(Vec<u8>, Vec<u8>)> = ["weather", "worldtime", "mapname", "weburl"]
        .iter()
        .filter_map(|k| l.rules.iter().find(|(n, _)| n == k.as_bytes()))
        .collect();
    for (k, v) in rules {
        let label: &[u8] = match k.as_slice() {
            b"worldtime" => b"Time",
            b"mapname" => b"Map",
            b"weburl" => b"Website",
            _ => b"Weather",
        };
        p.text(
            o,
            f.ox + f.u(x),
            f.oy + f.u(y),
            Size::Small,
            Weight::Regular,
            rgb(MUTED, 1.0),
            label,
        );
        let vw = p.measure(Size::Small, Weight::Bold, v).min(f.u(inner - 90.0));
        p.text_fit(
            o,
            f.ox + f.u(x + inner) - vw,
            f.oy + f.u(y),
            Size::Small,
            Weight::Bold,
            rgb(TEXT, 1.0),
            v,
            f.u(inner - 90.0),
        );
        y += 21.0;
    }
    y += 8.0;
    let players = l.clients.as_deref().unwrap_or(&[]);
    let head = latin1(&format!(
        "PLAYERS ONLINE ({})",
        l.info.as_ref().map_or(players.len(), |i| usize::from(i.players))
    ));
    p.text(
        o,
        f.ox + f.u(x),
        f.oy + f.u(y),
        Size::Tiny,
        Weight::Bold,
        rgb(MUTED, 1.0),
        &head,
    );
    y += 20.0;
    if players.is_empty() {
        let t: &[u8] = if l.clients.is_some() {
            b"Nobody is playing right now."
        } else {
            b"The list is not available."
        };
        p.text(
            o,
            f.ox + f.u(x),
            f.oy + f.u(y),
            Size::Small,
            Weight::Regular,
            rgb(SUB, 1.0),
            t,
        );
        return;
    }
    let rows = ((540.0 - y) / 19.0).floor().max(1.0) as usize;
    let col = inner / 2.0;
    for (i, (name, score)) in players.iter().take(rows * 2).enumerate() {
        let (cx, cy) = (x + (i / rows) as f32 * col, y + (i % rows) as f32 * 19.0);
        if i == rows * 2 - 1 && players.len() > rows * 2 {
            let more = latin1(&format!("+{} more", players.len() - i));
            p.text(
                o,
                f.ox + f.u(cx),
                f.oy + f.u(cy),
                Size::Small,
                Weight::Bold,
                rgb(ACCENT, 1.0),
                &more,
            );
            break;
        }
        p.disc(
            o,
            f.ox + f.u(cx + 4.0),
            f.oy + f.u(cy + 8.0),
            f.u(3.0),
            rgb(GREEN, 1.0),
        );
        p.text_fit(
            o,
            f.ox + f.u(cx + 14.0),
            f.oy + f.u(cy),
            Size::Small,
            Weight::Regular,
            rgb(TEXT, 1.0),
            name,
            f.u(col - 60.0),
        );
        let sc = latin1(&score.to_string());
        let sw = p.measure(Size::Small, Weight::Regular, &sc);
        p.text(
            o,
            f.ox + f.u(cx + col - 12.0) - sw,
            f.oy + f.u(cy),
            Size::Small,
            Weight::Regular,
            rgb(MUTED, 1.0),
            &sc,
        );
    }
}

/// `text` broken into lines of at most `w` pixels.
fn wrap_lines(p: &Painter, w: f32, size: Size, text: &str) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if p.measure(size, Weight::Regular, &latin1(&candidate)) > w && !line.is_empty() {
            lines.push(latin1(&std::mem::replace(&mut line, word.to_string())));
        } else {
            line = candidate;
        }
    }
    lines.push(latin1(&line));
    lines
}

/// The key hints and the status line.
fn footer(sc: &mut Screen, o: &mut Overlay, f: Frame) {
    let p = &mut sc.painter;
    let y = f.oy + f.u(716.0);
    let mut x = f.ox + f.u(42.0);
    let hints: &[(&str, &str)] = match sc.tab {
        Tab::Internet => &[
            ("Enter", "Connect"),
            ("Left/Right", "Tabs"),
            ("Up/Down", "Select"),
            ("Ins", "Favourite"),
            ("Space", "Query"),
            ("F2-F4", "Filters"),
            ("F5", "Refresh"),
            ("F6", "Sort"),
        ],
        Tab::Direct => &[
            ("Enter", "Connect"),
            ("Tab", "Nickname"),
            ("Left/Right", "Tabs"),
            ("Ins", "Favourite"),
            ("Space", "Query"),
        ],
        _ => &[
            ("Enter", "Connect"),
            ("Tab", "Nickname"),
            ("Left/Right", "Tabs"),
            ("Up/Down", "Select"),
            ("Ins", "Favourite"),
            ("Del", "Remove"),
            ("Space", "Query"),
            ("F5", "Refresh"),
        ],
    };
    let status_w = sc.status.as_ref().map_or(0.0, |(_, t)| {
        p.measure(Size::Small, Weight::Bold, &latin1(t)) + f.u(40.0)
    });
    let limit = f.ox + f.u(1240.0) - status_w;
    for (key, label) in hints {
        let k = latin1(key);
        let l = latin1(label);
        let need =
            p.measure(Size::Tiny, Weight::Bold, &k) + p.measure(Size::Small, Weight::Regular, &l) + f.u(40.0);
        if x + need > limit {
            break;
        }
        let r = chip(p, o, x, y, &k, TEXT, rgb(0xFFFFFF, 0.1), f);
        let lw = p.text(
            o,
            r.right() + f.u(7.0),
            y + (r.h - p.line_height(Size::Small)) / 2.0,
            Size::Small,
            Weight::Regular,
            rgb(SUB, 1.0),
            &l,
        );
        x = r.right() + lw + f.u(20.0);
    }
    if let Some((tone, text)) = &sc.status {
        let colour = match tone {
            Tone::Info => GREEN,
            Tone::Warning => AMBER,
            Tone::Error => RED,
        };
        let t = latin1(text);
        let tw = p.measure(Size::Small, Weight::Bold, &t);
        let r = Rect::new(
            f.ox + f.u(1240.0) - tw - f.u(36.0),
            y - f.u(4.0),
            tw + f.u(36.0),
            f.u(28.0),
        );
        p.round(o, r, f.u(14.0), rgb(colour, 0.14));
        p.disc(o, r.x + f.u(14.0), r.y + r.h / 2.0, f.u(4.0), rgb(colour, 1.0));
        p.text(
            o,
            r.x + f.u(26.0),
            r.y + (r.h - p.line_height(Size::Small)) / 2.0,
            Size::Small,
            Weight::Bold,
            rgb(colour, 1.0),
            &t,
        );
    }
}

/// The backdrop and a card saying what the game is doing.
pub fn draw_progress(sc: &mut Screen, o: &mut Overlay, pr: &Progress<'_>, a: f32) {
    let size = o.size();
    if !sc.painter.begin(size) {
        let _ = o.rect(0.0, 0.0, size.0 as f32, size.1 as f32, rgb(NIGHT, a));
        return;
    }
    backdrop(sc, o, a);
    let f = frame(&sc.painter, size);
    let p = &mut sc.painter;
    logo(p, o, f, 40.0, 26.0, a);
    let (title, server, line, cancel) = match pr {
        Progress::Approval { server } => (
            "Waiting for your permission",
            *server,
            "Answer the game's question above: Y allows SA:MP to reach this server.".to_string(),
            false,
        ),
        Progress::Joining {
            server,
            since_us,
            message,
        } => {
            let secs = now_us().saturating_sub(*since_us) / 1_000_000;
            let line = match message {
                Some(m) => m.clone(),
                None => format!("Connecting... {secs} s"),
            };
            ("Joining the server", *server, line, true)
        }
    };
    let card = Rect::new(
        f.w / 2.0 - f.u(280.0),
        f.h * 0.6 - f.u(100.0),
        f.u(560.0),
        f.u(200.0),
    );
    p.shadow(o, card, f.u(18.0), f.u(40.0), rgb(0x000000, 0.55 * a));
    p.outlined(
        o,
        card,
        f.u(18.0),
        f.u(1.0).max(1.0),
        rgb(0xFFFFFF, 0.08 * a),
        rgb(CARD, 0.88 * a),
    );
    spinner(p, o, card.x + f.u(56.0), card.y + f.u(64.0), f.u(18.0), a);
    let tx = card.x + f.u(100.0);
    p.text(
        o,
        tx,
        card.y + f.u(36.0),
        Size::Heading,
        Weight::Bold,
        rgb(TEXT, a),
        &latin1(title),
    );
    p.text(
        o,
        tx,
        card.y + f.u(68.0),
        Size::Body,
        Weight::Regular,
        rgb(ACCENT, a),
        server.as_bytes(),
    );
    p.text_fit(
        o,
        tx,
        card.y + f.u(100.0),
        Size::Small,
        Weight::Regular,
        rgb(SUB, a),
        &latin1(&line),
        card.w - f.u(124.0),
    );
    if cancel {
        let y = card.bottom() - f.u(44.0);
        let r = chip(p, o, tx, y, b"Esc", TEXT, rgb(0xFFFFFF, 0.1 * a), f);
        p.text(
            o,
            r.right() + f.u(8.0),
            y + (r.h - p.line_height(Size::Small)) / 2.0,
            Size::Small,
            Weight::Regular,
            rgb(SUB, a),
            b"Cancel",
        );
    }
    p.flush(o);
}
