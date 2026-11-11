# CGroupEventHandler (AI) - RE-SPEC-REST group B

## 0x5F9530 CGroupEventHandler::ComputeResponsLeaderQuitEnteringCar  (catalogue: specified/REA, port level 3)
- `__cdecl CTaskAllocator* (const CEvent* e [esp+4] unused, CPedGroup* g [esp+8], CPed* originator [esp+0xC] unused)`, plain `ret`,
  SEH frame. Always returns 0. `I = g + 0x30` (CPedGroupIntelligence), followers `m[k] = g->+0xC + 4k`, k = 0..6 (`delay base = 500*k`,
  EBX steps by 0x1F4 up to 0xDAC).
- For each non-null m[k]:
  - If `m->+0x46C & 0x100` (in vehicle) and `v = m->+0x58C` != 0: local `CTaskComplexLeaveCar t(v, 0, 500*k, true, false)` 0x63B8C0.
  - Else: `v = FindTaskByType(m->m_pIntelligence, 0x2CE /*ENTER_CAR_AS_PASSENGER_WAIT*/)` 0x600EE0 `->+0xC`; if that task is absent or
    v == 0: `v = FindTaskByType(..., 0x2BC /*ENTER_CAR_AS_PASSENGER*/)->+0xC`; if absent or 0 -> next k. `d = _ftol((f32)(rand() &
    0xFFFF) * [0x858B14]=3.0517578e-05f * [0x85AB78]=-500.0f)` (0x821B40, truncation; d in (-500, 0]); local `CTaskComplexLeaveCar t(v, 0,
    500*k - d + 250, false, false)`.
  - In both cases a local CTaskSimple with vtable 0x859CFC (CTaskSimpleLeaveGroup: CTaskSimple ctor 0x61A390, vtable store, later vtable
    store + CTaskSimple dtor 0x61A3A0) is constructed and destroyed around the call without being used (no observable effect).
  - `I->SetTask(m, &t, I + 0xC (ped/task pair array), -1, false)` 0x5F7540; `t.~CTaskComplexLeaveCar()` 0x63B970.
- gta-reversed: plugin::CallAndReturn (GroupEventHandler.cpp:468). Open questions: none.
