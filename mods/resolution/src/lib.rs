//! The resolution mod (MOD-2 M1): the game at the desktop resolution (in the browser: the canvas's size in device
//! pixels) or at a chosen one, in a window or full screen. The choice is kept in the mod's own `config.toml`:
//!
//! ```toml
//! resolution = "auto"     # "auto" (the desktop / canvas) or "WIDTHxHEIGHT", e.g. "1920x1080"
//! fullscreen = false      # natively: full screen instead of a window
//! follow_resize = true    # with "auto": follow the window / canvas when it changes size (when the adapter lists it)
//! ```
//!
//! A missing file is written with the defaults. The game applies the mode through its own video-mode selection
//! (docs/mods/getting-started.md, docs/arch/ADR-004-mods.md §4.1). Build: `mods/build.sh`.

use sa_mod::{Mod, display, files};

/// The config file in the mod's folder.
const CONFIG: &str = "config.toml";

/// The mod's settings.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Config {
    /// `None`: the desktop / canvas.
    size: Option<(u32, u32)>,
    fullscreen: bool,
    follow_resize: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config { size: None, fullscreen: false, follow_resize: true }
    }
}

impl Config {
    /// Reads the `key = value` lines it knows (a TOML subset: strings, booleans, `#` comments); the rest keeps its
    /// default.
    fn parse(text: &str) -> Config {
        let mut c = Config::default();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim().trim_matches('"');
            match k.trim() {
                "resolution" => c.size = parse_size(v),
                "fullscreen" => c.fullscreen = v == "true",
                "follow_resize" => c.follow_resize = v == "true",
                _ => {}
            }
        }
        c
    }

    fn to_text(&self) -> String {
        let res = self.size.map_or("auto".to_string(), |(w, h)| format!("{w}x{h}"));
        format!(
            "# The resolution mod's settings (mods/resolution).\n\
             # \"auto\" (the desktop; in the browser the canvas's size in device pixels) or \"WIDTHxHEIGHT\"\n\
             resolution = \"{res}\"\n\
             # natively: full screen instead of a window\n\
             fullscreen = {}\n\
             # with \"auto\": follow the window / canvas when it changes size\n\
             follow_resize = {}\n",
            self.fullscreen, self.follow_resize
        )
    }
}

/// `"WIDTHxHEIGHT"`; anything else (`"auto"`) is `None`.
fn parse_size(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once(['x', 'X'])?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

#[derive(Default)]
struct Resolution {
    config: Config,
}

impl Mod for Resolution {
    fn init(&mut self) -> Result<(), String> {
        self.config = match files::read(CONFIG) {
            Ok(Some(text)) => Config::parse(&String::from_utf8_lossy(&text)),
            Ok(None) => {
                let c = Config::default();
                if files::write(CONFIG, c.to_text().as_bytes()).is_err() {
                    sa_mod::log(sa_mod::level::WARN, "resolution: could not write config.toml");
                }
                c
            }
            Err(e) => {
                sa_mod::log(sa_mod::level::WARN, &format!("resolution: config.toml: {e:?}; using the defaults"));
                Config::default()
            }
        };
        Ok(())
    }

    fn settings(&mut self) {
        let (w, h) = self.config.size.unwrap_or_else(display::desktop);
        let modes = display::modes();
        sa_mod::info(&format!(
            "resolution: {w}x{h} {} (desktop {:?}, {} display modes)",
            if self.config.fullscreen { "full screen" } else { "window" },
            display::desktop(),
            modes.len()
        ));
        if let Err(e) = display::request_video_mode(w, h, self.config.fullscreen) {
            sa_mod::log(sa_mod::level::ERROR, &format!("resolution: {w}x{h} refused: {e:?}"));
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if self.config.size.is_none() && self.config.follow_resize && !self.config.fullscreen {
            let _ = display::request_video_mode(width, height, false);
        }
    }
}

sa_mod::entry!(Resolution);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trip() {
        let c = Config { size: Some((1920, 1080)), fullscreen: true, follow_resize: false };
        assert_eq!(Config::parse(&c.to_text()), c);
        assert_eq!(Config::parse(&Config::default().to_text()), Config::default());
        assert_eq!(Config::parse("resolution = \"1280X720\" # x\n").size, Some((1280, 720)));
        assert_eq!(Config::parse("resolution = 'big'").size, None);
    }
}
