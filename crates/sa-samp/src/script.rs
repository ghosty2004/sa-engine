//! Raw script (SCM) commands on the native engine: the native backend's `sa_samp_api::script` traits.
//!
//! Same mechanism as samp's ScriptCommand (samp-rs `script::Runner`): a private, zeroed `CRunningScript`
//! (0xE0 bytes) whose base and current IP point at one encoded instruction, run with
//! `CRunningScript::ProcessOneCommand` (0x469EB0). Natively that resolves through `orig!`'s address registry to
//! the sa-script port, so every opcode handler that is ported runs; an unported one panics with its name (the
//! phase B gap report). Thread buffers and code live in this process's memory, which is 32-bit addressable
//! in native sa-game (i686) and wasm32.

use sa_core::mem::GPtr;
use sa_samp_api::script::*;
use sa_samp_api::*;

use crate::{NativeGame, View};

sa_core::orig! {
    /// `CRunningScript::ProcessOneCommand()` (0x469EB0): returns the switch's AL.
    pub fn process_one_command(s: GPtr<u8>) -> i8 = thiscall 0x469EB0 => unported;
}

/// `CRunningScript` layout (gta-reversed `Scripts/RunningScript.h`).
pub(crate) mod layout {
    /// `sizeof(CRunningScript)`.
    pub const SIZE: usize = 0xE0;
    /// `m_szName[8]`.
    pub const NAME: usize = 0x08;
    /// `m_pBaseIP`.
    pub const BASE_IP: usize = 0x10;
    /// `m_pCurrentIP`.
    pub const IP: usize = 0x14;
    /// `m_aLocalVars[32]` (then the two timers).
    pub const LOCAL_VARS: usize = 0x3C;
    /// Number of local variables.
    pub const LOCALS: usize = 32;
    /// `m_bCondResult`.
    pub const COND_RESULT: usize = 0xC5;
}

/// The opcodes the typed methods issue (gta-reversed `eScriptCommands.h`; samp-rs `script::op`).
pub(crate) mod op {
    pub const SET_TIME_OF_DAY: u16 = 0x00C0;
    pub const CREATE_CAR: u16 = 0x00A5;
    pub const DELETE_CAR: u16 = 0x00A6;
    pub const CLEAR_PRINTS: u16 = 0x00BE;
    pub const ADD_ONE_OFF_SOUND: u16 = 0x018C;
    pub const SET_CHAR_COORDINATES: u16 = 0x00A1;
    pub const CREATE_OBJECT: u16 = 0x0107;
    pub const DELETE_OBJECT: u16 = 0x0108;
    pub const ADD_SCORE: u16 = 0x0109;
    pub const SET_FIXED_CAMERA_POSITION: u16 = 0x015F;
    pub const POINT_CAMERA_AT_POINT: u16 = 0x0160;
    pub const ADD_HOSPITAL_RESTART: u16 = 0x016C;
    pub const SET_CHAR_HEADING: u16 = 0x0173;
    pub const SET_CAR_HEADING: u16 = 0x0175;
    pub const GIVE_WEAPON_TO_CHAR: u16 = 0x01B2;
    pub const SET_PLAYER_CONTROL: u16 = 0x01B4;
    pub const SET_CURRENT_CHAR_WEAPON: u16 = 0x01B9;
    pub const SET_OBJECT_COORDINATES: u16 = 0x01BC;
    pub const SWITCH_ROADS_OFF: u16 = 0x01E8;
    pub const SET_CAR_DENSITY_MULTIPLIER: u16 = 0x01EB;
    pub const SET_MAX_WANTED_LEVEL: u16 = 0x01F0;
    pub const CREATE_PICKUP: u16 = 0x0213;
    pub const REMOVE_PICKUP: u16 = 0x0215;
    pub const CHANGE_CAR_COLOUR: u16 = 0x0229;
    pub const REQUEST_MODEL: u16 = 0x0247;
    pub const MARK_MODEL_AS_NO_LONGER_NEEDED: u16 = 0x0249;
    pub const SWITCH_WIDESCREEN: u16 = 0x02A3;
    pub const RESTORE_CAMERA_JUMPCUT: u16 = 0x02EB;
    pub const WARP_CHAR_FROM_CAR_TO_COORD: u16 = 0x0362;
    pub const WARP_CHAR_INTO_CAR: u16 = 0x036A;
    pub const SET_CAMERA_BEHIND_PLAYER: u16 = 0x0373;
    pub const LOAD_ALL_MODELS_NOW: u16 = 0x038B;
    pub const SET_PED_DENSITY_MULTIPLIER: u16 = 0x03DE;
    pub const CLEAR_CHAR_TASKS: u16 = 0x0687;
    pub const TASK_PLAY_ANIM_NON_INTERRUPTABLE: u16 = 0x0812;
    pub const WARP_CHAR_INTO_CAR_AS_PASSENGER: u16 = 0x0430;
    pub const SET_CHAR_SUFFERS_CRITICAL_HITS: u16 = 0x0446;
    pub const SET_OBJECT_ROTATION: u16 = 0x0453;
    pub const REMOVE_ALL_CHAR_WEAPONS: u16 = 0x048F;
    pub const SET_AREA_VISIBLE: u16 = 0x04BB;
    pub const FREEZE_CHAR_POSITION: u16 = 0x04D7;
    pub const REQUEST_COLLISION: u16 = 0x04E4;
    pub const REQUEST_ANIMATION: u16 = 0x04ED;
    pub const HAS_ANIMATION_LOADED: u16 = 0x04EE;
    pub const SET_PLAYER_DRUNKENNESS: u16 = 0x052C;
    pub const SWITCH_RANDOM_TRAINS: u16 = 0x06D7;
    pub const SET_VEHICLE_AREA_VISIBLE: u16 = 0x0840;
    pub const SET_CHAR_AREA_VISIBLE: u16 = 0x0860;
    pub const SET_CHAR_DROPS_WEAPONS_WHEN_DEAD: u16 = 0x087E;
    pub const DISPLAY_ZONE_NAMES: u16 = 0x09BA;
    pub const SET_PLAYER_MODEL: u16 = 0x09C7;
}

