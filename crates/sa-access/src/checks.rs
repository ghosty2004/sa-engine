//! Arguments the game's script command handlers use without checking them, refused before a command runs.
//!
//! The handlers trust the mission script: they index tables with a weapon type, a weather type or a radar sprite,
//! build a vehicle from whatever model info the id names, copy a text into a fixed buffer for as many bytes as it
//! has, and append to fixed arrays without a bound. Any caller of this layer (a mod's `script_command`, the typed
//! methods driven by a SA:MP server's values) can pass what no mission script does, so every command run here
//! first goes through [`Game::unchecked_argument`]; a refused command is not run and returns an error.

use sa_types::script::{Opcode, ScriptArg};
use sa_types::{ModelId, ModelKind, VehicleRef};

use crate::addr::{self, off};
use crate::script::op;
use crate::{Game, View};

/// What one argument must be.
enum Rule {
    /// A vehicle model: `CCarCtrl::GetNewVehicleDependingOnCarModel` switches on the vehicle type of whatever
    /// model info the id names (and returns null for any other kind).
    VehicleModel,
    /// A ped model: the ped is built from the model info as a `CPedModelInfo`.
    PedModel,
    /// A model an object can be made of (atomic, clump or weapon): a ped's or a vehicle's clump as a `CObject`
    /// faults when it is set up or drawn.
    ObjectModel,
    /// An integer in this range: an index into a table of that size, or a value the game switches on.
    Range(i32, i32),
    /// A script handle of one of `n` slots (or -1): the game reads slot `handle & 0xFFFF` before it compares the
    /// handle's counter.
    Slot(u32),
    /// A passenger seat of the vehicle handle at the given argument, below its `m_nMaxPassengers` (-1, any seat,
    /// too when the flag is set): a ped put into a seat the vehicle refuses stays half inside it.
    Seat(usize, bool),
    /// A text of at most this many bytes: the handler copies the whole text into a buffer one byte longer.
    Text(usize),
    /// A model the game can load now: a vehicle model takes a `CVehicleStructure` while it is loaded, and with the
    /// pool full `CVehicleModelInfo::SetClump` gets none and fills it at null.
    Loadable,
}

/// Radar traces (`CRadar::ms_RadarTrace[175]`).
const RADAR_TRACES: u32 = 175;
/// Radar sprites (`CRadar::RadarBlipSprites[64]`).
const RADAR_SPRITES: i32 = 64;
/// The highest weapon type with its own `CWeaponInfo` (`aWeaponInfo`, standard skill).
const LAST_WEAPON: i32 = addr::WEAPON_INFO_STD_COUNT as i32 - 1;
/// The highest weather type (`CWeather`'s tables have 23 columns).
const LAST_WEATHER: i32 = 22;
/// The longest text label of the commands that take no longer one: an 8-byte buffer.
const DEFAULT_TEXT: usize = 7;

/// The rules, by opcode and argument index.
const RULES: &[(u16, usize, Rule)] = &[
    (op::CREATE_CHAR, 1, Rule::PedModel),
    (op::CREATE_CAR, 0, Rule::VehicleModel),
    (op::CREATE_OBJECT, 0, Rule::ObjectModel),
    (op::CREATE_PICKUP, 0, Rule::ObjectModel),
    (op::SET_PLAYER_MODEL, 1, Rule::PedModel),
    (op::SET_TIME_OF_DAY, 0, Rule::Range(0, 23)),
    (op::SET_TIME_OF_DAY, 1, Rule::Range(0, 59)),
    (op::FORCE_WEATHER_NOW, 0, Rule::Range(0, LAST_WEATHER)),
    (op::GIVE_WEAPON_TO_CHAR, 1, Rule::Range(0, LAST_WEAPON)),
    (op::SET_CURRENT_CHAR_WEAPON, 1, Rule::Range(0, LAST_WEAPON)),
    (op::ALTER_WANTED_LEVEL, 1, Rule::Range(0, 6)),
    (op::SET_MAX_WANTED_LEVEL, 0, Rule::Range(0, 6)),
    (op::ADD_SPRITE_BLIP_FOR_COORD, 3, Rule::Range(0, RADAR_SPRITES - 1)),
    (op::ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD, 3, Rule::Range(0, RADAR_SPRITES - 1)),
    (op::ADD_SPRITE_BLIP_FOR_CONTACT_POINT, 3, Rule::Range(0, RADAR_SPRITES - 1)),
    (op::ADD_SHORT_RANGE_SPRITE_BLIP_FOR_CONTACT_POINT, 3, Rule::Range(0, RADAR_SPRITES - 1)),
    (op::REMOVE_BLIP, 0, Rule::Slot(RADAR_TRACES)),
    (op::CHANGE_BLIP_COLOUR, 0, Rule::Slot(RADAR_TRACES)),
    (op::CHANGE_BLIP_SCALE, 0, Rule::Slot(RADAR_TRACES)),
    (op::CHANGE_BLIP_DISPLAY, 0, Rule::Slot(RADAR_TRACES)),
    (op::HAS_PICKUP_BEEN_COLLECTED, 0, Rule::Slot(addr::PICKUP_COUNT)),
    (op::REMOVE_PICKUP, 0, Rule::Slot(addr::PICKUP_COUNT)),
    (op::WARP_CHAR_INTO_CAR_AS_PASSENGER, 2, Rule::Seat(1, false)),
    (op::TASK_ENTER_CAR_AS_PASSENGER, 3, Rule::Seat(1, true)),
    (op::REQUEST_MODEL, 0, Rule::Loadable),
    (op::REQUEST_ANIMATION, 0, Rule::Text(15)),
    (op::HAS_ANIMATION_LOADED, 0, Rule::Text(15)),
    (op::TASK_PLAY_ANIM_NON_INTERRUPTABLE, 1, Rule::Text(23)),
    (op::TASK_PLAY_ANIM_NON_INTERRUPTABLE, 2, Rule::Text(15)),
];

