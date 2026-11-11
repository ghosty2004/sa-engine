# CEventPotentialWalkIntoFire (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B1ED0 CEventPotentialWalkIntoFire::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 46 (0x2E) (0x4B1ED0).

## 0x4B1EE0 CEventPotentialWalkIntoFire::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B1EE0).

## 0x4B1EF0 CEventPotentialWalkIntoFire::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 31 (0x1F) (0x4B1EF0).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B6840 CEventPotentialWalkIntoFire::~CEventPotentialWalkIntoFire  (catalogue: specified/REA, port level 1)
- Scalar deleting destructor, `ret 4`: calls 0x4B1F00 (unnamed complete dtor: `vtbl = 0x85AB38` only); ECX not saved, reused as `this`; if flags & 1 event-pool free
  inlined (pool [0xB744AC], element 0x44, magic 0x78787879 >> 5, byteMap |= 0x80, firstFree = min, 0x4B684D-0x4B6880). Returns this.

## 0x4B7A40 CEventPotentialWalkIntoFire::CloneEditable  (catalogue: specified/REA, port level 1)
- `__thiscall`, `ret`, no SEH: `p = CEventPool::New([0xB744AC])` 0x4B5570; null -> 0 (0x4B7A50); else `CEventPotentialWalkIntoFire::Constructor(p, &this+0x14 firePos,
  this+0x20 radius (raw f32 dword), this+0x28 moveState)` 0x4B1E20 (0x4B7A52-0x4B7A60); returns its EAX.
