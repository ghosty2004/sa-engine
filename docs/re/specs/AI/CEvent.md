# CEvent (AI) - base event class, RE-SPEC-3 group I1

CEvent vtable 0x85AB38 (16 slots; gr Event.h): 0 dtor, 1 GetEventType, 2 GetEventPriority, 3 GetLifeTime, 4 Clone,
5 AffectsPed, 6 AffectsPedGroup, 7 IsCriminalEvent, 8 ReportCriminalEvent, 9 HasEditableResponse, 10 GetSourceEntity,
11 TakesPriorityOver, 12 GetLocalSoundLevel, 13 DoInformVehicleOccupants, 14 IsValid, 15 CanBeInterruptedBySameEvent
(CEventEditableResponse adds 16 CloneEditable). Layout: +0 vtbl, +4 m_nTimeActive (i32), +8 m_bValid (u8); size 0xC.
Many tiny bodies below are shared (identical-code-folded) by every event vtable whose slot has the same shape; a port
must make all those slots behave identically (they are one function in the original).

## 0x420EA0 CEvent::ReportCriminalEvent  (catalogue: specified/REA, port level 0)
- `__thiscall void (CEvent* this, CPed* ped [esp+4])`, body `ret 4` only (no-op). Slot 8 of ~69 event vtables. gr: inline empty, matches.

## 0x420EC0 CEvent::DoInformVehicleOccupants  (catalogue: specified/REA, port level 0)
- `__thiscall bool (CEvent*, CPed* [esp+4])`: `xor al,al; ret 4` -> returns false (AL only; upper EAX bits are whatever they were).
  Slot 13 of ~77 vtables (shared body). gr: inline `return false`, matches.

## 0x43E1E0 CEvent::HasEditableResponse  (catalogue: specified/REA, port level 0)
- `__thiscall bool (CEvent*)`: `xor al,al; ret` -> false. Slot 9 (shared ICF body: any 0-arg "return false" thiscall). gr matches.

## 0x43E1F0 CEvent::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall CEntity* (CEvent*)`: `xor eax,eax; ret` -> nullptr. Slot 10 (shared body). gr matches.

## 0x4ABC30 CEvent::TakesPriorityOver  (catalogue: specified/REA, port level 2)
- `__thiscall bool (CEvent* this, const CEvent* other [esp+4])`, `ret 4`, result in AL (0/1 via setge).
- Callers: CEventDamage::TakesPriorityOver 0x4ADB00, CEventGunAimedAt::TakesPriorityOver 0x4B0810 (direct), plus slot 11 of ~45 vtables.
- Behaviour: `o = other->GetEventPriority()` (vtbl+8, called FIRST, 0x4ABC3A); `t = this->GetEventPriority()` (0x4ABC43);
  `return t >= o` (signed, `cmp eax,edi; setge`, 0x4ABC48). Call order matters only if the overrides have side effects (they do not).
- gta-reversed: inline `GetEventPriority() >= other.GetEventPriority()` - same result; gr calls this first (order differs, harmless).

## 0x4ABFF0 CEvent::AffectsPedGroup  (catalogue: specified/REA, port level 0)
- `__thiscall bool (CEvent*, CPedGroup* [esp+4])`: `mov al,1; ret 4` -> true. Slot 6 of ~60 vtables (shared). gr matches.

## 0x4AC000 CEvent::CanBeInterruptedBySameEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (CEvent*)`: `xor al,al; ret` - byte-identical to 0x43E1E0 (separate copy). Slot 15. gr matches.

## 0x4AC020 CEvent::IsValid  (catalogue: specified/REA, port level 1)
- `__thiscall bool (CEvent* this, CPed* ped [esp+4])`, `ret 4`, returns EAX = 0 or 1. `ped` unused.
- Behaviour: if `m_bValid` (+8 byte) != 0 return 1 (0x4AC023-0x4AC028); `lt = this->GetLifeTime()` (vtbl+0xC, 0x4AC02C);
  `return (m_nTimeActive (+4) <= lt) ? 1 : 0` (signed `cmp [esi+4],eax; jle`, 0x4AC02F-0x4AC032). Slot 14 of ~74 vtables.
- gta-reversed: inline `m_bValid || m_nTimeActive <= GetLifeTime()`, matches.

## 0x4B5620 CEvent::operator new  (catalogue: specified/REA, port level 1)
- `__cdecl void* (uint32 size)` (size ignored): tail-jumps to `CEventPool::New` 0x4B5570 with ECX = `CPools::ms_pEventPool` [0xB744AC]
  (0x4B5620-0x4B5626). Returns the pool slot (or whatever New returns when full). gr Event.cpp:14 matches.

## 0x4B5630 CEvent::operator delete  (catalogue: specified/REA, port level 0)
- `__cdecl void (void* obj [esp+4])`, plain `ret`. Inlined `CPool<CEvent, 0x44>::Delete` on `ms_pEventPool` [0xB744AC]
  (pool layout +0 objects, +4 byteMap, +0xC firstFree):
  `idx = (obj - pool->objects) / 0x44` (signed magic divide 0x78787879, `sar edx,5` + sign fix: truncation toward zero,
  0x4B563B-0x4B564F); `pool->byteMap[idx] |= 0x80` (free flag, 0x4B5654; the 7-bit id is kept); `if (idx < pool->firstFree)
  pool->firstFree = idx` (signed, 0x4B5657-0x4B565C). No null check (delete of nullptr would corrupt a flag; MSVC callers test first).
- gta-reversed: `GetEventPool()->Delete(...)`, matches. Event object stride 0x44 (68).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x420E90 CEvent::IsCriminalEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (0 stack args)`, `ret`: returns false (AL=0; upper EAX bits unchanged) (0x420E90).

## 0x420EB0 CEvent::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 0.0f [0x858B50] (0x420EB0).

## 0x4ABFE0 CEvent::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4ABFE0).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B7C80 CEvent::~CEvent  (catalogue: specified/REA, port level 0)
- Scalar deleting destructor of CEvent itself (vtable 0x85AB38 slot 0): `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`. `this->vtbl = 0x85AB38` (always,
  0x4B7C86); if flags & 1: event-pool free inlined (pool = [0xB744AC]): `idx = (int32)(this - pool->m_pObjects) / 0x44` (magic 0x78787879 >> 5, 0x4B7C97-0x4B7CAA),
  `m_byteMap[idx] |= 0x80`, `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (0x4B7CB9-0x4B7CC1). Returns this.
