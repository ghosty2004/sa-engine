# AI: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x41b7c0 CPointRoute::operator new (proposed)  (catalogue: specified/REA, port level 1)
- `__cdecl void* (u32 size)`; 0x41b7c0 `nop; jmp 0x409aed`: `pushfd; ecx = [0x15608e4] + [0x156b98a]; ecx = [ecx]; popfd; jmp 0x41b7c6 -> jmp CPointRoutePool::New 0x41b5b0` (thiscall on the pool).
- Static .HOODLUM values: 0x7919b3e0 + 0x879d90d0 = 0xb744b0 (mod 2^32) = CPools::ms_pPointRoutePool, the pool the matching delete 0x41b7d0 uses.
- Port: `CPools::ms_pPointRoutePool->New()`.

## 0x41b7d0 CPointRoute::operator delete (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl void (void* p)`; body 0x1564fd0. Same as 0x4040a0 with pool [0xb744b0] and element size 0x64 (signed division via 0x51eb851f, sar 5): mark `m_byteMap[idx] |= 0x80`, `if (idx < m_nFirstFree) m_nFirstFree = idx`.

## 0x41b810 CPatrolRoute::operator new (proposed)  (catalogue: unknown/REA, port level 1)
- `__cdecl void* (u32 size)`: `ecx = [0xb744b4]`; `jmp 0x41b660` (pool New, thiscall). Size ignored.

## 0x41b820 CPatrolRoute::operator delete (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl void (void* p)`; body 0x156a220. As 0x4040a0 with pool [0xb744b4] and element size 0x1a4 (`imul 0x9c09c09d; add edx,ecx; sar edx,8`): `byteMap[idx] |= 0x80`; `if (idx < m_nFirstFree) m_nFirstFree = idx`.

## 0x420f30 CEventAcquaintancePed::~CEventAcquaintancePed (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void ()`: `[this] = 0x858e68`; tail `jmp 0x4af890`  (base destructor) (0x420f30).

## 0x43a6a0 type predicate (arg == 22 or arg == 5) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl bool (i32 x)`; body 0x15697d0: EAX = 1 if `x == 0x16 || x == 5`, else 0.
- Open question: which enum (callers in conversations / scripts) (0x15697d4).

## 0x453fa0 element ctor of global g_977be8 (trivial, returns this) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, EAX = this; nothing initialised. Element type unknown (0x34 bytes).

## 0x453fb0 element dtor of global g_977be8 type (empty) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret` (0x453fb0). Port: no-op.

## 0x4541e0 ctor of global g_977be8 type (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* ()`, EAX = this; body 0x1568bf0.
- `eh_vector_constructor_iterator(this+8, 0x34, 0x32, ctor 0x453fa0, dtor 0x453fb0)` 0x821d30; fields +0..+7 untouched.
- Open question: owner class of the global at 0x977be8 (subsystem guessed from address neighbourhood; region 0x45xxxx is scripts/AI).

## 0x4634c0 CDecisionMaker::~CDecisionMaker (empty) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret` (0x4634c0).

## 0x4650a0 CDecisionMaker::CDecisionMaker (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall CDecisionMaker* ()`, EAX = this; body 0x1564280.
- Loop 1: 0x29 times `CDecision::CDecision(this + i*0x3c)` 0x6040c0; loop 2: 0x29 times `CDecision::SetDefault` 0x600530 (same order, ascending).
- gta-reversed: `CDecisionMaker` ctor (DecisionMakers/DecisionMaker.h).

