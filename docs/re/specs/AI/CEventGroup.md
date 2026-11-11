# CEventGroup (AI) - RE-SPEC-REST group B

## 0x4ABA20 CEventGroup::~CEventGroup  (catalogue: specified/REA, port level 1)
- Scalar deleting destructor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: `this->vtbl = 0x85AAB0` (CEventGroup);
  `CEventGroup::Flush(this, false)` 0x4AB370; if `flags & 1` `operator delete(this)` 0x8214BD (cdecl); return this.
