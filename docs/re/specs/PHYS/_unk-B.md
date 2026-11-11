# PHYS: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x542080 CEntryInfoList::~CEntryInfoList (proposed)  (catalogue: unknown/REA, port level 1)
- Single `jmp CEntryInfoList::Flush 0x536E10` (thiscall, ECX = list); used as EH-unwind action of CPhysical::Constructor / Destructor for the CPhysical::m_pCollisionList member. Open questions: none.

## 0x5A2CB0 CPhysical::sub_5A2CB0 (proposed: set m_vecCentreOfMass)  (catalogue: unknown/REA, port level 0)
- Name evidence: thiscall storing three by-value dwords at `this+0xA4` (gr Physical.h `CVector m_vecCentreOfMass`, offset 0xA4 = after m_fBuoyancyConstant 0xA0); callers CObjectData::SetObjectData 0x5A2D00 and CAutomobile::SpawnFlyingComponent 0x6A8580 (4 sites), both of which set a centre of mass.
- Convention: `__thiscall void (CPhysical* this = ECX, float x [esp+4], float y [esp+8], float z [esp+0xC])` (or a by-value CVector), `ret 0xC`. Part of the body is relocated: 0x5A2CCC `jmp 0x404CAC` (.HOODLUM, plain code) does `ecx += 0xA4` and jumps back to 0x5A2CD1.
- Behaviour: copies x,y,z through a local temp (raw dword moves, no FPU) to `this+0xA4/+0xA8/+0xAC`.
- gta-reversed: absent (gr writes `m_vecCentreOfMass = ...` directly). Open questions: exact gr/original name (no symbol).

## 0x65DA70 CPhysical::ApplyTurnForce (by-value forwarding copy, proposed)  (catalogue: unknown/REA, port level 2)
- Name evidence: copies its six stack dwords (two CVectors) and calls CPhysical::ApplyTurnForce 0x542A50 with ECX untouched and the vectors in the same order; callers CTaskComplexProstituteSolicit::ControlSubTask, CPed::MakeTyresMuddySectorList (2), CVehicle::AddPassenger (2).
- Convention: `__thiscall void (CPhysical* this = ECX, CVector force [esp+4..+0xC], CVector point [esp+0x10..+0x18])`, `ret 0x18`.
- Behaviour: `ApplyTurnForce(force, point)` (re-pushed by value through a 0x18-byte local copy). No other effect. Open questions: none.
