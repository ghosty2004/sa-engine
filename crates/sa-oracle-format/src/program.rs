//! Scripted input programs: a small text language that produces the [`PadInput`] of every input tick.
//!
//! ```text
//! # comment
//! end 3600                    # stop (finish the recording) at this frame boundary
//! 0 key=space pulse           # frame 0 (before the first CTimer::Update): press space every other tick
//! 300 key=enter               # one frame
//! 1000..1199 ly=-128          # inclusive range: left stick forward
//! 1200+60 ly=-128 cross       # 60 frames starting at 1200: run (cross = sprint on foot)
//! 1300..1340 mouse=6,0        # camera turn (CPad::NewMouseControllerState.AmountMoved)
//! ```
//! Frames are frame-boundary numbers (1 = first `CTimer::Update`); ticks that run before the first
//! boundary (logo/intro states) see frame 0. Lines that cover the same frame are applied in order.
//! `pulse` restricts a line to odd ticks, which makes "just pressed" fire repeatedly while held.
//!
//! Actions: `lx= ly= rx= ry=` (stick, -128..128); buttons `l1 l2 r1 r2 up down left right start select
//! square triangle cross circle l3 r3 walk` (optionally `=value`, default 255); `mouse=DX,DY`, `lmb`,
//! `rmb`; `key=NAME` with NAME in `enter extenter space esc tab lalt f1..f12` or `std:0xNN`
//! (`CKeyboardState::standardKeys[NN]`).

use crate::pad::{PadInput, ctrl, key, mouse};

#[derive(Debug, Clone, PartialEq)]
enum Action {
    Ctrl(usize, i16),
    Key(usize, i16),
    Mouse(f32, f32),
    MouseButton(usize),
}

#[derive(Debug, Clone, PartialEq)]
struct Line {
    from: u32,
    to: u32,
    pulse: bool,
    actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    lines: Vec<Line>,
    pub end: Option<u32>,
    /// FNV-64 of the program text (recorded in the recording header).
    pub fnv: u64,
}

fn num<T: std::str::FromStr>(s: &str, what: &str) -> Result<T, String> {
    s.parse().map_err(|_| format!("bad {what} {s:?}"))
}

fn button(name: &str) -> Option<usize> {
    Some(match name {
        "l1" => ctrl::LEFT_SHOULDER1,
        "l2" => ctrl::LEFT_SHOULDER2,
        "r1" => ctrl::RIGHT_SHOULDER1,
        "r2" => ctrl::RIGHT_SHOULDER2,
        "up" => ctrl::DPAD_UP,
        "down" => ctrl::DPAD_DOWN,
        "left" => ctrl::DPAD_LEFT,
        "right" => ctrl::DPAD_RIGHT,
        "start" => ctrl::START,
        "select" => ctrl::SELECT,
        "square" => ctrl::SQUARE,
        "triangle" => ctrl::TRIANGLE,
        "cross" => ctrl::CROSS,
        "circle" => ctrl::CIRCLE,
        "l3" => ctrl::SHOCK_L,
        "r3" => ctrl::SHOCK_R,
        "walk" => ctrl::PED_WALK,
        _ => return None,
    })
}

fn key_offset(name: &str) -> Result<usize, String> {
    Ok(match name {
        "enter" => key::ENTER,
        "extenter" | "return" => key::EXTENTER,
        "space" => key::STANDARD + 2 * 0x20,
        "esc" => key::ESC,
        "tab" => key::TAB,
        "lalt" => key::LALT,
        _ => {
            if let Some(n) = name.strip_prefix('f').and_then(|n| n.parse::<usize>().ok())
                && (1..=12).contains(&n)
            {
                key::F1 + 2 * (n - 1)
            } else if let Some(h) = name.strip_prefix("std:0x") {
                let k = usize::from_str_radix(h, 16).map_err(|_| format!("bad key {name:?}"))?;
                if k > 0xFF {
                    return Err(format!("bad key {name:?}"));
                }
                key::STANDARD + 2 * k
            } else {
                return Err(format!("unknown key {name:?}"));
            }
        }
    })
}

fn action(tok: &str) -> Result<Option<Action>, String> {
    let (name, val) = match tok.split_once('=') {
        Some((n, v)) => (n, Some(v)),
        None => (tok, None),
    };
    let stick = |off| -> Result<Action, String> {
        let v: i16 = num(val.ok_or(format!("{name} needs a value"))?, name)?;
        Ok(Action::Ctrl(off, v))
    };
    Ok(Some(match name {
        "pulse" => return Ok(None),
        "lx" => stick(ctrl::LEFT_STICK_X)?,
        "ly" => stick(ctrl::LEFT_STICK_Y)?,
        "rx" => stick(ctrl::RIGHT_STICK_X)?,
        "ry" => stick(ctrl::RIGHT_STICK_Y)?,
        "mouse" => {
            let v = val.ok_or("mouse needs DX,DY")?;
            let (x, y) = v.split_once(',').ok_or("mouse needs DX,DY")?;
            Action::Mouse(num(x, "mouse dx")?, num(y, "mouse dy")?)
        }
        "lmb" => Action::MouseButton(mouse::LMB),
        "rmb" => Action::MouseButton(mouse::RMB),
        "key" => Action::Key(key_offset(val.ok_or("key needs a name")?)?, 255),
        b => match button(b) {
            Some(off) => Action::Ctrl(off, val.map(|v| num(v, b)).transpose()?.unwrap_or(255)),
            None => return Err(format!("unknown action {tok:?}")),
        },
    }))
}

