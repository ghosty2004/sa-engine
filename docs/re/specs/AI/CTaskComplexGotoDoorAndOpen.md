# CTaskComplexGotoDoorAndOpen (AI) - RE-SPEC-AI-2 group J2

Vtable 0x86FFC0 (11 slots), task type 0x3A4. Layout (gr TaskComplexGotoDoorAndOpen.h, size 0x38): +0xC m_Object CObject* (registered
ref), +0x10 m_Start CVector, +0x1C m_End CVector, +0x28..+0x31 a CTaskTimer-shaped block (+0x28 start time u32, +0x2C interval u32,
+0x30 started u8 (gr `byte30`), +0x31 refresh/stopped u8 (gr `m_bRefreshTime`)), +0x34 m_nFlags u8 (bit0 = built from a door object,
bit1 = final go-to issued, bit2 = door physical flag was cleared, bit3 = player pad bit set).
"PadBit" below = `CPad::GetPad(0)` 0x53FB70 then `pad->+0x10E |= 8` (byte OR); gr's dtor calls this field `bPlayerOnInteriorTransition`.

## 0x66BB90 CTaskComplexGotoDoorAndOpen::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3A4` (0x66BB90); `ret` -> returns 932 (TASK_COMPLEX_GOTO_DOOR_AND_OPEN). Vtable: slot 4 (+0x10) of vtable 0x86FFC0 (CTaskComplexGotoDoorAndOpen).

## 0x66BC80 CTaskComplexGotoDoorAndOpen::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`.
- If priority != 2 (0x66BC80 `cmp [esp+8],2; jne`) returns AL = 0 (0x66BC97). Else rewrites the priority slot to 2 (0x66BC8C) and tail-jumps
  to `m_pSubTask->MakeAbortable` (vtbl +0x18, 0x66BC94) with the same args; no null check of m_pSubTask. gr cpp:57 matches.

## 0x66BCA0 CTaskComplexGotoDoorAndOpen::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* ()`, plain `ret`, SEH frame (unwind frees with 0x61A5B0).
- If `m_nFlags & 1` (0x66BCB9): `p = CTask::operator new(0x38)` 0x61A5A0; if p: `CTaskComplexGotoDoorAndOpen(m_Object)` 0x66BB20 (0x66BCDF).
  Else `p = new(0x38)`; if p: `CTaskComplexGotoDoorAndOpen(&m_Start (+0x10), &m_End (+0x1C))` 0x66BBA0 (0x66BD16). Returns the new task, or 0
  if operator new returned 0 (0x66BD2F). Timer and the other flags are not copied. gr cpp:48 matches.

## 0x66BD40 CTaskComplexGotoDoorAndOpen::CreateFirstSubTask  (catalogue: specified/REA, port level 9)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame (stack CEventAreaCodes object, unwound with 0x4B2210).
- Callees: CPed::IsPlayer 0x5DF8F0, CPad::GetPad 0x53FB70, CEventAreaCodes ctor 0x4B2190 / dtor 0x4B2210, GetEventGlobalGroup 0x4ABA50,
  CEventGroup::Add 0x4AB420, RpAnimBlendClumpGetAssociation(clump, id) 0x4D68B0, CAnimManager::BlendAnimation 0x4D4610,
  CTask::operator new 0x61A5A0, CTaskSimpleGoToPoint ctor 0x667CD0 (`(moveState, const CVector* target, float radius, u8, u8)`).
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84 (read); .data floats 0.75 [0x8D2EE0], -0.5 [0x8D2EE4], 2.0 [0x8D2EE8]; 0.0 [0x858B50].
- Behaviour:
  1. If `ped->IsPlayer()` (0x66BD62): PadBit (0x66BD72) and `m_nFlags |= 8` (0x66BD7F).
  2. Construct `CEventAreaCodes ev(ped)` on the stack (0x66BD89); `GetEventGlobalGroup()->Add(&ev, false)` (0x66BD92-0x66BDA4).
  3. Timer: +0x28 = CTimer::m_snTimeInMilliseconds (0x66BDAF), +0x2C = 1000 (0x66BDB2), +0x30 = 1 (0x66BDB9).
  4. If m_Object == 0 (0x66BDC2 -> 0x66C041): if `m_nFlags & 1`: destroy ev, return 0 (0x66C05A). Else `m_nFlags |= 2` (0x66C06E/0x66C072),
     `new(0x30) CTaskSimpleGoToPoint(4, &m_End, 0.5f, 0, 0)` (0x66C08A-0x66C09B; radius pushed as immediate), destroy ev, return it (0 if alloc failed).
  5. Door path (m_Object != 0): P = object position (matrix +0x30 if `obj->+0x14` m_matrix != 0, else placement +0x4; 0x66BDC8-0x66BDD7).
     R = matrix right (+0x0), F = matrix forward (+0x10) read from `obj->m_matrix` WITHOUT a null check (0x66BDE7-0x66BE10; crash if the
     object has no matrix - original). Q = ped position (same matrix/placement rule, 0x66BE14-0x66BE28).
     - dot (x87, 0x66BE2B-0x66BE53): `dz = P.z-Q.z; dy = P.y-Q.y; dx = P.x-Q.x; s = F.y*dy + F.z*dz; s = s + F.x*dx`;
       `side = (s > 0.0 [0x858B50])` via `fcomp; test ah,0x41; jne` -> s <= 0 or NaN gives side = 0 (0x66BE55-0x66BE6A).
     - P' = P + R*0.75 [0x8D2EE0] per component (0x66BE6E-0x66BEA5, each product rounded then added: `P.x + R.x*0.75` etc.).
     - side = 1 (0x66BED3-0x66BF2D): `m_Start = P' + F*-0.5 [0x8D2EE4]`; `m_End = P' + F*2.0 [0x8D2EE8]` (z component kept on the x87
       stack, x/y via f32 temporaries; each `F.c*k` is rounded to f32 before the add except the z product which stays in st).
     - side = 0 (0x66BF2F-0x66BF85): `m_Start = P' - F*-0.5`; `m_End = P' - F*2.0` (fsubrp / fsub).
     - Stores m_Start (+0x10, 0x66BF89-0x66BF9E) then m_End (+0x1C, 0x66BFA9-0x66BFB9).
  6. If `obj->+0x40 & 8` (0x66BFBC): `m_nFlags |= 4`; `obj->+0x40 &= ~8`; then `m_Object->+0x40 &= ~4` (0x66BFC2-0x66BFCD; re-reads m_Object).
  7. If `RpAnimBlendClumpGetAssociation(ped->m_pRwClump (+0x18), 2)` != 0 (0x66BFD7): `BlendAnimation(clump, ped->+0x4D4 (anim group), 0,
     1000.0f)` (0x66BFEC-0x66BFF5; delta pushed as immediate).
  8. `new(0x30) CTaskSimpleGoToPoint(4, &m_Start, 0.35f, 0, 0)` (0x66BFFD-0x66C022, radius immediate 0x3EB33333); destroy ev (0x66C039);
     return the task (0 if alloc failed).
