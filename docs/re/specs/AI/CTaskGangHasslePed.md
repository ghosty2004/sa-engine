# CTaskGangHasslePed (AI) - RE-SPEC-3 group I2
Vtable (slot 4 = 0x65FF50), type 0x4BC (1212), size 0x2C. Layout (gr): +0xC m_Ped CPed*, +0x10 m_mode i32 (gr dword10; 0 = talk,
1 = abuse, 2 = abuse then attack), +0x14 m_RndMin, +0x18 m_RndMax, +0x1C m_bAnimBlockRefAdded u8 ("gangs" 0x86F92C), +0x20 CTaskTimer
{+0x20 start, +0x24 interval, +0x28 started, +0x29 stopped}. Ctor 0x65FED0 (ped, mode, min, max).

## 0x65FF50 CTaskGangHasslePed::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0x4BC.

## 0x65FFE0 CTaskGangHasslePed::ControlSubTask  (catalogue: specified/REA, port level 7)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH.
- If m_Ped == 0 return 0.
- If the timer is started: (stopped -> restart at now) if `now >= start + interval` (unsigned): mode 2: if the subtask type is not 0x3E8
  (KILL_PED_ON_FOOT): `p = new(0x38)`; return p ? `CTaskComplexKillPedOnFoot(p, m_Ped, -1, 0, 0, 0, 1)` 0x620E30 : 0 (if the subtask already
  is 0x3E8 continue below); mode 1: continue below; any other mode: return 0.
- "gangs" anim block: if !+0x1C: if `ShouldLoadGangAnims()` 0x65E7F0: loaded (`ms_aAnimBlocks[i].+0x10 == 1`) -> AddAnimBlockRef(i),
  `+0x1C = 1`; else RequestModel(i + 0x63E7, 8). Else if !ShouldLoadGangAnims: RemoveAnimBlockRef(i), `+0x1C = 0`.
- If +0x1C and `!ped->IsPlayingHandSignal()` and `!ped TaskMgr.GetTaskSecondary(4)`: `r = GetRandomNumberInRange(0, 200)`; if 50 < r < 56:
  `p = new(0x20)`; if p: `CTaskSimpleRunAnim(p, 0x34, 0x117 + _ftol(((float)(_rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) * 8.0f
  [0x859000]), 4.0f, false)`; `SetTaskSecondary(p, 4)`. Else if r == 100: `p = new(0x18)`; `p ? CTaskComplexPlayHandSignalAnim(p, -1, 4.0f)`;
  `SetTaskSecondary(p, 4)`.
- Speech (every call): mode 0 -> `ped->Say(0x56, 0, 1.0f, 0, 0, 0)`; mode 1 or 2 -> `CTaskComplexGangLeader::DoGangAbuseSpeech(ped, m_Ped)`
  0x65E860 (cdecl). Return m_pSubTask.

## 0x6620D0 CTaskGangHasslePed::Clone  (catalogue: specified/REA, port level 3)
- `p = new(0x2C)`; if p `CTaskGangHasslePed(p, m_Ped, +0x10, +0x14, +0x18)`; return p / 0.

## 0x6642A0 CTaskGangHasslePed::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- call 0x65FF60; `if (flags & 1) CTask::operator delete`; return this.

## 0x6642C0 CTaskGangHasslePed::CreateNextSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`. `s = ((float)_rand() * 3.0518509e-05f [0x858C7C]); s = s + s; s = s + 3.0f [0x858B3C]`
  (full rand() value, not masked; stored f32; drawn BEFORE the type test). If `m_pSubTask->GetTaskType() == 0x3E8` return 0. Else `p =
  new(0x4C)`; return p ? `CTaskComplexSeekEntityStandard(p, m_Ped, 999999, 1000, s * 1.0f [0x86FC2C], 2.0f [0x86FC28], 2.0f [0x86FC30],
  false, true)` 0x46AC10 : 0.

## 0x664380 CTaskGangHasslePed::CreateFirstSubTask  (catalogue: specified/REA, port level 3)
- If m_Ped == 0 return 0. `s` as in CreateNextSubTask (first _rand()); `f = (float)(_rand() & 0xFFFF) * 3.0517578e-05f`; `interval = m_RndMin
  + _ftol((float)(m_RndMax - m_RndMin) * f)`; timer: start = now, interval, started = 1. Then the same SeekEntityStandard construction.
