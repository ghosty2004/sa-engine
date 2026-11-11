# CScriptsForBrains (SCRIPT) — RE-SPEC-REST

## 0x46A8C0 CScriptsForBrains::Init  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this; 70 entries tScriptForBrains stride 0x14: +0 i16 streamedIdx, +2 i8 type, +3 i8 objectGroupingId, +4 u8 active,
  +8 f32 radius, +0xC union {i16 model, u16 chance @+0xE, u32 pad @+0x10} / char name[8]), no args, `ret`. Stub → .HOODLUM 0x156BB70.
  Callers: CTheScripts::Init 0x468D50, CScriptsForBrains ctor 0x46B260.
- Behaviour: for 70 entries (EDX = 0x46): `+0 = -1 (word); +0xC = 0 (word); +0xE = 0 (word); +0x10 = 0 (dword); +2 = -1; +3 = -1; +4 = 1;
  +8 = [0x859D9C]=5.0f`.
- gta-reversed: ScriptsForBrains.cpp:27 (hooked, `.Reversed = false`) assigns a default tScriptForBrains — same values.

## 0x46A900 CScriptsForBrains::SwitchAllObjectBrainsWithThisID(int8 id, bool status)  (catalogue: specified/REA, port level 0)
- `__thiscall`, i8 id [esp+4], u8 status [esp+8], `ret 8`. Stub → .HOODLUM 0x156FA50. Caller: ProcessCommands2600To2699 0x479DA0.
- If `id < 0` (signed byte) → return. For all 70 entries: if `entry.objectGroupingId (+3) == id` → `entry.active (+4) = status`.

## 0x46A930 CScriptsForBrains::AddNewScriptBrain(int16 imgIndex, int16 model, uint16 chance, int8 type, int8 groupingId, float radius)  (catalogue: specified/REA, port level 0)
- `__thiscall`, args [esp+4..+0x18] (words/bytes read from their dword slots), `ret 0x18`. Caller: ProcessCommands2300To2399 0x4762D0.
- Behaviour: find the first entry (u8 index < 70) with `streamedIdx == -1`; none → return (0x46A953). Then `+0 = imgIndex; +0xE = chance;
  +3 = arg5 (groupingId); +0xC = model; +2 = arg4 (type); +4 = 1`; radius: `fcomp [0x858B50]=0.0; test ah, 0x41; jp` → if radius > 0
  (or NaN) `+8 = radius`, else `+8 = [0x859D9C]=5.0f`.
- gta-reversed: install commented out (ScriptsForBrains.cpp:11).

## 0x46A9C0 CScriptsForBrains::AddNewStreamedScriptBrainForCodeUse(int16 imgIndex, const char* name, int8 type)  (catalogue: specified/REA, port level 0)
- `__thiscall`, args [esp+4], [esp+8], [esp+0xC]; `ret 0xC`. Stub → .HOODLUM 0x15628A0. Callers: ProcessCommands2100To2199 0x470A90,
  ProcessCommands2000To2099 0x472310.
- Behaviour: first entry with `streamedIdx == -1` (none → return); `+0 = imgIndex`; `strcpy(entry+0xC, name)` (byte loop including the
  terminator, NO length limit — names over 7 chars overwrite the next entry); `+2 = type; +3 = -1; +4 = 1; +8 = [0x859D9C]=5.0f`.

## 0x46AA80 CScriptsForBrains::RequestAttractorScriptBrainWithThisName(const char* name)  (catalogue: specified/REA, port level 2)
- `__thiscall`, name [esp+4], `ret 4`. Stub → .HOODLUM 0x156D980. Caller: CEntity::CreateEffects 0x533790. Callees:
  GetIndexOfScriptBrainWithThisName 0x46AA30 ((name, type 5) → i16), CStreaming::RequestModel 0x4087E0.
- `i = GetIndex(name, 5)`; if i < 0 return. `id = this[i].streamedIdx`; if `ms_aInfoForModel[id + 0x6676].m_nLoadState` ([0x964E08] +
  id*0x14) != 1: `RequestModel(id + 0x6676, 0x10)` (the 0x10 is computed by a protection idiom at 0x156D9B4: [0x15605A4] - [0x514979]).
  Then, loaded or not: `StreamedScripts` ref-count byte at 0xA47B64 + 32*id `+= 1`.

