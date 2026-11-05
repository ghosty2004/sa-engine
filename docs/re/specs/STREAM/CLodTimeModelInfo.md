# CLodTimeModelInfo (STREAM) - full specs, writer A (RE-SPEC-1)

Vtable of CLodTimeModelInfo: 0x85BCF0 (slot 0 = 0x4C6290). Store: `CModelInfo::ms_lodTimeModelInfoStore 0xB1E128` (`CStore<CLodTimeModelInfo,N>`: +0x0 i32 count,
+0x4 elements, element size 0x28, N = 1), element constructor 0x4C5690 (calls 0x4C4A60 CBaseModelInfo ctor,
then installs vptr = 0x85BCF0, plus class field inits). See CBaseModelInfo.md for the vtable slot list and the slot-0 naming rule.

## 0x4C56C0 CLodTimeModelInfo::GetModelType (catalogue: CLodTimeModelInfo::vfunc_4, specified/REA, port level 0)
Slot 4 of vtable 0x85BCF0 = GetModelType (slot list in CBaseModelInfo.md; the same slot holds 0x4C5660
`mov al,3` in CTimeModelInfo and 0x4C5600 `mov al,8` in CLodAtomicModelInfo).
- Convention: `__thiscall ModelInfoType (this)`; returns AL = 3 (MODEL_INFO_TIME); plain `ret`; upper EAX
  bytes unchanged (callers read AL only).
- Callers: virtual (catalogue lists CVehicleModelInfo::AssignRemapTxd 0x4C9360 as a typed-override site).
- gta-reversed: LodTimeModelInfo.cpp:10 hooks GetModelType at 0x4C5660 (CTimeModelInfo's address) - gr's
  address is wrong, the CLodTimeModelInfo slot is 0x4C56C0; both return 3, so no behavioural difference.
- Open questions: none.

## 0x4C56D0 CLodTimeModelInfo::GetTimeInfo (catalogue: CLodTimeModelInfo::vfunc_5, specified/REA, port level 0)
Slot 5 of vtable 0x85BCF0 = GetTimeInfo.
- Convention: `__thiscall CTimeInfo* (this)`; returns EAX = this + 0x24 (`m_lodTimeInfo`, after the 0x24-byte
  CLodAtomicModelInfo base; gr LodTimeModelInfo.h, size 0x28); plain `ret`.
- gta-reversed: LodTimeModelInfo.cpp:11 hooks GetTimeInfo at 0x4C5670 (CTimeModelInfo's, which returns
  this+0x20) - wrong address in gr; the CLodTimeModelInfo function returns this+0x24. A port of the class
  must return +0x24 (keep-original).
- Open questions: none.

## 0x4C56E0 CLodTimeModelInfo::~CLodTimeModelInfo (proposed name)  (catalogue: unknown/REA, port level 0)
Evidence: (1) the slot-0 function 0x4C6290 of vtable 0x85BCF0 calls it before the conditional `operator delete`;
(2) the atexit destructor of `ms_lodTimeModelInfoStore 0xB1E128` passes it as the element destructor to
`eh vector destructor iterator` (0x821E02) with element size 0x28 and count 1; (3) the matching
element constructor 0x4C5690 installs vtable 0x85BCF0. The class and all its bases have trivial destructors, so
MSVC reduced the whole chain to the final base-class vptr store.
- Convention: `__thiscall void (CLodTimeModelInfo* this)`; plain `ret`; EAX not set (callers ignore it).
- Behaviour: `this->vptr = 0x85BB9C` (CBaseModelInfo vtable). Nothing else.
- gta-reversed: implicit destructor; no code. Open questions: none.

## 0x4C6290 CLodTimeModelInfo::`scalar deleting destructor' (catalogue: CLodTimeModelInfo::vfunc_0, specified/REA, port level 1)
- Convention: `__thiscall CLodTimeModelInfo* (CLodTimeModelInfo* this, uint32 flags)`; flags at [esp+4]; `ret 4`; EAX = this.
  Preserves ESI.
- Callees: `orig!(0x4C56E0 ~CLodTimeModelInfo)`, `orig!(0x8214BD operator delete)`.
- Behaviour: call ~CLodTimeModelInfo(this) first; then if `(u8)flags & 1`: `operator delete(this)`; return this.
- Callers: none direct (vtable slot 0 of 0x85BCF0).
- gta-reversed: implicit; no difference. Open questions: none.
