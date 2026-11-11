# _unattributed-C1 (SCRIPT) — RE-SPEC-REST

## 0x40C1C0 sub_40c1c0 (proposed: `CStreaming::RemoveStreamedScript(int32 scmIndex)`)  (catalogue: unknown/REA, port level 9)
- `__cdecl`, i32 at [esp+4]: adds 0x6676 (first streamed-script resource id) to the argument slot and tail-jumps to
  CStreaming::RemoveModel 0x4089A0. Caller: ProcessCommands2300To2399 0x4762D0.

## 0x43E090 sub_43e090 (proposed: `CEntryExit::GetEntranceCentre(CVector* out)` — the +8..+0x18 layout matches CEntryExit's entrance CRect + z; UNCLEAR)  (catalogue: unknown/REA, port level 0)
- `__thiscall` (ECX = this), out [esp+4], `ret 4`. Caller: ProcessCommands2300To2399 0x4762D0.
- `out.x = (this[+0x10] + this[+8]) * [0x858B8C]=0.5f`; `out.y = (this[+0x14] + this[+0xC]) * 0.5f`; `out.z = this[+0x18]` (raw dword copy).

## 0x463490 sub_463490 (class UNCLEAR: byte field +0x2A of the object passed in ECX)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, u8 at [esp+4], `ret 4`. Stub → .HOODLUM 0x1565620. Caller: ProcessCommands100To199 0x466DE0.
- `if (this[+0x2A] != 0x39 && this[+0x2A] != 0x3A) this[+0x2A] = arg;` (values 57/58 are protected from being overwritten).

## 0x463640 sub_463640 = `CTaskSimpleFinishBrain::CTaskSimpleFinishBrain()`  (catalogue: unknown/REA, port level 1)
- `__thiscall`, returns EAX = this. Stub → .HOODLUM 0x156F250: CTaskSimple base ctor 0x61A390 (uncatalogued name), then
  `this->vtable = 0x859D20` (gr TaskSimpleFinishBrain.cpp:5 names vtable 0x859D20). Caller: ShutdownThisScript 0x465AA0 (size 8 alloc).

## 0x46A2F0 sub_46a2f0 (proposed: entity flag query `(m_nFlags & 0x280) != 0`, i.e. bIsVisible (bit 7) || bit 9; name UNCLEAR)  (catalogue: unknown/REA, port level 0)
- `__thiscall` (ECX = CEntity*), no args, returns AL; `ret`. Stub → .HOODLUM 0x1569D20. Callers: ProcessCommands1800To1899 0x46D050,
  ProcessCommands800To899 0x481300.
- `f = this[+0x1C]`; return 1 if `f & 0x200` (`test ah, 2`) or `f & 0x80` (`test al, al; jns`), else 0.

## 0x46A340 sub_46a340 (proposed: `Set2Names(const char* a, const char* b)` on a struct with name fields at +0 and +0x18; class UNCLEAR)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, a [esp+4], b [esp+8], `ret 8`. Stub → .HOODLUM 0x156E1D0. Caller: ProcessCommands1800To1899 0x46D050.
- `strcpy(this, a); strcpy(this + 0x18, b)` (byte loops incl. terminator, no bounds).

## 0x46A630 CTaskComplexClimb (= `CTaskComplexClimb::CTaskComplexClimb()`)  (catalogue: specified/REA, port level 2)
- `__thiscall`, returns EAX = this. Stub → .HOODLUM 0x156B420: `CTaskComplexJump::CTaskComplexJump(this, 1)` 0x67A030 (thiscall, arg 1),
  then `this->vtable = 0x859DCC`. Caller: ProcessCommands1900To1999 0x46B460.

## 0x46A7E0 sub_46a7e0 (proposed: `CTheScripts::RemoveFromSuppressedVehicleModels(int32 model)`)  (catalogue: unknown/REA, port level 0)
- `__cdecl`, model [esp+4], `ret`. Stub → .HOODLUM 0x15624A0. Caller: ProcessCommands1800To1899 0x46D050.
- For all 40 (0x28) i32 entries of CTheScripts::SuppressedVehicleModels (0xA44940): if entry == model → entry = -1 (no early exit).

