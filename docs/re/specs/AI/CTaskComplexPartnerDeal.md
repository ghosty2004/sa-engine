# CTaskComplexPartnerDeal (AI) - RE-SPEC-3 group I2
Vtable 0x870754, type 0x4B2 (1202), size 0x74.

## 0x681DF0 CTaskComplexPartnerDeal::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0x4B2.

## 0x6823B0 CTaskComplexPartnerDeal::CreateFirstSubTask  (catalogue: specified/REA, port level 1)
- `jmp 0x681F20` (CTaskComplexPartner::CreateFirstSubTask).

## 0x6823C0 CTaskComplexPartnerDeal::StreamRequiredAnims  (catalogue: specified/REA, port level 2)
- If `!+0x6E`: if `CTaskComplexGangLeader::ShouldLoadGangAnims()` 0x65E7F0: `i = GetAnimationBlockIndex(m_animBlockName)`; loaded ->
  `AddAnimBlockRef(i)`, `+0x6E = 1`; else `RequestModel(i + 0x63E7, 8)`. If `+0x6E` and `!ShouldLoadGangAnims()`:
  `RemoveAnimBlockRef(GetAnimationBlockIndex(name))`, `+0x6E = 0`. (No empty-name test, unlike the base.)

## 0x684D10 CTaskComplexPartnerDeal::Clone  (catalogue: specified/REA, port level 4)
- `p = new(0x74)`; if p: `0x684190(p, &m_commandName (+0x14), m_partner, (u8)m_leadSpeaker, m_distanceMultiplier, m_point (by value))`
  (CTaskComplexPartnerDeal ctor); return p or 0.

## 0x684F80 CTaskComplexPartnerDeal::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- call 0x684200 (complete dtor, catalogue unknown - proposed ~CTaskComplexPartnerDeal); `if (flags & 1) CTask::operator delete`; return this.
