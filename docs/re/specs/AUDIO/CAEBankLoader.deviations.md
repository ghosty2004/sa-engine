# CAEBankLoader (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Loaders/AEBankLoader.h/.cpp` (pin 20d8d2a). Layout: +0 CAEBankSlot* m_BankSlots
(stride 0x12D4: +0 OffsetBytes, +4 NumBytes), +4 AEBankLookup* m_BankLkups (stride 0xC), +8 m_PakLkups, +0xC i16
m_BankSlotCnt, +0xE i16 m_BankLkupCnt, +0x14 u8 m_IsInitialised, +0x18 u32 m_BufferSize, +0x1C u8* m_Buffer, +0x20 i32*
m_StreamHandles. `_free` = 0x8214BD, operator new = 0x82119A.

## 0x4DFB20 CAEBankLoader::Deconstructor  (gr_deviations = 1)
Original: `__thiscall void (this = ECX)`, plain `ret` (the non-virtual destructor body; callers 0x4D8670, Terminate 0x4D97A0).
Row AEBankLoader.h:109 (NOTSA wrapper `Deconstructor() { this->~CAEBankLoader(); }`).
- Original bytes: if `+0x14 != 0`: `_free(+0)` (0x4DFB2D), `CMemoryMgr::Free(+0x1C)` 0x72F430 (0x4DFB36), `_free(+4)`,
  `_free(+0x20)`; caller-cleaned (`add esp,0x10`). Pointers are not nulled; +8 (PakLookups) is not freed.
- Verdict: REFUTED (same frees; gr frees m_Buffer first, the original frees m_BankSlots first - order only). Class: harmless.

## 0x4E01B0 CAEBankLoader::GetBankLookup  (gr_deviations = 1)
Original: `__thiscall AEBankLookup* (this, u16 bankId [esp+4])`, `ret 4`.
Row AEBankLoader.cpp:127 ("Vanilla game returned a nullptr on invalid index/uninited state").
- Original bytes: if `+0x14 == 0` return 0 (0x4E01B5); if `!((u32)bankId < (i32)(i16)+0xE)` return 0 (zero-extended id vs
  sign-extended count, signed `jl`, 0x4E01C0); else return `+4 + bankId * 12` (0x4E01CC).
- Verdict: CONFIRMED (gr asserts/indexes instead of returning null). Class: keep-original (return null).

## 0x4E0590 CAEBankLoader::LoadBankSlotFile  (gr_deviations = 2)
Original: `__thiscall bool (this = ECX)`, plain `ret`.
- Original bytes: `f = CFileMgr::OpenFile("AUDIO\\CONFIG\\BANKSLOT.DAT" (0x85F15C), "rb" (0x859C6C))`; if `f <= 0` (signed
  `jg`, 0x4E05AA) return 0. `size = GetTotalSize(f)`; if `size <= 2` (unsigned, 0x4E05C1) close, return 0. `Read(f, +0xC, 2)`;
  `+0 = operator new((i16)+0xC * 0x12D4)`; if null: close, return 0. `n = Read(f, +0, size - 2)`; if `n != size - 2`: close,
  `_free(+0)`, `+0 = 0`, return 0. Else close (0x4E062A), `CalculateBankSlotsInfosOffsets()` 0x4DFBA0, `last = +0 +
  ((i16)+0xC - 1) * 0x12D4`, `+0x18 = last.+4 + last.+0` (0x4E0644-0x4E0658), `+0x1C = CMemoryMgr::Malloc(+0x18, 0)` 0x72F420,
  return 1 (regardless of the Malloc result).
Row AEBankLoader.cpp:58 (`m_Buffer = nullptr; // NOTSA: for return check`, return `m_Buffer != nullptr`):
- Verdict: CONFIRMED - the original never clears m_Buffer and returns 1 after the Malloc even if it returned null; failure
  paths return 0 without touching m_Buffer. Class: keep-original.
Row AEBankLoader.cpp:66 (`assert(m_BankSlotCnt > 0)`): no check in the original (count 0 reads slot -1 at 0x4E0644).
Verdict: REFUTED (debug-only). Class: harmless. Other differences: file closed before CalculateBankSlotsInfosOffsets
(gr after) - harmless; `f <= 0` vs gr `!f` - harmless for valid handles.
