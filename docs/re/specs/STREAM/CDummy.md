# CDummy (STREAM) - full specs, writer A (RE-SPEC-1)

## 0x5326E0 CDummy::operator new  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl void* operator new(size_t size)`; size at [esp+4] is ignored. Implemented as
  `mov ecx, [0xB744A0]; jmp 0x532630`: a tail jump into `CPool<CDummy>::New()` (thiscall, no args, plain
  `ret`), so the caller's cdecl argument stays on the stack and is cleaned by the caller. Returns EAX.
- Callers: 1 (CFileLoader::LoadObjectInstance(inst) 0x538090).
- Callees: `orig!(0x532630 CDummyPool::New)` with ECX = `CPools::ms_pDummyPool` (0xB744A0, CPool<CDummy>*,
  element 0x38; same algorithm as CPool<TxdDef>::New in CPool.md, returns NULL when full).
- Globals: R 0xB744A0 `CPools::ms_pDummyPool`.
- Behaviour: `return CPools::ms_pDummyPool->New();`. No null check of the pool pointer.
- gta-reversed: Dummy.cpp:12-15, same.
- Open questions: none.
