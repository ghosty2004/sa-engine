//! Raw script (SCM) commands on the original executable: the original backend's `sa_samp_api::script` traits.
//!
//! A script thread is samp's ScriptCommand mechanism (samp.dll 0x100B2350): a private, zeroed `CRunningScript`
//! whose base and current IP point at one encoded instruction, run with `CRunningScript::ProcessOneCommand`.
//! Output parameters land in the thread's local variables, which persist between commands. A handle is an index
//! into the backend's thread table; the client's `script::Runner` owns one thread and destroys it when dropped.
//! `run_command` is not routed through the adapter yet.

use sa_samp_api::script::*;
use sa_samp_api::*;

use std::sync::Mutex;

use super::OriginalGame;
use crate::patch::Memory;
use crate::script::layout;

/// One private script thread: the `CRunningScript` buffer and the encoded instruction it runs.
pub struct ScriptThread {
    thread: Box<[u8; layout::SIZE]>,
    code: Vec<u8>,
}

impl ScriptThread {
    fn new() -> Self {
        let mut thread = Box::new([0u8; layout::SIZE]);
        thread[0x08..0x0F].copy_from_slice(b"samp-rs");
        Self { thread, code: Vec::with_capacity(64) }
    }

    fn local(&self, i: usize) -> u32 {
        let o = layout::LOCAL_VARS + 4 * i;
        u32::from_le_bytes(self.thread[o..o + 4].try_into().unwrap_or_default())
    }

