//! Running single GTA script (SCM) commands from our code, the way samp drives most player and camera
//! changes: [`encode`] writes one instruction in the public SCM format (opcode u16, then one type byte +
//! value per parameter), and `Runner` points a private, zeroed `CRunningScript` at it and calls
//! `CRunningScript::ProcessOneCommand`. Output parameters land in the thread's local variables, which are
//! read back afterwards. Opcode and layout names follow gta-reversed (`eScriptCommands.h`,
//! `RunningScript.h`). Spec: docs/re/subsystems/player-state.md, "ScriptCommand".

use sa_samp_api::script::ScriptArg;

/// Opcodes used by the client (gta-reversed `eScriptCommands.h`).
pub mod op {
    pub const CREATE_PLAYER: u16 = 0x0053;
    pub const SET_CHAR_PROOFS: u16 = 0x02AB;
    pub const SET_CHAR_MONEY: u16 = 0x03FE;
    pub const MARK_MODEL_AS_NO_LONGER_NEEDED: u16 = 0x0249;
    pub const SET_CHAR_SUFFERS_CRITICAL_HITS: u16 = 0x0446;
    pub const REQUEST_ANIMATION: u16 = 0x04ED;
    pub const HAS_ANIMATION_LOADED: u16 = 0x04EE;
    pub const SET_CHAR_DROPS_WEAPONS_WHEN_DEAD: u16 = 0x087E;
    // Local per-frame polls and special actions.
    pub const SET_PLAYER_DRUNKENNESS: u16 = 0x052C;
    /// RPC 150's player command (parameters "ii"; public lists call the RPC "set player drunk handling",
    /// the opcode's name is unverified).
    pub const RPC150_PLAYER_COMMAND: u16 = 0x03FD;
    pub const GET_CURRENT_CHAR_WEAPON: u16 = 0x0470;
    pub const GET_AREA_VISIBLE: u16 = 0x077E;
    pub const TASK_HANDS_UP: u16 = 0x05C4;
    pub const TASK_USE_MOBILE_PHONE: u16 = 0x0729;
    pub const DISPLAY_ZONE_NAMES: u16 = 0x09BA;
    pub const SET_TIME_OF_DAY: u16 = 0x00C0;
    pub const SWITCH_ROADS_OFF: u16 = 0x01E8;
    pub const SET_MAX_WANTED_LEVEL: u16 = 0x01F0;
    pub const SET_CAR_DENSITY_MULTIPLIER: u16 = 0x01EB;
    pub const GET_PLAYER_CHAR: u16 = 0x01F5;
    pub const SET_PED_DENSITY_MULTIPLIER: u16 = 0x03DE;
    pub const ADD_HOSPITAL_RESTART: u16 = 0x016C;
    pub const BUILD_PLAYER_MODEL: u16 = 0x070D;
    pub const SET_FIXED_CAMERA_POSITION: u16 = 0x015F;
    pub const POINT_CAMERA_AT_POINT: u16 = 0x0160;
    pub const SET_CHAR_HEADING: u16 = 0x0173;
    /// `GET_GROUND_Z_FOR_3D_COORD` (x, y, z → float); SetPlayerPosFindZ (RPC 13).
    pub const GET_GROUND_Z_FOR_3D_COORD: u16 = 0x02CE;
    pub const GIVE_WEAPON_TO_CHAR: u16 = 0x01B2;
    pub const SET_CURRENT_CHAR_WEAPON: u16 = 0x01B9;
    pub const SET_PLAYER_CONTROL: u16 = 0x01B4;
    pub const REQUEST_MODEL: u16 = 0x0247;
    pub const HAS_MODEL_LOADED: u16 = 0x0248;
    pub const RESTORE_CAMERA_JUMPCUT: u16 = 0x02EB;
    pub const SET_CAMERA_BEHIND_PLAYER: u16 = 0x0373;
    pub const LOAD_ALL_MODELS_NOW: u16 = 0x038B;
    pub const FREEZE_CHAR_POSITION: u16 = 0x04D7;
    pub const REQUEST_COLLISION: u16 = 0x04E4;
    pub const SWITCH_RANDOM_TRAINS: u16 = 0x06D7;
    pub const CAMERA_RESET_NEW_SCRIPTABLES: u16 = 0x0925;
    pub const CAMERA_PERSIST_POS: u16 = 0x0930;
    // Vehicles (parameter lists from the public SCM opcode database).
    pub const CREATE_CAR: u16 = 0x00A5;
    pub const DELETE_CAR: u16 = 0x00A6;
    pub const POINT_CAMERA_AT_CAR: u16 = 0x0158;
    pub const ADD_BLIP_FOR_CAR_OLD: u16 = 0x0161;
    pub const REMOVE_BLIP: u16 = 0x0164;
    pub const CHANGE_BLIP_COLOUR: u16 = 0x0165;
    pub const CHANGE_BLIP_SCALE: u16 = 0x0168;
    pub const SET_CAR_HEADING: u16 = 0x0175;
    pub const SET_CAR_PROOFS: u16 = 0x02AC;
    pub const IS_CAR_IN_WATER: u16 = 0x02BF;
    pub const WARP_CHAR_FROM_CAR_TO_COORD: u16 = 0x0362;
    pub const WARP_CHAR_INTO_CAR: u16 = 0x036A;
    pub const WARP_CHAR_INTO_CAR_AS_PASSENGER: u16 = 0x0430;
    pub const SET_CAN_BURST_CAR_TYRES: u16 = 0x053F;
    pub const TASK_ENTER_CAR_AS_PASSENGER: u16 = 0x05CA;
    pub const TASK_ENTER_CAR_AS_DRIVER: u16 = 0x05CB;
    pub const TASK_LEAVE_CAR: u16 = 0x05CD;
    pub const CREATE_MISSION_TRAIN: u16 = 0x06D8;
    pub const ADD_VEHICLE_MOD: u16 = 0x06E7;
    pub const REMOVE_VEHICLE_MOD: u16 = 0x06E8;
    pub const REQUEST_VEHICLE_MOD: u16 = 0x06E9;
    pub const HAS_VEHICLE_MOD_LOADED: u16 = 0x06EA;
    pub const GIVE_VEHICLE_PAINTJOB: u16 = 0x06ED;
    pub const DETACH_TRAILER_FROM_CAB: u16 = 0x07AC;
    pub const DELETE_MISSION_TRAIN: u16 = 0x07BD;
    pub const SET_MISSION_TRAIN_COORDINATES: u16 = 0x07C7;
    pub const SET_CAR_HYDRAULICS: u16 = 0x07FF;
    pub const SET_VEHICLE_AREA_VISIBLE: u16 = 0x0840;
    pub const ATTACH_TRAILER_TO_CAB: u16 = 0x0893;
    pub const SET_PETROL_TANK_WEAKPOINT: u16 = 0x09C4;
    // Combat.
    /// `EXPLODE_CHAR_HEAD` (remote death, RPC 166).
    pub const EXPLODE_CHAR_HEAD: u16 = 0x0321;
    /// `ADD_EXPLOSION_VARIABLE_SHAKE`-style "fffii" command samp uses for RPC 79
    /// (docs/re/subsystems/player-state.md, "79 CreateExplosion").
    pub const ADD_EXPLOSION: u16 = 0x0948;
    /// `TASK_DRIVE_BY` (drive-by of a passenger).
    pub const TASK_DRIVE_BY: u16 = 0x0713;
    // Camera, spectating, interiors, animations, misc.
    pub const ADD_SCORE: u16 = 0x0109;
    pub const CLEAR_PRINTS: u16 = 0x00BE;
    pub const POINT_CAMERA_AT_CHAR: u16 = 0x0159;
    pub const SWITCH_WIDESCREEN: u16 = 0x02A3;
    pub const SET_AREA_VISIBLE: u16 = 0x04BB;
    pub const SET_CHAR_AREA_VISIBLE: u16 = 0x0860;
    pub const GIVE_MELEE_ATTACK_TO_CHAR: u16 = 0x07FE;
    pub const TASK_PLAY_ANIM_NON_INTERRUPTABLE: u16 = 0x0812;
    pub const CAMERA_SET_VECTOR_TRACK: u16 = 0x0920;
    pub const CAMERA_PERSIST_TRACK: u16 = 0x092F;
    pub const CAMERA_SET_VECTOR_MOVE: u16 = 0x0936;
    // Player markers.
    pub const ADD_BLIP_FOR_CHAR: u16 = 0x0187;
    pub const CHANGE_BLIP_DISPLAY: u16 = 0x018B;
    pub const ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD: u16 = 0x04CE;
    // Held items, piss and the remote parachute.
    pub const CREATE_OBJECT: u16 = 0x0107;
    pub const ADD_ONE_OFF_SOUND: u16 = 0x018C;
    pub const PLAY_FX_SYSTEM: u16 = 0x064C;
    pub const KILL_FX_SYSTEM: u16 = 0x0650;
    pub const CREATE_FX_SYSTEM_ON_CHAR_WITH_DIRECTION: u16 = 0x066A;
    pub const DETACH_OBJECT: u16 = 0x0682;
    pub const ATTACH_OBJECT_TO_CHAR: u16 = 0x069B;
    pub const TASK_PICK_UP_OBJECT: u16 = 0x070A;
    pub const SET_OBJECT_VISIBLE: u16 = 0x0750;
    pub const PLAY_OBJECT_ANIM: u16 = 0x075A;
    pub const SET_OBJECT_SCALE: u16 = 0x08D2;
    pub const REMOVE_OBJECT_ELEGANTLY: u16 = 0x09A2;
}

