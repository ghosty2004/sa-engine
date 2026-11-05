# CPedDamageResponseCalculator (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x4AD420 CPedDamageResponseCalculator::~CPedDamageResponseCalculator  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void()` (ECX unused), plain `ret`. Body is a single `ret` (0x4AD420).
- Callers: 38 sites (CCheat::SuicideCheat 0x438FF0, CRunningScript::ProcessCommands800To899 0x481300,
  CEventHandler::ComputeKnockOffBikeResponse 0x4B9FF0, CWorld::TriggerExplosionSectorList 0x567750, ...),
  many as EH-unwind actions of stack objects. Callees / globals: none.
- Behaviour: no-op (the class has no vtable and no owned resources). A port: empty `Drop` / no-op.
