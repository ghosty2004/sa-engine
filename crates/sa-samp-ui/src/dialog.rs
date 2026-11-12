//! Dialogs (RPC 61, `ShowPlayerDialog`): message box, input, password, list, tab list and tab list with
//! headers. Geometry from samp.dll 0x1006f500 (samp-rs samp-ui spec); keyboard and mouse navigation of the
//! DXUT controls; the response (`OnDialogResponse`, RPC 62).

use sa_samp_api::ui::{DialogSpec, DialogStyle, UiEvent};

use crate::colour::strip_codes;
use crate::draw::{DrawList, OUTLINE_4};
use crate::edit::{EditKey, LineEdit};
use crate::font::{FontId, Measure};

/// Button size.
pub const BUTTON_W: f32 = 100.0;
/// Button size.
pub const BUTTON_H: f32 = 30.0;
/// List box border.
pub const LIST_BORDER: f32 = 6.0;
/// List scrollbar width.
pub const LIST_SCROLLBAR_W: f32 = 16.0;
/// List height range.
pub const LIST_H: (f32, f32) = (200.0, 400.0);
/// List width range.
pub const LIST_W: (f32, f32) = (400.0, 800.0);
/// Minimum dialog width (two buttons).
pub const MIN_W: f32 = 2.0 * BUTTON_W + 30.0;
/// Edit capacity and response text limit.
pub const TEXT_MAX: usize = 128;
/// Columns of a tab list.
pub const MAX_COLUMNS: usize = 4;
/// Background.
pub const BACKGROUND: u32 = 0xDC05_0505;
/// Caption text.
pub const CAPTION_COLOUR: u32 = 0xFFD2_D2D2;
/// Body / header text.
pub const BODY_COLOUR: u32 = 0xFFA9_C4E4;
/// List row text.
pub const ROW_COLOUR: u32 = 0xFFFF_FFFF;
/// Selected row background.
pub const SELECTION_COLOUR: u32 = 0xFFB9_2228;
/// Double-click interval.
pub const DOUBLE_CLICK_MS: u32 = 500;

/// Dialog geometry in screen pixels (`[left, top, right, bottom]`).
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// Whole dialog.
    pub rect: [f32; 4],
    /// Caption height.
    pub caption_h: f32,
    /// Body text origin (styles 0, 1, 3).
    pub body: Option<(f32, f32)>,
    /// List box.
    pub list: Option<[f32; 4]>,
    /// Row height.
    pub row_h: f32,
    /// Column widths (tab lists).
    pub columns: Vec<f32>,
    /// Header row origin (style 5).
    pub header: Option<(f32, f32)>,
    /// Edit box.
    pub edit: Option<[f32; 4]>,
    /// Left button.
    pub button1: [f32; 4],
    /// Right button.
    pub button2: Option<[f32; 4]>,
}