## 0x46AAE0 CScriptsForBrains::MarkAttractorScriptBrainWithThisNameAsNoLongerNeeded(const char* name)  (catalogue: specified/REA, port level 1)
- `__thiscall`, name [esp+4], `ret 4`. Stub → .HOODLUM 0x156D940. Caller: CEntity::DestroyEffects 0x533BF0.
- `i = GetIndexOfScriptBrainWithThisName(name, 5)` 0x46AA30; if i >= 0 and the ref-count byte at 0xA47B64 + 32*this[i].streamedIdx is
  > 0 (unsigned) → decrement it.

## 0x46B270 CScriptsForBrains::StartNewStreamedScriptBrain(uint8 index, CEntity* entity, bool hasAScriptBrain)  (catalogue: specified/REA, port level 11)
- `__thiscall`, u8 index [esp+4], entity [esp+8], u8 flag [esp+0xC]; `ret 0xC`. Callers: StartOrRequestNewStreamedScriptBrain 0x46CD80,
  CTheScripts::ProcessWaitingForScriptBrainArray 0x46CF00. Callees: CStreamedScripts::StartNewStreamedScript 0x470890 (thiscall on
  CTheScripts::StreamedScripts 0xA47B60, arg i16 streamed index → CRunningScript*), CTheScripts::RemoveFromWaitingForScriptBrainArray
  0x46ABC0 (cdecl (entity, i16)).
- Behaviour: `e = this[index]`; `s = StartNewStreamedScript(e.streamedIdx)` (no NULL check); `s.m_ExternalType (+0xC9) = e.type`.
  Jump table 0x46B36C on `e.type` (> 5 unsigned → return): 0, 3, 5 → ped; 1, 4 → object; 2 → return.
  - Ped (0x46B2AB): `s.localVars[0] (+0x3C) = ped handle` (`index = (ped - pool.objects) / 0x7C4` via magic 0x41EE7CA7 / `sar 9`,
    handle = (index << 8) + flags[index], pool [0xB74490]); `ped.flags (+0x474) |= 0x800000` (bHasAScriptBrain); if `e.type == 5`:
    `s.localVars[1] (+0x40) = (u8)hasAScriptBrain`; then `ped.flags &= ~0x1000000` (bWaitingForScriptBrainToLoad);
    `RemoveFromWaitingForScriptBrainArray(ped, (u16)ped+0x794)`; `ped+0x794 = -1` (u16 0xFFFF).
  - Object (0x46B32A): `s.localVars[0] = object handle` (size 0x19C, magic 0x13E22CBD / `sar 5`, pool [0xB7449C]);
    `object.m_nObjectFlags (+0x140) |= 0x300000`.

