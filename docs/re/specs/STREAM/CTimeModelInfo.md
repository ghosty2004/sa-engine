# CTimeModelInfo (STREAM) - full specs, writer A (RE-SPEC-1)

Vtable of CTimeModelInfo: 0x85BCB0 (slot 0 = 0x4C6270). Store: `CModelInfo::ms_timeModelInfoStore 0xB1C960` (`CStore<CTimeModelInfo,N>`: +0x0 i32 count,
+0x4 elements, element size 0x24, N = 0xA9), element constructor 0x4C5640 (calls 0x4C4A60 CBaseModelInfo ctor,
then installs vptr = 0x85BCB0, plus class field inits). See CBaseModelInfo.md for the vtable slot list and the slot-0 naming rule.

## 0x4C5680 CTimeModelInfo::~CTimeModelInfo (proposed name)  (catalogue: unknown/REA, port level 0)
Evidence: (1) the slot-0 function 0x4C6270 of vtable 0x85BCB0 calls it before the conditional `operator delete`;
(2) the atexit destructor of `ms_timeModelInfoStore 0xB1C960` passes it as the element destructor to
`eh vector destructor iterator` (0x821E02) with element size 0x24 and count 0xA9; (3) the matching
element constructor 0x4C5640 installs vtable 0x85BCB0. The class and all its bases have trivial destructors, so
MSVC reduced the whole chain to the final base-class vptr store.
- Convention: `__thiscall void (CTimeModelInfo* this)`; plain `ret`; EAX not set (callers ignore it).
- Behaviour: `this->vptr = 0x85BB9C` (CBaseModelInfo vtable). Nothing else.
- gta-reversed: implicit destructor; no code. Open questions: none.

## 0x4C6270 CTimeModelInfo::`scalar deleting destructor' (catalogue: CTimeModelInfo::vfunc_0, specified/REA, port level 1)
- Convention: `__thiscall CTimeModelInfo* (CTimeModelInfo* this, uint32 flags)`; flags at [esp+4]; `ret 4`; EAX = this.
  Preserves ESI.
- Callees: `orig!(0x4C5680 ~CTimeModelInfo)`, `orig!(0x8214BD operator delete)`.
- Behaviour: call ~CTimeModelInfo(this) first; then if `(u8)flags & 1`: `operator delete(this)`; return this.
- Callers: none direct (vtable slot 0 of 0x85BCB0).
- gta-reversed: implicit; no difference. Open questions: none.
