# CPool (CORE) - RE-SPEC-FINAL group A

## 0x41CC10 CPool<CVehicle>::GetAt  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall CVehicle* (CPool* this, int32 index [esp+4])`, `ret 4`. 0x41CC10 jumps to the HOODLUM body 0x15638F0.
  Callers: CRunningScript::ProcessCommands1800To1899 0x46D050, CCamera::TryToStartNewCamMode 0x51E560 (2), CPlane::DoPlaneGenerationAndRemoval 0x6CD2F0.
- Behaviour: `if ((i8)flags[index] < 0) return 0` (free bit 0x80, `cmp byte,0; jns`); else `return objects + index*0xA18`. No range check.
- gta-reversed: CPool::GetAt - same. Open questions: none.

## 0x49C440 CPool<T>::Flush  (catalogue: unknown/REA, port level 2)
- Convention: `__thiscall void (CPool* this)`, plain `ret`. No code callers (an unreferenced template instance).
- Behaviour: if `this->size (+8) > 0`: if `this->bOwnsAllocations (+0x10, u8)` { `free(objects (+0))`; `free(flags (+4))` } (CRT _free 0x8213AE);
  then `+0 = 0; +4 = 0; +8 = 0; +0xC = 0` (firstFree). Size <= 0: nothing.
- gta-reversed: CPool::Flush (core/Pool.h) - same. Open questions: none.

## 0x49C950 CPool<CStuntJump>::Flush  (catalogue: unknown/REA, port level 2)
- Convention: `__thiscall void (CPool* this)`, plain `ret`. No code callers (an unreferenced template instance).
- Behaviour: if `this->size (+8) > 0`: if `this->bOwnsAllocations (+0x10, u8)` { `free(objects (+0))`; `free(flags (+4))` } (CRT _free 0x8213AE);
  then `+0 = 0; +4 = 0; +8 = 0; +0xC = 0` (firstFree). Size <= 0: nothing.
- gta-reversed: CPool::Flush (core/Pool.h) - same. Open questions: none.

## 0x49C9E0 CPool<CStuntJump>::New  (catalogue: unknown/REA, port level 0)
- Convention: `__thiscall CStuntJump* (CPool* this)`, plain `ret`, EAX = slot or 0. 0x49C9E0 jumps to the HOODLUM body 0x1566280.
  Callers: CStuntJumpManager::AddOne 0x49CB40, CStuntJumpManager::Load 0x5D5920. Element size 0x44.
- Behaviour: `wrapped = false`; loop: `firstFree = this->+0xC + 1` (stored back); if `firstFree == size` { `+0xC = 0`; if `wrapped` return 0;
  `wrapped = true` } (when wrapping, index 0 is tested without a further increment); if `flags[firstFree] & 0x80` (free) stop, else repeat.
  Found: `flags[i] &= 0x7F`; then `flags[i] = (flags[i] & 0x80) | ((flags[i] + 1) & 0x7F)` (xor/and/xor idiom, bumps the 7-bit id with wrap);
  return `objects + i*0x44`.
- Edge: with `size == 0` it wraps once and reads `flags[0]` (same as the generic CPool::New). gta-reversed: CPool<T>::New (core/Pool.h) - same
  algorithm. Open questions: none.

## 0x552BC0 CPool<CQuadTreeNode>::Delete  (catalogue: unknown/REA, port level 0)
- `__thiscall void (CPool* this, CQuadTreeNode* p [esp+4])`, `ret 4`. Caller: CQuadTreeNode 0x552520 (operator delete).
- Behaviour: `i = (p - objects) / 0x28` (signed magic division `imul 0x66666667; sar edx,4; + sign`); `flags[i] |= 0x80`; if `i < firstFree (+0xC)`
  `firstFree = i`.
- gta-reversed: CPool::Delete - same. Open questions: none.

## 0x59F610 CPool<CObject>::GetAtRef  (catalogue: unknown/REA, port level 0)
Name correction: catalogue CPool<CObject>::GetAtRef -> `CPool<T>::CreateAtRef(int32 handle)` (CPool<CObject> (caller `new` 0x5A1EF0 = CObject::operator new(size, handle))); it claims a slot, it does not look one up.
- Convention: `__thiscall void (CPool* this, int32 handle [esp+4])`, `ret 4`.
- Behaviour: `i = handle >> 8` (sar); `flags[i] &= 0x7F` (mark used); `flags[i] = (flags[i] & 0x80) | (handle & 0x7F)` (xor/and/xor idiom with the
  low byte of handle); `firstFree = 0`; then `while ((i8)flags[firstFree] >= 0) firstFree++` (first test at index 0; no upper bound - scans
  past the end if no slot is free).
- gta-reversed: CPool::CreateAtRef - same. Open questions: none.

## 0x6D5DD0 CPool<T>::GetAtRef (copy)  (catalogue: unknown/REA, port level 0)
Name correction: catalogue CPool<T>::GetAtRef (copy) -> `CPool<T>::CreateAtRef(int32 handle)` (CPool<CVehicle> (caller CVehicle::operator new(size_t,int) 0x6E2D60)); it claims a slot, it does not look one up.
- Convention: `__thiscall void (CPool* this, int32 handle [esp+4])`, `ret 4`.
- Behaviour: `i = handle >> 8` (sar); `flags[i] &= 0x7F` (mark used); `flags[i] = (flags[i] & 0x80) | (handle & 0x7F)` (xor/and/xor idiom with the
  low byte of handle); `firstFree = 0`; then `while ((i8)flags[firstFree] >= 0) firstFree++` (first test at index 0; no upper bound - scans
  past the end if no slot is free).
- gta-reversed: CPool::CreateAtRef - same. Open questions: none.
