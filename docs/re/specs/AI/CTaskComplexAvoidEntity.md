# CTaskComplexAvoidEntity (AI) - RE-SPEC-FINAL (helper B)

## 0x66AA20 sub_66aa20 (proposed: CTaskComplexAvoidEntity::CTaskComplexAvoidEntity)  (catalogue: unknown/REA, port level 2)
- Evidence: vtable 0x86FF00 (TASK_COMPLEX_AVOID_ENTITY); caller vt_86ff00::Clone 0x66D0C0.
- `__thiscall T* (this, int32 moveState [esp+4], CEntity* entity [esp+8], const CVector* pos [esp+0xC])`, `ret 0xC`; SEH frame.
- Behaviour: CTaskComplex ctor 0x61A3B0; `+0x10 = moveState; +0xC = entity; vtbl = 0x86FF00`; `+0x20..+0x28 = *pos` and `+0x2C..+0x34 = *pos` (copied twice, 3 dwords each); `+0x44 = 0; +0x48 = 0; +0x4C = 0 (u8); +0x4D = 0 (u8)` (timer); `+0x50 &= 0xFE`; `if entity` -> RegisterReference(entity, &+0xC) 0x571B70; return this.

## 0x66AAE0 sub_66aae0 (proposed: CTaskComplexAvoidEntity::~CTaskComplexAvoidEntity (non-deleting))  (catalogue: unknown/REA, port level 2)
- Evidence: stores vtable 0x86FF00 (TASK_COMPLEX_AVOID_ENTITY); caller 0x66F6A0 (slot 0, scalar deleting dtor).
- `__thiscall void (this)`, plain `ret`; SEH frame. `vtbl = 0x86FF00`; `if +0xC (entity)` -> CleanUpOldReference(+0xC entity, &this->+0xC) 0x571A00; CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x66AB40 CTaskComplexAvoidEntity::sub_66AB40 (proposed: CTaskComplexAvoidEntity::SetUpIK)  (catalogue: unknown/REA, port level 7)
- Name proposal: look-at helper of CTaskComplexAvoidEntity (caller vt_86ff00::ControlSubTask 0x66ADE0; sets the +0x50 bit 0 tested by QuitIK 0x66AD10).
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Fields: +4 parent task, +0x20 CVector (avoid target point), +0x50 flags.
- Behaviour (any failed test returns immediately):
  1. `pad = CPad::GetPad(0)`; if `ped == CWorld::FindPlayerPed(-1)` and `pad->+0x10E (DisablePlayerControls, u16) == 0` -> return (a controllable player does not auto-look).
  2. Require `CEntity::GetIsOnScreen(ped)` 0x534540; `!(+0x50 & 1)`; `IKChainManager_c::GetLookAtEntity(&g_ikChainMan, ped) == NULL` 0x6181D0; `CTaskManager::GetTaskSecondary(&ped->m_pIntelligence->m_TaskMgr, 5) == NULL` 0x681810; and if the parent task (+4) exists its type is neither 0x395 (TASK_COMPLEX_AVOID_OTHER_PED_WHILE_WANDERING) nor 0x3AB (TASK_COMPLEX_AVOID_ENTITY) (GetTaskType called twice).
  3. `VectorSub(&D, &this->+0x20, &P)` 0x40FE60 (P = ped position); `m = 0x406DA0(&D)` (magnitude); `fcomp [0x85EED4]=9.0f; test ah, 0x41; jne return`: continue only if `m > 9.0`.
  4. `CVector::Normalise(&D)` 0x59C910. `dot = (D.z*F.z + D.y*F.y) + D.x*F.x` with F = ped->m_matrix->forward (no NULL check); `fcomp [0xC18D48 CTaskSimpleGoTo::ms_fLookAtThresholdDotProduct]; test ah, 5; jp return`: continue only if `dot < threshold`.
  5. Look position (x87 order, 0x66AC73-0x66ACE7): `Lp.x = T.x + (D.x + D.x)`; `Lp.y = T.y + (D.y + D.y)`; `Lp.z = (P.z + (D.z + D.z)) + [0x86FD40]=0.61f` (2*D.z stored as f32 first) with T = this+0x20 and P.z = ped z. `IKChainManager_c::LookAt(&g_ikChainMan, "TaskAvoidEntity" [0x86FF2C], ped, NULL, 5000, -1, &Lp, false, 0.25f, 500, 3, false)` 0x618970.
  6. `+0x50 |= 1`.

## 0x66AD10 CTaskComplexAvoidEntity::sub_66AD10 (proposed: CTaskComplexAvoidEntity::QuitIK)  (catalogue: unknown/REA, port level 2)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Caller: vt_86ff00::ControlSubTask 0x66ADE0.
- Behaviour: `if this->+0x50 & 1` (look-at started): `if IKChainManager_c::IsLooking(&g_ikChainMan 0xC15448, ped)` 0x6181A0 -> `IKChainManager_c::AbortLookAt(&g_ikChainMan, ped, 0xFA (250 ms blend))` 0x618280. The flag is not cleared.

## 0x66FD30 CTaskComplexAvoidEntity::CreateFirstSubTask-helper (proposed: CTaskComplexAvoidEntity::ComputeAvoidRoute)  (catalogue: unknown/REA, port level 3)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`; 0x54 bytes of locals. Caller: vt_86ff00::CreateFirstSubTask 0x672530.
- Behaviour: local `E = { this->+0xC (entity), 15 zero dwords }` (`rep stosd`, 0x66FD49); `0x66F6C0(this, &S, &E)` (thiscall, 2 args: computes a bounding CColSphere S {centre x,y,z, radius} for the entity - see 0x66F6C0); `S.centre.z = ped position z` (matrix pos or placement); `CPedGeometryAnalyser::ComputeRouteRoundSphere(ped, &S, &this->+0x14 (from), &this->+0x20 (to), &this->+0x38 (out1), &this->+0x2C (out2))` 0x5F1890 (cdecl, 6 args); its AL result is returned unchanged in AL (only stack cleanup follows the call).
