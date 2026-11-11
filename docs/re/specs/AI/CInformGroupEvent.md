# CInformGroupEvent (AI) - RE-SPEC-FINAL (helper B)

## 0x4AC370 CInformGroupEvent::~CInformGroupEvent  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this: CInformGroupEvent*)`, plain `ret`. Unreferenced out-of-line copy; the hooked gr dtor is 0x4B29A0. Layout (gr InformGroupEvent.h): +0 CPed* m_Ped, +4 CPedGroup* m_PedGroup, +8 CEvent* m_Event, +0xC int32 m_Time.
- Behaviour: `if m_Ped` -> CleanUpOldReference(m_Ped, &m_Ped) 0x571A00 (0x4AC37A), `m_Ped = 0`; `if m_Event` (+8) -> virtual slot 0 with flags 1 (0x4AC390), `m_Event = 0`; `m_Time = -1` (0x4AC399). m_PedGroup (+4) untouched.
