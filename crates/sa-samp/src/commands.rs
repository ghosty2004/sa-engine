//! Raw script (SCM) commands: the native backend's `sa_samp_api::script` traits, on `sa-access`'s private script
//! threads.

use sa_samp_api::script::*;
use sa_samp_api::*;

use crate::{NativeGame, View};

impl<V: View> ScriptCommands for NativeGame<V> {
    fn run_command(&mut self, op: Opcode, args: &[ScriptArg<'_>], locals: &mut ScriptLocals) -> GameResult<bool> {
        self.game.run_command(op, args, locals)
    }

    fn create_script_thread(&mut self) -> GameResult<ScriptThreadRef> {
        Ok(self.game.create_script_thread())
    }

    fn destroy_script_thread(&mut self, thread: ScriptThreadRef) {
        self.game.destroy_script_thread(thread);
    }

    fn run_on_thread(&mut self, thread: ScriptThreadRef, op: Opcode, args: &[ScriptArg<'_>]) -> GameResult<bool> {
        self.game.run_on_thread(thread, op, args)
    }

    fn thread_local(&self, thread: ScriptThreadRef, i: usize) -> GameResult<u32> {
        self.game.thread_local(thread, i)
    }

    fn set_thread_local(&mut self, thread: ScriptThreadRef, i: usize, value: u32) -> GameResult<()> {
        self.game.set_thread_local(thread, i, value)
    }

    fn last_command(&self) -> (Opcode, bool) {
        self.game.last_command()
    }
}
