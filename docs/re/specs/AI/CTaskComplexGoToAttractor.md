# CTaskComplexGoToAttractor (AI) - RE-SPEC-3 group I3

Vtable 0x86FF3C (gr TaskComplexGoToAttractor.h, size 0x2C): +0xC m_Attractor (CPedAttractor*), +0x10 m_vecAttrPosn (CVector), +0x1C m_fAttrHeading,
+0x20 m_fAttrTime, +0x24 m_nQueueNumber, +0x28 m_MoveState. CPed: +0x14 m_matrix, +4 placement pos, +0x474 flags (bit 0x200000 bUseAttractorInstantly),
+0x558 m_fHeadingCurrent, +0x55C m_fHeadingGoal (plugin-sdk names).

## 0x66B6C0 CTaskComplexGoToAttractor::CreateNextSubTask  (catalogue: specified/REA, port level 14)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, returns 0 always (0x66B6FC). Slot 8.
- Behaviour: pushes (m_Attractor, ped) then `GetPedAttractorManager()` 0x5EE190 (no args; returns the manager) and
  `CPedAttractorManager::BroadcastArrival(mgr, ped, m_Attractor)` 0x5EFE20 (thiscall, uses the two values pushed before the getter call,
  0x66B6C8-0x66B6D4). If `ped->+0x474 & 0x200000` and `CPedAttractor::GetHeadOfQueue(m_Attractor)` 0x5EB590 != ped: clear that bit (0x66B6F1).
- gta-reversed: TaskComplexGoToAttractor.cpp:33 (hook `.Reversed = false`) - same logic as the bytes.

## 0x66B710 CTaskComplexGoToAttractor::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed*)`: `return m_pSubTask (+8)`, `ret 4`. Slot 10. gr matches.

## 0x670400 CTaskComplexGoToAttractor::`scalar deleting destructor'  (catalogue name CTaskComplexGoToAttractor::~CTaskComplexGoToAttractor)  (catalogue: specified/REA, port level 3)
- NAME CORRECTION: slot 0 of vtable 0x86FF3C. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x66B6A0 (0x670403): `vtbl = 0x86FF3C; jmp 0x61A3D0` (~CTaskComplex). Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x670408, call at 0x670410; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

## 0x670420 CTaskComplexGoToAttractor::CreateFirstSubTask  (catalogue: specified/REA, port level 9)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH (state var at [esp+0x18]). Slot 9.
- Callees: m_Attractor->GetType() (vtbl slot 0); CPedPlacement::FindZCoorForPed 0x616920 (cdecl, CVector* in/out); CTask::operator new 0x61A5A0;
  CTaskSimpleStandStill ctor 0x62F310; CTaskComplexSequence ctor 0x632BD0 / AddTask 0x632D10; CTaskComplexGoToPointAndStandStill ctor 0x668120;
  CTaskSimpleSlideToCoord ctor (no anim) 0x66C3E0.
- Behaviour:
  1. `ms = m_MoveState (+0x28)`; if `m_Attractor->GetType() == 4` (shelter) `ms = 6` (0x67043B-0x67044A).
  2. If `ped->+0x474 & 0x200000` (use attractor instantly) (0x670453):
     `FindZCoorForPed(&m_vecAttrPosn)` (modifies this->+0x10..+0x18 in place, 0x670463); then copy the position to the ped: if `ped->m_matrix`
     != 0 write matrix pos (+0x30/+0x34/+0x38) else placement (+4/+8/+0xC) (x87 load z, y, x then store x, y, z: plain f32 copies, 0x670468-0x670491);
     `ped->m_fHeadingGoal (+0x55C) = m_fAttrHeading`, `ped->m_fHeadingCurrent (+0x558) = m_fAttrHeading` (0x670494-0x6704A2);
     return `new CTaskSimpleStandStill(0, 0, 0, 8.0f)` (0x20 bytes; args pushed 8.0f, 0, 0, 0 -> (time 0, looped 0, idleStance 0, blend 8.0f);
     null if the pool is full) (0x6704A0-0x6704D9).
  3. Else (0x6704DE-0x670595): `seq = new CTaskComplexSequence` (0x40; null if full); `g = new CTaskComplexGoToPointAndStandStill(ms,
     &m_vecAttrPosn, 0.5f [0x86FC84], 2.0f [0x86FC88], 0, 0)` (0x28; null if full); `seq->AddTask(g)` (called even when seq is null: ECX = 0,
     crash - original bug kept); `s = new CTaskSimpleSlideToCoord(&m_vecAttrPosn, m_fAttrHeading (raw dword), 0.5f)` (0x80); `seq->AddTask(s)`;
     return seq.
- gta-reversed: `plugin::CallMethodAndReturn<0x670420>` - not reversed; spec from bytes.
