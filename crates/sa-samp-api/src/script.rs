//! Raw SCM command execution and samp's custom opcodes.
//!
//! samp drives most player, camera and world changes through single script commands
//! (samp-rs `script.rs` `Runner`: a private zeroed `CRunningScript` pointed at one encoded instruction and
//! run with `CRunningScript::ProcessOneCommand` 0x469EB0). The typed methods of the other traits name each
//! opcode they stand for. This trait keeps the raw path as an escape hatch, so that:
//! * samp-rs can migrate one call at a time (raw first, typed later);
//! * opcodes run by samp's own `main.scm` / `script.img` (from samp.saa) keep working, because those run
//!   inside the engine's script interpreter anyway;
//! * the native backend can serve a typed method by calling its ported opcode handler.

use crate::GameResult;

/// An SCM opcode (without the "not" bit 0x8000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Opcode(pub u16);

/// One parameter, in the public SCM encoding (samp-rs `script::Arg`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScriptArg<'a> {
    /// 32-bit integer immediate (type 0x01).
    Int(i32),
    /// Float immediate (type 0x06).
    Float(f32),
    /// Variable-length string (type 0x0E), raw bytes (network strings are not always UTF-8).
    Text(&'a [u8]),
    /// Global variable by byte offset into ScriptSpace (type 0x02).
    Global(u16),
    /// Local variable of the private thread (type 0x03), used for outputs.
    Local(u16),
}

/// Local variables of the private script thread after a command (outputs land here).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptLocals(pub [u32; 32]);

impl ScriptLocals {
    /// Local `i` as an integer.
    pub fn int(&self, i: usize) -> i32 {
        self.0.get(i).copied().unwrap_or(0) as i32
    }

    /// Local `i` as a float.
    pub fn float(&self, i: usize) -> f32 {
        f32::from_bits(self.0.get(i).copied().unwrap_or(0))
    }
}

/// Executes script commands.
pub trait ScriptCommands {
    /// Runs one command on a private thread; returns the condition result (meaningful for conditional
    /// opcodes). `locals` is both input and output. A fault inside the command returns
    /// [`crate::GameError::Fault`] (the original backend's SEH guard; samp's crash report shows the opcode).
    fn run_command(
        &mut self,
        op: Opcode,
        args: &[ScriptArg<'_>],
        locals: &mut ScriptLocals,
    ) -> GameResult<bool>;

    /// Last opcode run and whether a command is in progress (samp's crash report fields "SCM Op" / "lDbg").
    fn last_command(&self) -> (Opcode, bool);
}

/// The parameter interface a custom opcode handler gets (`CRunningScript::CollectParameters` 0x464080,
/// `ScriptParams` 0xA43C78, `UpdateCompareFlag` 0x4859D0).
pub trait CustomOpcodeParams {
    /// Collects `n` parameters of the running command and returns them raw.
    fn collect(&mut self, n: u8) -> GameResult<[u32; 8]>;
    /// Stores results into the command's output variables.
    fn store(&mut self, values: &[u32]) -> GameResult<()>;
    /// Sets the condition result.
    fn set_condition(&mut self, value: bool);
}

/// samp's custom opcodes, installed in the `ProcessCommands2500To2599` jump table (0x47BF54, 0x47BF88).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CustomOpcode {
    /// 0x09C6 (samp's network-event poll used by its main.scm).
    Op09C6,
    /// 0x09D3 (samp's in-vehicle test used by its main.scm).
    Op09D3,
}
