# CInformFriendsEvent (AI) - RE-SPEC-FINAL (helper B)

## 0x4AC0F0 CInformFriendsEvent::~CInformFriendsEvent  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this = ECX: CInformFriendsEvent*)`, plain `ret`. No callers and no data references in the image (unreferenced out-of-line copy; the queue code inlines the same logic, e.g. CInformFriendsEventQueue::Flush 0x4AC2A0). Layout per gr InformFriendsEventQueue.h: +0 CPed* m_Ped, +4 CEvent* m_Event, +8 int32 m_Time.
- Behaviour: `if m_Ped` (0x4AC0F5) -> `CEntity::CleanUpOldReference(m_Ped, &m_Ped)` 0x571A00 (0x4AC0FA), `m_Ped = 0` (0x4AC0FF). `if m_Event` (0x4AC108) -> virtual slot 0 (scalar deleting dtor) with flags = 1 (`push 1; call [vtbl]` 0x4AC110), `m_Event = 0` (0x4AC112). `m_Time = -1` (0x4AC119).
- gta-reversed: struct with defaulted dtor (no cleanup); the cleanup sequence matches gr Flush's per-entry body.

## 0x4AC160 sub_4ac160 (proposed: CInformFriendsEvent::Process)  (catalogue: unknown/REA, port level 9)
- `__thiscall void (this: CInformFriendsEvent*)`, plain `ret`. Unreferenced (the per-entry body of CInformFriendsEventQueue::Process 0x4AC2E0 as an out-of-line copy).
- Behaviour: `if m_Ped != 0` (0x4AC165): `if (u32)m_Time >= (u32)[0xB7CB84 CTimer::m_snTimeInMilliseconds]` (unsigned `jae` 0x4AC172) -> return immediately (nothing changed). Else `CEventGroup::Add(&m_Ped->m_pIntelligence (+0x47C)->+0x68 (m_eventGroup), m_Event, 0)` 0x4AB420 (thiscall, args event, bool=0; 0x4AC183); then `if m_Ped` -> CleanUpOldReference(m_Ped, &m_Ped) 0x571A00, `m_Ped = 0`. Then (also reached directly when m_Ped == 0): `if m_Event` -> virtual slot 0 (deleting dtor, flags 1) 0x4AC1A5, `m_Event = 0`; `m_Time = -1` (0x4AC1AE). Note: the event is deleted right after being passed to CEventGroup::Add (Add stores a clone).
- gta-reversed: matches gr CInformFriendsEventQueue::Process loop body (InformFriendsEventQueue.cpp).

## 0x4B2950 CInformFriendsEvent::~CInformFriendsEvent  (catalogue: unknown/REA, port level 1)
- `__thiscall void (this: CInformFriendsEvent*)`, plain `ret`. Referenced as the element dtor of the static array `CInformFriendsEventQueue::ms_informFriendsEvents` (0xA9AFB8, 8 x 0xC) by its initialiser 0x84BA30 and atexit destructor 0x8561F0 (vector-dtor iterator argument).
- Behaviour: identical to 0x4AC0F0: `if m_Ped (+0)` -> CleanUpOldReference(m_Ped, &m_Ped) 0x571A00 (0x4B295A), `m_Ped = 0`; `if m_Event (+4)` -> vslot 0 with flags 1 (0x4B2970), `m_Event = 0`; `m_Time (+8) = -1` (0x4B2979).
