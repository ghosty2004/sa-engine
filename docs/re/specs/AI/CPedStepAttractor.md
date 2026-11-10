# CPedStepAttractor (AI) - RE-SPEC-3 group I3

Vtable 0x86C62C (CPedAttractor vtables: slot 0 GetType, slot 1 deleting dtor). Attractor pool `CPools::ms_pPedAttractorPool` [0xB744C4],
object stride 0xC4 (196). PED_ATTRACTOR_STEP = 9.

## 0x5EEBD0 CPedStepAttractor::CPedStepAttractor  (catalogue: specified/REA, port level 4)
- `__thiscall CPedStepAttractor* (this, C2dEffectPedAttractor* effect [esp+4], CEntity* entity [esp+8], eMoveState moveState [esp+0xC])`, `ret 0xC`.
  Caller: CPedAttractorManager::RegisterPed<CPedAttractor> 0x5EF980.
- Behaviour: `CPedAttractor::CPedAttractor(this, effect, entity, moveState, 1, 1.0f [0x86C510], 30000.0f [0x86C514], 3000.0f [0x86C518],
  0.125f [0x86C51C], 0.1f [0x86C520], 0.1f [0x86C524], 0.1f [0x86C528])` 0x5EDFB0 (11 stack args; the floats are pushed as raw dwords
  loaded from .rdata, 0x5EEBD0-0x5EEC14), then `vtbl = 0x86C62C` (0x5EEC19), return this.
- gta-reversed: PedStepAttractor.h:9 - same argument list. Matches.

## 0x5EEC30 CPedStepAttractor::GetType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,9; ret` -> 9 (PED_ATTRACTOR_STEP). Slot 0 of 0x86C62C. gr matches.

## 0x5EEC40 CPedStepAttractor::`scalar deleting destructor'  (catalogue name ~CPedStepAttractor)  (catalogue: specified/REA, port level 4)
- NAME CORRECTION: slot 1 of 0x86C62C. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this.
- Calls 0x5EEC90 (0x5EEC44): `vtbl = 0x86C62C; jmp 0x5EC410` (CPedAttractor::~CPedAttractor). Then if `flags & 1` (0x5EEC49) inlined
  `CPool::Delete` on `ms_pPedAttractorPool` [0xB744C4]: `idx = (this - pool->objects) / 0xC4` (signed magic 0x5397829D, `sar edx,6`, + sign bit:
  truncation toward zero, 0x5EEC56-0x5EEC6E); `byteMap[idx] |= 0x80` (0x5EEC73); `if (idx < firstFree (+0xC)) firstFree = idx` (signed `jge`,
  0x5EEC76-0x5EEC7E).
- gta-reversed: virtual dtor + pool operator delete - matches.
