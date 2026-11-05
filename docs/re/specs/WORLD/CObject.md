# CObject (WORLD)

CObject: vtable 0x866f60, size 0x19c, pool `CPools::ms_pObjectPool` 0xb7449c (pool header layout as in
CDummy.md). CEntity fields used: `+0x14 m_matrix` (CMatrixLink*), `+0x18 m_pRwObject`, `+0x1c m_nFlags`
(u32; bit 9 = 0x200 `m_bRenderDamaged`, bit 28 = 0x10000000 `m_bLightObject`), `+0x22 m_nModelIndex`
(i16), `+0x2e m_IplIndex` (u8), `+0x2f m_AreaCode` (i8). `+0x4` = `m_placement` (CSimpleTransform).

## 0x59f050 CObject::sub_59f050 (proposed: `IsCraneMovingPartModel(int32 modelId)`)  (catalogue: unknown/REA, port level 0)
- Evidence: only caller `CObject::ProcessControl` 0x5a2130 passes `(i32)(i16) this->m_nModelIndex`; the
  body is gta-reversed's `CObject::IsCraneMovingPart()` (Object.cpp:1294) with the model index as a
  plain argument.
- Convention / signature: `__cdecl bool (int32 modelId [esp+4])`, return AL, plain `ret` (caller pops 4).
- Globals (u16, zero-extended with `movzx` before a 32-bit compare): `MI_CRANE_MAGNET` 0x8cd700,
  `MI_CRANE_HARNESS` 0x8cd6fc, `MI_MINI_MAGNET` 0x8cd740, `MI_WRECKING_BALL` 0x8cd71c.
- Behaviour: return true if `modelId` equals any of them, tested in the order MAGNET, HARNESS,
  MINI_MAGNET, WRECKING_BALL; else false.
- Edge cases: the MI globals are zero-extended (0xffff = unset stays 65535) while the argument is
  sign-extended by the caller, so an unset MI (0xffff) never matches model -1. Keep the zero extension.
- Open questions: none.

## 0x5a1df0 CObject::CObject(CDummyObject*)  (catalogue: specified/REA, port level 14)
- Convention / signature: `__thiscall CObject* (this = ECX, CDummyObject* d [esp+4])`, returns this,
  `ret 4`. SEH frame (unwind → `CPhysical::Destructor` 0x542450), irrelevant to a port.
- Callers: 1 (`CObject::Create(CDummyObject*)` 0x5a2070, 2 sites). Callees: `CPhysical::Constructor`
  0x542260, `CEntity::SetModelIndexNoCreate` 0x533700, `CEntity::AttachToRwObject` 0x533ed0,
  `CPlaceable::AllocateMatrix` 0x54f560 (`__thiscall ()`), `CSimpleTransform::UpdateMatrix` 0x54f1b0
  (`__thiscall (this = &d->m_placement, CMatrix* out)`), `CPlaceable::SetMatrix` 0x54f610
  (`__thiscall (const CMatrix*)`), `CEntity::DetachFromRwObject` 0x533fb0, `CObject::Init` 0x59f840,
  `RwHelper::GetFirstAtomic` 0x734820 (`__cdecl RpAtomic* (RpClump*)`),
  `CCustomBuildingRenderer::IsCBPCPipelineAttached` 0x5d7f40 (`__cdecl int (RpAtomic*)`).
- Behaviour:
  1. `CPhysical::Constructor(this)`; `vtable = 0x866f60`.
  2. `this->SetModelIndexNoCreate((i32)(i16) d->m_nModelIndex)`.
  3. If `d->m_pRwObject != NULL`: `this->AttachToRwObject(d->m_pRwObject, true)`.
     Else: `if (d->m_matrix == NULL) { d->AllocateMatrix(); d->m_placement.UpdateMatrix(d->m_matrix); }`
     then `this->SetMatrix(d->m_matrix)` (d->m_matrix re-read after the allocation).
  4. `d->DetachFromRwObject()`; `this->Init()`.
  5. `this->m_IplIndex = d->m_IplIndex`; `this->m_AreaCode = d->m_AreaCode`.
  6. `this->m_nFlags ^= (d->m_nFlags ^ this->m_nFlags) & 0x200` (copy `m_bRenderDamaged` from d; the
     rest of this->m_nFlags as read once before the merge).
  7. `rw = this->m_pRwObject` (read before step 6's store). If `rw != NULL`: `a = (byte rw[0] == 1
     (rpATOMIC)) ? rw : GetFirstAtomic(rw)`; `if (IsCBPCPipelineAttached(a) == 0) this->m_nFlags |=
     0x10000000`. `IsCBPCPipelineAttached` is called even when `GetFirstAtomic` returned NULL.
- gta-reversed: provided (Object.cpp:95). Differences: gr skips the pipeline check when the atomic is
  NULL (`if (auto* atomic = ...)`), the original passes NULL to `IsCBPCPipelineAttached` (whatever that
  function does with NULL — see its own spec). gr's `dummyObj->GetMatrix()` hides the allocate +
  `UpdateMatrix` sequence; same effect. Port the bytes.
- Open questions: none.

## 0x5a2bd0 CObject::vfunc_0 (scalar deleting destructor)  (catalogue: specified/REA, port level 15)
- Convention / signature: `__thiscall CObject* (this = ECX, u32 flags [esp+4])`, returns this, `ret 4`.
  Callers: virtual (typed call from `WaterCreature_c::Exit` 0x6e3e60).
- Behaviour: `CObject::~CObject(this)` (0x59f660); if `flags & 1`: inlined pool delete on
  `ms_pObjectPool` (read after the dtor): `idx = (int32)(this - pool->m_pObjects) / 0x19c` (signed;
  magic 0x13e22cbd, `sar 5`, add sign bit), `byteMap[idx] |= 0x80`, `if (idx < firstFree)
  firstFree = idx`. Return this.
- Open questions: none.
