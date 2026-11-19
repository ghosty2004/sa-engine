//! The connect screen: a server browser over the whole screen before the game world is shown. Tabs: the Internet
//! list (`server_list_url`, default the open.mp list), the user's favourite servers, the recent ones and a direct
//! `HOST:PORT`; a details panel with the selected server's live information, rules and players; the nickname and
//! Connect. Its storage keys: `server` (the direct address, the last server joined), `name`, `favourites` and
//! `recent` (one `HOST:PORT` per line, the recent ones newest first), `server_list_url`.
//!
//! Keys: Left / Right switch tabs, Up / Down / Page Up / Page Down / Home / End select, Tab moves between the list
//! and the text fields, Enter connects, Insert / Delete add / remove a favourite (Delete also forgets a recent
//! server), Space asks for the selected server's live details, F5 refreshes, F2 / F3 / F4 hide full / empty /
//! locked servers, F6 changes the order. The mouse clicks tabs, rows (a double click connects), fields and buttons;
//! the wheel scrolls.

mod paint;
mod query;
mod servers;
mod view;

use sa_mod::http::Request;
use sa_mod::input::Input;
use sa_mod::{Error, Overlay, storage};

use self::paint::Painter;
use self::query::{Access, Querier};
use self::servers::{
    DEFAULT_LIST_URL, Filters, Server, Sort, parse_addresses, parse_list, remember, valid_server,
};

/// What the user did on the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Join `server` as `name`.
    Connect { server: String, name: String },
}

/// The tabs, in their order on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Internet,
    Favourites,
    Recent,
    Direct,
}

const TABS: [Tab; 4] = [Tab::Internet, Tab::Favourites, Tab::Recent, Tab::Direct];

/// Where the keys go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    List,
    Search,
    Name,
    Address,
}

/// The Internet list.
enum Internet {
    NotLoaded,
    Loading(Request),
    Loaded(Vec<Server>),
    Failed(String),
}

/// How a status line reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Warning,
    Error,
}

/// What the screen shows while the game connects (drawn by [`Screen::draw_progress`]).
#[derive(Clone, Debug)]
pub enum Progress<'a> {
    /// The host asks the user whether the mod may reach `server`.
    Approval { server: &'a str },
    /// The client is joining `server`; `message`: the client's last line.
    Joining {
        server: &'a str,
        since_us: u64,
        message: Option<String>,
    },
}

/// Something clickable, as the last frame drew it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hit {
    Tab(Tab),
    Row(usize),
    List,
    Search,
    Name,
    Address,
    Connect,
    Favourite,
    Query,
    Refresh,
    HideFull,
    HideEmpty,
    HideLocked,
    Sort,
}

/// The screen's state.
pub struct Screen {
    /// The direct address (and the last server joined).
    pub server: String,
    pub name: String,
    favourites: Vec<String>,
    recent: Vec<String>,
    list_url: String,
    tab: Tab,
    focus: Focus,
    internet: Internet,
    search: String,
    filters: Filters,
    sort: Sort,
    /// The Internet tab's rows (indices into its list), filtered and ordered.
    view: Vec<usize>,
    view_dirty: bool,
    /// Per tab: the selected row and the first row shown.
    selected: [usize; 4],
    scroll: [usize; 4],
    /// Rows the list showed in the last frame.
    page: usize,
    querier: Querier,
    /// A line under the panel: why the last attempt did not go through, what happened.
    pub status: Option<(Tone, String)>,
    painter: Painter,
    hits: Vec<(paint::Rect, Hit)>,
    cursor: (f32, f32),
    /// The last click: when (host microseconds) and on what, for double clicks.
    last_click: (u64, Option<Hit>),
    /// When the screen came up (host microseconds), for its animations.
    shown_us: u64,
    backdrop: view::Backdrop,
}

fn now_us() -> u64 {
    sa_mod::time::now().monotonic_us
}

fn save_list(key: &str, list: &[String]) {
    let _ = storage::write(key, list.join("\n").as_bytes());
}

