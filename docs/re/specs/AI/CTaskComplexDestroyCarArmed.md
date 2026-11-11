# CTaskComplexDestroyCarArmed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x628C60 CTaskComplexDestroyCarArmed::~CTaskComplexDestroyCarArmed  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x622010 CTaskComplexDestroyCarArmed::Destructor (0x628C63); if `flags & 1` (0x628C68) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x628C70); returns this (0x628C78).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x628DA0 CTaskComplexDestroyCarArmed::CreateSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame. Callers: ControlSubTask 0x628FA0, CreateNextSubTask 0x62DF20 (5), CreateFirstSubTask 0x62E0A0 (4). Fields (gr TaskComplexDestroyCarArmed.h): +0x10 m_VehToDestroy, +0x20 m_VehiclePos.
- Behaviour (each `new` returning NULL yields 0):
  - `0x387` (TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL): `new(0x28) CTaskComplexGoToPointAndStandStill(6, &m_VehiclePos, [0x86FC84]=0.5f, [0x86FC88]=2.0f, false, false)` 0x668120.
  - `0xCA` (TASK_SIMPLE_PAUSE): `new(0x18) CTaskSimplePause(100)` 0x48E750.
  - `0x38B` (TASK_COMPLEX_SEEK_ENTITY): `new(0x4C) CTaskComplexSeekEntity<Standard>(m_VehToDestroy, 50000 (0xC350), 1000, [0x85BACC]=1.0f, [0x859E30]=2.0f, [0x859E34]=2.0f, 1, 1)` 0x46AC10.
  - `0x3FC` (TASK_SIMPLE_GUN_CTRL): `w = &ped->m_aWeapons[(int8)ped->m_nActiveWeaponSlot (+0x718)]` (+0x5A0 + slot*0x1C); `wi = CWeaponInfo::GetWeaponInfo(w->m_eWeaponType, CPed::GetWeaponSkill(ped) 0x5E6580)` 0x743C60; if `(wi->m_nFlags (+0x18) >> 8) & 1` (throw flag) -> `new(0x1C) CTaskSimpleThrowControl(m_VehToDestroy, NULL)` 0x61F8B0; else `new(0x3C) CTaskSimpleGunControl(m_VehToDestroy, NULL, NULL, 3, 5, 5000)` 0x61F3F0.
  - Any other value -> 0.
- gta-reversed (deviation row TaskComplexDestroyCarArmed.cpp:62, "locked original redirect"): gr calls the original via plugin::CallMethodAndReturn; its commented-out reconstruction (lines 69-101) matches the bytes case by case (same constants and argument order). Verdict: REFUTED (no behavioural difference; the marker is an implementation note). Class: harmless.
