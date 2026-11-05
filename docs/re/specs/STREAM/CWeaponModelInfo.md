# CWeaponModelInfo (STREAM) - full specs, writer A (RE-SPEC-1)

Vtable of CWeaponModelInfo: 0x85BD78 (slot 0 = 0x4C62D0). Store: `CModelInfo::ms_weaponModelInfoStore 0xB1E158` (`CStore<CWeaponModelInfo,N>`: +0x0 i32 count,
+0x4 elements, element size 0x28, N = 0x33), element constructor 0x4C5760 (calls 0x4C4A60 CBaseModelInfo ctor,
then installs vptr = 0x85BD78, plus class field inits). See CBaseModelInfo.md for the vtable slot list and the slot-0 naming rule.

## 0x4C5790 CWeaponModelInfo::~CWeaponModelInfo (proposed name)  (catalogue: unknown/REA, port level 0)
Evidence: (1) the slot-0 function 0x4C62D0 of vtable 0x85BD78 calls it before the conditional `operator delete`;
(2) the atexit destructor of `ms_weaponModelInfoStore 0xB1E158` passes it as the element destructor to
`eh vector destructor iterator` (0x821E02) with element size 0x28 and count 0x33; (3) the matching
element constructor 0x4C5760 installs vtable 0x85BD78. The class and all its bases have trivial destructors, so
MSVC reduced the whole chain to the final base-class vptr store.
- Convention: `__thiscall void (CWeaponModelInfo* this)`; plain `ret`; EAX not set (callers ignore it).
- Behaviour: `this->vptr = 0x85BB9C` (CBaseModelInfo vtable). Nothing else.
- gta-reversed: implicit destructor; no code. Open questions: none.

## 0x4C62D0 CWeaponModelInfo::`scalar deleting destructor' (catalogue: CWeaponModelInfo::vfunc_0, specified/REA, port level 1)
- Convention: `__thiscall CWeaponModelInfo* (CWeaponModelInfo* this, uint32 flags)`; flags at [esp+4]; `ret 4`; EAX = this.
  Preserves ESI.
- Callees: `orig!(0x4C5790 ~CWeaponModelInfo)`, `orig!(0x8214BD operator delete)`.
- Behaviour: call ~CWeaponModelInfo(this) first; then if `(u8)flags & 1`: `operator delete(this)`; return this.
- Callers: none direct (vtable slot 0 of 0x85BD78).
- gta-reversed: implicit; no difference. Open questions: none.
