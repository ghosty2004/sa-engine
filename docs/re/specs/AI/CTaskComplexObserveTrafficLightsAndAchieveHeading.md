# CTaskComplexObserveTrafficLightsAndAchieveHeading (AI) - RE-SPEC-3 group I2
Vtable 0x86DFC4, type 0xE1 (225). Layout: +0xC m_durationMs (i32), +0x10 m_fTargetHeading (f32) (gr names; ctor arg order).

## 0x631910 ...::CTaskComplexObserveTrafficLightsAndAchieveHeading  (catalogue: specified/REA, port level 1)
- `__thiscall this (int32 a [esp+4], f32 heading [esp+8])`, `ret 8`: `CTaskComplex()`; `+0xC = a`; vtbl = 0x86DFC4; `+0x10 = heading`.

## 0x631940 ...::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0xE1.

## 0x631980 ...::~CTaskComplexObserveTrafficLightsAndAchieveHeading  (catalogue: specified/REA, port level 0)
- `vtbl = 0x86DFC4; jmp 0x61A3D0`.

## 0x631990 ...::CreateSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, int32 type)`, `ret 4`, SEH; entry obfuscated (`nop; jmp 0x401A79`).
- 0xE0: `p = new(0x18)`; if p: inlined CTaskComplexObserveTrafficLights ctor (`CTaskComplex()`, vtbl 0x86DF98, `+0xC = 0`, `+0x10 = 0`,
  `+0x14 = 0`, `+0x15 = 0`) - note the allocation is 0x18 bytes and the extra fields are written although the class vtable is the
  0xC-byte ObserveTrafficLights; return p or 0.
- 0x386 ACHIEVE_HEADING: `new(0x18) CTaskSimpleAchieveHeading(m_fTargetHeading, 0.5f [0x86FC7C], 0.2f [0x86FC80])` 0x667E20.
- 0x516 / other: 0.

## 0x637F80 ...::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- call 0x631980; `if (flags & 1) CTask::operator delete`; return this; `ret 4`.
