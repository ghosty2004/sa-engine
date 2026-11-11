# AUDIO: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x446fe0 CAESound-like member ctor (sets +8 = 0) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* ()`: `[this+8] = 0`, EAX = this (0x446fe2).
- Open question: which element type of CAESoundManager is constructed (fnref at 0x4efd30); the +8 field is the only one initialised.

## 0x447000 audio entity dtor (vtable 0x859988, CAESound unregister at +8) (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void ()`: `[this] = 0x859988`; `ecx = this+8`; `jmp 0x4ef660` (clears the entity reference at this+0x10).
- Open question: class of vtable 0x859988 (not gr-named; same body at 0x447050, 0x4a9f60, 0x4e34c0, 0x506c30, 0x506c40, 0x506cc0).

## 0x447050 audio entity dtor (vtable 0x859988, CAESound unregister at +8) (proposed)  (catalogue: unknown/REA, port level 2)
- Same body as 0x447000.

## 0x4d7e30 empty function (ret 8) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall`/`__stdcall` with 8 bytes of stack args, no body (0x4d7e30). Open question: owning vtable (no references in the catalogue).

## 0x4d8670 CAEBankLoader::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `CAEBankLoader::Deconstructor 0x4dfb20(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4d8670).

## 0x4d8690 CAEMP3TrackLoader::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `CAEMP3TrackLoader::Deconstructor 0x4e0940(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4d8690).

## 0x4e3330 tPedSpeechSlot element ctor (trivial) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, EAX = this, no stores (0x4e3330). Element ctor of CAEPedSpeechAudioEntity::s_PedSpeechSlots.

## 0x4e3340 tPedSpeechSlot element dtor (empty) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x4e3340). Port: no-op.

## 0x4ee9a0 CAESmoothFadeThread::CAESmoothFadeThread (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* ()`, EAX = this: `[+0] = -1`; `[+4] = 0`; `byte[+0x808] = byte[+0x809] = 0`; `word[+0x80a] = word[+0x80c] = 0`; `[+0x810] = 0`; `[+0x818] = 0`.
- gta-reversed: CAESmoothFadeThread (Audio/AESmoothFadeThread.h) ctor; field names not checked (open question 0x4ee9a4).

## 0x4ef070 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x4ef070). Port: no-op.

## 0x4ef660 CAESound::UnregisterWithPhysicalEntity (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`: `if ([this+8]) { CEntity::CleanUpOldReference([this+8], this+8) 0x571a00; [this+8] = 0 }`.
- Byte-identical to CAESound::UnregisterWithPhysicalEntity 0x4ef1a0 (catalogue "CAESound" here; 73 callers incl. CGarage/FxSystem_c dtors). Open question: whether this copy is a different class's inline (same +8 layout).

## 0x4f63b0 CAESound-like ctor for m_sRainSoundL/R (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void* ()`, EAX = this: `[+0x10] = 0`; `[+0] = 0x85f438`; `word[+0x88] = 0`; `[+0xa0] = 0`; `[+0xa4] = 0`.
- Open question: class of vtable 0x85f438 (not gr-named in data-tables); likely CAESound (0x4f63b7).

## 0x506dc0 CAEGlobalWeaponAudioEntity::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 16)
- `__thiscall void* (u32 flags)`, `ret 4`: `CAEGlobalWeaponAudioEntity::Destructor 0x506c50(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x506dc0).
