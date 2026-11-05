# CBaseModelInfo (STREAM) - full specs, writer A (RE-SPEC-1)

Vtable of CBaseModelInfo: 0x85BB9C (gr BaseModelInfo.cpp:12, 15 slots). Slot order (validated by dumping the
vtables 0x85BB9C, 0x85BBF0, 0x85BC30, 0x85BC70, 0x85BCB0, 0x85BCF0, 0x85BD30, 0x85BD78, 0x85BDC0, 0x85C5C8):
0 scalar deleting destructor, 1 AsAtomicModelInfoPtr, 2 AsDamageAtomicModelInfoPtr, 3 AsLodAtomicModelInfoPtr,
4 GetModelType, 5 GetTimeInfo, 6 Init, 7 Shutdown, 8 DeleteRwObject, 9 GetRwModelType, 10/11 CreateInstance,
12 SetAnimFile, 13 ConvertAnimFileIndex, 14 GetAnimFileIndex (+15 SetClump/SetAtomic in derived classes).

Slot-0 naming rule used in all model-info files: a slot-0 function that (a) runs the class destructor
(inline or by call), (b) tests `byte [esp+4] & 1` and only then calls 0x8214BD (`operator delete`, gr
MemoryMgr.cpp:458) with `this`, (c) returns `this` in EAX with `ret 4`, is the MSVC *scalar deleting
destructor* `T::`scalar deleting destructor'(unsigned int flags)`. All `vfunc_0` rows below match this
pattern exactly.

## 0x4C4AF0 CBaseModelInfo::`scalar deleting destructor' (catalogue: CBaseModelInfo::vfunc_0, specified/REA, port level 1)
- Convention: `__thiscall CBaseModelInfo* (CBaseModelInfo* this, uint32 flags)`; flags at [esp+4]; `ret 4`;
  EAX = this. Preserves ESI.
- Callees: `orig!(0x8214BD operator delete)(void*)` (cdecl).
- Behaviour: test `flags & 1` (low byte) first; `this->vptr = 0x85BB9C` (inlined ~CBaseModelInfo, the
  store happens regardless of flags); if `flags & 1`: `operator delete(this)`. Return this.
- Callers: none direct (reached through vtable slot 0 of 0x85BB9C; CBaseModelInfo is abstract, so never via
  a live object in practice).
- gta-reversed: `virtual ~CBaseModelInfo() { assert(0); }` (BaseModelInfo.h:143) - the original has no
  assert; port the bytes.
- Open questions: none.

## 0x4C6200 CBaseModelInfo::Shutdown vcall thunk (proposed name: `CBaseModelInfo::`vcall'{0x1C}`)  (catalogue: unknown/REA, port level 0)
Hint `sub_4c6200_from_CModelInfo__ShutDown`. Evidence: body `mov eax,[ecx]; jmp [eax+0x1C]` = a jump through
vtable slot 7 (`Shutdown`, see slot list above) with ECX unchanged; the only caller CModelInfo::ShutDown
(0x4C63E0, 9 call sites) calls it with ECX = each element of each model-info store (e.g. `mov edi,0xAAE954;
mov ecx,edi; call 0x4C6200; add edi,0x20`). This is the MSVC virtual-call thunk produced for a call through a
member pointer / out-of-line virtual dispatch of `Shutdown()`.
- Convention: `__thiscall void (CBaseModelInfo* this)`; tail-jumps into the virtual, so the ABI is that of
  `Shutdown` (thiscall, no args, plain ret). Clobbers EAX.
- Behaviour: `this->Shutdown()` (virtual dispatch through vtable +0x1C).
- Port: a Rust function that performs the virtual call `vtable[7](this)`.
- Open questions: none.
