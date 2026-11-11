# CObjectPotentialCollisionScanner (AI) - RE-SPEC-FINAL group A

## 0x606890 CObjectPotentialCollisionScanner::ScanForObjectPotentialCollisionEvents  (catalogue: unknown/REA, port level 9)
- Convention: `__thiscall void (CTaskTimer* this, CPed* ped [esp+4])`, `ret 4`. `this` = CEventScanner+0x10 (call 0x607E93). SEH frame
  (handler 0x83EA03; state byte/dword [E-4], E = esp at entry). Locals: outLen/centreZ f32 [E-0xA0], moveState [E-0x9C], diff/offset CVector
  [E-0x98], objLow f32 [E-0x8C], pedHigh f32 [E-0x88], event [E-0x84], start CVector [E-0x68], CObjectScanner [E-0x5C] (0x50 bytes;
  closest object at [E-0x10] = scanner+0x4C).
- Callees: CTaskManager::GetSimplestActiveTask 0x6819D0; CTask::IsGoToTask 0x61A360; CPed::IsPlayer 0x5DF8F0; CObjectScanner ctor 0x606010;
  CObjectScanner::ScanForObjectsInRange 0x5FFF30; CVector::Difference 0x40FE00 (out = a - b); 0x406DA0 SquaredMagnitude (ST0);
  CEntity::GetBoundCentre(CVector& out) 0x534250 (returns &out); CPedGeometryAnalyser::GetIsLineOfSightClear(const CPed&, const CVector&
  target, CEntity&, float& outLen) 0x5F5A30 (cdecl); VectorSub 0x40FE60 (cdecl out = a - b, computes z, y, x); CPedGeometryAnalyser::
  GetIsLineOfSightClear(const CVector& start, const CVector& target, CEntity&) 0x5F2F00 (cdecl); CEventPotentialWalkIntoObject ctor 0x4AE5D0
  (thiscall (CObject*, int32 moveState)) / dtor 0x4AE670; CEventGroup::Add 0x4AB420; CEntityScanner::Clear 0x5FF9D0.
- Globals: now [0xB7CB84]=0; 7.5f [0x86C8FC]=7.5f; 1.0f [0x858624]=1.0f; 0.5f [0x86C900]=0.5f; CModelInfo::ms_modelInfoPtrs 0xA9B0C8;
  vtable 0x86CD40 (CEntityScanner, written by the inlined dtor); immediate 0.75f (0x606AEF).
- Behaviour:
  1. Timer (0x6068A5-0x60690B): if `!bStarted` { start = now; bStarted = 1; interval = 500 (0x1F4) } (bStarted re-tested). If `bStopped`
     { start = now; bStopped = 0 }. One `now` read (0x6068E7): if `now < start + interval` (unsigned) return; else interval = 500, start = now,
     bStarted = 1.
  2. `st = GetSimplestActiveTask(intel+4)`; `moveState = (st && IsGoToTask(st)) ? st->+8 : 1`. Return if `ped->IsPlayer()`; return if
     `moveState == 1`. `g = GetSimplestActiveTask(intel+4)` again; return if 0 or `!IsGoToTask(g)`.
  3. Construct CObjectScanner on the stack (0x606010), SEH state 0, `ScanForObjectsInRange(ped)`; `obj = scanner.m_pClosestEntityInRange`.
     The rest is skipped (to step 9) when any test fails:
  4. `obj != 0`; `!(obj->m_nObjectFlags (+0x140) & 0x400)` (bIsBroken, `test ah,4`); `!(obj->m_nObjectFlags & 1)` (bIsPickup); `obj->m_nFlags (+0x1C) & 1`
     (bUsesCollision).
  5. `diff = pedPos - objPos` (each = matrix ? matrix->pos : placement.pos); `q = diff.SquaredMagnitude()`; `fld 7.5; fmul 7.5; fcompp`
     -> need `56.25 > q` (`test ah,0x41; jne skip`; NaN skips).
  6. `target = &g->+0xC`. `cz = obj->GetBoundCentre(tmp).z` (f32 local). x87 (0x606A3B-0x606A9F): `zLow = pedPos.z - 1.0f` (kept in st),
     `zHigh = pedPos.z + 1.0f` -> f32; `R = CModelInfo::ms_modelInfoPtrs[obj->m_nModelIndex (+0x22, i16)]->m_pColModel (+0x14)->+0x24`
     (bounding-sphere radius); `objLow = cz - R` -> f32; `objHigh = R + cz` (fld R; fadd cz); compare `zLow` with objHigh: skip if
     `zLow > objHigh` (`test ah,0x41; je skip`; equal or NaN continue). Then `fld zHigh; fcomp objLow; test ah,5; jnp skip`: skip if
     `zHigh < objLow` (equal/greater/NaN continue).
  7. `outLen = 0`; `if (GetIsLineOfSightClear(*ped, *target, *obj, outLen)) skip`; need `outLen > 0.5f` (`test ah,0x41; jne skip`).
  8. `offset = (0, 0, 0.75f)`; `start = VectorSub(pedPos, offset)` (pedPos re-derived); `if (GetIsLineOfSightClear(start, *target, *obj)) skip`.
     Raise: `CEventPotentialWalkIntoObject ev(obj, moveState)`, SEH state 1, `ped->m_pIntelligence->m_eventGroup.Add(&ev, false)`, state 0,
     destroy (0x4AE670).
  9. Inlined ~CObjectScanner (0x606B60): SEH state -1, scanner vtable = 0x86CD40, `CEntityScanner::Clear(&scanner)`.
- Notes: a fresh object scanner is built and filled on every timer tick (the ped intelligence's own object scanner is not used). Step 7 and 8
  raise only when BOTH line-of-sight tests are blocked by this object (ped-centre path blocked for more than 0.5 and the lowered path too).
- gta-reversed: absent (Events/ObjectPotentialCollisionScanner.h only). Flag names from gr Object.h bitfield order. Open questions: none.