/// Parameter type bytes of the SCM format.
const T_INT32: u8 = 0x01;
const T_GLOBAL: u8 = 0x02;
const T_LOCAL: u8 = 0x03;
const T_FLOAT: u8 = 0x06;
const T_VARLEN_STRING: u8 = 0x0E;
/// Zero bytes after the instruction, so a handler reading one parameter too many stays in the buffer.
const PADDING: usize = 16;

/// Encodes one instruction in the public SCM format: opcode (u16 LE), then one type byte + value per
/// parameter (samp-rs `script::encode`).
pub fn encode(op: Opcode, args: &[ScriptArg<'_>], out: &mut Vec<u8>) {
    out.clear();
    out.extend_from_slice(&op.0.to_le_bytes());
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
            ScriptArg::Text(t) => {
                let b = &t[..t.len().min(255)];
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

/// Runs one command of the thread at the given address; returns `ProcessOneCommand`'s result.
pub type CommandExecutor = fn(thread: GPtr<u8>) -> i8;

/// The engine's executor: the sa-script port through the address registry.
pub(crate) fn engine_executor(thread: GPtr<u8>) -> i8 {
    process_one_command(thread)
}

/// One private script thread.
pub(crate) struct Thread {
    script: Box<[u8; layout::SIZE]>,
    code: Vec<u8>,
}

impl Thread {
    fn new() -> Self {
        let mut script = Box::new([0u8; layout::SIZE]);
        script[layout::NAME..layout::NAME + 6].copy_from_slice(b"sa-smp");
        Self {
            script,
            code: Vec::with_capacity(64),
        }
    }

    fn local(&self, i: usize) -> u32 {
        let o = layout::LOCAL_VARS + 4 * i;
        u32::from_le_bytes(self.script[o..o + 4].try_into().unwrap_or_default())
    }

    fn set_local(&mut self, i: usize, v: u32) {
        let o = layout::LOCAL_VARS + 4 * i;
        self.script[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
}

/// Script-thread state of the adapter.
pub(crate) struct Threads {
    /// Thread slots; a destroyed thread leaves `None`, reused by the next `create`.
    threads: Vec<Option<Thread>>,
    /// The thread `run_command` uses (locals copied in and out).
    one_shot: Option<ScriptThreadRef>,
    /// The backend's own thread for the typed methods that issue an opcode.
    backend: Option<ScriptThreadRef>,
    /// Every command issued (opcode and encoded instruction), for host tests.
    #[cfg(test)]
    pub(crate) issued: Vec<Vec<u8>>,
    last: Opcode,
    in_command: bool,
    pub(crate) exec: CommandExecutor,
}

impl Threads {
    pub(crate) fn new(exec: CommandExecutor) -> Self {
        Self {
            threads: Vec::new(),
            one_shot: None,
            backend: None,
            #[cfg(test)]
            issued: Vec::new(),
            last: Opcode(0),
            in_command: false,
            exec,
        }
    }

    fn get(&self, t: ScriptThreadRef) -> GameResult<&Thread> {
        self.threads
            .get(t.0 as usize)
            .and_then(Option::as_ref)
            .ok_or(GameError::InvalidHandle)
    }

    fn get_mut(&mut self, t: ScriptThreadRef) -> GameResult<&mut Thread> {
        self.threads
            .get_mut(t.0 as usize)
            .and_then(Option::as_mut)
            .ok_or(GameError::InvalidHandle)
    }

    fn create(&mut self) -> ScriptThreadRef {
        if let Some(i) = self.threads.iter().position(Option::is_none) {
            self.threads[i] = Some(Thread::new());
            return ScriptThreadRef(i as u32);
        }
        self.threads.push(Some(Thread::new()));
        ScriptThreadRef(self.threads.len() as u32 - 1)
    }

    fn destroy(&mut self, t: ScriptThreadRef) {
        if Some(t) == self.one_shot || Some(t) == self.backend {
            return;
        }
        if let Some(slot) = self.threads.get_mut(t.0 as usize) {
            *slot = None;
        }
    }

    fn run(&mut self, t: ScriptThreadRef, op: Opcode, args: &[ScriptArg<'_>]) -> GameResult<bool> {
        let exec = self.exec;
        let th = self.get_mut(t)?;
        encode(op, args, &mut th.code);
        #[cfg(test)]
        let issued = th.code.clone();
        th.code.extend_from_slice(&[0; PADDING]);
        let ip = th.code.as_ptr() as usize as u32;
        th.script[layout::BASE_IP..layout::BASE_IP + 4].copy_from_slice(&ip.to_le_bytes());
        th.script[layout::IP..layout::IP + 4].copy_from_slice(&ip.to_le_bytes());
        th.script[layout::COND_RESULT] = 0;
        let script = GPtr::from_raw(th.script.as_mut_ptr());
        #[cfg(test)]
        self.issued.push(issued);
        self.last = op;
        self.in_command = true;
        exec(script);
        self.in_command = false;
        Ok(self.get(t)?.script[layout::COND_RESULT] != 0)
    }
}

impl<V: View> NativeGame<V> {
    /// Runs one command on the backend's own script thread (made on first use) and returns the condition
    /// result: the typed methods that samp implements with an opcode run the same opcode through the ported
    /// interpreter.
    pub(crate) fn run_op(&mut self, op: u16, args: &[ScriptArg<'_>]) -> GameResult<bool> {
        let t = match self.threads.backend {
            Some(t) => t,
            None => {
                let t = self.threads.create();
                self.threads.backend = Some(t);
                t
            }
        };
        self.threads.run(t, Opcode(op), args)
    }

    /// Like [`NativeGame::run_op`], then reads local 0 of the backend thread (the output parameter of a
    /// command whose last parameter is `ScriptArg::Local(0)`).
    pub(crate) fn run_op_out(&mut self, op: u16, args: &[ScriptArg<'_>]) -> GameResult<u32> {
        self.run_op(op, args)?;
        let t = self.threads.backend.ok_or(GameError::InvalidHandle)?;
        Ok(self.threads.get(t)?.local(0))
    }
}

impl<V: View> ScriptCommands for NativeGame<V> {
    fn run_command(
        &mut self,
        op: Opcode,
        args: &[ScriptArg<'_>],
        locals: &mut ScriptLocals,
    ) -> GameResult<bool> {
        let t = match self.threads.one_shot {
            Some(t) => t,
            None => {
                let t = self.threads.create();
                self.threads.one_shot = Some(t);
                t
            }
        };
        for (i, v) in locals.0.iter().enumerate() {
            self.threads.get_mut(t)?.set_local(i, *v);
        }
        let cond = self.threads.run(t, op, args)?;
        let th = self.threads.get(t)?;
        for (i, v) in locals.0.iter_mut().enumerate() {
            *v = th.local(i);
        }
        Ok(cond)
    }

    fn create_script_thread(&mut self) -> GameResult<ScriptThreadRef> {
        Ok(self.threads.create())
    }

    fn destroy_script_thread(&mut self, thread: ScriptThreadRef) {
        self.threads.destroy(thread);
    }

    fn run_on_thread(
        &mut self,
        thread: ScriptThreadRef,
        op: Opcode,
        args: &[ScriptArg<'_>],
    ) -> GameResult<bool> {
        self.threads.run(thread, op, args)
    }

    fn thread_local(&self, thread: ScriptThreadRef, i: usize) -> GameResult<u32> {
        if i >= layout::LOCALS {
            return Err(GameError::InvalidHandle);
        }
        Ok(self.threads.get(thread)?.local(i))
    }

    fn set_thread_local(&mut self, thread: ScriptThreadRef, i: usize, value: u32) -> GameResult<()> {
        if i >= layout::LOCALS {
            return Err(GameError::InvalidHandle);
        }
        self.threads.get_mut(thread)?.set_local(i, value);
        Ok(())
    }

    fn last_command(&self) -> (Opcode, bool) {
        (self.threads.last, self.threads.in_command)
    }
}