/// One script command parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Arg<'a> {
    /// 32-bit integer immediate (type byte 0x01).
    Int(i32),
    /// Float immediate (type byte 0x06).
    Float(f32),
    /// Variable-length string (type byte 0x0E + u8 length + bytes).
    Text(&'a str),
    /// The same encoding for raw bytes (network strings, not necessarily UTF-8).
    Bytes(&'a [u8]),
    /// Global variable (type byte 0x02 + u16 byte offset into ScriptSpace).
    Global(u16),
    /// Local variable of the private thread (type byte 0x03 + u16 index);
    /// used for output parameters.
    Local(u16),
}

/// Parameter type bytes of the SCM format.
const T_INT32: u8 = 0x01;
const T_GLOBAL: u8 = 0x02;
const T_LOCAL: u8 = 0x03;
const T_FLOAT: u8 = 0x06;
const T_VARLEN_STRING: u8 = 0x0E;

/// The adapter's form of a parameter (`Text` and `Bytes` are both raw bytes there).
pub fn to_api<'a>(a: &Arg<'a>) -> ScriptArg<'a> {
    match *a {
        Arg::Int(v) => ScriptArg::Int(v),
        Arg::Float(v) => ScriptArg::Float(v),
        Arg::Text(t) => ScriptArg::Text(t.as_bytes()),
        Arg::Bytes(b) => ScriptArg::Text(b),
        Arg::Global(o) => ScriptArg::Global(o),
        Arg::Local(i) => ScriptArg::Local(i),
    }
}

