# CRunningScript::ProcessCommands0To99 (SCRIPT) - script switch spec

Shared conventions for all `ProcessCommandsXToY` switches (verified at the callers):
- Callers: `CRunningScript::ProcessOneCommand` 0x469EB0 and the inlined copy inside `CRunningScript::Process`
  0x469F00. Both read the 16-bit command word at `this->m_IP` (+0x14), advance m_IP by 2 (0x469EBF/0x469FBF),
  store bit 15 into `m_NotFlag` (+0xD2) (0x469ED6/0x469FD4), mask the id with 0x7FFF, and call
  `s_OriginalCommandHandlerTable[id / 100]` (table at 0x8A6168; 0x469EED/0x469FEB) as
  `__thiscall (CRunningScript* this = ECX, int command = [esp+4])`, `ret 4`. The pushed command is the
  masked id (0..0x7FFF), i.e. the full opcode number, not the offset within the range.
- Return value in AL: `Process` loops `call edx; test al,al; je 0x469FB0` (0x469FF5..0x469FF9): AL == 0 runs the
  next command, any non-zero AL (1 = WAIT, 0xFF = default/unknown) ends this script's processing for the frame.
- CRunningScript layout (gr RunningScript.h:232..251): +0x10 m_BaseIP, +0x14 m_IP, +0xC5 m_CondResult,
  +0xCC m_WakeTime, +0xD0 m_AndOrState (u16), +0xD2 m_NotFlag. `ScriptParams` = 0xA43C78 (tScriptParam[32], 4 bytes each).
- gta-reversed status `unimplemented` = `REGISTER_COMMAND_UNIMPLEMENTED` -> `notsa::script::detail::NotImplemented`
  (Parser.hpp:68): logs, debug-breaks, returns OR_INTERRUPT (-1) WITHOUT reading any parameter. This equals the
  original only for opcodes that reach the switch `default`; for an opcode with a real case it is a deviation.

## 0x465E60 CRunningScript::ProcessCommands0To99  (catalogue: specified/REA, port level 38)
- Dispatcher: `__thiscall` (this = ECX CRunningScript*, kept in ESI from 0x465E81; command id at [esp+4]), `ret 4`.
  SEH frame: `push -1; push 0x837D3B; push fs:[0]` (0x465E66..0x465E6D), `sub esp,0x14` (0x465E79), then
  `push esi; push edi`. Range check `cmp eax,0x63; ja 0x466C3A` (0x465E7C/0x465E83), direct jump table
  `jmp [eax*4 + 0x466C50]` (0x465E89; index = command, base 0). The function extent also includes the EH
  continuation stub 0x4013D4..0x4013E0 (`mov fs:[0],ecx; jmp 0x4662F6`), part of the 0x004D CREATE_PLAYER-area
  task code (a handler case, not re-specified).
- Default label 0x466C3A: `mov ecx,[esp+0x1C]; pop edi; or al,0xFF; pop esi; mov fs:[0],ecx; add esp,0x20; ret 4`
  -> returns -1 (0xFF), IP unchanged past the opcode word (no parameters consumed).
- Common epilogue (e.g. 0x466C25 = the COMMAND_NOP 0x0000 case): `pop edi; xor al,al; pop esi;
  mov ecx,[esp+0x14]; mov fs:[0],ecx; add esp,0x20; ret 4` -> AL 0. Cases returning 1 use `mov al,1` in the
  same shape (e.g. 0x465EAD, COMMAND_WAIT).
- Opcodes (100 in range): 85 with a case; gta-reversed handler: 84 (not re-specified); specified below: 1 (0052).
  Default-reaching (15): 003D, 003E, 003F, 0040, 0041, 0047, 0048, 0049, 004A, 004B, 004C (gr unimplemented),
  0054, 0056, 0057 (gr unsupported -> NotImplemented), 0055 (gr unimplemented). All of them return 0xFF in the
  original, and gr's NotImplemented also returns OR_INTERRUPT without reading parameters -> no behavioural
  difference. No gr `handler` opcode of this range reaches the default.

### 0x4668BE 0052 COMMAND_LINE  (gr: unimplemented, used: 0/12/0)
- Parameters: `CollectParameters(6)` 0x464080 (`push 6; mov ecx,esi; call` at 0x4668BE..0x4668C2); the six
  values in ScriptParams[0..5] are never read.
- Behaviour: returns AL = 0 through the inline epilogue (0x4668C7 `pop edi`, 0x4668C8 `xor al,al`, ..., 0x4668D9
  `ret 4`). A debug-line command compiled out in the release exe: it only skips its 6 arguments.
- gta-reversed: registered as unimplemented (BasicCommands.cpp:426) -> `NotImplemented` returns OR_INTERRUPT
  without consuming the 6 parameters. DEVIATION (keep original): gr would leave m_IP on the first argument and the
  next frame would execute argument bytes as an opcode; the port must collect 6 params and return 0.
  Used by 12 mission scripts, so this matters in real play.
