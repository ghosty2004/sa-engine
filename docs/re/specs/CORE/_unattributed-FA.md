# _unattributed-FA (CORE) - RE-SPEC-FINAL group A

## 0x49C9A0 Clear  (catalogue: specified/REA, port level 0)
Proposed full name: `CPool<CStuntJump>::Clear()` (inlined pool clear; caller CStuntJumpManager::ShutdownForRestart 0x49CB10; `this` = CPool
{+0 objects, +4 flags u8[], +8 size i32, +0xC firstFree}). 0x49C9A0 jumps to the HOODLUM body 0x1562930.
- Convention: `__thiscall void (CPool* this)`, plain `ret`.
- Behaviour: `for (i = 0; i < this->size; i++)` (size re-read each iteration; `size <= 0` -> nothing): `flags[i] |= 0x80` (free bit), then
  `flags[i] &= 0x80` (clear the 7-bit id) - net `flags[i] = 0x80`. firstFree is NOT reset.
- gta-reversed: CPool::Clear (core/Pool.h) - same effect. Open questions: none.

## 0x5617A0 OS_TimeMS  (catalogue: unknown/REA, port level 0)
Name correction: catalogue OS_TimeMS -> `GetOSWPerformanceTime` (gr Timer.cpp: CTimer::Initialise 0x5617E0 stores this function pointer into
the timer-function global [0xB7CB28] when QueryPerformanceFrequency succeeds (0x561882); the millisecond fallback is GetMillisecondTime 0x5617C0).
- Convention: `__cdecl uint64 ()`, result EDX:EAX, plain `ret`. 8-byte local LARGE_INTEGER.
- Behaviour: `QueryPerformanceCounter(&li)` via the IAT slot [0x8580C8]; return li (low in EAX, high in EDX). The BOOL result is ignored.
- gta-reversed: GetOSWPerformanceTime (Timer.cpp) - same. Open questions: none.
