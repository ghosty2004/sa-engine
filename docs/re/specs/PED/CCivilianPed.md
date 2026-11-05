# CCivilianPed (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x5DDBE0 CCivilianPed::~CCivilianPed  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall void()`. `this->vtable = 0x86C0A8` (CCivilianPed vtable), then tail-jump `CPed::Destructor` 0x5E8620.
  Caller: deleting destructor 0x5DDC30.
