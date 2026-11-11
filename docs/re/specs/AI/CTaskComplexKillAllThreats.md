# CTaskComplexKillAllThreats (AI) - RE-SPEC-FINAL (helper B)

## 0x622180 CTaskComplexKillAllThreats::CTaskComplexKillAllThreats  (catalogue: unknown/REA, port level 1)
- `__thiscall CTaskComplexKillAllThreats* (this, int32 a1 [esp+4], int32 a2 [esp+8], int32 a3 [esp+0xC])`, `ret 0xC`. No callers (inlined elsewhere; Clone 0x623750 inlines the same).
- Behaviour: CTaskComplex ctor 0x61A3B0; `+0xC = a1; vtbl = 0x86DA1C` (slot 4 returns TASK_KILL_ALL_THREATS); `+0x10 = a2; +0x14 = a3`; return this.

## 0x6221C0 CTaskComplexKillAllThreats::~CTaskComplexKillAllThreats  (catalogue: unknown/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86DA1C` (0x6221C0); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x6221C6). No other member cleanup.

## 0x6221D0 CTaskComplexKillAllThreats::CreateSubTask (proposed: CTaskComplexKillAllThreats::FindThreat helper)  (catalogue: unknown/REA, port level 12)
- `__thiscall CPed* (this unused, CPed* ped [esp+4])`, `ret 4`. Callers: vt_86da1c::CreateNextSubTask 0x6293C0, CreateFirstSubTask 0x629480.
- Behaviour: builds a 0x14-byte local `L`: `L+0 = 0 (result); L+4 = 0; L+8 = 0; L+0xC = 0 (u8); L+0xD = 0 (u8); L+0x11 = 0 (u8); L+0x12 = 0 (u8); L+0x10 = 1 (u8)` (0x6221D3-0x6221E9, 0x622215); calls `0x607A90` (thiscall, ECX = &L+4) with args `(ped, 4, &ped->m_pIntelligence (+0x47C)->+0x130 (ped-scanner entity list), 0x10 (16 entries), &L+0 (out), &ped (address of the argument slot))` (0x62221A); returns `L+0` (the CPed* written by 0x607A90, NULL if none).
- Open question: semantics of 0x607A90 (unnamed, not in this assignment) and of the flag bytes; the result is the threat ped used to create the kill subtask (0x62221F).
