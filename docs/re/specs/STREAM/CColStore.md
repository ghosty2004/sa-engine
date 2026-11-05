# CColStore (STREAM) - full specs, writer A (RE-SPEC-1)

## 0x410390 CColStore::FindColSlot(const char*)  (catalogue: unknown/REA, port level 0; proposed name)
Name hint was `sub_410390_from_CStreaming__LoadCdDirectory_char`. Evidence: the only call site
(CStreaming::LoadCdDirectory(char*) at 0x5B62F5) runs on entries whose extension compares equal to "COL"
(0x869644), pushes the entry name, and if the result is -1 calls 0x411140 `CColStore::AddColSlot(name)`;
the slot is then offset by 25000 (0x61A8, COL resource base). gta-reversed Streaming.cpp:1162 has exactly
`colSlot = CColStore::FindColSlot(); if (colSlot == -1) colSlot = CColStore::AddColSlot(name)` and
ColStore.h:40 `static int32 FindColSlot() { return -1; }` (the original's name lookup is compiled out).
- Convention: `__cdecl int32 (const char* name)`; the argument is pushed by the caller but never read;
  return EAX; plain `ret`.
- Behaviour: `return -1;` (`or eax, 0xffffffff`). No other register/flag effects that callers use.
- gta-reversed: ColStore.h:40 `FindColSlot()` returns -1 - matches. Its `FixBugs` path calls the real
  `FindColSlot(name)` (ColStore.cpp:116), which the original does not do (keep-original).
- Open questions: none.
