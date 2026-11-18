//! Script (SCM) commands.

/// An SCM opcode (without the "not" bit 0x8000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Opcode(pub u16);

/// One parameter, in the public SCM encoding.
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

/// A private script thread (its own `CRunningScript` and locals).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScriptThreadRef(pub u32);

/// The local variables of a private script thread (command outputs land here).
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

