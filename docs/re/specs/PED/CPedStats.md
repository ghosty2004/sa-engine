# CPedStats (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x608940 CPedStats::FindIndexWithPedStat  (catalogue: specified/REA, gr hook `.Reversed = false`, port level 0)
- Convention: `__cdecl int32(CPedStat* stat [esp+4])`, plain `ret`, return EAX. Callers: none found statically.
- Globals: `CPedStats::ms_apPedStats` 0xC0BBEC (CPedStat* array base, stride 0x34).
- Behaviour: `p = ms_apPedStats; for (i = 0; i < 43 /*0x2B*/; i++, p += 0x34) if (stat == p) return i; return 0;`
  (0x608940-0x60895D). Not-found returns 0 (same as index 0).
- gta-reversed: `PedStats.cpp:129` is a `plugin::Call` stub with a wrong signature (`void(void*)`, no return); port from the bytes.
