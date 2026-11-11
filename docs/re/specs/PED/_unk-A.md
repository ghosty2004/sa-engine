# PED: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x4411a0 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Single `ret` (0x4411a0). Referenced as a function pointer by CPed::Load/Save (likely a trivial element destructor passed to an EH vector iterator). Port: no-op.
- Open question: what the reference passes it to (callers' sites).
