//! The trainer HUD: a read-only overlay in the top left corner with the local player, the vehicle, the world and the
//! camera as the game has them this frame, a mini-map of the peds and vehicles around the player, the last game
//! events, and two hooks: the result of `CWorld::FindPlayerHeading` (after) and the peds `CPed::ProcessControl`
//! runs for (before). F7 hides and shows it. Build: `examples/mods/build.sh` (docs/mods/getting-started.md).

use std::collections::VecDeque;

use sa_mod::hooks::{self, Call, HookId, Phase};
use sa_mod::{Event, Mod, Overlay, PedRef, Rgba, Value, Vec3, event_kind, state};

const SIZE: f32 = 12.0;
const LINE: f32 = 15.0;
const MARGIN: f32 = 8.0;
const PAD: f32 = 6.0;
const PANEL_CHARS: f32 = 50.0;
const MAP_SIZE: f32 = 160.0;
/// World units from the map's centre to its edge.
const MAP_RANGE: f32 = 100.0;
const EVENTS_SHOWN: usize = 6;
const VK_F7: u16 = 0x76;

const BACKGROUND: Rgba = Rgba::new(0, 0, 0, 0xA8);
const TEXT: Rgba = Rgba::new(0xE8, 0xE8, 0xE8, 0xFF);
const TITLE: Rgba = Rgba::new(0x60, 0xD0, 0xFF, 0xFF);
const PLAYER: Rgba = Rgba::new(0xFF, 0xFF, 0xFF, 0xFF);
const PED: Rgba = Rgba::new(0xFF, 0xC0, 0x40, 0xFF);
const VEHICLE: Rgba = Rgba::new(0x40, 0xE0, 0x80, 0xFF);

#[derive(Default)]
struct TrainerHud {
    hidden: bool,
    heading_hook: Option<HookId>,
    process_hook: Option<HookId>,
    /// `CWorld::FindPlayerHeading`'s last result, in radians.
    hooked_heading: Option<f32>,
    /// Peds `CPed::ProcessControl` ran for in the game step so far, and in the last whole step.
    processed: Vec<PedRef>,
    processed_last: usize,
    events: VecDeque<String>,
    frame: u32,
}

fn degrees(radians: f32) -> f32 {
    radians.to_degrees().rem_euclid(360.0)
}

fn speed_kmh(v: Vec3) -> f32 {
    // move speed is in units per 1/50 s
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt() * 50.0 * 3.6
}

impl TrainerHud {
    fn remember(&mut self, text: String) {
        sa_mod::info(&format!("trainer-hud: {text}"));
        self.events.push_front(format!("{:>6} {text}", self.frame));
        self.events.truncate(EVENTS_SHOWN);
    }

    fn status_lines(&self) -> Vec<String> {
        let mut out = vec![format!("TRAINER HUD (read-only)   frame {}", self.frame)];
        if let Some(w) = state::world() {
            out.push(format!(
                "{:?}  {:02}:{:02}  weather {}->{}  area {}",
                w.game_state, w.hour, w.minute, w.weather_old, w.weather_new, w.area
            ));
        }
        match state::player() {
            Some(p) if p.handle.is_some() => {
                let ped = &p.ped;
                out.push(format!(
                    "player {:#x} at {:.1} {:.1} {:.1}",
                    p.handle.map_or(0, |h| h.0),
                    ped.pos.x,
                    ped.pos.y,
                    ped.pos.z
                ));
                out.push(format!(
                    "health {:.0}  armour {:.0}  ${}  wanted {}",
                    ped.health, ped.armour, p.money, p.wanted
                ));
                out.push(format!(
                    "heading {:.1} (hook: {})  {:.0} km/h",
                    degrees(ped.heading),
                    self.hooked_heading.map_or("-".into(), |h| format!("{:.1}", degrees(h))),
                    speed_kmh(ped.move_speed)
                ));
                match ped.vehicle.and_then(|v| state::vehicle(v).map(|i| (v, i))) {
                    Some((v, i)) => out.push(format!(
                        "vehicle {:#x} model {}  health {:.0}  {:.0} km/h",
                        v.0,
                        i.model.0,
                        i.health,
                        speed_kmh(i.move_speed)
                    )),
                    None => out.push("on foot".into()),
                }
            }
            _ => out.push("no player".into()),
        }
        out.push(format!(
            "peds {}  vehicles {}  ProcessControl {}/step",
            state::peds().len(),
            state::vehicles().len(),
            self.processed_last
        ));
        if let Some(c) = state::camera() {
            out.push(format!("camera mode {}  fov {:.0}", c.mode, c.fov));
        }
        out.push("events:".into());
        out.extend(self.events.iter().cloned());
        out
    }