    fn set_local(&mut self, i: usize, v: u32) {
        let o = layout::LOCAL_VARS + 4 * i;
        self.thread[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
}

/// Number of local variables of a `CRunningScript`.
const LOCALS: usize = 32;

/// The threads, by handle - 1. A slot is `None` once released, and while its thread runs a command (the
/// thread is taken out so that no lock is held while game code runs).
static THREADS: Mutex<Vec<Option<Box<ScriptThread>>>> = Mutex::new(Vec::new());

fn table() -> std::sync::MutexGuard<'static, Vec<Option<Box<ScriptThread>>>> {
    THREADS.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Runs `f` on the thread `t` names.
fn with_thread<R>(t: ScriptThreadRef, f: impl FnOnce(&mut ScriptThread) -> R) -> GameResult<R> {
    let i = (t.0 as usize).checked_sub(1).ok_or(GameError::InvalidHandle)?;
    let mut th = table().get_mut(i).and_then(Option::take).ok_or(GameError::InvalidHandle)?;
    let r = f(&mut th);
    table()[i] = Some(th);
    Ok(r)
}

#[cfg(windows)]
mod run {
    use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

    use sa_samp_api::script::{Opcode, ScriptArg};
    use sa_samp_api::{GameError, GameResult};

    use super::ScriptThread;
    use crate::script::{encode, layout, op};

    /// The last opcode run, shown by the crash report as "SCM Op". Mirrors samp.dll B 0x1026B3D4.
    static LAST_OPCODE: AtomicU16 = AtomicU16::new(0);
    /// Whether a command is running, shown by the crash report as "lDbg". Mirrors samp.dll B 0x1026B3D8.
    static IN_COMMAND: AtomicBool = AtomicBool::new(false);

    pub fn last() -> (Opcode, bool) {
        (Opcode(LAST_OPCODE.load(Ordering::SeqCst)), IN_COMMAND.load(Ordering::SeqCst))
    }

    /// `CRunningScript::ProcessOneCommand` (thiscall, returns OpcodeResult).
    const PROCESS_ONE_COMMAND: usize = 0x469EB0;

    /// Executes one command; returns the thread's condition result (meaningful for conditional opcodes only).
    /// A fault inside the command is caught, logged, reported to `swap` and returned as `GameError::Fault`.
    /// Mirrors samp.dll 0x100B2350 (samp's ScriptCommand).
    pub fn run(t: &mut ScriptThread, opcode: u16, args: &[ScriptArg<'_>]) -> GameResult<bool> {
        encode(opcode, args, &mut t.code);
        // Padding so a handler reading one parameter too many stays inside our buffer.
        t.code.extend_from_slice(&[0; 16]);
        let ip = t.code.as_ptr() as u32;
        t.thread[layout::BASE_IP..layout::BASE_IP + 4].copy_from_slice(&ip.to_le_bytes());
        t.thread[layout::IP..layout::IP + 4].copy_from_slice(&ip.to_le_bytes());
        t.thread[layout::COND_RESULT] = 0;
        // SAFETY: the game's interpreter on the main thread, with a zeroed CRunningScript whose IP points at a
        // well-formed instruction that outlives the call; faults are caught by the shared SEH guard.
        LAST_OPCODE.store(opcode, Ordering::SeqCst);
        IN_COMMAND.store(true, Ordering::SeqCst);
        let code = unsafe { crate::swap::guarded_thiscall(PROCESS_ONE_COMMAND as u32, t.thread.as_mut_ptr() as u32) };
        IN_COMMAND.store(false, Ordering::SeqCst);
        if code != 0 {
            let component = match (opcode, args.get(1)) {
                (op::ADD_VEHICLE_MOD, Some(ScriptArg::Int(c))) => Some(*c as u32),
                _ => None,
            };
            let address = crate::swap::last_fault_address();
            crate::log!(
                "script: opcode {opcode:#06x} faulted: {code:#x} at {address:#x} ({})",
                crate::swap::last_fault_details()
            );
            crate::swap::push_fault(crate::swap::FaultSite::Opcode { opcode, component }, code, address);
            return Err(GameError::Fault { code, address });
        }
        Ok(t.thread[layout::COND_RESULT] != 0)
    }
}

/// The backend's own thread for typed methods that issue an opcode (handle, 0 = not created yet).
static BACKEND_THREAD: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

impl<M: Memory> OriginalGame<M> {
    /// Runs one command on the backend's own script thread (made on first use): the typed methods that the
    /// client used to issue through its own `Runner` run the same opcode with the same parameters here.
    pub(super) fn run_op(&mut self, op: u16, args: &[ScriptArg<'_>]) -> GameResult<bool> {
        backend_op(op, args)
    }
}

/// A new thread in the table (handle = index + 1).
fn new_thread() -> ScriptThreadRef {
    let mut tab = table();
    let th = Some(Box::new(ScriptThread::new()));
    let i = match tab.iter().position(Option::is_none) {
        Some(i) => {
            tab[i] = th;
            i
        }
        None => {
            tab.push(th);
            tab.len() - 1
        }
    };
    ScriptThreadRef(i as u32 + 1)
}

/// [`OriginalGame::run_op`] without the adapter value, for the `&self` queries that issue a conditional opcode
/// (`HAS_MODEL_LOADED`, ...): the thread table is process-wide.
pub(super) fn backend_op(op: u16, args: &[ScriptArg<'_>]) -> GameResult<bool> {
    use std::sync::atomic::Ordering;
    let mut t = BACKEND_THREAD.load(Ordering::Relaxed);
    if t == 0 {
        t = new_thread().0;
        BACKEND_THREAD.store(t, Ordering::Relaxed);
    }
    #[cfg(windows)]
    {
        with_thread(ScriptThreadRef(t), |th| run::run(th, op, args))?
    }
    #[cfg(not(windows))]
    {
        with_thread(ScriptThreadRef(t), |th| crate::script::encode(op, args, &mut th.code))?;
        Err(GameError::Unsupported("script commands outside the game"))
    }
}

impl<M: Memory> ScriptCommands for OriginalGame<M> {
    fn run_command(&mut self, _op: Opcode, _args: &[ScriptArg<'_>], _locals: &mut ScriptLocals) -> GameResult<bool> {
        Err(GameError::Unsupported("ScriptCommands::run_command"))
    }

    fn create_script_thread(&mut self) -> GameResult<ScriptThreadRef> {
        Ok(new_thread())
    }

    /// Frees a thread made by `create_script_thread`; the handle must not be used afterwards.
    fn destroy_script_thread(&mut self, t: ScriptThreadRef) {
        if let Some(slot) = (t.0 as usize).checked_sub(1).and_then(|i| table().get_mut(i).map(Option::take)) {
            drop(slot);
        }
    }

    #[cfg(windows)]
    fn run_on_thread(&mut self, thread: ScriptThreadRef, op: Opcode, args: &[ScriptArg<'_>]) -> GameResult<bool> {
        with_thread(thread, |t| run::run(t, op.0, args))?
    }

    #[cfg(not(windows))]
    fn run_on_thread(&mut self, thread: ScriptThreadRef, op: Opcode, args: &[ScriptArg<'_>]) -> GameResult<bool> {
        // Host builds have no game to run it in; the instruction is still encoded into the thread.
        with_thread(thread, |t| crate::script::encode(op.0, args, &mut t.code))?;
        Err(GameError::Unsupported("ScriptCommands::run_on_thread outside the game"))
    }

    fn thread_local(&self, thread: ScriptThreadRef, i: usize) -> GameResult<u32> {
        if i >= LOCALS {
            return Err(GameError::InvalidHandle);
        }
        with_thread(thread, |t| t.local(i))
    }

    fn set_thread_local(&mut self, thread: ScriptThreadRef, i: usize, value: u32) -> GameResult<()> {
        if i >= LOCALS {
            return Err(GameError::InvalidHandle);
        }
        with_thread(thread, |t| t.set_local(i, value))
    }

    #[cfg(windows)]
    fn last_command(&self) -> (Opcode, bool) {
        run::last()
    }

    #[cfg(not(windows))]
    fn last_command(&self) -> (Opcode, bool) {
        (Opcode(0), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::fake::FakeMemory;

    #[test]
    fn threads_keep_their_locals_until_released() {
        let mut game = OriginalGame::new(FakeMemory::new(0x0040_0000, 0x0100_0000));
        let a = game.create_script_thread().unwrap();
        let b = game.create_script_thread().unwrap();
        assert_ne!(a, b);
        game.set_thread_local(a, 0, 41).unwrap();
        game.set_thread_local(b, 31, 7).unwrap();
        assert_eq!(game.thread_local(a, 0), Ok(41));
        assert_eq!(game.thread_local(b, 31), Ok(7));
        assert_eq!(game.thread_local(a, 32), Err(GameError::InvalidHandle));
        assert_eq!(game.thread_local(ScriptThreadRef(0), 0), Err(GameError::InvalidHandle));
        assert_eq!(with_thread(a, |t| t.thread[0x08..0x0F] == *b"samp-rs"), Ok(true));
        assert!(game.run_on_thread(a, Opcode(0x01B4), &[ScriptArg::Int(0), ScriptArg::Int(1)]).is_err());
        assert_eq!(with_thread(a, |t| t.code.clone()), Ok(vec![0xB4, 0x01, 0x01, 0, 0, 0, 0, 0x01, 1, 0, 0, 0]));
        game.destroy_script_thread(a);
        assert_eq!(game.thread_local(a, 0), Err(GameError::InvalidHandle));
        let c = game.create_script_thread().unwrap();
        assert_eq!(game.thread_local(c, 0), Ok(0));
        game.destroy_script_thread(b);
        game.destroy_script_thread(c);
    }
}