- gta-reversed: plugin::Call (cpp:67), not reversed. Open questions: none.

## 0x66C0D0 CTaskComplexGotoDoorAndOpen::CreateNextSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH frame.
- `t = m_pSubTask->GetTaskType()` (vtbl +0x10, 0x66C0F0). If t != 0x384 (TASK_SIMPLE_GO_TO_POINT) return 0 (0x66C0F8 -> 0x66C1D4, EDI = 0).
- If `m_nFlags & 2` (0x66C0FE): if ped->IsPlayer() 0x5DF8F0: PadBit (0x66C11A); return 0 (0x66C121).
- Else: `tm = &ped->m_pIntelligence (+0x47C)->m_TaskMgr (+4)`; if `tm->GetTaskSecondary(4)` 0x681810 == 0 (0x66C14C): `a = new(0x20)`
  `CTaskSimpleRunAnim(0 /*group*/, 0x99 /*anim*/, 8.0f, 0)` 0x61A8B0 (0x66C168-0x66C178; 0 if alloc fails) and `tm->SetTaskSecondary(a, 4)`
  0x681B60 (0x66C195, called even with a == 0). Then `new(0x30) CTaskSimpleGoToPoint(4, &m_End (+0x1C), 0.2f, 0, 0)` 0x667CD0 (0x66C1B4-0x66C1C5);
  `m_nFlags |= 2` (0x66C1D0, also when the allocation failed); return the task.
- gta-reversed: plugin::Call (cpp:62). Open questions: none.

## 0x66C1F0 CTaskComplexGotoDoorAndOpen::ControlSubTask  (catalogue: specified/REA, port level 1)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84.
- Timer test (CTaskTimer::IsOutOfTime inline): if +0x30 != 0 (0x66C1F6): if +0x31 != 0 { +0x28 = now; +0x31 = 0 } (0x66C202-0x66C20A);
  `expired = now >= +0x28 + +0x2C` (unsigned, `cmp; sbb; inc`, 0x66C214-0x66C220). If expired -> abort path.
- Otherwise (timer not started or not expired): if `m_Object == 0 && (m_nFlags & 1)` (0x66C226-0x66C231) -> abort path; else return m_pSubTask.
- Abort path (0x66C233): if `m_pSubTask->MakeAbortable(ped, 1, nullptr)` (vtbl +0x18) is true: if ped->IsPlayer() PadBit (0x66C25D); return 0.
  Else return m_pSubTask (0x66C26B).
- gta-reversed: plugin::Call (cpp:72). Open questions: none.

## 0x6711F0 CTaskComplexGotoDoorAndOpen::~CTaskComplexGotoDoorAndOpen  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexGotoDoorAndOpen::Destructor 0x66BC00 (0x6711F3); if `flags & 1` (0x6711F8) calls CTask::operator delete 0x61A5B0 (this) (0x671200, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FFC0 (CTaskComplexGotoDoorAndOpen).
