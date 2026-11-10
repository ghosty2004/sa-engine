# CTaskSimple and simple-task deleting destructors (AI) - RE-SPEC-3 group I1

CTaskSimple slots: 0-6 as CTask, 7 ProcessPed(ped), 8 SetPedPosition(ped). Layout: +0 vtbl, +4 m_Parent; size 8.

## 0x43E300 CTaskSimple::GetSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* ()`: `xor eax,eax` -> nullptr. Slot 2 of every simple task (shared body). gr matches.

## 0x43E310 CTaskSimple::IsSimple  (catalogue: specified/REA, port level 0)
- `__thiscall bool ()`: `mov al,1` -> true. Slot 3. gr matches.

## 0x43E320 CTaskSimple::SetPedPosition  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* [esp+4])`: `xor al,al; ret 4` -> false. Slot 8. gr matches.

## Deleting destructors (catalogue name `CTaskSimple::~CTaskSimple`; NAME CORRECTION)
Same shape as the complex ones (CTaskComplex.md): call the complete dtor, `if (flags & 1) CTask::operator delete(this)`
0x61A5B0, return this, `ret 4`. Port level 2.

| address | vtable (slot 0) | proposed name | complete dtor called |
|---|---|---|---|
| 0x48E4C0 | 0x85A1DC | CTaskSimpleSay::`scalar deleting destructor' | 0x48E4E0 |
| 0x48E880 | 0x85A278 | CTaskSimplePause::`scalar deleting destructor' | 0x48E8A0 (unnamed in catalogue; proposed ~CTaskSimplePause) |
| 0x637A40 | 0x86DF18 | CTaskSimpleSitIdle::`scalar deleting destructor' | 0x6311B0 |
| 0x64B000 | 0x86ED2C | CTaskSimpleCarCloseDoorFromOutside::`scalar deleting destructor' | 0x646570 |
| 0x64BDC0 | 0x86EE4C | CTaskSimpleCarGetOut::`scalar deleting destructor' | 0x647260 |
| 0x6644A0 | 0x86FA68 | CTaskSimpleDoHandSignal::`scalar deleting destructor' | 0x6608B0 |
