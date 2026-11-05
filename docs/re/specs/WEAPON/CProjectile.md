# CProjectile (WEAPON) - full specs

## 0x5A40F0 CProjectile::vfunc_0 (= scalar deleting destructor)  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall CProjectile* (uint32 flags [esp+4])`, returns `this` in EAX, `ret 4`. Vtable slot 0 of CProjectile
  (no direct callers; invoked as `obj->vtbl[0](1)`, e.g. by CProjectileInfo::RemoveIfThisIsAProjectile 0x739A40).
- Behaviour: call 0x5A40E0 (the CProjectile destructor body: `*this = 0x867030` (CProjectile vtable) then tail-jump to
  `CObject::~CObject` 0x59F660); if `flags & 1`: `CObject::operator delete(this)` 0x5A1F20 (returns the slot to
  `CPools::ms_pObjectPool`, cdecl(void*)). Return this.
- gta-reversed: compiler-generated (`~CProjectile` + pool `operator delete`). Open questions: none.
