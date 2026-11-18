//! Courier: a small gameplay mod. F6 spawns a car in front of the player (the next of a few models each time), F10
//! removes the cars it spawned. F8 starts a run: a marker appears 150 m ahead and the player has 45 seconds to reach
//! it; a win pays $500 through the game's own `ADD_SCORE` command, clears the wanted level and plays the game's
//! pick-up sound, a timeout plays its fail sound. F9 toggles a damage shield: a before-hook on the game's damage
//! calculation skips it for the player's ped. Build: `examples/mods/build.sh` (docs/mods/getting-started.md).

use sa_mod::hooks::{self, Call, HookId, Phase};
use sa_mod::script::Arg;
use sa_mod::{Event, Mod, ObjectRef, Overlay, PedRef, Rgba, Value, Vec3, VehicleRef, entities, event_kind, state, world};

const VK_F6: u16 = 0x75;
const VK_F8: u16 = 0x77;
const VK_F9: u16 = 0x78;
const VK_F10: u16 = 0x79;

/// Infernus, Sultan, NRG-500, Bullet, BMX.
const CARS: [i32; 5] = [411, 560, 522, 541, 481];
/// The arrow marker object.
const MARKER_MODEL: i32 = 1318;
const RUN_DISTANCE: f32 = 150.0;
const RUN_SECONDS: u32 = 45;
const REACHED: f32 = 6.0;
const REWARD: i32 = 500;
/// `ADD_SCORE` (0109): money for the player through the game's own command.
const ADD_SCORE: u16 = 0x0109;
/// `AE_FRONTEND_PICKUP_INFO` / `AE_FRONTEND_ERROR`.
const SOUND_WIN: u32 = 0x0D;
const SOUND_FAIL: u32 = 0x05;

const SIZE: f32 = 10.0;
const LINE: f32 = 13.0;
const PANEL: Rgba = Rgba::new(0, 0, 0, 0xA8);
const TEXT: Rgba = Rgba::new(0xE8, 0xE8, 0xE8, 0xFF);
const TITLE: Rgba = Rgba::new(0xFF, 0xC0, 0x40, 0xFF);

/// A run to a marker.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Run {
    target: Vec3,
    marker: Option<ObjectRef>,
    /// Game time when the run ends, in milliseconds.
    deadline_ms: u32,
}

#[derive(Default)]
struct Courier {
    cars: Vec<VehicleRef>,
    next_car: usize,
    run: Option<Run>,
    shield: bool,
    damage_hook: Option<HookId>,
    message: String,
    wins: u32,
    /// Damage the shield kept off the player.
    blocked: u32,
}

/// The point `distance` ahead of a ped at `pos` facing `heading` (radians, 0 = north).
fn ahead(pos: Vec3, heading: f32, distance: f32) -> Vec3 {
    Vec3::new(pos.x - heading.sin() * distance, pos.y + heading.cos() * distance, pos.z)
}

