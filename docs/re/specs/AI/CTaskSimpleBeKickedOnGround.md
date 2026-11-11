# CTaskSimpleBeKickedOnGround (AI) - RE-SPEC-FINAL (helper B)

## 0x61FC20 CTaskSimpleBeKickedOnGround::CTaskSimpleBeKickedOnGround  (catalogue: unknown/REA, port level 1)
- `__thiscall CTaskSimpleBeKickedOnGround* (this)`, plain `ret`. No callers (inlined in Clone 0x623120 etc.).
- Behaviour: CTaskSimple ctor 0x61A390; `+8 m_bIsFinished = 0 (u8); +0xC m_pAnim = 0; vtbl = 0x86D7D8` (TASK_SIMPLE_BE_KICKED_ON_GROUND); return this.

## 0x61FC50 CTaskSimpleBeKickedOnGround::~CTaskSimpleBeKickedOnGround  (catalogue: unknown/REA, port level 1)
- Complete dtor, `__thiscall void (this)`, plain `ret`; SEH frame (.HOODLUM stub). Caller: scalar deleting dtor 0x625B80.
- Behaviour: `vtbl = 0x86D7D8`; `if m_pAnim (+0xC)`: `m_pAnim->m_BlendDelta (+0x1C) = -4.0f` (immediate 0xC0800000, 0x61FC84) then `m_pAnim->SetFinishCallback(0x4D1390 CDefaultAnimCallback::DefaultAnimCB (empty), NULL)` 0x4CEBE0 (0x61FC93); then CTaskSimple::~CTaskSimple 0x61A3A0. m_pAnim is not nulled.

## 0x61FD10 CTaskSimpleBeKickedOnGround::FinishAnimCB  (catalogue: unknown/REA, port level 0)
- `static void __cdecl (CAnimBlendAssociation* assoc [esp+4], void* data [esp+8])`, plain `ret`. Installed by 0x61FD20.
- Behaviour: `task = data; task->m_pAnim (+0xC) = 0; task->m_bIsFinished (+8) = 1` (0x61FD14/0x61FD1B).

## 0x61FD20 sub_61fd20 (proposed: CTaskSimpleBeKickedOnGround::StartAnim)  (catalogue: unknown/REA, port level 9)
- Name proposal: StartAnim of CTaskSimpleBeKickedOnGround (vtable 0x86D7D8); caller vt_86d7d8::ProcessPed 0x61FE00; arms FinishAnimCB 0x61FD10.
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`; 0xC bytes of locals.
- Behaviour:
  1. `if !CLocalisation::KickingWhenDown()` 0x56D270 -> `this->+8 (finished) = 1`; return (0x61FD2F).
  2. `first = RpAnimBlendClumpGetFirstAssociation(ped->m_pRwClump (+0x18), 0x800)` 0x4D6A70 (cdecl); `animId = first ? 0x27 : 0x24` (`neg; sbb; and 3; add 0x24`, 0x61FD50-0x61FD5C); `a = CAnimManager::BlendAnimation(ped->m_pRwClump, 0, animId, 8.0f)` 0x4D4610 (8.0f pushed as 0x41000000); `this->+0xC = a`; `a->SetCurrentTime(0.0f)` 0x4CEA80 (dword 0 pushed); `a->m_Flags (+0x2E) |= 1`; `a->m_Flags &= ~8` (`and byte, 0xF7`).
  3. `if CLocalisation::KickingWhenDown()` (called again, 0x61FD85): local `v = (0,0,0)`; `h = GetAnimHierarchyFromSkinClump(ped->m_pRwClump)` 0x734A40; `idx = RpHAnimIDGetIndex(h, (ped->+0x490)->+0x14)` 0x7C51A0 (bone node id read through the ped's frame-data pointer at +0x490); `m = RpHAnimHierarchyGetMatrixArray(h) + idx*0x40` 0x7C5120; `RwV3dTransformPoints(&v, &v, 1, m)` 0x7EDD90. The transformed point is never used (dead computation, keep the calls only if they have side effects; they do not).
  4. `a->SetFinishCallback(0x61FD10 (CTaskSimpleBeKickedOnGround::FinishAnimCB), this)` 0x4CEBE0 (uses the re-read `this->+0xC`).
