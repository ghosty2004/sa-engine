# CTaskSequences (SCRIPT) — RE-SPEC-REST

## 0x632E00 CTaskSequences::GetAvailableSlot(bool forMissions?)  (catalogue: specified/REA, port level 0)
- `__cdecl`, u8 [esp+4], plain `ret`; returns EAX = slot index or -1. Caller: ProcessCommands1500To1599 0x490DB0.
- Range: arg != 0 → slots 32..63, arg == 0 → slots 0..31. Returns the first i in range with `ms_bIsOpened[i]` (u8 table 0xC17898) == 0
  AND `*(u32*)(0xC17900 + 0x40*i) == 0` (first dword of sequence i — its first task pointer, so the sequence is empty); else -1.
