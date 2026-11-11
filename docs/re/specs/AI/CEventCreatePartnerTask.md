# CEventCreatePartnerTask (AI) - RE-SPEC-FINAL (helper B)

## 0x5F6190 CEventCreatePartnerTask::CEventCreatePartnerTask  (catalogue: unknown/REA, port level 2)
- `__thiscall CEventCreatePartnerTask* (this, int32 partnerType [esp+4], CPed* partner [esp+8], uint8 isLeadSpeaker [esp+0xC], float meetDist [esp+0x10] (raw dword))`, `ret 0x10`; SEH frame (handler 0x83DE18); returns this. Callers: vt_86c6d0::Clone 0x5F6280, CGroupEventHandler::ComputeDoDealResponse 0x5FA290, ComputeGreetResponse 0x5FA550, CTaskComplexGangLeader::ScanForStuff 0x65E200 (2 sites).
- Behaviour: `CEvent::CEvent(this)` 0x4ABFC0 (0x5F61AD); `+0xC m_partnerType = partnerType` (0x5F61C0); `+0x14 m_isLeadSpeaker = (u8)isLeadSpeaker` (0x5F61C3); `vtbl = 0x86C6D0` (0x5F61D5); `+0x10 m_partner = partner` (0x5F61DB); `+0x18 m_meetDist = meetDist` (dword copy, 0x5F61DD); `if partner` -> `CEntity::RegisterReference(partner, &this->+0x10)` 0x571B70 (0x5F61E3).
- gta-reversed: field names from Events/EventCreatePartnerTask.h (m_partnerType, m_partner, m_isLeadSpeaker, m_meetDist).

## 0x5F6200 CEventCreatePartnerTask::~CEventCreatePartnerTask  (catalogue: unknown/REA, port level 1)
- Complete (non-deleting) dtor, `__thiscall void (this)`, plain `ret`; SEH frame (handler 0x83DE38). Callers: scalar deleting dtor 0x5F6300 and the stack instances in 0x5FA290, 0x5FA550, 0x65E200.
- Behaviour: `vtbl = 0x86C6D0` (0x5F621D); `if m_partner (+0x10)` -> CleanUpOldReference(m_partner, &this->+0x10) 0x571A00 (0x5F6236); then base dtor 0x4AC010 (CEvent::~CEvent, thiscall, 0x5F6245).