## 0x46AE80 sub_46ae80 (proposed: `AddEntry(const CVector* pos, const NamePair* names)` on an 8-slot table; class UNCLEAR)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, pos [esp+4], names [esp+8], `ret 8`; returns AL. Stub → .HOODLUM 0x15622E0. Caller: ProcessCommands1800To1899 0x46D050.
- Layout implied: i32 count at +0; 8 name records of 0x28 bytes at +4 (string at +0 and at +0x18 of each record); 8 CVectors at +0x144.
- `n = this[+0]`; if n >= 8 → return 0. `pos[n] (this + 0x144 + 12n) = *pos` (3 dwords); `strcpy(this + 4 + 40n, names)`;
  `strcpy(this + 4 + 40n + 0x18, names + 0x18)` (unbounded); `this[+0] = n + 1`; return 1. (0x46A340 fills the same two-name shape.)

## 0x46B1A0 sub_46b1a0 (proposed: `CTheScripts::AddToSuppressedVehicleModels(int32 model)`)  (catalogue: unknown/REA, port level 0)
- `__cdecl`, model [esp+4], `ret`. Caller: ProcessCommands1800To1899 0x46D050.
- If any of the 40 entries of CTheScripts::SuppressedVehicleModels (0xA44940) equals model → return. Else find the first entry == -1
  (u8 index, stops at 40) and store model there. ORIGINAL BUG (keep): when no -1 slot exists the index is 40 and the model is written to
  0xA449E0 (one past the array).

## 0x470070 sub_470070 (proposed: CPed flag setter for bit 22 of the first ped-flags dword at +0x46C; gr flag name not verified)  (catalogue: unknown/REA, port level 0)
- `__thiscall` (ECX = CPed*), u8 at [esp+4], `ret 4`. Stub → .HOODLUM 0x1561090. Caller: ProcessCommands2000To2099 0x472310.
- `ped[+0x46C] = (ped[+0x46C] & ~0x400000) | ((arg << 22) & 0x400000)` (xor-mask idiom: only bit 0 of the argument byte is used).

## 0x4700E0 sub_4700e0 (proposed: `CStreaming::SetMissionDoesntRequireStreamedScript(int32 scmIndex)` / gr-style `SetMissionDoesntRequireModel(SCMToModelId(i))`)  (catalogue: unknown/REA, port level 10)
- `__cdecl`, i32 at [esp+4]; adds 0x6676 (26230, first streamed-script resource id) to the argument in place and tail-jumps to
  CStreaming::SetMissionDoesntRequireModel 0x409C90. Caller: ProcessCommands2300To2399 0x4762D0.

## 0x47C0E0 sub_47c0e0 (proposed: `CCarGenerator::SwitchOn(uint16 count)` overload)  (catalogue: unknown/REA, port level 1)
- `__thiscall` (ECX = CCarGenerator* → ESI), u16 at [esp+4], `ret 4`. Stub → .HOODLUM 0x15689B0. Caller: ProcessCommands300To399 0x47C100.
- `CCarGenerator::SwitchOn(this)` 0x6F32C0, then `this[+0x1A] (u16) = arg`.

## 0x47E070 sub_47e070 (proposed: `CCutsceneMgr::SetCutsceneOffset(const CVector& offset)`)  (catalogue: unknown/REA, port level 0)
- `__cdecl`, `const CVector*` at [esp+4], `ret`. Stub → .HOODLUM 0x15688D0. Caller: ProcessCommands500To599 0x47E090.
- Copies x, y, z (raw dwords) to CCutsceneMgr::ms_cutsceneOffset 0xBC4034 / 0xBC4038 / 0xBC403C.

## 0x486AE0 sub_486ae0 (proposed: compiled-out debug helper; no-op)  (catalogue: unknown/REA, port level 0)
- Body is a single `ret` (plain, no stack cleanup: the caller ProcessCommands700To799 0x47FA30 must clean its own arguments, i.e. cdecl).

