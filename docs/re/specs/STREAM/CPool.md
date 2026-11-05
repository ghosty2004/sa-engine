# CPool<T> (STREAM) - full specs, writer A (RE-SPEC-1)

Layout (gta-reversed Core/Pool.h, size 0x14): +0x0 T* m_Storage, +0x4 u8* m_SlotState (one byte per slot:
bits 0-6 = Ref counter, bit 7 = IsEmpty), +0x8 i32 m_Capacity, +0xC i32 m_LastFreeSlot (gr: m_nFirstFree),
+0x10 u8 m_OwnsAllocations, +0x11 u8 m_DealWithNoMemory (m_bIsLocked).

All three constructors below are instances of one template; they differ only in sizeof(T) and in which
allocation entry point is called (0x82119A `operator new` vs 0x821195 `operator new[]`; 0x821195 is a
single `jmp 0x82119A`, so both behave identically, but `orig!` must target the address the original calls).

### Common constructor behaviour `CPool(int32 size, const char* name)`
- Convention: `__thiscall CPool* (CPool* this, int32 size, const char* name)`; size [esp+4], name [esp+8]
  (never read); `ret 8`; returns EAX = this. Preserves ESI, EDI.
- Behaviour, in order:
  1. `m_Storage = alloc(size * sizeof(T));` (32-bit multiply, wraps)
  2. `m_SlotState = alloc(size);`
  3. `m_OwnsAllocations = 1; m_Capacity = size; m_LastFreeSlot = -1;` (byte at +0x11 is NOT written)
  4. If `size > 0` (signed): for i = 0 .. size-1 ascending: `s = m_SlotState[i]; m_SlotState[i] = s | 0x80;`
     then re-read and `m_SlotState[i] = m_SlotState[i] & 0x80;` -> every state byte ends as 0x80
     (IsEmpty = 1, Ref = 0). `m_SlotState` is re-loaded from `this` each iteration.
- Edge cases: allocation results are not null-checked (null -> write fault in step 4). Uses fresh
  heap memory, so the read of the uninitialised byte has no visible effect.
- gta-reversed: Pool.h:63 constructor - same final state, but gr also fills storage with debug patterns
  (`DoFill`, debug only) and leaves m_DealWithNoMemory value-initialised to 0; the original leaves +0x11
  as whatever the containing memory held (the pools are heap-allocated by `new CPool`, i.e. operator new
  at the caller; port: do not write +0x11).

## 0x410F80 CPool<ColDef>::CPool(int32, const char*)  (catalogue: unknown/REA, port level 1; proposed name)
Hint `sub_410f80_from_CColStore__Initialise`. Body in `.HOODLUM` at 0x15611F0 (0x410F80 is a jmp stub).
- sizeof(T) = 0x2C (ColDef, gr ColStore.h), alloc = `orig!(0x821195 operator new[])` for both arrays.
- Caller: CColStore::Initialise (0x4113F0). Behaviour: see common section.

## 0x5B8340 CPool<CVehicleModelInfo::CVehicleStructure>::CPool(int32, const char*)  (catalogue: unknown/REA, port level 1; proposed name)
Hint `sub_5b8340_from_CVehicleModelInfo__SetupCommonData`. sizeof(T) = 0x314 (gr VALIDATE_SIZE
CVehicleStructure 0x314); alloc = 0x821195 for both arrays. Caller: CVehicleModelInfo::SetupCommonData
(0x5B8F00), which stores the pool in `CVehicleStructure::m_pInfoPool` (0xB4E680).
- Behaviour: see common section.

## 0x731AD0 CPool<TxdDef>::CPool(int32, const char*)  (catalogue: unknown/REA, port level 0; proposed name)
Hint `sub_731ad0_from_CTxdStore__Initialise`. sizeof(T) = 0xC (`lea eax,[edi+edi*2]; shl eax,2`);
alloc = `orig!(0x82119A operator new)` for both arrays. Caller: CTxdStore::Initialise (0x731F20).
- Behaviour: see common section.

## 0x731B80 CPool<TxdDef>::New()  (catalogue: unknown/REA, port level 0; proposed name)
Evidence: callers CTxdStore::AddTxdSlot (0x731C80) and CTxdStore::Initialise; body is gr Pool.h
`New()`/`FindFreeSlot` for a 12-byte element; identical code exists for CDummy at 0x532630 (stride 0x38).
- Convention: `__thiscall TxdDef* (CPool* this)`; no args; plain `ret`; return EAX (NULL when full).
  Preserves ESI, EDI, EBX.
- Behaviour:
  1. `size = m_Capacity` (read once, before the loop); `wrapped = false;`
  2. Loop: `i = ++m_LastFreeSlot` (stored back immediately).
     If `i == size`: `m_LastFreeSlot = 0` (stored BEFORE the test); if `wrapped` -> return NULL;
     else `wrapped = true`.
     Then test `m_SlotState[m_LastFreeSlot] & 0x80`; if clear (slot used) -> next iteration.
  3. On a free slot i = m_LastFreeSlot: `m_SlotState[i] &= 0x7F;` then
     `b = m_SlotState[i]; m_SlotState[i] = (b & 0x80) | ((b + 1) & 0x7F);` (Ref incremented modulo 128,
     bit 7 kept - here 0).
  4. Return `m_Storage + i * 0xC`.
- Edge cases: when the pool is full, the function returns NULL with `m_LastFreeSlot == 0` (the second
  wrap stores 0 before returning). Comparison is equality only: if m_LastFreeSlot was >= size on entry the
  loop walks past the end of m_SlotState (not reachable normally). Starting value -1 makes the first search
  begin at slot 0.
- gta-reversed: Pool.h:201 `New()` uses `FindFreeSlot()` and debug logging; the result and state
  changes are the same for valid states when a slot is found. On failure gr stores FindFreeSlot's -1 into
  m_LastFreeSlot, whereas the original leaves 0 (keep-original).
- Open questions: none.
