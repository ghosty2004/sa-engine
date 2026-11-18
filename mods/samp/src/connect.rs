//! The connect screen: the server (`HOST:PORT`) and the nickname to join with, and the user's favourite servers, kept
//! in the mod's storage (`server`, `name`, `favourites`: one `HOST:PORT` per line).

use sa_mod::input::Input;
use sa_mod::{Overlay, Rgba, storage};

const SIZE: f32 = 12.0;
const LINE: f32 = SIZE + 8.0;
const WIDTH: f32 = 640.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Server,
    Name,
    Favourites,
}

/// What the user did on the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Join `server` as `name`.
    Connect { server: String, name: String },
}

/// The screen's state.
pub struct Screen {
    pub server: String,
    pub name: String,
    favourites: Vec<String>,
    selected: usize,
    focus: Field,
    /// A line under the fields (why the last attempt did not go through, or that the host is asking the user).
    pub status: Option<String>,
}

fn valid_server(s: &str) -> bool {
    s.rsplit_once(':').is_some_and(|(h, p)| !h.is_empty() && p.parse::<u16>().is_ok_and(|p| p != 0))
}

impl Screen {
    /// The screen with the last server and name used and the favourites.
    pub fn load(default_server: &str, default_name: &str) -> Screen {
        let favourites: Vec<String> =
            crate::setting("favourites").map(|t| t.lines().map(str::trim).filter(|l| valid_server(l)).map(String::from).collect())
                .unwrap_or_default();
        Screen {
            server: crate::setting("server").unwrap_or_else(|| default_server.to_string()),
            name: crate::setting("name").unwrap_or_else(|| default_name.to_string()),
            favourites,
            selected: 0,
            focus: Field::Server,
            status: None,
        }
    }

    fn save_favourites(&self) {
        let _ = storage::write("favourites", self.favourites.join("\n").as_bytes());
    }

    fn field(&mut self) -> Option<&mut String> {
        match self.focus {
            Field::Server => Some(&mut self.server),
            Field::Name => Some(&mut self.name),
            Field::Favourites => None,
        }
    }

    /// A key or character for the screen (it takes them all while it is up).
    pub fn input(&mut self, e: Input) -> Option<Action> {
        match e {
            Input::Char(c) if (' '..='~').contains(&c) => {
                if let Some(f) = self.field().filter(|f| f.len() < 64) {
                    f.push(c);
                }
            }
            Input::KeyDown { vk, .. } => match vk {
                0x08 => {
                    if let Some(f) = self.field() {
                        f.pop();
                    }
                }
                0x09 => {
                    self.focus = match self.focus {
                        Field::Server => Field::Name,
                        Field::Name if !self.favourites.is_empty() => Field::Favourites,
                        _ => Field::Server,
                    }
                }
                0x26 | 0x28 if !self.favourites.is_empty() => {
                    let n = self.favourites.len();
                    self.selected = if vk == 0x26 { (self.selected + n - 1) % n } else { (self.selected + 1) % n };
                    self.server = self.favourites[self.selected].clone();
                    self.focus = Field::Favourites;
                }
                0x2D if valid_server(&self.server) && !self.favourites.contains(&self.server) => {
                    self.favourites.push(self.server.clone());
                    self.save_favourites();
                }
                0x2E if self.focus == Field::Favourites && self.selected < self.favourites.len() => {
                    self.favourites.remove(self.selected);
                    self.selected = self.selected.saturating_sub(1);
                    self.save_favourites();
                }
                0x0D => return self.connect(),
                _ => {}
            },
            _ => {}
        }
        None
    }

    /// The Connect action for the fields as they are.
    pub fn connect(&mut self) -> Option<Action> {
        let server = self.server.trim().to_string();
        let name = self.name.trim().to_string();
        if !valid_server(&server) {
            self.status = Some("The server must be HOST:PORT".into());
            return None;
        }
        if !(3..=24).contains(&name.len()) || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_[]$=()@.".contains(&b)) {
            self.status = Some("The nickname: 3 to 24 of A-Z a-z 0-9 _ [ ] $ = ( ) @ .".into());
            return None;
        }
        let _ = storage::write("server", server.as_bytes());
        let _ = storage::write("name", name.as_bytes());
        Some(Action::Connect { server, name })
    }

    /// Draws the screen in the middle of the overlay.
    pub fn draw(&self, o: &mut Overlay) {
        let (w, h) = o.size();
        let rows = 7 + self.favourites.len().max(1);
        let height = rows as f32 * LINE + 24.0;
        let (x, y) = (((w as f32 - WIDTH) / 2.0).max(0.0), ((h as f32 - height) / 2.0).max(0.0));
        let white = Rgba::new(0xFF, 0xFF, 0xFF, 0xFF);
        let grey = Rgba::new(0xA9, 0xC4, 0xE4, 0xFF);
        let focus = Rgba::new(0x2A, 0x4A, 0x7A, 0xFF);
        let _ = o.rect(x, y, WIDTH, height, Rgba::new(0x10, 0x14, 0x20, 0xE8));
        let mut line = |row: usize, text: &str, colour: Rgba, highlight: bool| {
            let ty = y + 12.0 + row as f32 * LINE;
            if highlight {
                let _ = o.rect(x + 8.0, ty - 4.0, WIDTH - 16.0, LINE, focus);
            }
            let _ = o.text(x + 16.0, ty, SIZE, colour, text);
        };
        line(0, "SA:MP - connect to a server", white, false);
        let caret = |f: Field| if self.focus == f { "_" } else { "" };
        line(1, &format!("Server:   {}{}", self.server, caret(Field::Server)), white, self.focus == Field::Server);
        line(2, &format!("Nickname: {}{}", self.name, caret(Field::Name)), white, self.focus == Field::Name);
        line(3, "Favourites:", grey, false);
        if self.favourites.is_empty() {
            line(4, "  (none: Insert adds the server)", grey, false);
        }
        for (i, f) in self.favourites.iter().enumerate() {
            let on = self.focus == Field::Favourites && i == self.selected;
            line(4 + i, &format!("  {f}"), white, on);
        }
        let n = 4 + self.favourites.len().max(1);
        line(n, "Enter: connect  Tab: next field  Up/Down: favourite", grey, false);
        line(n + 1, "Insert: add to favourites  Delete: remove", grey, false);
        if let Some(s) = &self.status {
            line(n + 2, s, Rgba::new(0xFF, 0xC0, 0x60, 0xFF), false);
        }
    }
}