fn distance_2d(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn now_ms() -> u32 {
    state::world().map_or(0, |w| w.time_ms)
}

impl Courier {
    fn say(&mut self, text: String) {
        sa_mod::info(&format!("courier: {text}"));
        self.message = text;
    }

    fn spawn_car(&mut self) {
        let Some(p) = state::player().filter(|p| p.handle.is_some()) else { return };
        if p.ped.vehicle.is_some() {
            return self.say("leave the vehicle first".into());
        }
        let model = CARS[self.next_car % CARS.len()];
        self.next_car += 1;
        let at = ahead(p.ped.pos, p.ped.heading, 5.0);
        match entities::create_vehicle(model, at, p.ped.heading.to_degrees() + 90.0) {
            Ok(v) => {
                self.cars.push(v);
                self.say(format!("spawned model {model} as {:#x}", v.0));
            }
            Err(e) => self.say(format!("could not spawn model {model}: {e:?}")),
        }
    }

    fn remove_cars(&mut self) {
        let n = self.cars.len();
        for v in self.cars.drain(..) {
            let _ = entities::destroy_vehicle(v);
        }
        self.say(format!("removed {n} cars"));
    }

    fn start_run(&mut self) {
        let Some(p) = state::player().filter(|p| p.handle.is_some()) else { return };
        let heading = p.ped.vehicle.and_then(state::vehicle).map_or(p.ped.heading, |v| (-v.forward.x).atan2(v.forward.y));
        let target = ahead(p.ped.pos, heading, RUN_DISTANCE);
        let marker = entities::create_object(MARKER_MODEL, Vec3::new(target.x, target.y, target.z + 2.0)).ok();
        self.end_run();
        self.run = Some(Run { target, marker, deadline_ms: now_ms() + RUN_SECONDS * 1000 });
        self.say(format!("run: reach the marker {RUN_DISTANCE:.0} m ahead in {RUN_SECONDS} s"));
    }

    fn end_run(&mut self) {
        if let Some(Run { marker: Some(m), .. }) = self.run.take() {
            let _ = entities::destroy_object(m);
        }
    }

    /// Checks the run at the start of a frame.
    fn check_run(&mut self) {
        let Some(run) = self.run else { return };
        let Some(p) = state::player().filter(|p| p.handle.is_some()) else { return };
        if distance_2d(p.ped.pos, run.target) <= REACHED {
            self.end_run();
            self.wins += 1;
            let _ = sa_mod::script::run(ADD_SCORE, &[Arg::Int(0), Arg::Int(REWARD)]);
            let _ = world::set_wanted_level(0);
            let _ = sa_mod::audio::frontend(SOUND_WIN);
            self.say(format!("delivered! +${REWARD} (wins: {})", self.wins));
        } else if now_ms() >= run.deadline_ms {
            self.end_run();
            let _ = sa_mod::audio::frontend(SOUND_FAIL);
            self.say("too late".into());
        }
    }

    fn lines(&self) -> Vec<String> {
        let mut out = vec!["COURIER  F6 car  F8 run  F9 shield  F10 clear".to_string()];
        let shield = if self.shield { "on" } else { "off" };
        out.push(format!("cars {}  shield {shield} ({} blocked)  wins {}", self.cars.len(), self.blocked, self.wins));
        if let (Some(run), Some(p)) = (self.run, state::player()) {
            let left = run.deadline_ms.saturating_sub(now_ms()) / 1000;
            out.push(format!("marker {:.0} m  {left} s left", distance_2d(p.ped.pos, run.target)));
        }
        if !self.message.is_empty() {
            out.push(self.message.clone());
        }
        out
    }
}

impl Mod for Courier {
    fn init(&mut self) -> Result<(), String> {
        sa_mod::events::subscribe(&[event_kind::FRAME_BEGIN, event_kind::KEY_DOWN, event_kind::PLAYER_DEATH]);
        self.damage_hook = hooks::subscribe("CPedDamageResponseCalculator::ComputeDamageResponse", true, false).ok();
        Ok(())
    }

    fn event(&mut self, event: Event) {
        match event {
            Event::FrameBegin(_) => self.check_run(),
            Event::KeyDown(VK_F6) => self.spawn_car(),
            Event::KeyDown(VK_F8) => self.start_run(),
            Event::KeyDown(VK_F9) => {
                self.shield = !self.shield;
                self.say(format!("damage shield {}", if self.shield { "on" } else { "off" }));
            }
            Event::KeyDown(VK_F10) => self.remove_cars(),
            Event::PlayerDeath(_) if self.run.is_some() => {
                self.end_run();
                self.say("run lost".into());
            }
            _ => {}
        }
    }

    fn hook(&mut self, hook: HookId, phase: Phase, call: &Call) {
        if !self.shield || Some(hook) != self.damage_hook || phase != Phase::Before {
            return;
        }
        let player: Option<PedRef> = state::player().and_then(|p| p.handle);
        if call.arg(1) == player.map(Value::Ped) && call.skip(Value::Int(0)).is_ok() {
            self.blocked += 1;
            if self.blocked == 1 {
                sa_mod::info("courier: the shield blocked damage to the player");
            }
        }
    }

    fn overlay(&mut self, _frame: u32, o: &mut Overlay) {
        let lines = self.lines();
        let (w, h) = o.size();
        let width = 46.0 * SIZE;
        let height = lines.len() as f32 * LINE + 12.0;
        // bottom right, clear of the radar and the trainer HUD's panel
        let (x, y) = (w as f32 - width - 8.0, h as f32 - height - 8.0);
        let _ = o.rect(x, y, width, height, PANEL);
        for (i, l) in lines.iter().enumerate() {
            let _ = o.text(x + 6.0, y + 6.0 + i as f32 * LINE, SIZE, if i == 0 { TITLE } else { TEXT }, l);
        }
    }
}

sa_mod::entry!(Courier);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ahead_follows_the_heading() {
        let p = ahead(Vec3::new(10.0, 20.0, 3.0), 0.0, 5.0);
        assert!((p.x - 10.0).abs() < 1e-5 && (p.y - 25.0).abs() < 1e-5 && p.z == 3.0);
        // 90 degrees: facing west (-x)
        let p = ahead(Vec3::new(0.0, 0.0, 0.0), std::f32::consts::FRAC_PI_2, 5.0);
        assert!((p.x + 5.0).abs() < 1e-5 && p.y.abs() < 1e-5);
        assert_eq!(distance_2d(Vec3::new(0.0, 0.0, 9.0), Vec3::new(3.0, 4.0, 0.0)), 5.0);
    }

    #[test]
    fn keys_toggle_the_shield() {
        let mut m = Courier::default();
        m.event(Event::KeyDown(VK_F9));
        assert!(m.shield);
        assert!(m.lines()[1].contains("shield on"));
    }
}