impl<V: View> Game<V> {
    /// Why the game must not run `op` with `args`, if it must not: an argument breaks a rule of [`RULES`], a float
    /// is not finite (a position or angle the game would carry into its world sectors and matrices), a text is
    /// longer than its handler's buffer, or the command appends to a full table.
    pub fn unchecked_argument(&self, op: Opcode, args: &[ScriptArg<'_>]) -> Option<String> {
        for (i, a) in args.iter().enumerate() {
            match *a {
                ScriptArg::Float(v) if !v.is_finite() => return Some(format!("argument {i} is {v}")),
                ScriptArg::Text(t) => {
                    let max = RULES
                        .iter()
                        .find_map(|(o, j, r)| match r {
                            Rule::Text(n) if *o == op.0 && *j == i => Some(*n),
                            _ => None,
                        })
                        .unwrap_or(DEFAULT_TEXT);
                    if t.len() > max {
                        return Some(format!("text argument {i} is longer than {max} bytes"));
                    }
                }
                _ => {}
            }
        }
        for (_, i, rule) in RULES.iter().filter(|(o, _, _)| *o == op.0) {
            let Some(&ScriptArg::Int(v)) = args.get(*i) else { continue };
            if let Some(why) = self.breaks(rule, v, args) {
                return Some(format!("argument {i} ({v}): {why}"));
            }
        }
        if op.0 == op::ADD_HOSPITAL_RESTART
            && usize::from(self.view.read_u16(addr::HOSPITAL_RESTART_COUNT)) >= addr::RESTART_POINTS
        {
            return Some(format!("the game's {} hospital restart points are all used", addr::RESTART_POINTS));
        }
        None
    }

    /// Why `v` breaks `rule`, if it does.
    fn breaks(&self, rule: &Rule, v: i32, args: &[ScriptArg<'_>]) -> Option<String> {
        let kind = |k: &[ModelKind], what: &str| {
            (!k.contains(&self.model_kind(ModelId(v)))).then(|| format!("not {what} model"))
        };
        match *rule {
            Rule::VehicleModel => kind(&[ModelKind::Vehicle], "a vehicle"),
            Rule::PedModel => kind(&[ModelKind::Ped], "a ped"),
            Rule::ObjectModel => kind(&[ModelKind::Atomic, ModelKind::Clump, ModelKind::Weapon], "an object"),
            Rule::Range(lo, hi) => (!(lo..=hi).contains(&v)).then(|| format!("outside {lo}..={hi}")),
            Rule::Slot(n) => (v != -1 && v as u32 & 0xFFFF >= n).then(|| format!("not a handle of {n} slots")),
            Rule::Seat(vehicle, any) => {
                let Some(&ScriptArg::Int(h)) = args.get(vehicle) else { return Some("no vehicle".into()) };
                let a = self.vehicle_addr(VehicleRef(h as u32)).ok()?;
                let seats = i32::from(self.view.read_u8(a + off::VEH_MAX_PASSENGERS));
                let ok = (0..seats).contains(&v) || (any && v == -1);
                (!ok).then(|| format!("not a passenger seat of a vehicle with {seats}"))
            }
            Rule::Text(_) => None,
            Rule::Loadable => {
                let state = |m: i32| self.view.read_u8(addr::STREAMING_INFO + m as u32 * addr::STREAMING_INFO_SIZE + 0x10);
                if !addr::VEHICLE_MODELS.contains(&v) || state(v) != 0 {
                    return None;
                }
                // the vehicle models requested and not loaded yet take a structure each when they load
                let pending = addr::VEHICLE_MODELS.filter(|&m| (2..=4).contains(&state(m))).count() as u32;
                let free = addr::VEHICLE_STRUCTURE_POOL.free_slots(&self.view);
                (free <= pending).then(|| format!("{free} vehicle structures free for {} vehicle models", pending + 1))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{self, put};
    use sa_types::script::ScriptArg::{Float as F, Int as I, Text as T};

    #[test]
    fn ranges_texts_floats_and_slots() {
        let g = fake::game();
        let refused = |o: u16, a: &[ScriptArg<'_>]| g.unchecked_argument(Opcode(o), a).is_some();
        assert!(!refused(op::SET_TIME_OF_DAY, &[I(23), I(59)]));
        assert!(refused(op::SET_TIME_OF_DAY, &[I(24), I(0)]));
        assert!(refused(op::FORCE_WEATHER_NOW, &[I(23)]));
        assert!(refused(op::SET_CURRENT_CHAR_WEAPON, &[I(1), I(47)]));
        assert!(refused(op::ADD_SPRITE_BLIP_FOR_COORD, &[F(0.0), F(0.0), F(0.0), I(64), I(0)]));
        assert!(!refused(op::ADD_SPRITE_BLIP_FOR_COORD, &[F(0.0), F(0.0), F(0.0), I(63), I(0)]));
        assert!(refused(op::SET_CHAR_COORDINATES, &[I(1), F(f32::NAN), F(0.0), F(0.0)]));
        assert!(refused(op::CHANGE_BLIP_COLOUR, &[I(0x0001_C6FD), I(0)]));
        assert!(!refused(op::CHANGE_BLIP_COLOUR, &[I(0x0001_00AE), I(0)]));
        assert!(!refused(op::REMOVE_BLIP, &[I(-1)]));
        assert!(refused(op::REQUEST_ANIMATION, &[T(b"0123456789abcdef")]));
        assert!(!refused(op::REQUEST_ANIMATION, &[T(b"0123456789abcde")]));
        assert!(!refused(op::TASK_PLAY_ANIM_NON_INTERRUPTABLE, &[I(1), T(b"01234567890123456789012"), T(b"PED")]));
    }

    #[test]
    fn model_kinds() {
        let mut g = fake::game();
        // a vehicle model info at 0x1700000 for model 411
        put(&mut g, addr::MODEL_INFO_PTRS + 4 * 411, 0x0170_0000u32);
        put(&mut g, 0x0170_0000, crate::objects::VTBL_VEHICLE_MODEL_INFO);
        assert_eq!(g.unchecked_argument(Opcode(op::CREATE_CAR), &[I(411), F(0.0), F(0.0), F(0.0)]), None);
        assert!(g.unchecked_argument(Opcode(op::CREATE_OBJECT), &[I(411), F(0.0), F(0.0), F(0.0)]).is_some());
        assert!(g.unchecked_argument(Opcode(op::CREATE_CAR), &[I(412), F(0.0), F(0.0), F(0.0)]).is_some());
    }

    #[test]
    fn vehicle_models_need_a_free_structure() {
        let mut g = fake::game();
        // a pool of 2 structures, one used; model 411 requested (state 2)
        fake::pool(&mut g, addr::VEHICLE_STRUCTURE_POOL.global, 0x0180_0000, 0x0181_0000, 0x0180_1000, 2);
        put(&mut g, 0x0180_1000u32, 0x01u8);
        put(&mut g, addr::STREAMING_INFO + 411 * addr::STREAMING_INFO_SIZE + 0x10, 2u8);
        let request = |g: &Game<_>, m: i32| g.unchecked_argument(Opcode(op::REQUEST_MODEL), &[I(m)]);
        assert!(request(&g, 412).is_some());
        assert_eq!(request(&g, 411), None);
        assert_eq!(request(&g, 1337), None);
        put(&mut g, addr::STREAMING_INFO + 411 * addr::STREAMING_INFO_SIZE + 0x10, 1u8);
        assert_eq!(request(&g, 412), None);
    }

    #[test]
    fn seats_and_restart_points() {
        let mut g = fake::game();
        // vehicle slot 3, reuse counter 1, with 1 passenger seat
        put(&mut g, fake::VEH_MAP + 3, 0x01u8);
        put(&mut g, fake::VEHS + 3 * addr::VEHICLE_POOL.element + off::VEH_MAX_PASSENGERS, 1u8);
        let warp = |g: &Game<_>, seat| g.unchecked_argument(Opcode(op::WARP_CHAR_INTO_CAR_AS_PASSENGER), &[I(0x205), I(0x301), I(seat)]);
        assert_eq!(warp(&g, 0), None);
        assert!(warp(&g, 1).is_some());
        assert!(warp(&g, -1).is_some());
        let task = |seat| g.unchecked_argument(Opcode(op::TASK_ENTER_CAR_AS_PASSENGER), &[I(0x205), I(0x301), I(-1), I(seat)]);
        assert_eq!(task(-1), None);
        let restart = [F(0.0), F(0.0), F(0.0), F(0.0), I(0)];
        assert_eq!(g.unchecked_argument(Opcode(op::ADD_HOSPITAL_RESTART), &restart), None);
        put(&mut g, addr::HOSPITAL_RESTART_COUNT, 10u16);
        assert!(g.unchecked_argument(Opcode(op::ADD_HOSPITAL_RESTART), &restart).is_some());
    }
}
