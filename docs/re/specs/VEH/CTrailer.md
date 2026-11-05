# CTrailer (VEH) - full specs

## 0x6CED10 CTrailer::~CTrailer  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall void (CTrailer* this)`; tail-jumps into CAutomobile::Destructor 0x6A61E0 (which returns).
- Callers: CTrailer::vfunc_0 0x6CF180.
- Behaviour: `*(void**)this = 0x871C28` (CTrailer vtable); `jmp CAutomobile::~CAutomobile` (0x6CED10-0x6CED16).
  CTrailer has no members needing destruction.
- gta-reversed: compiler-generated.

## 0x6CF180 CTrailer::vfunc_0  (catalogue: specified/REA, port level 17)
- Convention: `__thiscall CTrailer* (CTrailer* this, u8 flags [esp+4])`, `ret 4` (scalar deleting destructor, vtable
  slot 0; proposed name `CTrailer::scalar_deleting_destructor`).
- Callers: virtual (same four as CAutomobile::vfunc_0). Callees: ~CTrailer 0x6CED10, CVehicle::operator delete
  0x6E2D90.
- Behaviour: `~CTrailer(this)`; `if (flags & 1) CVehicle::operator delete(this)`; return `this`.
