# CPedIntelligence (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x6074A0 CPedIntelligence::operator new  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl void*(uint32 size [esp+4])`; the size argument is ignored. Tail-jump (0x6074A6) to
  `CPedIntelligencePool::New` 0x605EC0 (thiscall) with ECX = `CPools::ms_pPedIntelligencePool` (0xB744C0, read at 0x6074A0);
  the pool function's EAX is the result. The stack argument stays in place for the callee (which, as a plain `ret` thiscall
  without args, does not look at it).
- Callers: CPed::Constructor 0x5E8030.