## 0x4650f0 CDecisionMakerTypes::CDecisionMakerTypes (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall CDecisionMakerTypes* ()`, EAX = this; body 0x1567930, SEH frame.
- `[this] = 0` (m_NoOfDecisionMakers); `eh_vector_constructor_iterator(this+4, 0x99c, 0x14, CDecisionMaker::CDecisionMaker 0x4650a0, dtor 0x4634c0)` 0x821d30.
- Then five inline CDecisionMaker ctors (41 x CDecision ctor + 41 x SetDefault each) at this+0xc1b4, +0xcb50, +0xd4ec, +0xde88, +0xe824. The 0x180 bytes at +0xc034..+0xc1b3 (m_EventIndices) are NOT initialised (gr `{}` zeroes them).
- Statics: dwords [0xc0afe0..0xc0aff0] = 0 (5); `rep stosd` 10 dwords of 0x10001 at 0xc0aff4 (CDecisionMakerTypes::ScriptReferenceIndex, 20 u16 = 1); [0xc0b01c] (m_IsActive, first 4 bytes) and [0xc0b020..0xc0b02c] = 0 (m_IsActive is 20 bools = 5 dwords).
- gta-reversed: ctor is default/compiler-generated; static init order as listed.
- Open question: what the five dwords 0xc0afe0..0xc0aff0 are (0x1567a6b).

## 0x46a380 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret`.

## 0x46a5f0 CEntitySeekPosCalculator::ComputeEntitySeekPos (empty, ret 0xc) (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall void (a, b, c)`: `ret 0xc` only. Port: no-op.

## 0x46a610 CEntitySeekPosCalculator::~CEntitySeekPosCalculator (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x859dc4`; return (the `ret` sits at 0x401621, reached by jmp).

## 0x46f690 CTaskSimpleCarDriveTimed::~CTaskSimpleCarDriveTimed (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void ()`: `[this] = 0x859e50` (CTaskSimpleCarDriveTimed vtable), `jmp CTaskSimpleCarDrive::Destructor 0x63c460`.

## 0x48e600 CTaskComplexWanderStandard::~CTaskComplexWanderStandard (proposed)  (catalogue: unknown/REA, port level 3)
- `__thiscall void ()`: `[this] = 0x85a200`; tail `jmp 0x669d20`  (base destructor) (0x48e600).

## 0x492d40 task dtor (vtable 0x85a334, jmp 0x61a3a0) (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`: `[this] = 0x85a334`; tail `jmp 0x61a3a0`  (base destructor) (0x492d40).

## 0x492f60 CEntitySeekPosCalculator::~CEntitySeekPosCalculator (proposed)  (catalogue: unknown/REA, port level 0)
- Same effect as 0x46a610 (no relocation).

## 0x4994e0 task dtor (vtable 0x85a428) (proposed)  (catalogue: unknown/REA, port level 5)
- `__thiscall void ()`: `[this] = 0x85a428`; tail `jmp 0x460d80` CTaskComplexWanderCop::CTaskComplexWanderCop (base destructor) (0x4994e0).

## 0x4a6da0 FxInfoEmWeather_c::~FxInfoEmWeather_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6da0).

## 0x4a6e10 FxInfoEmRotation_c::~FxInfoEmRotation_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6e10).

## 0x4a6e80 FxInfoNoise_c::~FxInfoNoise_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6e80).

## 0x4a70b0 FxInfoGroundCollide_c::~FxInfoGroundCollide_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a70b0).

## 0x4a73b0 FxInfoWind_c::~FxInfoWind_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a73b0).

## 0x4a7420 FxInfoJitter_c::~FxInfoJitter_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7420).

## 0x4a7490 FxInfoRotSpeed_c::~FxInfoRotSpeed_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7490).

## 0x4a76c0 FxInfoSpriteRect_c::~FxInfoSpriteRect_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo255_c::~FxInterpInfo255_c 0x4a8b70(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a76c0).

## 0x4a7730 FxInfoHeatHaze_c::~FxInfoHeatHaze_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo255_c::~FxInterpInfo255_c 0x4a8b70(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7730).

## 0x4a7810 FxInfoFlat_c::~FxInfoFlat_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7810).

## 0x4ab980 CEventGroup::~CEventGroup (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`: `[this] = 0x85aab0`; `CEventGroup::Flush(this, 0)` 0x4ab370 (thiscall, pushes 0).