/// Encodes one instruction: opcode (u16 LE) followed by the parameters.
pub fn encode(opcode: u16, args: &[ScriptArg], out: &mut Vec<u8>) {
    out.clear();
    out.extend_from_slice(&opcode.to_le_bytes());
    for a in args {
        match *a {
            ScriptArg::Int(v) => {
                out.push(T_INT32);
                out.extend_from_slice(&v.to_le_bytes());
            }
            ScriptArg::Float(v) => {
                out.push(T_FLOAT);
                out.extend_from_slice(&v.to_le_bytes());
            }
            ScriptArg::Text(raw) => {
                let b = &raw[..raw.len().min(255)];
                out.push(T_VARLEN_STRING);
                out.push(b.len() as u8);
                out.extend_from_slice(b);
            }
            ScriptArg::Global(o) => {
                out.push(T_GLOBAL);
                out.extend_from_slice(&o.to_le_bytes());
            }
            ScriptArg::Local(i) => {
                out.push(T_LOCAL);
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
    }
}

/// `CRunningScript` layout (gta-reversed `Scripts/RunningScript.h`).
pub mod layout {
    pub const SIZE: usize = 0xE0;
    pub const BASE_IP: usize = 0x10;
    pub const IP: usize = 0x14;
    pub const LOCAL_VARS: usize = 0x3C;
    pub const COND_RESULT: usize = 0xC5;
}

#[cfg(windows)]
pub use runner::{Runner, in_command, last_opcode};

#[cfg(windows)]
mod runner {
    use sa_samp_api::script::{Opcode, ScriptArg, ScriptCommands, ScriptThreadRef};

    use super::{Arg, to_api};
    use crate::adapter::original::process;

    /// The last opcode run, shown by the crash report as "SCM Op". Mirrors samp.dll B 0x1026B3D4.
    pub fn last_opcode() -> u16 {
        process().last_command().0.0
    }

    /// Whether a command is running, shown by the crash report as "lDbg". Mirrors samp.dll B 0x1026B3D8.
    pub fn in_command() -> bool {
        process().last_command().1
    }

    /// Most parameters of one command (samp's longest has 13).
    const MAX_ARGS: usize = 16;

    /// A private script thread of the game adapter (`ScriptCommands::create_script_thread`). Only use it on the
    /// game's main thread.
    pub struct Runner {
        thread: ScriptThreadRef,
    }

    impl Default for Runner {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for Runner {
        fn drop(&mut self) {
            process().destroy_script_thread(self.thread);
        }
    }

    impl Runner {
        pub fn new() -> Self {
            // The original backend's thread creation is an allocation and cannot fail.
            let thread = process().create_script_thread().unwrap_or(ScriptThreadRef(0));
            Self { thread }
        }

        /// Executes one command; returns the thread's condition result (meaningful for conditional opcodes
        /// only). A fault inside the command is caught and logged by the adapter and reads as `false`.
        /// Mirrors samp.dll 0x100B2350 (samp's ScriptCommand).
        pub fn run(&mut self, opcode: u16, args: &[Arg]) -> bool {
            let mut buf = [ScriptArg::Int(0); MAX_ARGS];
            let owned: Vec<ScriptArg>;
            let api: &[ScriptArg] = if args.len() <= MAX_ARGS {
                for (d, a) in buf.iter_mut().zip(args) {
                    *d = to_api(a);
                }
                &buf[..args.len()]
            } else {
                owned = args.iter().map(to_api).collect();
                &owned
            };
            process().run_on_thread(self.thread, Opcode(opcode), api).unwrap_or(false)
        }

        pub fn local_i32(&self, i: u16) -> i32 {
            process().thread_local(self.thread, usize::from(i)).unwrap_or(0) as i32
        }

        pub fn set_local_i32(&mut self, i: u16, v: i32) {
            let _ = process().set_thread_local(self.thread, usize::from(i), v as u32);
        }

        pub fn local_f32(&self, i: u16) -> f32 {
            f32::from_bits(self.local_i32(i) as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_typed_parameters() {
        let mut b = Vec::new();
        encode(op::SET_PLAYER_CONTROL, &[to_api(&Arg::Int(0)), to_api(&Arg::Int(1))], &mut b);
        assert_eq!(b, [0xB4, 0x01, 0x01, 0, 0, 0, 0, 0x01, 1, 0, 0, 0]);
        encode(op::REQUEST_COLLISION, &[to_api(&Arg::Float(1.0)), to_api(&Arg::Local(3))], &mut b);
        assert_eq!(b, [0xE4, 0x04, 0x06, 0x00, 0x00, 0x80, 0x3F, 0x03, 3, 0]);
        encode(op::HAS_ANIMATION_LOADED, &[to_api(&Arg::Text("BAR"))], &mut b);
        let mut c = Vec::new();
        encode(op::HAS_ANIMATION_LOADED, &[to_api(&Arg::Bytes(b"BAR"))], &mut c);
        assert_eq!(b, c);
        assert_eq!(b, [0xEE, 0x04, 0x0E, 3, b'B', b'A', b'R']);
    }

    #[test]
    fn layout_is_consistent() {
        // 32 locals + 2 timers end where the flags begin.
        assert_eq!(layout::LOCAL_VARS + 34 * 4, 0xC4);
        const { assert!(layout::COND_RESULT < layout::SIZE) };
    }
}