fn contains(r: &[f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && x < r[2] && y >= r[1] && y < r[3]
}

/// An open dialog.
#[derive(Clone, Debug)]
pub struct Dialog {
    spec: DialogSpec,
    rows: Vec<Vec<String>>,
    header: Vec<String>,
    columns: usize,
    selected: usize,
    top: usize,
    edit: LineEdit,
    last_click: Option<(usize, u32)>,
}

impl Dialog {
    /// Opens `spec`: list rows split at `\n` (empty rows skipped), cells at `\t`; style 5's first row is the
    /// header.
    pub fn new(spec: &DialogSpec) -> Self {
        let mut rows: Vec<Vec<String>> = spec
            .body
            .split('\n')
            .filter(|r| !r.is_empty())
            .map(|r| r.split('\t').take(MAX_COLUMNS).map(str::to_string).collect())
            .collect();
        let header = if spec.style == DialogStyle::TablistHeaders && !rows.is_empty() {
            rows.remove(0)
        } else {
            Vec::new()
        };
        let columns = match spec.style {
            DialogStyle::Tablist | DialogStyle::TablistHeaders => {
                header.len().max(rows.iter().map(Vec::len).max().unwrap_or(1))
            }
            _ => 1,
        };
        Self {
            spec: spec.clone(),
            rows,
            header,
            columns,
            selected: 0,
            top: 0,
            edit: LineEdit::new(TEXT_MAX),
            last_click: None,
        }
    }

    /// The spec.
    pub fn spec(&self) -> &DialogSpec {
        &self.spec
    }

    fn is_list(&self) -> bool {
        matches!(
            self.spec.style,
            DialogStyle::List | DialogStyle::Tablist | DialogStyle::TablistHeaders
        )
    }

    fn has_edit(&self) -> bool {
        matches!(self.spec.style, DialogStyle::Input | DialogStyle::Password)
    }

    fn has_button2(&self) -> bool {
        !self.spec.button2.is_empty()
    }

    /// List rows.
    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }

    /// Selected row.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// First visible row.
    pub fn top(&self) -> usize {
        self.top
    }

    /// The input edit.
    pub fn edit_mut(&mut self) -> &mut LineEdit {
        &mut self.edit
    }

    /// Geometry for a `w × h` screen.
    pub fn layout(&self, m: &dyn Measure, w: f32, h: f32) -> Layout {
        let main_h = m.line_height(FontId::Main);
        let small_h = m.line_height(FontId::Small);
        let h0 = m.line_height(FontId::Gui0);
        let h1 = m.line_height(FontId::Gui1);
        let c = if self.spec.title.is_empty() {
            0.0
        } else {
            main_h + 4.0
        };
        let list_h = (self.rows.len() as f32 * h1 + 2.0 * LIST_BORDER).clamp(LIST_H.0, LIST_H.1);
        let edit_h = (1.5 * h0).round();
        let mut columns = Vec::new();
        let mut header_h = 0.0;
        let (mut dw, dh) = match self.spec.style {
            DialogStyle::List => {
                let widest = self
                    .rows
                    .iter()
                    .map(|r| m.width(FontId::Gui1, &strip_codes(&r[0])))
                    .fold(0.0f32, f32::max);
                (
                    (widest + 60.0).clamp(LIST_W.0, LIST_W.1),
                    c + BUTTON_H + 15.0 + list_h,
                )
            }
            DialogStyle::Tablist | DialogStyle::TablistHeaders => {
                let mut raw = [0.0f32; MAX_COLUMNS];
                for row in std::iter::once(&self.header).chain(self.rows.iter()) {
                    for (k, cell) in row.iter().enumerate().take(MAX_COLUMNS) {
                        raw[k] = raw[k].max(m.width(FontId::Gui1, &strip_codes(cell)));
                    }
                }
                let total: f32 = raw.iter().sum::<f32>() + 80.0;
                let n = self.columns.max(1);
                columns = (0..n.min(3)).map(|k| raw[k] + 20.0).collect();
                let mut dw = (total + 40.0).max(LIST_W.0);
                if total + 40.0 < LIST_W.0 {
                    dw = LIST_W.0;
                    if n > 1 {
                        let extra = ((360.0 - total) / (n as f32 - 1.0)).trunc();
                        for cw in columns.iter_mut().take(n - 1) {
                            *cw += extra;
                        }
                    }
                }
                if n == MAX_COLUMNS {
                    columns.push(dw - 10.0 - columns.iter().sum::<f32>());
                }
                if self.spec.style == DialogStyle::TablistHeaders {
                    header_h = small_h;
                }
                (dw, c + BUTTON_H + 15.0 + list_h + header_h)
            }
            _ => {
                let body = strip_codes(&self.spec.body);
                let bw = body
                    .split('\n')
                    .map(|l| m.width(FontId::Small, l))
                    .fold(0.0f32, f32::max);
                let bh = body.split('\n').count() as f32 * small_h;
                let mut dh = c + bh + 25.0 + BUTTON_H;
                if self.has_edit() {
                    dh += (1.6 * h1 + 14.0).trunc();
                }
                (bw + 40.0, dh)
            }
        };
        dw = dw.max(MIN_W);
        let x = ((w - dw) / 2.0).round();
        let y = ((h - dh) / 2.0).round();
        let ctl = |lx: f32, ly: f32, cw: f32, ch: f32| [x + lx, y + c + ly, x + lx + cw, y + c + ly + ch];
        let by = dh - c - BUTTON_H - 5.0;
        let half = (dw / 2.0).trunc();
        let (button1, button2) = if self.has_button2() {
            (
                ctl(half - BUTTON_W - 10.0, by, BUTTON_W, BUTTON_H),
                Some(ctl(half + 10.0, by, BUTTON_W, BUTTON_H)),
            )
        } else {
            (ctl(half - BUTTON_W / 2.0, by, BUTTON_W, BUTTON_H), None)
        };
        Layout {
            rect: [x, y, x + dw, y + dh],
            caption_h: c,
            body: (!self.is_list()).then_some((x + 20.0, y + c + 5.0)),
            list: self
                .is_list()
                .then(|| ctl(5.0, header_h + 5.0, dw - 10.0, list_h)),
            row_h: h1,
            columns,
            header: (self.spec.style == DialogStyle::TablistHeaders).then_some((x + 16.0, y + c + 3.0)),
            edit: self
                .has_edit()
                .then(|| ctl(5.0, dh - c - edit_h - BUTTON_H - 24.0, dw - 10.0, 14.0 + edit_h)),
            button1,
            button2,
        }
    }

    /// Rows that fit in the list box.
    pub fn page_rows(l: &Layout) -> usize {
        l.list.map_or(0, |r| {
            ((r[3] - r[1] - 2.0 * LIST_BORDER) / l.row_h).max(1.0) as usize
        })
    }

    fn select(&mut self, row: usize, page: usize) {
        if self.rows.is_empty() {
            return;
        }
        self.selected = row.min(self.rows.len() - 1);
        if self.selected < self.top {
            self.top = self.selected;
        } else if page > 0 && self.selected >= self.top + page {
            self.top = self.selected + 1 - page;
        }
    }

    /// Up / Down / Page Up / Page Down / Home / End on the list (`delta` rows; `isize::MIN`/`MAX` = ends).
    pub fn list_move(&mut self, delta: isize, page: usize) {
        if !self.is_list() || self.rows.is_empty() {
            return;
        }
        let target = match delta {
            isize::MIN => 0,
            isize::MAX => self.rows.len() - 1,
            d => (self.selected as isize + d).clamp(0, self.rows.len() as isize - 1) as usize,
        };
        self.select(target, page);
    }

    /// Mouse wheel on the list: moves the first visible row.
    pub fn wheel(&mut self, notches: i32, page: usize) {
        let max = self.rows.len().saturating_sub(page) as isize;
        self.top = (self.top as isize - 3 * notches as isize).clamp(0, max) as usize;
    }

    /// A typed character (input styles).
    pub fn char(&mut self, c: char) {
        if self.has_edit() {
            self.edit.insert(c);
        }
    }

    /// An editing key (input styles).
    pub fn edit_key(&mut self, k: EditKey) {
        if self.has_edit() {
            self.edit.key(k);
        }
    }

    /// The response for a button (1 = left / Enter, 0 = right / Esc).
    pub fn response(&self, button: u8) -> UiEvent {
        let (list_item, text) = if self.is_list() {
            match self.rows.get(self.selected) {
                Some(row) => {
                    let first = strip_codes(row.first().map_or("", String::as_str));
                    (self.selected as i16, first.chars().take(TEXT_MAX).collect())
                }
                None => (-1, String::new()),
            }
        } else if self.has_edit() {
            (-1, self.edit.text())
        } else {
            (-1, String::new())
        };
        UiEvent::DialogResponse {
            id: self.spec.id,
            button,
            list_item,
            text,
        }
    }

    /// A left click at `(x, y)`: a button answers (`Some(response)`); a row selects, a second click on the
    /// same row within [`DOUBLE_CLICK_MS`] answers with button 1.
    pub fn click(&mut self, l: &Layout, x: f32, y: f32, time_ms: u32) -> Option<UiEvent> {
        if contains(&l.button1, x, y) {
            return Some(self.response(1));
        }
        if l.button2.is_some_and(|b| contains(&b, x, y)) {
            return Some(self.response(0));
        }
        let list = l.list?;
        let sel = [
            list[0] + LIST_BORDER,
            list[1] + LIST_BORDER,
            list[2] - LIST_SCROLLBAR_W - LIST_BORDER,
            list[3] - LIST_BORDER,
        ];
        if !contains(&sel, x, y) {
            return None;
        }
        let row = self.top + ((y - sel[1]) / l.row_h) as usize;
        if row >= self.rows.len() {
            return None;
        }
        self.select(row, Self::page_rows(l));
        let double = self
            .last_click
            .is_some_and(|(r, t)| r == row && time_ms.wrapping_sub(t) <= DOUBLE_CLICK_MS);
        self.last_click = Some((row, time_ms));
        double.then(|| self.response(1))
    }

    /// Draws the dialog; `cursor` highlights the button under it.
    pub fn draw(
        &self,
        m: &dyn Measure,
        w: f32,
        h: f32,
        cursor: Option<(f32, f32)>,
        time_ms: u32,
        d: &mut DrawList,
    ) {
        let l = self.layout(m, w, h);
        let [x0, y0, x1, y1] = l.rect;
        d.rect(x0, y0, x1, y1, BACKGROUND);
        if l.caption_h > 0.0 {
            d.rect(x0, y0, x1, y0 + l.caption_h, 0xFF00_0000);
            let ty = (y0 + (l.caption_h - m.line_height(FontId::Main)) / 2.0).round();
            d.text(
                x0 + 5.0,
                ty,
                FontId::Main,
                CAPTION_COLOUR,
                &strip_codes(&self.spec.title),
            );
        }
        if let Some((bx, by)) = l.body {
            let sh = m.line_height(FontId::Small);
            for (i, line) in self.spec.body.split('\n').enumerate() {
                d.coloured(
                    m,
                    bx,
                    by + i as f32 * sh,
                    FontId::Small,
                    line,
                    BODY_COLOUR,
                    &OUTLINE_4,
                );
            }
        }
        if let Some((hx, hy)) = l.header {
            let mut x = hx;
            for (k, cell) in self.header.iter().take(self.columns).enumerate() {
                d.coloured(m, x, hy, FontId::Small, cell, BODY_COLOUR, &[]);
                x += l.columns.get(k).copied().unwrap_or(0.0);
            }
        }
        if let Some(lr) = l.list {
            d.rect(lr[0], lr[1], lr[2], lr[3], 0x8000_0000);
            d.frame(lr[0], lr[1], lr[2], lr[3], 0xC880_8080);
            let page = Self::page_rows(&l);
            let tx = lr[0] + LIST_BORDER + 5.0;
            let sel_r = lr[2] - LIST_SCROLLBAR_W - LIST_BORDER;
            for (k, row) in self.rows.iter().enumerate().skip(self.top).take(page) {
                let ry = lr[1] + LIST_BORDER + (k - self.top) as f32 * l.row_h;
                if k == self.selected {
                    d.rect(lr[0] + LIST_BORDER, ry, sel_r, ry + l.row_h, SELECTION_COLOUR);
                }
                let mut cx = tx;
                for (ci, cell) in row.iter().take(self.columns).enumerate() {
                    d.coloured(m, cx, ry, FontId::Gui1, cell, ROW_COLOUR, &[]);
                    cx += l.columns.get(ci).copied().unwrap_or(0.0);
                }
            }
            // Scrollbar: track, thumb.
            let (sx0, sx1) = (lr[2] - LIST_SCROLLBAR_W - 2.0, lr[2] - 2.0);
            let (ty0, ty1) = (lr[1] + 2.0, lr[3] - 2.0);
            d.rect(sx0, ty0, sx1, ty1, 0xC820_2020);
            let n = self.rows.len().max(1) as f32;
            let frac = (page as f32 / n).min(1.0);
            let th = ((ty1 - ty0) * frac).max(8.0);
            let tt = ty0
                + (ty1 - ty0 - th)
                    * if self.rows.len() > page {
                        self.top as f32 / (n - page as f32)
                    } else {
                        0.0
                    };
            d.rect(sx0 + 2.0, tt, sx1 - 2.0, tt + th, 0xC8A0_A0A0);
        }
        if let Some(er) = l.edit {
            d.rect(er[0], er[1], er[2], er[3], 0xC000_0000);
            d.frame(er[0], er[1], er[2], er[3], 0xFFC8_C8C8);
            let pw = self.spec.style == DialogStyle::Password;
            let ty = ((er[1] + er[3] - m.line_height(FontId::Gui0)) / 2.0).round();
            d.text(er[0] + 9.0, ty, FontId::Gui0, 0xFFFF_FFFF, &self.edit.display(pw));
            if (time_ms / 500).is_multiple_of(2) {
                let cx = er[0] + 9.0 + m.width(FontId::Gui0, &self.edit.display_before_caret(pw));
                d.rect(cx, ty, cx + 2.0, ty + m.line_height(FontId::Gui0), 0xFFFF_FFFF);
            }
        }
        let buttons = [
            (Some(l.button1), &self.spec.button1),
            (l.button2, &self.spec.button2),
        ];
        for (b, label) in buttons {
            let Some(b) = b else { continue };
            let hover = cursor.is_some_and(|(cx, cy)| contains(&b, cx, cy));
            d.rect(
                b[0],
                b[1],
                b[2],
                b[3],
                if hover { 0xC860_6060 } else { 0xC830_3030 },
            );
            d.frame(b[0], b[1], b[2], b[3], 0xC8A0_A0A0);
            let t = strip_codes(label);
            let tw = m.width(FontId::Gui0, &t);
            let tx = ((b[0] + b[2] - tw) / 2.0).round();
            let ty = ((b[1] + b[3] - m.line_height(FontId::Gui0)) / 2.0).round();
            d.text(tx, ty, FontId::Gui0, 0xFFFF_FFFF, &t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::MonoMeasure;

    // Line heights: every font 18 (main/small/gui as one mono measure).
    const M: MonoMeasure = MonoMeasure {
        char_w: 8.0,
        line_h: 18.0,
    };

    fn spec(style: DialogStyle, body: &str, b2: &str) -> DialogSpec {
        DialogSpec {
            id: 7,
            style,
            title: "Title".into(),
            body: body.into(),
            button1: "OK".into(),
            button2: b2.into(),
        }
    }

    #[test]
    fn msgbox_geometry_is_centred_with_two_buttons() {
        let d = Dialog::new(&spec(DialogStyle::MsgBox, "Hello world\nline 2", "Cancel"));
        let l = d.layout(&M, 1280.0, 720.0);
        // body 11 chars × 8 = 88 → w = max(128, 230) = 230; h = 22 + 36 + 25 + 30 = 113.
        assert_eq!(l.rect, [525.0, 304.0, 755.0, 417.0]);
        assert_eq!(l.caption_h, 22.0);
        assert_eq!(l.body, Some((545.0, 331.0)));
        // Buttons at h − c − 35 below the caption: 304 + 22 + 56.
        assert_eq!(l.button1, [530.0, 382.0, 630.0, 412.0]);
        assert_eq!(l.button2.unwrap()[0], 525.0 + 125.0);
        assert!(l.list.is_none() && l.edit.is_none());
    }

    #[test]
    fn input_has_edit_and_password_masks() {
        let mut d = Dialog::new(&spec(DialogStyle::Password, "Enter password", ""));
        let l = d.layout(&M, 800.0, 600.0);
        assert!(l.edit.is_some());
        assert!(l.button2.is_none());
        for c in "s3cret".chars() {
            d.char(c);
        }
        d.edit_key(EditKey::Backspace);
        assert_eq!(
            d.response(1),
            UiEvent::DialogResponse {
                id: 7,
                button: 1,
                list_item: -1,
                text: "s3cre".into()
            }
        );
        let mut dl = DrawList::default();
        d.draw(&M, 800.0, 600.0, None, 600, &mut dl);
        assert!(dl.texts().contains(&"*****"));
    }

    #[test]
    fn list_navigation_and_response() {
        let body: String = (0..30).map(|i| format!("{{FF0000}}Item {i}\n")).collect();
        let mut d = Dialog::new(&spec(DialogStyle::List, &body, "Close"));
        assert_eq!(d.rows().len(), 30);
        let l = d.layout(&M, 1280.0, 720.0);
        let page = Dialog::page_rows(&l);
        // 30 × 18 + 12 = 552 → clamped to 400; (400 − 12) / 18 = 21 rows.
        assert_eq!(page, 21);
        d.list_move(1, page);
        d.list_move(1, page);
        assert_eq!(d.selected(), 2);
        d.list_move(isize::MAX, page);
        assert_eq!((d.selected(), d.top()), (29, 9));
        d.list_move(-100, page);
        assert_eq!((d.selected(), d.top()), (0, 0));
        assert_eq!(
            d.response(0),
            UiEvent::DialogResponse {
                id: 7,
                button: 0,
                list_item: 0,
                text: "Item 0".into()
            }
        );
    }

    #[test]
    fn mouse_selects_rows_double_click_and_buttons_answer() {
        let mut d = Dialog::new(&spec(DialogStyle::List, "a\nb\nc", "Close"));
        let l = d.layout(&M, 1280.0, 720.0);
        let lr = l.list.unwrap();
        let (x, y) = (lr[0] + 20.0, lr[1] + LIST_BORDER + l.row_h * 1.5);
        assert_eq!(d.click(&l, x, y, 1000), None);
        assert_eq!(d.selected(), 1);
        assert!(matches!(
            d.click(&l, x, y, 1200),
            Some(UiEvent::DialogResponse {
                button: 1,
                list_item: 1,
                ..
            })
        ));
        let b2 = l.button2.unwrap();
        assert!(matches!(
            d.click(&l, b2[0] + 1.0, b2[1] + 1.0, 5000),
            Some(UiEvent::DialogResponse { button: 0, .. })
        ));
    }

    #[test]
    fn tablist_headers_split_cells_and_columns() {
        let d = Dialog::new(&spec(
            DialogStyle::TablistHeaders,
            "Name\tPrice\nM4\t$500\nAK-47\t$400\n",
            "Close",
        ));
        assert_eq!(d.rows().len(), 2);
        let l = d.layout(&M, 1280.0, 720.0);
        assert_eq!(l.columns.len(), 2);
        assert!(l.header.is_some());
        assert_eq!(
            d.response(1),
            UiEvent::DialogResponse {
                id: 7,
                button: 1,
                list_item: 0,
                text: "M4".into()
            }
        );
    }
}