## 0x4ac010 CEvent::~CEvent (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85ab38` (base-class vtable store of an inlined trivial destructor) (0x4ac010).

## 0x4ac0d0 CInformFriendsEvent::CInformFriendsEvent (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* ()`, EAX = this: `[+0] = 0` (ped), `[+4] = 0` (event), `[+8] = -1` (time).

## 0x4ac720 CEventGunShot::~CEventGunShot (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 during the body).
- `[this] = 0x85abe0`; `if ([this+0x14]) CEntity::CleanUpOldReference([this+0x14], &this[0x14])` 0x571a00 (thiscall on the entity, arg = slot address); `[this] = 0x85ab38` (CEvent).
- Pattern shared by the CEvent subclass destructors below ("event dtor pattern").

## 0x4ac920 CEventVehicleCollision::~CEventVehicleCollision (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85ac28`; for each reference slot in order +0x14: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4ac920).

## 0x4aca70 CEventPedCollisionWithPed::~CEventPedCollisionWithPed (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720): own vtable 0x85ac68, clean-up of the entity reference at +0x14, CEvent vtable.

## 0x4acdd0 CEventObjectCollision::~CEventObjectCollision (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern on +0x14 with vtable 0x85aca8. The SEH handler address is computed at 0x406a96-0x406aa5 (`[0x1560838] xor [0x572363]` = 0x839a28 statically) and placed by `xchg [esp],ecx`; extent piece 0x406a96-0x406aad belongs here.

## 0x4acff0 CEventBuildingCollision::~CEventBuildingCollision (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern on +0x14 (building reference) with vtable 0x85ace8.

## 0x4ad960 CEventDamage::~CEventDamage (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern on +0x14 with vtable 0x85ad78.

## 0x4ade90 CEventDeath::~CEventDeath (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85ab38` (CEvent vtable) (0x4ade90). Derived class with no members to destroy.

## 0x4ae270 CEvent subclass dtor (vtable 0x85ae88) (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85ae88`; for each reference slot in order +0x14: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4ae270).

## 0x4ae3c0 CEventPotentialWalkIntoVehicle::~CEventPotentialWalkIntoVehicle (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85aed0`; for each reference slot in order +0x14: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4ae3c0).

## 0x4ae670 CEventPotentialWalkIntoObject::~CEventPotentialWalkIntoObject (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85af18`; for each reference slot in order +0x14: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4ae670).

## 0x4ae7a0 CEventPotentialWalkIntoPed::~CEventPotentialWalkIntoPed (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85af60`; for each reference slot in order +0x20: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4ae7a0).

## 0x4aeb90 vt_85afe8 dtor (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85afe8`; for each reference slot in order +0x14, +0x18: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4aeb90).

## 0x4aeca0 CEvent::~CEvent (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85ab38` (base-class vtable store of an inlined trivial destructor) (0x4aeca0).

## 0x4aed60 vt_85b070 dtor (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b070`; for each reference slot in order +0x10: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4aed60).

## 0x4aeea0 vt_85b0b0 dtor (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b0b0`; for each reference slot in order +0x14: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4aeea0).

## 0x4af0a0 vt_85b0f8 dtor (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b0f8`; for each reference slot in order +0xc: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4af0a0).

## 0x4af400 CEventAttractor::~CEventAttractor (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b1b8`; for each reference slot in order +0x18: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4af400).

## 0x4af700 CEventVehicleToSteal::~CEventVehicleToSteal (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b210`; for each reference slot in order +0xc: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4af700).

## 0x4af890 CEventAcquaintancePed::~CEventAcquaintancePed (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b250`; for each reference slot in order +0x14: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4af890).

## 0x4afbf0 vt_85b298 dtor (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b298`; for each reference slot in order +0x14: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4afbf0).

