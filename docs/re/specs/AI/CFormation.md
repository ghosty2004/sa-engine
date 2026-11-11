# CFormation (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x69B1B0 CFormation::FindNearestAvailableDestination  (catalogue: specified/REA, port level 0)
- `__cdecl int32 (CVector pos [esp+4..0xC] by value, float* inOutDist [esp+0x10])`, `ret`; returns EAX = index or -1.
- best = 10000000.0f [0x86D270] (x87), idx = -1; for i = 0 .. m_Destinations.m_Count [0xC1A318] - 1 (signed `jle` skip when count <= 0): if claimed byte [0xC1A43C + i]
  == 0: `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `dc = dest[i].c - pos.c` (dest table 0xC1A31C stride 12; x87 order x, y, z subtractions, then z*z, y*y, x*x;
  0x69B1DA-0x69B1FE); d is stored as f32 [esp+8]; if `d < best` (strict, `test ah,5; jp`) -> best = (f32)d reloaded, idx = i (0x69B206-0x69B217).
- End: `*inOutDist = best + *inOutDist` (`fadd [eax]; fstp [eax]`, 0x69B227) - it ADDS the minimum (or 1e7 when nothing is free) to the caller's value.