## 0x492CB0 sub_492cb0 (proposed: CScripted2dEffects user-list `Add(int32 value)` on a 0x24-byte ms_userLists entry)  (catalogue: unknown/REA, port level 0)
- `__thiscall`, value [esp+4], `ret 4`. Stub → .HOODLUM 0x156E990. Caller: ProcessCommands1600To1699 0x493FE0.
- `this[+0x20] (u8) = 1`; first i in 0..3 with `this[i] (i32) == -1`: `this[i] = -2; this[+0x10 + 4i] = value`; only one slot is taken
  (the flag stops the loop); none free → nothing else. Layout matches the userLists entries initialised by CScripted2dEffects::Init 0x6FA6F0.

## 0x571970 sub_571970 (proposed: compiled-out query returning false)  (catalogue: unknown/REA, port level 0)
- `xor al, al; ret 0xC`: returns AL = 0 and pops 12 bytes (3 stack args; callee-cleaned). Caller: ProcessCommands1800To1899 0x46D050.

## 0x571980 sub_571980 (proposed: compiled-out query returning false)  (catalogue: unknown/REA, port level 0)
- Identical to 0x571970: `xor al, al; ret 0xC`. Caller: ProcessCommands1800To1899 0x46D050.

## 0x6070F0 sub_6070f0 (proposed: `CDecisionMakerTypes::AddDecisionMaker(int32 index, int32 type)` overload)  (catalogue: unknown/REA, port level 3)
- `__thiscall` (ECX = CDecisionMakerTypes*), index [esp+4], type [esp+8]; tail-jumps to CDecisionMakerTypes::AddDecisionMaker 0x607050
  (thiscall (CDecisionMaker*, int32 type)) after replacing argument 1 in place. Caller: ProcessCommands2000To2099 0x472310.
- `index == -1`: pointer = `this + 0xCB50` when type == 0, else `this + 0xE824` (the two default decision makers); otherwise
  pointer = `this + 4 + index*0x99C`. Type is passed through unchanged. No range check on index.

## 0x6348A0 CTaskComplexUseMobilePhone (= `CTaskComplexUseMobilePhone::CTaskComplexUseMobilePhone(int32 duration)`)  (catalogue: specified/REA, port level 1)
- `__thiscall`, i32 [esp+4], `ret 4`, returns EAX = this. Caller: ProcessCommands1800To1899 0x46D050.
- CTaskComplex base ctor 0x61A3B0; `this[+0xC] = arg; vtable = 0x86E454; this[+0x10] = 0; this[+0x14] = 0; bytes +0x18, +0x19, +0x1C,
  +0x1D = 0`.

## 0x659410 sub_659410 (proposed: ctor of the CTaskComplex subclass with vtable 0x86F59C, one i32/pointer argument)  (catalogue: unknown/REA, port level 1)
- `__thiscall`, arg [esp+4], `ret 4`, returns EAX = this. Caller: ProcessCommands1200To1299 0x48B590.
- CTaskComplex base ctor 0x61A3B0; `this[+0xC] = arg; this[+0x10] = 0; this[+0x18] = 0; vtable = 0x86F59C; this[+0x14] &= 0xFE` (only
  bit 0 cleared; the other bits of that byte keep uninitialised heap contents — original). Class: the vtable owner (helper B maps vtable
  0x86F544 to PresentIDToCop; 0x86F59C is not identified here — UNCLEAR).

## 0x45AD40 sub_45ad40 (proposed: `CRemote::GivePlayerRemoteControlOfExistingCar(CVehicle* v)`; sibling of CRemote::GivePlayerRemoteControlledCar 0x45AB10)  (catalogue: unknown/REA, port level 3)
- `__cdecl`, v [esp+4], plain `ret`. Stub → .HOODLUM 0x156E620. Caller: ProcessCommands1800To1899 0x46D050. Callees as in 0x45AB10
  (FindPlayerVehicle 0x56E0D0, CleanUpOldReference 0x571A00, RegisterReference 0x571B70, CCamera::TakeControl 0x50C7C0,
  SetZoomValueCamStringScript 0x50C1B0).