    fn draw_map(&self, o: &mut Overlay, x: f32, y: f32) {
        let _ = o.rect(x, y, MAP_SIZE, MAP_SIZE, BACKGROUND);
        let Some(player) = state::player().filter(|p| p.handle.is_some()) else { return };
        let centre = player.ped.pos;
        let half = MAP_SIZE / 2.0;
        let dot = |o: &mut Overlay, p: Vec3, size: f32, colour: Rgba| {
            let (dx, dy) = ((p.x - centre.x) / MAP_RANGE * half, (p.y - centre.y) / MAP_RANGE * half);
            if dx.abs() < half - size && dy.abs() < half - size {
                // world y points north: up on the map
                let _ = o.rect(x + half + dx - size / 2.0, y + half - dy - size / 2.0, size, size, colour);
            }
        };
        for v in state::vehicles() {
            if let Some(i) = state::vehicle(v) {
                dot(o, i.pos, 6.0, VEHICLE);
            }
        }
        for p in state::peds() {
            if Some(p) != player.handle
                && let Some(i) = state::ped(p)
            {
                dot(o, i.pos, 4.0, PED);
            }
        }
        dot(o, centre, 6.0, PLAYER);
    }
}

impl Mod for TrainerHud {
    fn init(&mut self) -> Result<(), String> {
        sa_mod::events::subscribe(&[
            event_kind::FRAME_BEGIN,
            event_kind::GAME_STATE,
            event_kind::PLAYER_SPAWN,
            event_kind::PLAYER_DEATH,
            event_kind::VEHICLE_ENTER,
            event_kind::VEHICLE_EXIT,
            event_kind::AREA,
            event_kind::KEY_DOWN,
        ]);
        self.heading_hook = hooks::subscribe("CWorld::FindPlayerHeading", false, true).ok();
        self.process_hook = hooks::subscribe("CPed::ProcessControl", true, false).ok();
        sa_mod::info(&format!(
            "trainer-hud: loaded (hooks: FindPlayerHeading {}, CPed::ProcessControl {})",
            self.heading_hook.is_some(),
            self.process_hook.is_some()
        ));
        Ok(())
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::FrameBegin(n) => {
                self.frame = n;
                // a main-loop iteration where the game's logic did not run (the frame limiter) processes no ped
                if !self.processed.is_empty() {
                    self.processed_last = self.processed.len();
                    self.processed.clear();
                }
            }
            Event::KeyDown(VK_F7) => self.hidden = !self.hidden,
            Event::KeyDown(_) | Event::KeyUp(_) | Event::FrameEnd(_) => {}
            Event::GameState { new, old } => self.remember(format!("game state {old:?} -> {new:?}")),
            Event::PlayerSpawn(p) => self.remember(format!("player {:#x} spawned", p.0)),
            Event::PlayerDeath(p) => self.remember(format!("player {:#x} died", p.0)),
            Event::VehicleEnter { vehicle, .. } => {
                let model = state::vehicle(vehicle).map_or(-1, |v| v.model.0);
                self.remember(format!("entered vehicle {:#x} (model {model})", vehicle.0))
            }
            Event::VehicleExit { vehicle, .. } => self.remember(format!("left vehicle {:#x}", vehicle.0)),
            Event::Area { new, old } => self.remember(format!("area {old} -> {new}")),
        }
    }

    fn hook(&mut self, hook: HookId, phase: Phase, call: &Call) {
        if Some(hook) == self.heading_hook && phase == Phase::After {
            if let Some(Value::Float(h)) = call.result() {
                self.hooked_heading = Some(h);
            }
        } else if Some(hook) == self.process_hook
            && let Some(Value::Ped(p)) = call.arg(0)
            && !self.processed.contains(&p)
        {
            self.processed.push(p);
        }
    }

    fn overlay(&mut self, _frame: u32, o: &mut Overlay) {
        if self.hidden {
            return;
        }
        let lines = self.status_lines();
        let (x, y) = (MARGIN, MARGIN);
        let w = PANEL_CHARS * SIZE + 2.0 * PAD;
        let h = lines.len() as f32 * LINE + 2.0 * PAD;
        let _ = o.rect(x, y, w, h, BACKGROUND);
        for (i, line) in lines.iter().enumerate() {
            let colour = if i == 0 { TITLE } else { TEXT };
            let _ = o.text(x + PAD, y + PAD + i as f32 * LINE, SIZE, colour, line);
        }
        self.draw_map(o, x, y + h + MARGIN);
    }
}

sa_mod::entry!(TrainerHud);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units() {
        assert_eq!(degrees(-std::f32::consts::FRAC_PI_2), 270.0);
        assert!((speed_kmh(Vec3::new(0.0, 0.5, 0.0)) - 90.0).abs() < 1e-3);
    }

    #[test]
    fn keeps_the_last_events() {
        let mut m = TrainerHud::default();
        for i in 0..10 {
            m.event(Event::Area { new: i, old: i - 1 });
        }
        assert_eq!(m.events.len(), EVENTS_SHOWN);
        assert!(m.events[0].ends_with("area 8 -> 9"));
        m.event(Event::KeyDown(VK_F7));
        assert!(m.hidden);
    }
}
