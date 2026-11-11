# CEventInAir (AI) - RE-SPEC-3 group I2
Vtable 0x85B3C0, type 0x21 (33 EVENT_IN_AIR), size 0xC (no own members).

## 0x4B0CD0 CEventInAir::GetEventType  (catalogue: specified/REA, port level 0)
- returns 0x21.

## 0x4B0CF0 CEventInAir::GetEventPriority  (catalogue: specified/REA, port level 0)
- returns 0x3D (61).

## 0x4B64D0 CEventInAir::`scalar deleting destructor'  (catalogue name ~CEventInAir)  (catalogue: specified/REA, port level 1)
- `__thiscall this (u32 flags)`, `ret 4`: calls 0x4B0D00 (complete dtor) WITHOUT saving ECX first (`push esi; call` - relies on the
  callee preserving ECX; the free path then uses ECX as `this`); if `flags & 1` the inlined event-pool delete on [0xB744AC] (stride
  0x44; byteMap |= 0x80, firstFree lowered) (0x4B64DD-0x4B6510); returns this. Calling-convention oddity: a port must pass `this`
  through, the callee 0x4B0D00 must not clobber ECX.

## 0x4B76A0 CEventInAir::Clone  (catalogue: specified/REA, port level 1)
- `p = CEventPool::New(ms_pEventPool)` 0x4B5570; if p: `+4 = 0`, `+8 = 0` (byte), `vtbl = 0x85B3C0`; return p (or 0). No SEH (inlined
  trivial ctor). gr inline `new CEventInAir()` - same.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B0CE0 CEventInAir::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B0CE0).
