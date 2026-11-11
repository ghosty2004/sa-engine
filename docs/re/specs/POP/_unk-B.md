# POP: functions without gta-reversed source (RE-SPEC-UNK group B)

## 0x56CDE0 CAccident::CAccident (proposed)  (catalogue: unknown/REA, port level 0)
- gr Accident.h:14 (`// 0x56CDE0`, inline ctor). Element ctor used by CAccidentManager ctor 0x56CE30.
- Convention: `__thiscall CAccident* (this = ECX)`, returns this. Behaviour: `+0 m_pPed = 0` (dword), `+4 m_bIsTreated = 0`, `+5 m_bIsRevived = 0` (byte stores). +6/+7 untouched. Open questions: none.

## 0x56CE30 CAccidentManager::CAccidentManager (proposed)  (catalogue: unknown/REA, port level 0)
- Name evidence: only caller CAccidentManager::GetInstance 0x56CF20 (function-local static construction); builds 16 CAccident (gr AccidentManager.h `CAccident m_Accidents[16]`).
- Convention: `__thiscall CAccidentManager* (this = ECX)`, returns this, plain `ret`.
- Behaviour: `eh_vector_constructor_iterator(this, 8, 16, CAccident::CAccident 0x56CDE0, 0x56CDF0 /*empty dtor*/)`; then a second inline loop over the 16 elements (eax = this+5, step 8, ecx = 16 down to 0) writing `dword [e+0] = 0; byte [e+4] = 0; byte [e+5] = 0` (the ctor body of gr's CAccidentManager re-initialising each accident). Net state identical to the vector ctor alone.
- Open questions: none.

## 0x5DE520 CGangInfo::CGangInfo  (catalogue: specified/REA, port level 0)
- gr GangInfo.h:11. Element ctor referenced by __init_CGangs__Gang 0x850CE0. Convention: `__thiscall CGangInfo* (this = ECX)`, returns this.
- Behaviour: byte +0 = 0xFF (m_nPedModelOverride = -1); dwords +4, +8, +0xC = 0 (m_nGangWeapons[3]). Bytes +1..+3 untouched. Open questions: none.
