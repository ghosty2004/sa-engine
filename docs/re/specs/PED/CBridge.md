# CBridge (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x41BCB0 CBridge::ThisIsABridgeObjectMovingUp  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl bool(...)`; no stack argument is read (callers may push a model index; the callee
  cleans nothing, plain `ret`). Return AL.
- Callers: CPlayerInfo::Process 0x56F8D0 (1 site). Callees: none. Globals: none.
- Behaviour: `xor al, al; ret` (0x41BCB0-0x41BCB2): always returns false. Only AL is defined (EAX upper
  bytes are whatever the caller had); a port returns `false`.
- gta-reversed: not present as a function (a III/VC leftover; SA stubbed it out).
