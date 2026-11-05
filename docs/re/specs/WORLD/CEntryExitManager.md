# CEntryExitManager (WORLD) - full specs

Also contains, by assignment: the free function `IsTeleportPointValid` (0x43EA90, used only by
`CEntryExit::WarpGangWithPlayer`) and the unnamed pool allocator 0x43F7D0 (only caller CEntryExitManager::AddOne).

Pool layout used (`CPool<CEntryExit>`, object `CEntryExitManager::mp_poolEntryExits` 0x96A7D8 holds a pointer):
+0x0 `m_pObjects` (CEntryExit*, stride 0x3C), +0x4 `m_byteMap` (u8 per slot: bit 7 = empty, bits 0-6 = id),
+0x8 `m_nSize` (int32), +0xC `m_nFirstFree` (int32). CEntryExit layout: see CEntryExit.md (+0x00 name[8],
+0x30 `m_nFlags` u16, 0x4000 = bEnableAccess).

Most bodies live in `.HOODLUM`: the catalogue address is a 5-byte `jmp` stub; the port hooks the stub address.

## 0x43E400 CEntryExitManager::WeAreInInteriorTransition  (catalogue: specified/REA, port level 0)
- `__cdecl bool()`; body at 0x1560F30. Returns `AL = (ms_exitEnterState (0x96A7CC, int32) != 0)` via `setne al`;
  bits 8-31 of EAX keep the upper bytes of ms_exitEnterState (callers use AL only).
- Callers: CPlayerPed::ProcessControl 0x60EA90. gr: same (`ms_exitEnterState != 0`).

## 0x43EA90 IsTeleportPointValid  (catalogue: specified/REA, port level 7)
- `__cdecl bool (const CVector* origin [esp+4], const CVector* target [esp+8])`, returns AL, plain `ret`.
  Callers: CEntryExit::WarpGangWithPlayer 0x43F1F0 (3 sites). Callees: CWorld::TestSphereAgainstWorld 0x569E20
  (cdecl, CVector by value, 0x2C bytes of args), CWorld::GetIsLineOfSightClear 0x56A490 (cdecl, 9 args).
- Behaviour:
  1. `hit = TestSphereAgainstWorld(*target /*copied by value*/, 0.35f /*imm 0x3EB33333*/, nullptr, true, true, true, true, true, false)`.
  2. if `hit != nullptr` return false.
  3. return `GetIsLineOfSightClear(origin, target, true, true, false, true, true, false, false) ? true : false`
     (the `false` args are the zero EAX from step 1).
- gr: matches (`!TestSphereAgainstWorld(target, 0.35f, nullptr, 1,1,1,1,1,0) && GetIsLineOfSightClear(origin, target, 1,1,0,1,1,0,0)`).

## 0x43EF00 CEntryExitManager::GetEntryExit  (catalogue: specified/REA, port level 0)
- `__cdecl CEntryExit* (int32 index [esp+4])`; body 0x156DA90. Callers: CRunningScript::ProcessCommands2100To2199,
  ProcessCommands2200To2299.
- Behaviour: `p = mp_poolEntryExits` (0x96A7D8); if `(int8)p->m_byteMap[index] < 0` return nullptr; else return
  `p->m_pObjects + index*0x3C` (byte offset). No bounds check (negative/oversized index reads out of range).
- gr: equivalent to `mp_poolEntryExits->GetAt(index)`.

## 0x43EF20 CEntryExitManager::SetEntryExitFlag  (catalogue: specified/REA, port level 0)
- `__cdecl void (const char* name [esp+4], uint32 flag [esp+8], bool enable [esp+0xC])`; body 0x15615C0.
  Callers: SetEnabledByName 0x43F9B0. Callees: `_strnicmp` 0x835AC7 (CRT, cdecl).
- Behaviour:
  1. `n = mp_poolEntryExits->m_nSize` (read once); if `n <= 0` return (signed `jle`).
  2. for `i = 0; i < n; ++i` (ascending; pool pointer re-read from 0x96A7D8 every iteration after the first):
     if `(int8)byteMap[i] >= 0` (slot used): `e = m_pObjects + i*0x3C`; if `_strnicmp(e->m_szName, name, 8) == 0`:
     `enable ? (e->m_nFlags |= (u16)flag) : (e->m_nFlags &= ~(u16)flag)` (16-bit RMW at +0x30, only the low word
     of `flag` is used). Does not stop at the first match.
- gr: not hooked; behaviour as above (gr `SetEntryExitFlagWithIndex` is a different function).

## 0x43F7D0 CEntryExitManager::sub_43F7D0 (proposed: `CPool<CEntryExit>::New`)  (catalogue: unknown/REA, port level 0)
- Evidence: only caller CEntryExitManager::AddOne 0x43FA00; `this` (ECX) layout is exactly CPool (objects, byteMap,
  size, firstFree) with stride 0x3C = sizeof(CEntryExit); behaviour is the standard CPool allocate.
- `__thiscall CEntryExit* (CPool* this = ECX)`, plain `ret`; body 0x15651D0.
- Behaviour (edx = size read once at entry; `wrapped` = AL, starts 0):
  1. loop: `m_nFirstFree += 1` (re-read from memory, stored back);
     if `m_nFirstFree == size` (equality only): `m_nFirstFree = 0`; if `wrapped` return nullptr (firstFree stays 0);
     `wrapped = 1`.
     if `(byteMap[m_nFirstFree] & 0x80) == 0` repeat loop (slot used).
  2. `i = m_nFirstFree`; `byteMap[i] &= 0x7F` (mark used);
     `b = byteMap[i]`; `byteMap[i] = (b & 0x80) | ((b + 1) & 0x7F)` (id incremented modulo 128; computed as
     `b ^ (((b+1) ^ b) & 0x7F)` with 8-bit wrap).
  3. return `m_pObjects + i*0x3C`.
- gr: equals the generic `CPool<T>::New()` template (not hooked at this address).

## 0x43F9B0 CEntryExitManager::SetEnabledByName  (catalogue: specified/REA, port level 1)
- `__cdecl void (const char* name, bool enable)`; body 0x156AE10. Callers: CRunningScript::ProcessCommands2000To2099
  (2 sites). Behaviour: `SetEntryExitFlag(name, 0x4000 /*bEnableAccess*/, enable)` (enable forwarded as the full
  dword from [esp+8]).
- gr: declared, not hooked; same.