- Behaviour (P = Players[PlayerInFocus [0xB7CD74]], remote slot at 0xB7CE48 + 0x190*idx):
  1. `switched = 0`; if `P.m_pRemoteVehicle`: its `+0x36 = (s & 7) | 0x18` (status 3); `switched = (P.m_pRemoteVehicle != v)`.
  2. `v+0x36 = (s & 7) | 0x40` (status 8, remote); `v+0x428 |= 8`; bytes `+0x3BA, +0x3BB, +0x3B9 = 0`; `+0x3C8 = 9.0f` (immediate
     0x41100000); `+0x3D0 = 9`; `+0x3B7, +0x3B8 = 0`; bit 0x10 of +0x428 = !(v+0x42D & 2). (No CWorld::Add, no JoinCarWithRoadSystem.)
  3. If `FindPlayerVehicle(-1, false)`: its `+0x36 = (s & 7) | 0x48` (status 9).
  4. If `P.m_pRemoteVehicle`: `P.m_pRemoteVehicle->CleanUpOldReference(&slot)`. Then `slot = v; v->RegisterReference(&slot)`.
  5. `TheCamera (0xB6F028).TakeControl(v, 0x12, style, 1)` with `style = 2` when switched, otherwise a value produced by the protection
     dispatcher at 0x156E742 (`call 0xCB73E0` with a context pointer, result swapped into the pushed slot). OPEN QUESTION (0x156E742):
     the non-switched style value was not resolved statically. Then `TheCamera.SetZoomValueCamStringScript(1)`.

## 0x6002F0 sub_6002f0 (proposed: `FindNearestPedAttractor(const CVector* pos, f32 radius, int32 modelId, int32 attractorType, const char* scriptName, bool needEmptySlot, const C2dEffect* exclude, C2dEffect** outEffect, CEntity** outEntity)`)  (catalogue: unknown/REA, port level 4)
- `__cdecl`, 9 args [esp+4..+0x24] (seen at +0x1020.. after the 0x101C-byte frame allocated with `__chkstk` 0x822070), plain `ret`;
  returns AL = found. Callers: ProcessCommands2200To2299 0x474900, ProcessCommands2300To2399 0x4762D0. Callees: CWorld::FindObjectsInRange
  0x564A20, CBaseModelInfo::Get2dEffect 0x4C4C70, `_stricmp` 0x8229B6, CPedAttractorManager::GetPedAttractorManager 0x5EE190 /
  HasEmptySlot 0x5EBB00 (thiscall (fx, entity)), CEntity::TransformFromObjectSpace 0x5334F0 (thiscall (CVector* out, const CVector& offset)).
- Behaviour:
  1. `*outEffect = NULL; *outEntity = NULL; found = 0; best = 100000000.0f` (immediate 0x4CBEBC20).
  2. `FindObjectsInRange(*pos, radius, false, &count, 0x400, list, buildings=1, vehicles=0, peds=0, objects=1, dummies=0)` (local list of
     1024 entities). If count <= 0 → return 0.
  3. For each entity e in the list: if `modelId != -1 && modelId != e.m_nModelIndex (+0x22, i16)` → next. `mi = ms_modelInfoPtrs[model]`
     (0xA9B0C8); for j < `mi.m_n2dfxCount (+0xD, u8)`: `fx = mi->Get2dEffect(j)`; skip if `exclude && fx == exclude`; require
     `fx.type (+0xC) == 3` (ped attractor), `fx.attractorType (+0x34, u8) == attractorType`, `scriptName == NULL || _stricmp(scriptName,
     fx + 0x38) == 0`, and if needEmptySlot: `GetPedAttractorManager()->HasEmptySlot(fx, e)`. Then `w = e->TransformFromObjectSpace(fx->pos)`;
     `d2 = ((dz*dz) + (dy*dy)) + (dx*dx)` with `d = *pos - w`; if `d2 < best` (C0 only; NaN/equal skip): `best = d2; *outEffect = fx;
     *outEntity = e; found = 1`.
  4. Return found. The nearest candidate wins; ties keep the first one found.
