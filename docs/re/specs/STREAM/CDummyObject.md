# CDummyObject (STREAM) - full specs, writer A (RE-SPEC-1)

## 0x59EA00 CDummyObject::CDummyObject()  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall CDummyObject* (this)`; plain `ret`; EAX = this. Preserves ESI.
- Callees: `orig!(0x532540 CDummy::CDummy)` (thiscall).
- Callers: 1 (CFileLoader::LoadObjectInstance(inst) 0x538090, after CDummy::operator new 0x5326E0).
- Behaviour: `CDummy::CDummy(this); this->vptr = 0x866E78` (CDummyObject vtable, gr DummyObject.cpp:7);
  return this.
- Edge cases: `this` is not null-checked here (any NULL check of the operator new result is the caller's).
- gta-reversed: DummyObject.cpp:14-17 `CDummyObject() : CDummy() {}` - matches. Open questions: none.
