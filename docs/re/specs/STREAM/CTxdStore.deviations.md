# CTxdStore: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/TxdStore.cpp` (pin 20d8d2a). TxdDef stride 0xC, hash at +8.

## 0x731850 CTxdStore::FindTxdSlot(const char*)  (gr_deviations = 1)
Row TxdStore.cpp:177 (pool-iteration `GetAllValidWithIndex`).
- The flagged line belongs to the **hash overload** `FindTxdSlot(uint32)` (0x7318E0), not to the name
  overload at 0x731850; CAT-14 attributed it to the wrong overload. The gr name overload (TxdStore.cpp:148)
  does not use GetAllValid.
- Original bytes 0x731850: `key = CKeyGen::GetUppercaseKey(name)`; first loop from
  `i = ms_lastSlotFound (0xC88014)` **down** to 0 (0x731880-0x731897, skipped when the start is negative),
  then from `ms_lastSlotFound + 1` **up** to `pool->size - 1` (0x7318B0-0x7318C7). Match =
  used slot, non-null, `[def+8] == key`. On a match `ms_lastSlotFound = i` and return i; else return -1
  without updating ms_lastSlotFound. gr's name overload does the same.
- Hash overload 0x7318E0 (checked in passing): ascending `0 .. size-1`, first match returned, else -1,
  no cache update. Matches gr's `GetAllValidWithIndex` loop.
- Verdict: REFUTED.
- Class: harmless.
