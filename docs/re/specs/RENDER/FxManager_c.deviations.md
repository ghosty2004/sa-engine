# FxManager_c (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Fx/FxManager.cpp` (pin 20d8d2a). FxManager_c +0x28 m_nFxTxdIndex, +0xAC m_Pool (FxMemoryPool_c).

## 0x5C2420 FxManager_c::LoadFxProject  (gr_deviations = 2)
Original: `__thiscall bool (const char* path [esp+4])` (ECX = this), `ret 4`; result in AL.
Sequence: copy path into a 256-byte local, overwrite from `strlen - 4` with "PC.txd\0" (byte stores 0x5C245F-0x5C247D); `m_nFxTxdIndex =
CTxdStore::AddTxdSlot("fx")` 0x731C80; `LoadTxd(idx, txdPath)` 0x7320B0; `AddRef(idx)` 0x731A00; `PushCurrentTxd()` 0x7316A0;
`SetCurrentTxd(idx)` 0x7319C0; `file = CFileMgr::OpenFile(path, "r")` (thunk 0x4AB240 -> 0x538900); `if ((int)file <= 0)` (SIGNED `jg`,
0x5C24BD) return 0 - without popping the txd. Then: ReadLine (thunk 0x4AB250 -> 0x5389B0, 256) + `sscanf(line, "%s", buf)`;
ReadLine; ReadLine + sscanf; while `buf` equals the 16 bytes "FX_SYSTEM_DATA:\0" (0x86B2EC, `repe cmpsb`, 0x5C2551):
`LoadFxSystemBP(path, file)` 0x5C1F50, ReadLine, ReadLine + sscanf. Then fclose (0x4AB260), `PopCurrentTxd()`, `m_Pool.Optimise()` 0x4A9CD0,
return 1.
- Row FxManager.cpp:131 (`if (!fs::exists(txdPath)) NOTSA_LOG_ERR`). Verdict: CONFIRMED (gr-only filesystem check + log; no control-flow
  change). Class: harmless.
- Row FxManager.cpp:148 (`ReadField<void>(file, "FX_PROJECT_DATA:"); // NOTSA`). Original bytes: the first line is also read and sscanf'd
  (0x5C24DC/0x5C24F6) before the loop; gr's ReadField<void> ignores the field name (FxTools.h:67), and its per-iteration
  `ReadField<void>(file)` matches the original's discarded ReadLine. Same line consumption. Verdict: REFUTED. Class: harmless.
Found in passing: the original treats a FILE* with the top bit set as failure (signed compare at 0x5C24BD); gr tests `!file`. Not reachable
for normal user-space pointers without /LARGEADDRESSAWARE; keep the original compare.