impl Program {
    pub fn parse(text: &str) -> Result<Program, String> {
        let mut lines = Vec::new();
        let mut end = None;
        for (ln, raw) in text.lines().enumerate() {
            let l = raw.split('#').next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }
            let err = |e: String| format!("line {}: {e}", ln + 1);
            let mut toks = l.split_whitespace();
            let first = toks.next().unwrap_or_default();
            if first == "end" {
                end = Some(
                    num(
                        toks.next().ok_or_else(|| err("end needs a frame".into()))?,
                        "frame",
                    )
                    .map_err(err)?,
                );
                continue;
            }
            let (from, to) = if let Some((a, b)) = first.split_once("..") {
                (num(a, "frame").map_err(err)?, num(b, "frame").map_err(err)?)
            } else if let Some((a, n)) = first.split_once('+') {
                let a: u32 = num(a, "frame").map_err(err)?;
                let n: u32 = num(n, "count").map_err(err)?;
                if n == 0 {
                    return Err(err("empty range".into()));
                }
                (a, a + n - 1)
            } else {
                let a = num(first, "frame").map_err(err)?;
                (a, a)
            };
            if to < from {
                return Err(err("range end before start".into()));
            }
            let mut line = Line {
                from,
                to,
                pulse: false,
                actions: Vec::new(),
            };
            for t in toks {
                match action(t).map_err(err)? {
                    Some(a) => line.actions.push(a),
                    None => line.pulse = true,
                }
            }
            lines.push(line);
        }
        Ok(Program {
            lines,
            end,
            fnv: crate::fnv::hash(text.as_bytes()),
        })
    }

    /// The input of tick `tick` (0-based count of ticks so far) running at frame boundary `frame`.
    pub fn input(&self, frame: u32, tick: u32) -> PadInput {
        let mut p = PadInput::default();
        for l in &self.lines {
            if frame < l.from || frame > l.to || (l.pulse && tick.is_multiple_of(2)) {
                continue;
            }
            for a in &l.actions {
                match *a {
                    Action::Ctrl(off, v) => p.set_ctrl(off, v),
                    Action::Key(off, v) => p.set_key(off, v),
                    Action::Mouse(dx, dy) => p.set_mouse_moved(dx, dy),
                    Action::MouseButton(off) => p.set_mouse_button(off, true),
                }
            }
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_eval() {
        let p = Program::parse(
            "# demo\nend 500\n0 key=space pulse\n10 key=enter\n20..29 ly=-128 cross # run\n25+2 mouse=6,-1.5 lmb\n",
        )
        .unwrap();
        assert_eq!(p.end, Some(500));
        let space = key::STANDARD + 0x40;
        assert_eq!(
            &p.input(0, 1).temp_key_state[space..space + 2],
            &255i16.to_le_bytes()
        );
        assert_eq!(p.input(0, 2), PadInput::default());
        assert_eq!(
            &p.input(10, 7).temp_key_state[key::ENTER..key::ENTER + 2],
            &[255, 0]
        );
        assert_eq!(p.input(11, 7), PadInput::default());
        let i = p.input(26, 0);
        assert_eq!(
            &i.pc_temp_key[ctrl::LEFT_STICK_Y..][..2],
            &(-128i16).to_le_bytes()
        );
        assert_eq!(&i.pc_temp_key[ctrl::CROSS..][..2], &[255, 0]);
        assert_eq!(&i.new_mouse[mouse::MOVED_X..][..4], &6.0f32.to_le_bytes());
        assert_eq!(&i.new_mouse[mouse::MOVED_Y..][..4], &(-1.5f32).to_le_bytes());
        assert_eq!(i.new_mouse[mouse::LMB], 1);
        assert_eq!(p.input(27, 0).new_mouse, PadInput::default().new_mouse);
        assert_eq!(p.input(29, 0).new_mouse, PadInput::default().new_mouse);
        assert_ne!(p.input(29, 0), PadInput::default());
        assert_eq!(p.input(30, 0), PadInput::default());
        for bad in [
            "5..2 lx=1",
            "x lx=1",
            "1 lx",
            "1 bogus",
            "1 key=nope",
            "1+0 cross",
            "end",
        ] {
            assert!(Program::parse(bad).is_err(), "{bad}");
        }
        assert_eq!(key_offset("f12").unwrap(), 22);
        assert_eq!(key_offset("std:0x41").unwrap(), key::STANDARD + 0x82);
    }
}
