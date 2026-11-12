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
    threads: Vec<Thread>,
    /// The thread `run_command` uses (locals copied in and out).
    one_shot: Option<ScriptThreadRef>,
    last: Opcode,
    in_command: bool,
    pub(crate) exec: CommandExecutor,
}

impl Threads {
    pub(crate) fn new(exec: CommandExecutor) -> Self {
        Self {
            threads: Vec::new(),
            one_shot: None,
            last: Opcode(0),
            in_command: false,
            exec,
        }
    }

    fn get(&self, t: ScriptThreadRef) -> GameResult<&Thread> {
        self.threads.get(t.0 as usize).ok_or(GameError::InvalidHandle)
    }

    fn get_mut(&mut self, t: ScriptThreadRef) -> GameResult<&mut Thread> {
        self.threads.get_mut(t.0 as usize).ok_or(GameError::InvalidHandle)
    }

    fn create(&mut self) -> ScriptThreadRef {
        self.threads.push(Thread::new());
        ScriptThreadRef(self.threads.len() as u32 - 1)
    }

    fn run(&mut self, t: ScriptThreadRef, op: Opcode, args: &[ScriptArg<'_>]) -> GameResult<bool> {
        let exec = self.exec;
        let th = self.get_mut(t)?;
        encode(op, args, &mut th.code);
        th.code.extend_from_slice(&[0; PADDING]);
        let ip = th.code.as_ptr() as usize as u32;
        th.script[layout::BASE_IP..layout::BASE_IP + 4].copy_from_slice(&ip.to_le_bytes());
        th.script[layout::IP..layout::IP + 4].copy_from_slice(&ip.to_le_bytes());
        th.script[layout::COND_RESULT] = 0;
        let script = GPtr::from_raw(th.script.as_mut_ptr());
        self.last = op;
        self.in_command = true;
        exec(script);
        self.in_command = false;
        Ok(self.get(t)?.script[layout::COND_RESULT] != 0)
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