impl Screen {
    /// The screen with the last server and name used, the favourites and the recent servers.
    pub fn load(default_server: &str, default_name: &str) -> Screen {
        let favourites = crate::setting("favourites")
            .map(|t| parse_addresses(&t))
            .unwrap_or_default();
        let recent = crate::setting("recent")
            .map(|t| parse_addresses(&t))
            .unwrap_or_default();
        let tab = if !favourites.is_empty() {
            Tab::Favourites
        } else if !recent.is_empty() {
            Tab::Recent
        } else {
            Tab::Internet
        };
        Screen {
            server: crate::setting("server").unwrap_or_else(|| default_server.to_string()),
            name: crate::setting("name").unwrap_or_else(|| default_name.to_string()),
            favourites,
            recent,
            list_url: crate::setting("server_list_url").unwrap_or_else(|| DEFAULT_LIST_URL.to_string()),
            tab,
            focus: Focus::List,
            internet: Internet::NotLoaded,
            search: String::new(),
            filters: Filters::default(),
            sort: Sort::default(),
            view: Vec::new(),
            view_dirty: true,
            selected: [0; 4],
            scroll: [0; 4],
            page: 10,
            querier: Querier::load(),
            status: None,
            painter: Painter::default(),
            hits: Vec::new(),
            cursor: (-1.0, -1.0),
            last_click: (0, None),
            shown_us: now_us(),
            backdrop: view::Backdrop::default(),
        }
    }

    /// Builds the fonts for overlays of `sizes` ahead of the first frame.
    pub fn prepare(&mut self, sizes: &[(u32, u32)]) {
        self.painter.prepare(sizes);
    }

    fn tab_index(&self) -> usize {
        TABS.iter().position(|&t| t == self.tab).unwrap_or(0)
    }

    /// The servers of the current tab, as rows.
    fn rows(&self) -> Vec<Server> {
        let known = |a: &String| {
            let mut s = match &self.internet {
                Internet::Loaded(l) => l.iter().find(|s| &s.address == a).cloned(),
                _ => None,
            }
            .unwrap_or_else(|| Server::unknown(a));
            if let Some(i) = self.querier.live(a).and_then(|l| l.info.as_ref()) {
                s.apply(i);
            }
            s
        };
        match self.tab {
            Tab::Internet => match &self.internet {
                Internet::Loaded(l) => self
                    .view
                    .iter()
                    .map(|&i| {
                        let mut s = l[i].clone();
                        if let Some(info) = self.querier.live(&s.address).and_then(|l| l.info.as_ref()) {
                            s.apply(info);
                        }
                        s
                    })
                    .collect(),
                _ => Vec::new(),
            },
            Tab::Favourites => self.favourites.iter().map(known).collect(),
            Tab::Recent => self.recent.iter().map(known).collect(),
            Tab::Direct => vec![known(&self.server.trim().to_string())]
                .into_iter()
                .filter(|s| valid_server(&s.address))
                .collect(),
        }
    }

    /// The address of the selected server.
    fn selected_address(&self) -> Option<String> {
        let i = self.selected[self.tab_index()];
        match self.tab {
            Tab::Internet => match &self.internet {
                Internet::Loaded(l) => self.view.get(i).map(|&k| l[k].address.clone()),
                _ => None,
            },
            Tab::Favourites => self.favourites.get(i).cloned(),
            Tab::Recent => self.recent.get(i).cloned(),
            Tab::Direct => Some(self.server.trim().to_string()).filter(|s| valid_server(s)),
        }
    }

    fn row_count(&self) -> usize {
        match self.tab {
            Tab::Internet => self.view.len(),
            Tab::Favourites => self.favourites.len(),
            Tab::Recent => self.recent.len(),
            Tab::Direct => usize::from(valid_server(self.server.trim())),
        }
    }

    fn select(&mut self, i: usize) {
        let n = self.row_count();
        let t = self.tab_index();
        self.selected[t] = if n == 0 { 0 } else { i.min(n - 1) };
        let page = self.page.max(1);
        if self.selected[t] < self.scroll[t] {
            self.scroll[t] = self.selected[t];
        } else if self.selected[t] >= self.scroll[t] + page {
            self.scroll[t] = self.selected[t] + 1 - page;
        }
    }

