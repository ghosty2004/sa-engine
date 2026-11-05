# CAnimatedBuilding (STREAM) - full specs, writer A (RE-SPEC-1)

## 0x537A10 CAnimatedBuilding::CAnimatedBuilding  (catalogue: "CAnimatedBuilding", specified/REA, port level 3)
- Convention: `__thiscall CAnimatedBuilding* (this)`; plain `ret`; EAX = this. Preserves ESI.
- Callees: `orig!(0x403E00 CBuilding::CBuilding)` (thiscall).
- Callers: 1 (CFileLoader::LoadObjectInstance(inst) 0x538090, on memory from CBuilding's pool operator new).
- Behaviour: `CBuilding::CBuilding(this); this->vptr = 0x8639B0` (CAnimatedBuilding vtable, gr
  AnimatedBuilding.cpp:7, 22 slots); return this. No fields written.
- gta-reversed: class has no explicit ctor (implicit). Matches. Open questions: none.
