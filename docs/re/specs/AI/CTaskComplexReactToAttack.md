# CTaskComplexReactToAttack (AI) - RE-SPEC-FINAL (helper B)

## 0x620B00 CTaskComplexReactToAttack::CTaskComplexReactToAttack  (catalogue: unknown/REA, port level 2)
- `__thiscall CTaskComplexReactToAttack* (this, int32 a1 [esp+4], CEntity* attacker [esp+8], int32 a3 [esp+0xC], int32 a4 [esp+0x10], int32 a5 [esp+0x14])`, `ret 0x14`; SEH frame (.HOODLUM stub); returns this. Caller: Clone 0x623300.
- Behaviour: CTaskComplex ctor 0x61A3B0; `+0x18 = a3; +0x10 = a1; +0x1C = a4; vtbl = 0x86D868` (TASK_COMPLEX_REACT_TO_ATTACK); `+0xC = 0 (u8); +0xD = 0 (u8); +0x14 = attacker; +0x20 = a5`; `if attacker` -> RegisterReference(attacker, &this->+0x14) 0x571B70 (0x620B5F). No gr class.

## 0x620B90 CTaskComplexReactToAttack::~CTaskComplexReactToAttack  (catalogue: unknown/REA, port level 2)
- Complete dtor, `__thiscall void (this)`, plain `ret`; SEH frame. Caller: scalar deleting dtor 0x625C00.
- Behaviour: `vtbl = 0x86D868`; `if +0x14 (attacker)` -> CleanUpOldReference(+0x14 entity, &this->+0x14) 0x571A00 (0x620BC6); CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x620C30 sub_620c30 (proposed: CTaskComplexReactToAttack::CreateSubTask)  (catalogue: unknown/REA, port level 3)
- Name proposal: CreateSubTask of CTaskComplexReactToAttack (callers vt_86d868::CreateNextSubTask 0x625C20 and CreateFirstSubTask 0x625C70; fields of the 0x86D868 layout).
- `__thiscall CTask* (this, int32 taskType [esp+4])`, `ret 4`; SEH frame via .HOODLUM stub (entry `nop; jmp 0x4020FF`). Fields: +0x14 attacker (CEntity*), +0x18 float (hit round), +0x1C hit side, +0x20 hit piece.
- Behaviour:
  - `taskType == 0xD0` (TASK_COMPLEX_FALL_AND_GET_UP): `p = CTask::operator new(0x18)`; NULL -> 0; return `CTaskComplexFallAndGetUp::CTaskComplexFallAndGetUp(p, (+0x1C == 2) ? 0x1A : 0x14 (anim id), 0 (group), 1000 (0x3E8 ms))` 0x6786C0.
  - `taskType == 0xC8` (TASK_NONE): `p = CTask::operator new(8)`; NULL -> 0; CTaskSimple ctor 0x61A390; `p->vtbl = 0x859CFC` (CTaskSimpleNone); return p.
  - `taskType == 0x3F0` (TASK_SIMPLE_BE_HIT): `p = CTask::operator new(0x28)`; NULL -> 0; `round = _ftol2(+0x18)` 0x821B40 (truncation); `a = (+0x14 != NULL && (+0x14->m_nType (+0x36) & 7) == 3 (ENTITY_TYPE_PED)) ? +0x14 : NULL`; return `CTaskSimpleBeHit::CTaskSimpleBeHit(p, a, +0x20 (hit piece), +0x1C (hit side), round)` 0x620780.
  - Any other value -> 0.
