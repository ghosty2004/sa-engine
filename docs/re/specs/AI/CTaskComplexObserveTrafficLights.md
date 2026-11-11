# CTaskComplexObserveTrafficLights (AI) - RE-SPEC-3 group I2
Vtable 0x86DF98, type 0xE0 (224), size 0xC (no own members).

## 0x6317B0 ...::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0xE0.

## 0x6317C0 ...::~CTaskComplexObserveTrafficLights  (catalogue: specified/REA, port level 0)
- complete dtor: `vtbl = 0x86DF98; jmp 0x61A3D0` (~CTaskComplex).

## 0x6317D0 ...::MakeAbortable  (catalogue: specified/REA, port level 0)
- `mov al,1; ret 0xC` -> always true.

## 0x6317E0 ...::CreateNextSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH.
- If `m_pSubTask == 0` or its type == 0xCB (STAND_STILL): `p = new(0x20)`; return p ? `CTaskSimpleRunAnim(p, ped->+0x4D4 (anim group),
  4, 4.0f, false)` 0x61A8B0 : 0.
- Else if type == 0x190 (RUN_ANIM): `t = CGeneral::GetRandomNumberInRange(3000, 5000)` 0x407180 - drawn BEFORE the allocation;
  `p = new(0x20)`; return p ? `CTaskSimpleStandStill(p, t, false, false, 8.0f)` : 0.
- Else return 0.
- gta-reversed: cpp:17 uses its own GetRandomNumberInRange (gr-deviations row rng-helper: CONFIRMED, formula differs from 0x407180;
  port uses 0x407180).

## 0x6318D0 ...::CreateFirstSubTask  (catalogue: specified/REA, port level 0)
- tail-jump through `vtbl+0x20` (CreateNextSubTask) with the same arg.

## 0x6318E0 ...::ControlSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`: if `CTrafficLights::LightForPeds()` 0x49D400 returns 0 and
  `m_pSubTask->MakeAbortable(ped, 0, nullptr)` succeeds -> return 0 (task ends); otherwise return m_pSubTask.

## 0x637F60 ...::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- call 0x6317C0; `if (flags & 1) CTask::operator delete`; return this; `ret 4`.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x631790 CTaskComplexObserveTrafficLights::CTaskComplexObserveTrafficLights  (catalogue: specified/REA, port level 1)
- `__thiscall T* (this)`, plain `ret`. No callers.
- Behaviour: CTaskComplex ctor 0x61A3B0; `vtbl = 0x86DF98` (TASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS); `+0xC = 0; +0x10 = 0; +0x14 = 0 (u8); +0x15 = 0 (u8)` (a CTaskTimer {start, interval, started, stopped}); return this.

## 0x636410 CTaskComplexObserveTrafficLights::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame. Slot 1 of vtable 0x86DF98.
- Behaviour: `p = CTask::operator new(0x18)`; if NULL return 0 (0x63646E). Else CTaskComplex ctor on p; `p->vtbl = 0x86DF98; p+0xC = 0; p+0x10 = 0; p+0x14 = 0 (u8); p+0x15 = 0 (u8)`; return p (fresh timer; nothing copied).
