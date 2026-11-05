# CDummyObject (WORLD)

CDummyObject = CDummy, vtable 0x866e78, same pool (`CPools::ms_pDummyPool`).

## 0x59ea20 CDummyObject::CDummyObject(CObject*)  (catalogue: specified/REA, port level 14)
- Convention / signature: `__thiscall CDummyObject* (this = ECX, CObject* obj [esp+4])`, returns this in
  EAX, `ret 4`. Has an SEH frame (unwind calls 0x532560 = CDummy dtor) — irrelevant to a port.
- Callers: none recorded (reached from `CObject`-to-dummy conversion code). Callees: `CDummy::CDummy`
  0x532540, `CEntity::SetModelIndexNoCreate` 0x533700 (`__thiscall (u32 model)`),
  `CEntity::AttachToRwObject` 0x533ed0 (`__thiscall (RwObject*, bool)`), `CEntity::DetachFromRwObject`
  0x533fb0 (`__thiscall ()`), `CIplStore::IncludeEntity` 0x404c90 (`__cdecl (int32 ipl, CEntity*)`).
- Behaviour:
  1. `CDummy::CDummy(this)`; `vtable = 0x866e78`.
  2. `this->SetModelIndexNoCreate((i32)(i16) obj+0x22)` — **obj's** model index, sign-extended.
  3. `if (obj+0x18 (m_pRwObject) != NULL) this->AttachToRwObject(obj->m_pRwObject, true)`.
  4. `obj->DetachFromRwObject()`.
  5. `byte this+0x2e = byte obj+0x2e` (`m_IplIndex`), `byte this+0x2f = byte obj+0x2f` (`m_AreaCode`).
  6. `CIplStore::IncludeEntity((u32)(u8) this->m_IplIndex, this)` (zero-extended).
  No matrix is copied when obj has no RwObject (unlike `CObject(CDummyObject*)`).
- gta-reversed: provided (DummyObject.cpp:19) with a **bug**: `CEntity::SetModelIndexNoCreate(GetModelIndex())`
  uses this dummy's own (just-constructed) model index instead of `obj->m_nModelIndex`. CONFIRMED
  deviation; port the bytes (obj's index). Rest equivalent.
- Open questions: none.

## 0x59eab0 CDummyObject::~CDummyObject (proposed name; catalogue: unknown/REA, port level 14)
- Evidence: sets the CDummyObject vtable 0x866e78 and tail-jumps to the CDummy destructor; only caller is
  the scalar deleting destructor 0x59eb50.
- Convention / signature: `__thiscall void (this = ECX)`; `vtable = 0x866e78; jmp 0x532560`
  (0x532560: `vtable = 0x8638c0; jmp CEntity::Destructor 0x535e90`).
- gta-reversed: compiler generated. Open questions: none.

## 0x59eb50 CDummyObject::vfunc_0 (scalar deleting destructor)  (catalogue: specified/REA, port level 15)
- Convention / signature: `__thiscall CDummyObject* (this = ECX, u32 flags [esp+4])`, returns this,
  `ret 4`. Vtable slot 0 of 0x866e78.
- Behaviour: `0x59eab0(this)`; `if (flags & 1) CDummy::operator delete(this)` (0x5326f0, `__cdecl`,
  caller pops 4). Return this.
- Open questions: none.