## 0x46B3D0 CScriptsForBrains::IsObjectWithinBrainActivationRange(CObject* obj, const CVector& point)  (catalogue: specified/REA, port level 0)
- `__thiscall`, obj [esp+4], point [esp+8]; returns AL; `ret 8`. Caller: CTheScripts::ProcessWaitingForScriptBrainArray 0x46CF00.
- Behaviour: `e = this[(i16)obj+0x168]` (the object's script brain index, sign-extended, no range check). If `e.type != 1` → return 0.
  `p = obj.m_matrix (+0x14) ? matrix.pos : obj.m_placement.pos` (copied to a local). `d = fsqrt(((dz*dz) + (dy*dy)) + (dx*dx))` with
  `dx = point.x - p.x` etc. (0x46B40E-0x46B436). Return 1 if `d < e.radius (+8)` (`fcomp; test ah, 5; jp` → only C0), else 0 (also for
  equality and NaN).
- gta-reversed: `NOTSA_UNREACHABLE()` stub (ScriptsForBrains.cpp:61; gr-deviations row "maybe"): gr has no behaviour; port from bytes.

## 0x46CD80 CScriptsForBrains::StartOrRequestNewStreamedScriptBrain(uint8 index, CEntity* entity, int8 attachType, bool addToWaitingArray)  (catalogue: specified/REA, port level 12)
- `__thiscall` (ECX = this → EDI), u8 index [esp+4] (BL), entity [esp+8] (ESI), i8 attachType [esp+0xC], u8 addToWaiting [esp+0x10];
  `ret 0x10`. Callers: StartOrRequestNewStreamedScriptBrainWithThisName 0x46CED0, ProcessWaitingForScriptBrainArray 0x46CF00,
  CheckIfNewEntityNeedsScript 0x46FF20. Callees: CStreaming::RequestModel 0x4087E0 (cdecl (id, flags)), CTheScripts::
  AddToWaitingForScriptBrainArray 0x46AB60 (cdecl (entity, i16 index)), StartNewStreamedScriptBrain 0x46B270.
- Behaviour:
  1. If addToWaiting (0x46CD91): if `this[index].active (+4) == 0` → return. Jump table 0x46CEB8 on attachType (> 4 unsigned → step 2):
     0, 3 (ped): if `ped.flags (+0x474) & 0x800000` → return; if `& 0x1000000` → return; 1, 4 (object): if `obj+0x140 & 0x300000` →
     return; 2 → step 2.
  2. If `attachType == 1` (0x46CDE6): if addToWaiting: `obj+0x168 = (u16)index`, `obj+0x140 = (flags & ~0x200000) | 0x100000`,
     `AddToWaitingForScriptBrainArray(obj, index)`; else `obj+0x140 = (flags & ~0x100000) | 0x200000`.
  3. `id = this[index].streamedIdx`; if `CStreaming::ms_aInfoForModel[id + 0x6676].m_nLoadState == 1` ([0x964E08] + id*0x14, LOADED) →
     `StartNewStreamedScriptBrain(index, entity, 0)`; return.
  4. Else `RequestModel(id + 0x6676, 4)` (flag 4 = mission-required); then if attachType is 0, or 3, or 4 (`test; je` / `cmp 2; jle` /
     `cmp 4; jg`) and addToWaiting: `ped.flags (+0x474) |= 0x1000000; ped+0x794 = (u16)index; AddToWaitingForScriptBrainArray(entity,
     index)`. (For attachType 4, an object, this writes the PED offsets +0x474/+0x794 into the CObject — original behaviour, keep.)
- gta-reversed: `NOTSA_UNREACHABLE()` (ScriptsForBrains.cpp:50); port from bytes.

## 0x46CED0 CScriptsForBrains::StartOrRequestNewStreamedScriptBrainWithThisName(const char* name, CEntity* entity, int8 attachType)  (catalogue: specified/REA, port level 13)
- `__thiscall`, args [esp+4]/[esp+8]/[esp+0xC], `ret 0xC`. Stub → .HOODLUM 0x1562360. Caller: InteriorGroup_c::SetupHousePeds 0x5965E0.
- `i = GetIndexOfScriptBrainWithThisName(name, attachType)` 0x46AA30; if i >= 0: `StartOrRequestNewStreamedScriptBrain((u8)i, entity,
  attachType, 1)` 0x46CD80.

## 0x46FF20 CScriptsForBrains::CheckIfNewEntityNeedsScript(CEntity* entity, int8 attachType, void* unused)  (catalogue: specified/REA, port level 13)
- `__thiscall` (EDI = this), entity [esp+4] (EBP), i8 attachType [esp+8] (BL), unused [esp+0xC]; `ret 0xC`. Callers:
  CDummyObject::CreateObject 0x59EAC0, CPopulation::AddToPopulation 0x614720, CCarGenerator::DoInternalProcessing 0x6F34D0.
  Callees: `rand` 0x821B1E, `_ftol` 0x821B40, StartOrRequestNewStreamedScriptBrain 0x46CD80.
- Behaviour:
  1. attachType == 0 (ped): if `ped+0x474 & 0x800000` or `& 0x1000000` → return. attachType != 0: if `entity+0x140 & 0x300000` → return.
     (An attachType of 3/4 is tested with the object flags.)
  2. `started = 0`; for i = 0 .. 69 (u8) while !started: if `this[i].type (+2) == attachType` and `this[i].model (+0xC) == entity.
     m_nModelIndex (+0x22)` (16-bit compare): `r = _ftol((float)(rand() & 0xFFFF) * [0x858B14]=3.0517578e-05f * [0x858628]=100.0f)`
     (fild of the masked int, two fmuls, 0x46FF90-0x46FFB2); if `r < this[i].chance (+0xE, u16)`: `started = 1`;
     `StartOrRequestNewStreamedScriptBrain(i, entity, attachType, 1)`.
- gta-reversed: `plugin::CallMethod` (ScriptsForBrains.cpp:33).
