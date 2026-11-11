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

<!-- RE-SPEC-FINAL helper B additions -->

## 0x682440 CTaskComplexPartnerDeal::GetPartnerSequence  (catalogue: specified/REA, port level 3)
- `__thiscall CTaskComplexSequence* (this)`, plain `ret`; SEH frame. Caller: CTaskComplexPartner::CreateNextSubTask 0x683AD0 (virtual). Fields: +0x38 partner (CPed*), +0x58 (u8) lead-speaker flag.
- Behaviour: `seq = new(0x40) CTaskComplexSequence()` 0x632BD0 (seq = NULL if the allocation fails; the AddTask calls below are then made on NULL - original bug). Two orders by `+0x58`:
  - `+0x58 != 0`: `seq->AddTask(new(0x28) CTaskComplexTurnToFaceEntityOrCoord(partner (+0x38), [0x86FC7C]=0.5f, 0.02f (0x3CA3D70A)))` 0x66B890 / 0x632D10; `seq->AddTask(new(0x20) CTaskSimpleRunAnim(0x34, 0x11F, 4.0f, false))` 0x61A8B0.
  - `+0x58 == 0`: same turn task, then `CTaskSimpleRunAnim(0x34, 0x120, 4.0f, false)`.
  Each `new` that fails passes NULL to AddTask. Return seq.

## 0x684200 CTaskComplexPartnerDeal::~CTaskComplexPartnerDeal  (catalogue: unknown/REA, port level 3)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x870754` (CTaskComplexPartnerDeal) (0x684200); tail-jump to base dtor CTaskComplexPartner::~CTaskComplexPartner 0x683A40 (0x684206). No other member cleanup.
