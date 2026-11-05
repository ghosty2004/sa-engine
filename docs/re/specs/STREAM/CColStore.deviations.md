# CColStore: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/Collision/ColStore.cpp` (pin 20d8d2a). ColDef stride 0x2C, pool
at CColStore::ms_pColPool (0x965560).

## 0x4107A0 CColStore::AddRef  (gr_deviations = 1)
Row ColStore.cpp:112 (bug-note "we don't check whether GetAt returned nullptr").
- Original bytes (.HOODLUM 0x1569960): if the slot is free (`flags[i] < 0`) EAX = 0 and it still executes
  `inc word [eax+0x26]` (0x1569975), i.e. writes to address 0x26 (access violation); otherwise
  `inc word [objects + i*0x2C + 0x26]`.
- Verdict: REFUTED (gr keeps the missing null check, as the original).
- Class: keep-original (no null check; a free slot is a crash in the original).

## 0x4107D0 CColStore::RemoveRef  (gr_deviations = 1)
Row ColStore.cpp:388 (same bug-note).
- Original bytes (.HOODLUM 0x156DBE0): same shape with `dec word [eax+0x26]` (0x156DBF5 for the free-slot
  path, 0x156DC01 otherwise).
- Verdict: REFUTED.
- Class: keep-original.

## 0x4114D0 CColStore::Shutdown  (gr_deviations = 1)
Row ColStore.cpp:35 (pool-iteration `GetAllValidWithIndex`).
- Original bytes: fixed loop of 255 slots (`edi += 0x2C` while `edi < 0x2BD4`, 0x411501-0x41150B),
  ascending from slot 0, calling RemoveColSlot(i) (0x4114F4) for used non-null slots; the pool pointer is
  re-read after each call. The bound is the constant 255, not `pool->size` (equal for the created pool).
  Then the pool destructor is inlined (frees objects and flags when `ownsArrays` byte +0x10 is set, zeroes
  the four fields, frees the pool), `ms_pColPool = 0`, quad tree destroyed + deleted, `ms_pQuadTree = 0`.
- Verdict: REFUTED (ascending order, same slots).
- Class: harmless.
