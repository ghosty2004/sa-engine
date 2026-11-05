# CAnimatedBuilding (ANIM) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x537A50 CAnimatedBuilding::vfunc_0 = scalar deleting destructor  (catalogue: specified/REA, port level 14)
- Convention: virtual slot 0, `__thiscall CAnimatedBuilding*(uint8 flags [esp+4])`, `ret 4`, returns `this`.
- Behaviour: `~CAnimatedBuilding()` (0x537A70); if `flags & 1`: `operator delete(this)` (CBuilding pool delete 0x4040A0, cdecl) (0x537A58-0x537A65);
  return this.

## 0x537A70 CAnimatedBuilding::~CAnimatedBuilding (proposed; catalogue: unnamed, unknown/REA, port level 13)
- Evidence: only caller is the deleting destructor CAnimatedBuilding::vfunc_0 0x537A50. Body: `jmp CEntity::Destructor 0x535E90`
  (ECX unchanged) - the class adds no members needing destruction (no vtable store either; CEntity's destructor stores its own).
