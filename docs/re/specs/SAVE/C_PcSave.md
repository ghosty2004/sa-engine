# C_PcSave (SAVE) - full specs

## 0x6190A0 C_PcSave::GenerateGameFilename  (catalogue: specified/REA, port level 0)
- `__thiscall void (int32 slot [esp+4], char* out [esp+8])` (ECX = C_PcSave*, unused), `ret 8`.
- Caller: CGenericGameStorage::CheckSlotDataValid 0x5D1380 (also the gr callers in GenericGameStorage.cpp:759/803 go to
  the gr version). Callee: CRT `sprintf` 0x821BB5 (cdecl, `add esp, 0x14`).
- Behaviour: `sprintf(out, "%s%i%s" (0x86BAD4), C_PcSave::DefaultPCSaveFileName (char[260] at 0xC16F18), slot + 1, ".b"
  (0x86BADC))` (0x6190A8-0x6190BA). No bounds, no slot check (slot + 1 computed with plain `inc`).
- gta-reversed: C_PcSave.cpp:30, hooked with `.Reversed = false`; it adds `assert(slot < MAX_SAVEGAME_SLOTS)` and uses
  bounded `sprintf_s` (row C_PcSave.cpp:31, harmless). Same output for valid input. Open questions: none.