## 0x4afe70 CEventKnockOffBike::~CEventKnockOffBike (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern with vtable 0x85b2e0, two references cleaned in order +0x38 (0x4afe93) then +0x30 (0x4afeab).

## 0x4b0a50 CEventScriptCommand::~CEventScriptCommand (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame: `[this] = 0x85b378`; `if (m_task [+0x10]) m_task->vtbl[0](1)` (scalar deleting dtor, 0x4b0a82-0x4b0a86); `[this] = 0x85ab38`. The pointer is not nulled.

## 0x4b0d00 CEventInAir::~CEventInAir (proposed)  (catalogue: unknown/REA, port level 0)
- Same body as 0x4ade90.

## 0x4b0da0 CEventVehicleDied::~CEventVehicleDied (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b400`; for each reference slot in order +0xc: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4b0da0).

## 0x4b1170 CEventVehicleOnFire::~CEventVehicleOnFire (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern on +0x14 with vtable 0x85b440.

## 0x4b1320 CEvent::~CEvent (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85ab38` (base-class vtable store of an inlined trivial destructor) (0x4b1320).

## 0x4b13c0 CEventInWater::~CEventInWater (proposed)  (catalogue: unknown/REA, port level 0)
- Same body as 0x4ade90. Open question: owner class from caller context (vtable 0x85b4c8 unnamed in the catalogue).

## 0x4b1520 CEventStuckInAir::~CEventStuckInAir (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b508`; for each reference slot in order +0xc: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4b1520).

## 0x4b17f0 CEventCopCarBeingStolen::~CEventCopCarBeingStolen (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern with vtable 0x85b548, references cleaned in order +0xc then +0x10.

## 0x4b1990 CEventVehicleDamage::~CEventVehicleDamage (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b588`; for each reference slot in order +0x14, +0x18: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4b1990).

## 0x4b1b50 CEvent::~CEvent (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85ab38` (base-class vtable store of an inlined trivial destructor) (0x4b1b50).

## 0x4b1c10 CEventGotKnockedOverByCar::~CEventGotKnockedOverByCar (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720 in AI/_unk-A.md): `[this] = 0x85b618`; for each reference slot in order +0x14: `if (slot) CEntity::CleanUpOldReference(slot, &slot)` 0x571a00; `[this] = 0x85ab38` (base vtable) (0x4b1c10).

## 0x4b1f00 CEvent::~CEvent (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85ab38` (base-class vtable store of an inlined trivial destructor) (0x4b1f00).

## 0x4b1f10 CEventFireNearby::CEventFireNearby (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CEvent* (const CVector& pos)`, EAX = this, `ret 4`.
- `[+4] = 0` (m_nTimeActive), `byte[+8] = 0`, `byte[+0xc] = 1`, `word[+0xe] = 0xc8` (200), `word[+0x10] = 0xffff`, `[+0] = 0x85b6e8`, `+0x14..+0x1c = pos`.
- Open question: vtable 0x85b6e8 is not gr-named in data-tables; class from address adjacency and layout (0x4b1f2f).

## 0x4b2070 CEventGunShotWhizzedBy::~CEventGunShotWhizzedBy (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void ()`: `[this] = 0x85b730`; `jmp 0x4ac720` (CEventGunShot::~CEventGunShot).

## 0x4b2210 CEventAreaCodes::~CEventAreaCodes (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern on +0xc with vtable 0x85b7c0.

## 0x4b2470 CEventPlayerCommandToGroup::~CEventPlayerCommandToGroup (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern (see 0x4ac720) with vtable 0x85b800, reference at +0x18 (0x4b2493).

## 0x4b26a0 CEventDanger::~CEventDanger (proposed)  (catalogue: unknown/REA, port level 1)
- Event dtor pattern with vtable 0x85b848, reference at +0x14.

## 0x4b8750 vt_85b8dc dtor (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void ()`: `[this] = 0x85b8dc`; tail `jmp 0x62f360`  (base destructor) (0x4b8750).