    fn scroll_by(&mut self, rows: isize) {
        let t = self.tab_index();
        let max = self.row_count().saturating_sub(self.page.max(1));
        self.scroll[t] = self.scroll[t].saturating_add_signed(rows).min(max);
    }

    fn set_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.focus = match tab {
            Tab::Direct => Focus::Address,
            _ => Focus::List,
        };
        if tab == Tab::Internet && matches!(self.internet, Internet::NotLoaded) {
            self.fetch();
        }
    }

    /// (Re)loads the Internet list.
    fn fetch(&mut self) {
        self.internet = match Request::get(&self.list_url) {
            Ok(r) => Internet::Loading(r),
            Err(e) => Internet::Failed(format!("cannot request {}: {e:?}", self.list_url)),
        };
    }

    /// F5: the Internet list again, and every server on screen queried again.
    fn refresh(&mut self) {
        if self.tab == Tab::Internet {
            self.fetch();
        }
        let targets: Vec<String> = self.rows().into_iter().map(|s| s.address).collect();
        self.querier.refresh(&targets);
    }

    fn text_field(&mut self) -> Option<&mut String> {
        match self.focus {
            Focus::Search => Some(&mut self.search),
            Focus::Name => Some(&mut self.name),
            Focus::Address => Some(&mut self.server),
            Focus::List => None,
        }
    }

    fn edited(&mut self) {
        match self.focus {
            Focus::Search => {
                self.view_dirty = true;
                self.selected[0] = 0;
                self.scroll[0] = 0;
            }
            Focus::Address => self.querier.refresh(&[self.server.trim().to_string()]),
            _ => {}
        }
    }

    fn toggle_favourite(&mut self) {
        let Some(s) = self.selected_address() else { return };
        if let Some(i) = self.favourites.iter().position(|f| *f == s) {
            self.favourites.remove(i);
            self.status = Some((Tone::Info, format!("{s} removed from the favourites")));
            if self.tab == Tab::Favourites {
                self.select(i.saturating_sub(usize::from(i >= self.favourites.len())));
            }
        } else {
            self.favourites.push(s.clone());
            self.status = Some((Tone::Info, format!("{s} added to the favourites")));
            // the user picked this server: its details may ask for the permission
            self.querier.ask(&s);
        }
        save_list("favourites", &self.favourites);
    }

    fn remove_selected(&mut self) {
        let i = self.selected[self.tab_index()];
        let (list, key) = match self.tab {
            Tab::Favourites => (&mut self.favourites, "favourites"),
            Tab::Recent => (&mut self.recent, "recent"),
            _ => return,
        };
        if i < list.len() {
            let s = list.remove(i);
            save_list(key, list);
            self.status = Some((Tone::Info, format!("{s} removed")));
            self.select(i.saturating_sub(1));
        }
    }

    fn click(&mut self, hit: Hit) -> Option<Action> {
        let t = now_us();
        let double = self.last_click.1 == Some(hit) && t.saturating_sub(self.last_click.0) < 400_000;
        self.last_click = (t, Some(hit));
        match hit {
            Hit::Tab(tab) => self.set_tab(tab),
            Hit::Row(i) => {
                self.focus = Focus::List;
                self.select(i);
                if double {
                    return self.connect();
                }
            }
            Hit::List => self.focus = Focus::List,
            Hit::Search => self.focus = Focus::Search,
            Hit::Name => self.focus = Focus::Name,
            Hit::Address => self.focus = Focus::Address,
            Hit::Connect => return self.connect(),
            Hit::Favourite => self.toggle_favourite(),
            Hit::Query => {
                if let Some(s) = self.selected_address() {
                    self.querier.ask(&s);
                }
            }
            Hit::Refresh => self.refresh(),
            Hit::HideFull => self.filter(|f| f.hide_full = !f.hide_full),
            Hit::HideEmpty => self.filter(|f| f.hide_empty = !f.hide_empty),
            Hit::HideLocked => self.filter(|f| f.hide_locked = !f.hide_locked),
            Hit::Sort => {
                self.sort = self.sort.next();
                self.view_dirty = true;
            }
        }
        None
    }

    fn filter(&mut self, f: impl FnOnce(&mut Filters)) {
        f(&mut self.filters);
        self.view_dirty = true;
        self.selected[0] = 0;
        self.scroll[0] = 0;
    }

    /// An input event for the screen (it takes them all while it is up).
    pub fn input(&mut self, e: Input) -> Option<Action> {
        match e {
            Input::MouseMove { x, y } => self.cursor = (x as f32, y as f32),
            Input::MouseButton {
                button: 0,
                down: true,
            } => {
                let hit = self
                    .hits
                    .iter()
                    .rev()
                    .find(|(r, _)| r.contains(self.cursor))
                    .map(|h| h.1);
                if let Some(h) = hit {
                    return self.click(h);
                }
            }
            Input::Wheel(d) => self.scroll_by(-(d as isize) * 3 / 120),
            Input::Char(c) if (' '..='~').contains(&c) || ('\u{A0}'..='\u{FF}').contains(&c) => {
                if self.focus == Focus::List && self.tab == Tab::Internet && c != ' ' {
                    self.focus = Focus::Search;
                }
                if self.focus == Focus::List && c == ' ' {
                    if let Some(s) = self.selected_address() {
                        self.querier.ask(&s);
                    }
                    return None;
                }
                let limit = if self.focus == Focus::Name { 24 } else { 64 };
                if let Some(f) = self.text_field().filter(|f| f.chars().count() < limit) {
                    f.push(c);
                    self.edited();
                }
            }
            Input::KeyDown { vk, .. } => return self.key(vk),
            _ => {}
        }
        None
    }

    fn key(&mut self, vk: u16) -> Option<Action> {
        let t = self.tab_index();
        match vk {
            // Backspace
            0x08 => {
                if let Some(f) = self.text_field() {
                    f.pop();
                    self.edited();
                }
            }
            // Tab: the list, then the tab's text fields
            0x09 => {
                let order: &[Focus] = match self.tab {
                    Tab::Internet => &[Focus::List, Focus::Search, Focus::Name],
                    Tab::Direct => &[Focus::Address, Focus::Name],
                    _ => &[Focus::List, Focus::Name],
                };
                let i = order
                    .iter()
                    .position(|&f| f == self.focus)
                    .map_or(0, |i| (i + 1) % order.len());
                self.focus = order[i];
            }
            0x0D => {
                if self.focus == Focus::Search {
                    self.focus = Focus::List;
                } else {
                    return self.connect();
                }
            }
            0x1B if self.focus != Focus::List && self.tab != Tab::Direct => self.focus = Focus::List,
            // Left / Right: the tabs (outside text fields)
            0x25 | 0x27 if self.text_field().is_none() || self.tab == Tab::Direct => {
                let n = TABS.len();
                let i = if vk == 0x25 { (t + n - 1) % n } else { (t + 1) % n };
                self.set_tab(TABS[i]);
            }
            // Up / Down / Page Up / Page Down / Home / End
            0x26 | 0x28 | 0x21 | 0x22 | 0x24 | 0x23
                if self.focus == Focus::List || self.focus == Focus::Name =>
            {
                self.focus = Focus::List;
                let cur = self.selected[t];
                let page = self.page.max(1);
                let next = match vk {
                    0x26 => cur.saturating_sub(1),
                    0x28 => cur + 1,
                    0x21 => cur.saturating_sub(page),
                    0x22 => cur + page,
                    0x24 => 0,
                    _ => usize::MAX,
                };
                self.select(next);
            }
            0x2D => self.toggle_favourite(),
            0x2E if self.focus == Focus::List => self.remove_selected(),
            0x71 => self.filter(|f| f.hide_full = !f.hide_full),
            0x72 => self.filter(|f| f.hide_empty = !f.hide_empty),
            0x73 => self.filter(|f| f.hide_locked = !f.hide_locked),
            0x74 => self.refresh(),
            0x75 => {
                self.sort = self.sort.next();
                self.view_dirty = true;
            }
            _ => {}
        }
        None
    }

    /// The Connect action for the selected server and the nickname.
    pub fn connect(&mut self) -> Option<Action> {
        let Some(server) = self.selected_address() else {
            self.status = Some((
                Tone::Warning,
                "Select a server first (or type HOST:PORT in Direct connect)".into(),
            ));
            return None;
        };
        let name = self.name.trim().to_string();
        if !valid_server(&server) {
            self.status = Some((Tone::Error, "The server must be HOST:PORT".into()));
            return None;
        }
        if !(3..=24).contains(&name.len())
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_[]$=()@.".contains(&b))
        {
            self.status = Some((
                Tone::Error,
                "Nickname: 3 to 24 of A-Z a-z 0-9 _ [ ] $ = ( ) @ .".into(),
            ));
            self.focus = Focus::Name;
            return None;
        }
        self.server = server.clone();
        let _ = storage::write("server", server.as_bytes());
        let _ = storage::write("name", name.as_bytes());
        self.querier.stop();
        Some(Action::Connect { server, name })
    }

    /// The host allowed the mod to reach `server` and the client starts joining it: a recent server from now on.
    pub fn joining(&mut self, server: &str) {
        self.querier.allow(server);
        remember(&mut self.recent, server);
        save_list("recent", &self.recent);
    }

    /// One frame of the screen's work: the Internet list's request, the queries.
    fn tick(&mut self) {
        if let Internet::Loading(r) = &self.internet {
            match r.status() {
                Err(Error::Again) => {}
                Ok(200) => {
                    self.internet = match r
                        .body()
                        .map_err(|e| format!("{e:?}"))
                        .and_then(|b| parse_list(&b))
                    {
                        Ok(l) => Internet::Loaded(l),
                        Err(e) => Internet::Failed(e),
                    };
                    self.view_dirty = true;
                }
                Ok(code) => self.internet = Internet::Failed(format!("the server list answered HTTP {code}")),
                Err(e) => {
                    self.internet = Internet::Failed(format!("the server list could not be loaded ({e:?})"))
                }
            }
        }
        if self.view_dirty
            && let Internet::Loaded(l) = &self.internet
        {
            let q = &self.querier;
            let keep = self.view.get(self.selected[0]).map(|&i| l[i].address.clone());
            self.view = servers::view(l, self.filters, &self.search, self.sort, |a| {
                q.live(a).and_then(|l| l.ping)
            });
            self.view_dirty = false;
            if let Some(k) = keep
                && let Some(p) = self.view.iter().position(|&i| l[i].address == k)
            {
                self.selected[0] = p;
            }
            let n = self.view.len();
            if self.selected[0] >= n {
                self.selected[0] = n.saturating_sub(1);
            }
        }
        let t = self.tab_index();
        let shown: Vec<String> = match self.tab {
            Tab::Internet => match &self.internet {
                Internet::Loaded(l) => self
                    .view
                    .iter()
                    .skip(self.scroll[t])
                    .take(self.page)
                    .map(|&i| l[i].address.clone())
                    .collect(),
                _ => Vec::new(),
            },
            _ => self.rows().into_iter().map(|s| s.address).collect(),
        };
        let selected = self.selected_address();
        self.querier.frame(&shown, selected.as_deref());
    }

    /// Draws the screen over the whole overlay.
    pub fn draw(&mut self, o: &mut Overlay) {
        self.tick();
        if matches!(self.internet, Internet::NotLoaded) && self.tab == Tab::Internet {
            self.fetch();
        }
        view::draw(self, o);
    }

    /// Draws the backdrop and a card saying what the game is doing (waiting for the user's permission, joining);
    /// `alpha` fades it out once the game has joined.
    pub fn draw_progress(&mut self, o: &mut Overlay, p: &Progress<'_>, alpha: f32) {
        view::draw_progress(self, o, p, alpha);
    }

    /// Whether the selected server's details need the user's permission first.
    fn access(&self, server: &str) -> Access {
        self.querier.access(server)
    }
}
