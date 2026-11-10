# CTaskSimpleNone (AI) - RE-SPEC-3 group I1

Vtable 0x859CFC (unnamed in data-tables; slots 6/7 shared with CTaskSimpleFinishBrain 0x859D20).

## 0x4635D0 CTaskSimpleNone::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed*, int32 priority, const CEvent*)`: `mov al,1; ret 0xC` -> true. Slot 6 of 0x859CFC and of
  CTaskSimpleFinishBrain 0x859D20 (shared body). gr matches.

## 0x4635E0 CTaskSimpleNone::ProcessPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed*)`: `mov al,1; ret 4` -> true (task finished immediately). Slot 7 of 0x859CFC and 0x859D20. gr matches.
