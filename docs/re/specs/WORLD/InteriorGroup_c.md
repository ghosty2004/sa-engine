# InteriorGroup_c - full specs

Source of truth: the 1.0 US disassembly. gta-reversed reverses only Init 0x5947E0, ReferenceAnims and
DereferenceAnims for this class (not in this assignment); every function below is a `plugin::Call` stub
(`.Reversed = false`) or a commented-out ctor/dtor hook, so behaviour is read from the bytes.

RNG notation: `R16 = (f32)(int)(rand() & 0xFFFF) * f32[0x858b14] (1/32768)` with `_rand` 0x821b1e, then
`_ftol` 0x821b40 (truncation). Calls to `CGeneral::GetRandomNumberInRange(int,int)` mean the original helper
0x407180: `min + _ftol(R16 * (int)(max - min))` (cat14 finding 1: gr's helper differs). Raw `rand()`
compares (no masking) are noted where they occur.

## Layout (size 0xBC)

| off | size | type | gr name | meaning |
|---|---|---|---|---|
| 0x00 | 8 | prev/next | ListItem_c | list links |
| 0x08 | 4 | CEntity* | m_pEntity | owning building entity |
| 0x0C | 1 | u8 (read movsx) | m_id | path interior id (StartNewInterior / RemoveInterior arg); pool index set by InteriorManager_c::Init |
| 0x0D | 1 | u8 | m_groupId | effect group id |
| 0x0E | 1 | u8 | m_groupType | 0 house, 1 shop, 2 office, 0xFF none (Setup) |
| 0x0F | 1 | s8 | m_numInteriors | incremented by AddInterior |
| 0x10 | 0x20 | Interior_c*[8] | m_interiors | filled from slot 0 upwards; not cleared by Exit |
| 0x30 | 4 | CEntryExit* | m_EnEx | peds are set up / updated only when non-null |
| 0x34 | 1 | bool | m_isVisible | |
| 0x35 | 1 | bool | m_lastIsVisible | |
| 0x36 | 1 | s8 | m_numPeds | next free m_peds slot (no bound check) |
| 0x38 | 0x40 | CPed*[16] | m_peds | |
| 0x78 | 0x40 | CPed*[16] | m_pedsToRemove | |
| 0xB8 | 1 | s8 | m_pathSetupComplete | |
| 0xB9 | 1 | s8 | m_updatePeds | |
| 0xBA | 1 | s8 | m_animBlockReferenced | |

InteriorManager_c members used here: `+0x0 Interior_c[8]` (stride 0x794), `+0x3CA0 TList_c<Interior_c>`
pool, `+0x3CAC InteriorGroup_c[8]` (stride 0xBC), `+0x428C`/`+0x4298 TList_c<InteriorGroup_c>` (active list,
pool), `+0x472C bool m_InteriorPedsAliveState[16]` (0xBB3D9C), `+0x4740 CRect m_EnExRect`,
`+0x4752 bool m_ArePedsEnabled` (0xBB3DC2); g_interiorMan = 0xBAF670. TList_c: `+0 head, +4 tail, +8 count`;
AddItem 0x4a8df0 inserts at the head.

Also in this file: 0x44df60, an unnamed function whose only caller is InteriorGroup_c::SetupPaths. The
evidence puts it in CPathFind (this = &ThePaths); it is documented here because it is in this assignment
and the CPathFind spec file is not.

## 0x44df60 CPathFind::sub_44df60 (proposed: CPathFind::RemoveInteriorLinks)  (catalogue: unknown/REA, port level 0)
- Attribution: called once, from InteriorGroup_c::SetupPaths 0x596590 with `ECX = &ThePaths` (0x96F050). It reads `CPathFind::NumNodesGiven` and the 6-byte connection rows that CPathFind::AddNodeToNewInterior 0x450e90 / AddInteriorLink 0x44ded0 fill (`ConnectsToGiven` at 0x96EAC0). gr's PathFind.h declares `void RemoveInteriorLinks(uint32 intIdx);` with no address, right after AddInteriorLinkToExternalNode (0x44df30), and 0x44df60 is the next function. Proposed class CPathFind (POP subsystem); documented here because its only caller is SetupPaths.
- Convention / signature: `__thiscall` (ECX = CPathFind*, ignored: ECX is overwritten), `[esp+4] int32 node`; void; `ret 4`.
- Code location: 0x44df60 `jmp 0x1562390` (.HOODLUM). Body 0x1562390..0x15623ea. It contains a no-CD indirection: `pushfd; push eax; push esp; push 0x155ff20; mov eax, 0xfa09415f ^ 0xfac232bf (= 0x00CB73E0); call eax` — 0xCB73E0 calls `[0x1422ea0](0x155ff20, esp)` and returns a pointer in EAX (plus `0x28` if the trap flag is set — anti-debug), which becomes the row base ECX; then `pop eax; popfd`.
- Globals: R `0x96EF80 int32 CPathFind::NumNodesGiven` (compared signed); RW the byte table returned by the indirection (expected `ConnectsToGiven` 0x96EAC0, `int8[64][6]`).
- Behaviour:
  1. `n = NumNodesGiven`; if `n <= 0` return.
  2. `row = <pointer from the indirection>` (once, before the loop).
  3. For `i = 0; i < n; i++, row += 6`: for `k = 0; k < 6; k++`: `if (i == node || (int32)(int8)row[k] == node) row[k] = 0xFF (-1)`.
  Effect: clears every link of node `node` and every link from other nodes to `node`.
- Edge cases: when not under a debugger the pointer is the plain table; a port uses the table directly.
- gta-reversed: absent (only the declaration). Note: gr declares `ConnectsToGiven` as `int8[64][8]` (PathFind.cpp:22), but the original uses a 6-byte row stride here and in AddInteriorLink 0x156ec90 (`lea eax,[ebx+ebx*2]; shl eax,1`) and AddNodeToNewInterior (`lea eax,[ecx+ecx*2]` with 6 byte stores). Out of this spec's scope; flag for the CPathFind spec.
- Open questions: 0x15623bc `call eax` — the returned pointer was not executed/decoded here; that it equals 0x96EAC0 is inferred from the stride, the NumNodesGiven bound and the paired AddInteriorLink code (confirm with one runtime read, e.g. a breakpoint at 0x15623be reading ECX).

## 0x594840 InteriorGroup_c::AddInterior  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), arg `[esp+4] Interior_c* interior`, returns int32 in EAX, `ret 4`.
- Callers: 1 (InteriorManager_c::Update 0x598f50). Callees: none.
- Globals: none.
- Fields: `+0x10 Interior_c* m_interiors[8]`, `+0xF uint8 m_numInteriors`.
- Behaviour: `for (i = 0; i < 8; i++)` ascending over `m_interiors[i]`; at the first slot whose pointer is 0: `m_interiors[i] = interior; m_numInteriors++` (8-bit `inc byte`), return `i`. If no slot is free the loop ends with EAX = 8 and returns 8 without writing anything.
- Edge cases: return value is the slot index (0..7) or 8 when full; callers ignore it. No duplicate check.
- gta-reversed: plugin::Call stub (declared `int32 AddInterior(Interior_c*)`), no code.

## 0x5948c0 InteriorGroup_c::GetRandomInterior  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), no stack args, returns `Interior_c*` in EAX (gr declares int32), plain `ret`.
- Callers: 1 (InteriorGroup_c::SetupShopPeds 0x594c10). Callees: `_rand` 0x821b1e, `_ftol` 0x821b40.
- Globals: read `0x858b14` f32 = 1/32768 (3.0517578125e-05).
- Fields: `+0xF m_numInteriors` (read with `movsx`, i.e. as signed int8), `+0x10 m_interiors[8]`.
- Behaviour:
  1. `n = (int32)(int8)m_numInteriors`.
  2. RNG at 0x5948cf: `r = _rand() & 0xFFFF; k = _ftol( (x87: fild r) * f32[0x858b14] (fmul) * n (fimul int32) )` (0x5948dd..0x5948eb). Inline, no CGeneral helper. `k` is an index into the list of non-null interiors.
  3. `j = 0`; for `i = 0..7` ascending: if `m_interiors[i] != 0`: if `j == k` return `m_interiors[i]`; else `j++`. (Null slots are skipped and do not advance `j`.)
  4. If not found, return 0 (NULL).
- Edge cases: with n = 0, k = 0 and the first non-null slot would be returned if any (cannot happen when the count is consistent). If `m_numInteriors` exceeds the actual number of non-null slots, k may be out of range and NULL is returned.
- gta-reversed: plugin::Call stub, no code.

## 0x594920 InteriorGroup_c::GetNumInteriorInfos  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), `[esp+4] int32 type`, returns int32 count in EAX, `ret 4`.
- Callers: 1 (InteriorGroup_c::UpdateOfficePeds 0x594e90). Callees: none.
- Globals: none.
- Fields: `this+0x10 m_interiors[8]`; Interior_c `+0x40D int8 m_interiorInfosCount`, `+0x590 InteriorInfo_t m_interiorInfos[]` (stride 0x20, `+0` int8 Type).
- Behaviour: `count = 0`; for each of the 8 slots ascending: skip if null; `n = (int8)interior->m_interiorInfosCount` (movsx); if `n <= 0` skip; for each info `k = 0..n-1` (stride 0x20 from +0x590): if `(int32)(int8)info.Type == type` then `count++`. Return `count`. The `IsInUse` byte (+1) is NOT checked.
- gta-reversed: plugin::Call stub, no code.

## 0x594970 InteriorGroup_c::FindInteriorInfo  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), args `[esp+4] int32 type`, `[esp+8] InteriorInfo_t** outInfo`, `[esp+0xC] Interior_c** outInterior`; returns bool in AL; `ret 0xC`.
- Code location: 0x594970 is `nop; jmp 0x4075a5`. 0x4075a5 is a no-CD trampoline: `sub esp, 0x208` (this is the function's real frame allocation), then it computes `[0x1560874] + [0x4556c6] = 0x5e7f85 + 0xfffac9f1 = 0x594976` (mod 2^32) and `ret`s there with EBP and EFLAGS restored. The body is at 0x594976..0x594a41 (not shown in the bundle; disassembled from the image). ECX is preserved through the trampoline.
- Callers: 3 (CTaskInteriorBeInHouse::GetInfoForPedToUse 0x6750a0 x7, CTaskInteriorBeInOffice::GetInfoForPedToUse 0x675300, CTaskInteriorShopKeeper::CreateFirstSubTask 0x676550). Callees: `_rand` 0x821b1e, `_ftol` 0x821b40.
- Globals: read `0x858b14` f32 = 1/32768.
- Locals (frame of 0x208 bytes at entry-0x208): `count` int32 at frame+0, `InteriorInfo_t* infos[64]` at frame+8, `Interior_c* ints[64]` at frame+0x108.
- Fields: `this+0x10 m_interiors[8]`; Interior_c `+0x40D int8 m_interiorInfosCount`, `+0x590 m_interiorInfos[]` (stride 0x20): `+0 int8 Type`, `+1 bool IsInUse`.
- Behaviour:
  1. `count = 0`. For slot `i = 0..7` ascending: skip null; `n = (int8)interior->m_interiorInfosCount`, skip if `n <= 0`; for `k = 0..n-1`: `info = &interior->m_interiorInfos[k]`; if `(int8)info->Type == type` AND `info->IsInUse == 0`: `infos[count] = info; ints[count] = interior; count++`.
  2. If `count <= 0`: `*outInfo = 0; *outInterior = 0;` return 0.
  3. Else RNG at 0x5949d6: `r = _rand() & 0xFFFF; k = _ftol( (fild r) * f32[0x858b14] * count (fimul int32) )`. `*outInfo = infos[k]; *outInterior = ints[k];` return 1.
- Edge cases: the local arrays hold 64 entries with no bound check; 8 interiors x up to 16 infos could exceed 64 (overflowing `infos` into `ints` and `ints` into the saved registers / return address). Callers in practice never see more than 64 matches; a port should keep the 64-entry arrays and may treat overflow as unreachable (record in divergence ledger if a bound check is added).
- gta-reversed: plugin::Call stub, no code.

## 0x594a50 InteriorGroup_c::FindClosestInteriorInfo  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), stack args: `[esp+4] int32 type` (-1 = any), `[esp+8..0x10] CVector point` by value (x,y,z), `[esp+0x14] float radius`, `[esp+0x18] InteriorInfo_t** outInfo`, `[esp+0x1C] Interior_c** outInterior`, `[esp+0x20] float* outDistSq`; returns bool in AL; `ret 0x20`. The radius arg slot is overwritten with the outer loop counter after it has been read.
- Callers: 2 (CTaskInteriorBeInShop::ScanForStuff 0x6754d0, vt_870314::ProcessPed 0x676d30). Callees: Interior_c::IsPtInside 0x5913e0 (`__thiscall`, args `const CVector* pt, CVector bias` with bias pushed as three 0 dwords; returns AL).
- Globals: none (999999.0 is an immediate 0x497423f0).
- Fields: `this+0x10 m_interiors[8]`; Interior_c `+0x40D int8 m_interiorInfosCount`, `+0x590 m_interiorInfos[]` stride 0x20: `+0 int8 Type`, `+1 bool IsInUse`, `+4/+8/+0xC CVector Pos`.
- Behaviour:
  1. `maxSq = radius * radius` (fld; fmul; fstp f32). `best = 999999.0f`. `bestInfo = 0; bestInt = 0`.
  2. For `i = 0..7` ascending over `m_interiors`: skip null; skip if `!interior->IsPtInside(&point, {0,0,0})`; `n = (int8)m_interiorInfosCount`, skip if `n <= 0`; for each info `k = 0..n-1` (stride 0x20):
     - if `type != -1` and `(int8)info.Type != type` skip; if `info.IsInUse != 0` skip.
     - `dx = point.x - Pos.x; dy = point.y - Pos.y; dz = point.z - Pos.z` (each Pos component copied to an f32 temp, then fsub m32), `d = (dz*dz + dy*dy) + dx*dx` (exact x87 order: `dz*dz`, `dy*dy`, add, `dx*dx`, add).
     - `if (d < maxSq && d < best)` (two `fcom m32` with `test ah,5; jp`: NaN and equal fail): `best = d` (fstp f32), `bestInfo = info`, `bestInt = interior`.
  3. If `bestInfo != 0`: `*outInfo = bestInfo; *outInterior = bestInt; *outDistSq = best` (squared distance, f32); return 1. Else return 0 without touching the outputs.
- Edge cases: `maxSq` is never updated; the second compare against `best` does the minimisation. The returned distance is squared despite the radius being linear.
- gta-reversed: plugin::Call stub.

## 0x594b90 InteriorGroup_c::GetPed  (catalogue: specified/REA, port level 0)
- `__thiscall` (this = ECX: InteriorGroup_c*), `[esp+4] int32 idx`; returns `m_peds[idx]` = `*(CPed**)(this + 0x38 + idx*4)` in EAX; `ret 4`. No bounds check.
- Callers: 1 (CTaskInteriorUseInfo::CreateNextSubTask 0x676880). Callees/globals: none.
- gta-reversed: plugin::Call stub.

## 0x594bd0 InteriorGroup_c::GetEntity  (catalogue: specified/REA, port level 0)
- `__thiscall` (this = ECX: InteriorGroup_c*), no args; returns `m_pEntity` (`*(CEntity**)(this+8)`) in EAX; `ret`.
- Callers: 2 (Interior_c::CalcMatrix 0x5914d0, Interior_c::Init 0x593bf0). Callees/globals: none.
- gta-reversed: plugin::Call stub.

## 0x594bf0 InteriorGroup_c::SetupOfficePeds  (catalogue: specified/REA, port level 16)
- `__thiscall` (this = ECX), no args, `ret`. `CStreaming::StreamPedsForInterior(2)` (0x40bbb0, cdecl); `CStreaming::LoadAllRequestedModels(0)` (0x40ea10, cdecl; both args popped together by `add esp, 8`); `m_numPeds (+0x36) = 0`. No peds are created here (UpdateOfficePeds 0x594e90 does that).
- Callers: 1 (SetupPeds 0x596890).
- gta-reversed: plugin::Call stub.

## 0x594c10 InteriorGroup_c::SetupShopPeds  (catalogue: specified/REA, port level 18)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), no args, `ret` (gr: int32; EAX on return is garbage/last value, unused by SetupPeds). SEH frame (handler 0x83d096) only for the two `new` + ctor pairs.
- Callers: 1 (SetupPeds 0x596890). Callees: CStreaming::StreamPedsForInterior 0x40bbb0 (cdecl), CStreaming::LoadAllRequestedModels 0x40ea10 (cdecl), CStreaming::FindMIPedSlotForInterior 0x407fb0 (cdecl, 1 int arg, returns model index), GetRandomInterior 0x5948c0, Interior_c::GetRandomTile 0x591b20 (`thiscall(int status, int* outX, int* outY)`, ret 0xC), Interior_c::GetTileCentre 0x591bd0 (`thiscall(float x, float y, CVector* out)`), CPopulation::AddPed 0x612710 (cdecl `(ePedType, int32 model, const CVector& pos, bool makeWander)`), CPed::SetCharCreatedBy 0x5e47e0, CPedIntelligence::SetPedDecisionMakerType 0x600b50, CTask::operator new 0x61a5a0 (cdecl size), CTaskInteriorShopKeeper ctor 0x675660, CTaskInteriorBeInShop ctor 0x6753e0, CTaskManager::SetTask 0x681af0, `_rand` 0x821b1e (x2), `_ftol` 0x821b40 (x2).
- Globals: R `0x858b14` f32 = 1/32768; `0x859024` f32 = -3.0; `0x866d24` f32 = -7.0; `0x858624` f32 = 1.0; `0xA9B0C8 ms_modelInfoPtrs`.
- Fields: `+0xF m_numInteriors` (movsx), `+0x36 int8 m_numPeds`, `+0x38 CPed* m_peds[16]`. CPedModelInfo `+0x28 ePedType m_nPedType`; CPed `+0x47C CPedIntelligence*`; CPedIntelligence `+4 CTaskManager`.
- Behaviour:
  1. `CStreaming::StreamPedsForInterior(1); CStreaming::LoadAllRequestedModels(0); m_numPeds = 0`.
  2. RNG #1 at 0x594c40: `e = _ftol((float)(rand() & 0xFFFF) * f32[0x858b14] * f32[0x859024](-3.0))` (fild, fmul m32, fmul m32) → e in {0, -1, -2} (truncation toward zero). `count = (2 - e) * (int32)(int8)m_numInteriors + 1` (i.e. 2, 3 or 4 peds per interior, plus one).
  3. If `count <= 0` return. For `i = 0; i < count; i++`:
     a. `slotArg = (i == 0) ? 0 : 1 - _ftol((float)(rand() & 0xFFFF) * f32[0x858b14] * f32[0x866d24](-7.0))` (RNG #2 at 0x594c9c, only for i > 0; gives 1..7). `model = CStreaming::FindMIPedSlotForInterior(slotArg)`; `pedType = ms_modelInfoPtrs[model]->m_nPedType` (+0x28).
     b. `pos = (0,0,0)`; `in = GetRandomInterior()` (consumes one rand); `in->GetRandomTile(3, &tx, &ty)` (consumes rand pairs until a tile with status 3 is found; no NULL check on `in`); `in->GetTileCentre((float)tx, (float)ty, &pos)` (fild int → f32 args); `pos.z = pos.z + 1.0f` (f32 store).
     c. `slot = (int8)m_numPeds` (read before AddPed). `m_peds[slot] = CPopulation::AddPed(pedType, model, pos, false)`. If NULL: continue with the next `i` (the NULL is still stored in `m_peds[slot]`).
     d. `m_numPeds++` (byte inc); `ped->SetCharCreatedBy(2 /*PED_MISSION*/)`; `ped->m_pIntelligence->SetPedDecisionMakerType(7)`.
     e. If `i == 0`: `task = new(0x20) CTaskInteriorShopKeeper(this, 0)` (ctor args: group ptr, 0). Else `task = new(0x34) CTaskInteriorBeInShop(this)`. A failed `new` gives task = NULL (still passed on).
     f. `ped->m_pIntelligence->m_TaskMgr.SetTask(task, 4 /*TASK_PRIMARY_DEFAULT*/, 0)`.
  RNG consumption order per iteration: [type rand if i>0], GetRandomInterior rand, GetRandomTile rand pairs.
- Edge cases: `count` can exceed 16 (m_numInteriors up to 8 → up to 33): `m_peds[slot]` has no bound check, writes past `m_peds[15]` into `m_pedsToRemove`. GetRandomTile loops forever if no tile has status 3.
- gta-reversed: plugin::Call stub.
- Open question: the value 4 for the task slot is taken as `TASK_PRIMARY_DEFAULT` from gr's ePrimaryTasks enum (not re-verified against CTaskManager::SetTask bytes; behaviour is fully defined by passing 4).

## 0x594e30 InteriorGroup_c::RemovePed  (catalogue: specified/REA, port level 4)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), `[esp+4] CPed* ped`; `ret 4`. Return value (gr: int8) is not meaningful: AL = new low byte of `m_numPeds` when found; when not found EAX = the last slot value read (`m_peds[15]`). Callers (SetupHousePeds, UpdatePeds) ignore it.
- Callees: CPopulation::RemovePed 0x610f20 (`__cdecl`, 1 arg, caller pops).
- Fields: `+0x38 CPed* m_peds[16]`, `+0x36 int8 m_numPeds`.
- Behaviour: for `i = 0..15` ascending: if `m_peds[i] != 0 && m_peds[i] == ped`: `CPopulation::RemovePed(ped); m_peds[i] = 0; m_numPeds--` (8-bit); return. Not found: return without changes.
- Edge cases: with `ped == NULL` nothing matches (null slots are skipped).
- gta-reversed: plugin::Call stub.

## 0x594e90 InteriorGroup_c::UpdateOfficePeds  (catalogue: specified/REA, port level 18)
- Convention: `__thiscall` (this = ECX: InteriorGroup_c*), no args, void, `ret`. SEH frame (handler 0x83d0ab) around `new` + ctor.
- Callers: 1 (UpdatePeds 0x596830, tail call). Callees: CPlaceable::AllocateMatrix 0x54f560, CSimpleTransform::UpdateMatrix 0x54f1b0, `operator*(CMatrix const&, CVector const&)` 0x59c890 (cdecl, hidden result ptr first), GetNumInteriorInfos 0x594920, CClock::GetIsTimeInRange 0x52cee0 (cdecl `(uint8 from, uint8 to)`, args pushed as ints), CGeneral::GetRandomNumberInRange (int) 0x407180 (cdecl `(int min, int max)`), `_rand` 0x821b1e, `_ftol` 0x821b40, CStreaming::FindMIPedSlotForInterior 0x407fb0, CPopulation::AddPed 0x612710, CPed::SetCharCreatedBy 0x5e47e0, CPedIntelligence::SetPedDecisionMakerType 0x600b50, CTask::operator new 0x61a5a0, CTaskInteriorBeInOffice ctor 0x675220 (`thiscall(InteriorGroup_c*)`), CTaskManager::SetTask 0x681af0, CPed::IsPointerValid 0x5e4220, CPopulation::RemovePed 0x610f20.
- Globals: R `0x858b14` f32 = 1/32768, `0x859000` f32 = 8.0, `0xA9B0C8 ms_modelInfoPtrs`.
- Fields: `+8 m_pEntity`, `+0x34 m_isVisible`, `+0x35 m_lastIsVisible`, `+0x36 m_numPeds`, `+0x38 m_peds[16]`.
- Behaviour:
  1. If `m_isVisible == 0`:
     - if `m_lastIsVisible != 0` (just became invisible): for `i = 0..15`: if `p = m_peds[i]` non-null: `if (CPed::IsPointerValid(p)) CPopulation::RemovePed(p);` `m_peds[i] = 0`. Then `m_numPeds = 0`. Return.
     - else return.
  2. If `m_isVisible != 0 && m_lastIsVisible != 0`: return.
  3. (just became visible) `pos = *m_pEntity->m_matrix * CVector(0,0,0)` (allocating + updating the entity matrix first if `m_matrix == NULL`) — i.e. the entity's position, computed through the matrix multiply.
  4. `n = GetNumInteriorInfos(6)`. Choose the ped count:
     - if `CClock::GetIsTimeInRange(9, 18)`: `num = CGeneral::GetRandomNumberInRange(n/2, n)` (n/2 = signed division truncating toward zero: `cdq; sub eax,edx; sar eax,1`);
     - else if `GetIsTimeInRange(18, 22)` or (evaluated only if false) `GetIsTimeInRange(6, 9)`: `num = GetRandomNumberInRange(0, n/2)`;
     - else `num = 0` (skips the clamp).
     RNG helper 0x407180 (original formula, NOT gr's): `min + _ftol((float)(rand() & 0xFFFF) * f32[0x858b14] * (max - min))` — note max itself is never returned except via rounding effects, and differs from gr's lerp (cat14 finding 1).
     After a helper call: `if (num >= 16) num = 16`.
  5. If `num <= 0` return. Repeat `num` times:
     a. RNG at 0x594f97: `k = _ftol((float)(rand() & 0xFFFF) * f32[0x858b14] * f32[0x859000](8.0))` (fild, fmul m32, fmul m32) → 0..7. `model = CStreaming::FindMIPedSlotForInterior(k)`. `pedType = ms_modelInfoPtrs[model]->m_nPedType` (+0x28).
     b. `slot = (int8)m_numPeds; m_peds[slot] = CPopulation::AddPed(pedType, model, pos, false)`. If NULL continue.
     c. `m_numPeds++` (byte); `ped->SetCharCreatedBy(2)`; `ped->m_pIntelligence->SetPedDecisionMakerType(7)`; `task = new(0x18) CTaskInteriorBeInOffice(this)` (NULL if allocation failed); `ped->m_pIntelligence->m_TaskMgr.SetTask(task, 4, 0)`.
- Edge cases: all peds are created at the entity's origin position (no tile selection). If `m_numPeds` is nonzero on entry to the spawn path, up to 16 more are written from that slot with no bound check (in practice it is 0: SetupOfficePeds and the invisible path reset it).
- gta-reversed: plugin::Call stub. A port must call the original-formula RNG helper (finding 1), not gr's `CGeneral::GetRandomNumberInRange`.

## 0x595200 InteriorGroup_c::CalcIsVisible  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), no args, `ret`. AL on return is not a meaningful bool (see edge cases); the only caller (Update 0x5968e0) ignores it.
- Callers: 1 (InteriorGroup_c::Update). Callees: Interior_c::IsVisible 0x5929f0 (`__thiscall`, returns AL).
- Fields: `+0x34 bool m_isVisible`, `+0x35 bool m_lastIsVisible`, `+0xF m_numInteriors` (movsx, signed), `+0x10 m_interiors[8]`.
- Behaviour: `m_lastIsVisible = m_isVisible; m_isVisible = 0; n = (int8)m_numInteriors;` for `i = 0; i < n; i++` (n re-read from the field each iteration): `if (m_interiors[i]->IsVisible()) { m_isVisible = 1; return; }`.
- Edge cases: the loop uses the first `m_numInteriors` slots without a NULL check (AddInterior fills slots from 0 upwards, so they are contiguous in practice). EAX/AL at return: IsVisible's result (nonzero when found, 0 when none visible) or the low byte of `m_numInteriors` when `n <= 0`.
- gta-reversed: plugin::Call stub.

## 0x595250 InteriorGroup_c::ContainsInteriorType  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), `[esp+4] int32 type`; returns bool in AL; `ret 4`.
- Code location: 0x595250..0x59525b then `jmp 0x403c80` (a 2-instruction no-CD hop: `lea ebx,[ebx]; jmp 0x595260`), body continues at 0x595260.
- Callers: 1 (InteriorGroup_c::Setup 0x595320, 4 sites). Callees/globals: none.
- Fields: `this+0x10 m_interiors[8]`; Interior_c `+0x14 tEffectInterior* m_box`, whose `+0 uint8 m_type`.
- Behaviour: for `i = 0..7` ascending: if `m_interiors[i] != 0` and `(uint32)m_interiors[i]->m_box->m_type == type` (movzx) return 1. Return 0.
- gta-reversed: plugin::Call stub.

## 0x595290 InteriorGroup_c::Exit  (catalogue: specified/REA, port level 6)
- Convention: `__thiscall` (this = ECX: InteriorGroup_c*), no args; ends with tail `jmp` to InteriorGroup_c::DereferenceAnims 0x595160 (void; gr declares int8).
- Callers: 2 (InteriorManager_c::Exit 0x598010, InteriorManager_c::Update 0x598f50). Callees: Interior_c::Exit 0x592230, InteriorManager_c::ReturnInteriorToPool 0x5984b0 (`this = &g_interiorMan` 0xBAF670, arg Interior_c*), CPed::IsPointerValid 0x5e4220 (thiscall, AL), CPed::IsAlive 0x5e0170 (thiscall, AL), CPopulation::RemovePed 0x610f20 (cdecl), CPathFind::RemoveInterior 0x44e1a0 (`this = &ThePaths` 0x96F050, 1 int arg), DereferenceAnims 0x595160 (tail).
- Globals: W `0xBB3D9C bool g_interiorMan.m_InteriorPedsAliveState[16]` (= 0xBAF670 + 0x472C), indexed by the ped slot.
- Fields: `+0x10 m_interiors[8]`, `+0x38 m_peds[16]`, `+0x36 m_numPeds`, `+0xC uint8 m_id`.
- Behaviour:
  1. For `i = 0..7` ascending: if `m_interiors[i] != 0`: `m_interiors[i]->Exit(); g_interiorMan.ReturnInteriorToPool(m_interiors[i])`. The slot is NOT cleared (m_interiors keeps stale pointers; Init 0x5947e0 clears them on reuse).
  2. For `i = 0..15` ascending over `m_peds`: if `p = m_peds[i]` is non-null:
     - if `!CPed::IsPointerValid(p)`: `m_InteriorPedsAliveState[i] = 0`.
     - else: `if (!p->IsAlive()) m_InteriorPedsAliveState[i] = 0;` then `CPopulation::RemovePed(p)`.
     - `m_peds[i] = 0`.
  3. `m_numPeds = 0`.
  4. `ThePaths.RemoveInterior((int32)(int8)m_id)` (movsx of +0xC).
  5. Tail call `DereferenceAnims()`.
- Edge cases: invalid pointers are not removed (only marked dead). `m_pedsToRemove` (+0x78) is untouched.
- gta-reversed: plugin::Call stub.

## 0x595320 InteriorGroup_c::Setup  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall` (this = ECX: InteriorGroup_c*), no args; ends with a tail `jmp` to InteriorGroup_c::ReferenceAnims 0x5950d0 on every path (void).
- Callers: 1 (InteriorManager_c::Update 0x598f50). Callees: ContainsInteriorType 0x595250 (up to 4 calls), ReferenceAnims 0x5950d0 (tail).
- Fields: `+0xE uint8 m_groupType` (written).
- Behaviour (interior types are tEffectInterior::m_type values):
  1. If `ContainsInteriorType(2)`: `m_groupType = 0` (HOUSE).
  2. Else if `ContainsInteriorType(0)` or (evaluated only if the first is false) `ContainsInteriorType(6)`: `m_groupType = 1` (SHOP).
  3. Else: `m_groupType = ContainsInteriorType(1) ? 2 (OFFICE) : 0xFF` (computed branch-free: `neg al; sbb al,al; and al,3; dec al`).
  4. `ReferenceAnims()` (tail call).
- Edge cases: 0xFF (-1) is a real outcome (none of types 2/0/6/1 present). ReferenceAnims then runs with an unknown group type; check its behaviour for 0xFF when porting 0x5950d0 (gr's NOTSA GetAnimBlockName hits NOTSA_UNREACHABLE for it).
- gta-reversed: plugin::Call stub.

## 0x595380 InteriorGroup_c::ArePathsLoaded  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), no args; returns bool in AL (result of CPathFind::AreNodesLoadedForArea passed through); `ret`. Frame 0x84 bytes.
- Callers: 1 (SetupPaths 0x595590). Callees: CPlaceable::AllocateMatrix 0x54f560, CSimpleTransform::UpdateMatrix 0x54f1b0 (`this = &entity->m_placement (+4)`, arg = matrix ptr), `CMatrix operator*(const CMatrix&, const CVector&)` 0x59c890 (`__cdecl`, args (CVector* result, CMatrix* m, CVector* v), returns result ptr in EAX), CPathFind::AreNodesLoadedForArea 0x44dd10 (`this = &ThePaths`, args `float minX, float maxX, float minY, float maxY`).
- Globals: R `0xA9B0C8 CModelInfo::ms_modelInfoPtrs[]`, `0x96F050 ThePaths`.
- Fields: this `+8 CEntity* m_pEntity`; CEntity `+0x22 int16 m_nModelIndex` (movsx), `+0x14 CMatrix* m_matrix`, `+4 CSimpleTransform`; CBaseModelInfo `+0x14 CColModel*`; CColModel bbox `min` at +0, `max` at +0xC.
- Behaviour:
  1. `bb = ms_modelInfoPtrs[entity->m_nModelIndex]->m_pColModel->m_boundBox`. Build 8 corners in a local array (in this order): `c0 = (min.x,min.y,min.z)`, `c1 = (min.x,max.y,min.z)`, `c2 = (max.x,max.y,min.z)`, `c3 = (max.x,min.y,min.z)`, `c4 = (min.x,min.y,max.z)`, `c5 = (min.x,max.y,max.z)`, `c6 = (max.x,max.y,max.z)`, `c7 = (max.x,min.y,max.z)`.
  2. `lo = (999999.0f, 999999.0f, 999999.0f)`, `hi = (-999999.0f, -999999.0f, -999999.0f)`.
  3. For `k = 0..7`: (inside the loop, every iteration) `e = this->m_pEntity; if (!e->m_matrix) { e->AllocateMatrix(); e->m_placement.UpdateMatrix(e->m_matrix); }`; `c[k] = *e->m_matrix * c[k]` (written back into the array). Then, each with `fcomp m32` (NaN never updates):
     `if (c.x < lo.x) lo.x = c.x; if (c.y < lo.y) lo.y = c.y; if (c.z < lo.z) lo.z = c.z;` (tests `test ah,5; jp`)
     `if (c.x > hi.x) hi.x = c.x; if (c.y > hi.y) hi.y = c.y; if (c.z > hi.z) hi.z = c.z;` (tests `test ah,0x41; jne`).
  4. `return ThePaths.AreNodesLoadedForArea(lo.x, hi.x, lo.y, hi.y)`.
- Edge cases: the z extents are computed but unused. No NULL checks on entity / model info / col model.
- gta-reversed: plugin::Call stub.

## 0x595590 InteriorGroup_c::SetupPaths  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall` (this = ECX: InteriorGroup_c*), no args, `ret`. gr declares `int8`; AL on return is not meaningful (0 from ArePathsLoaded on the early exit, whatever CompleteNewInterior left otherwise). The caller (Update 0x5968e0) only tests `m_pathSetupComplete` afterwards. Frame: `sub esp, 0x250` + 4 saved regs. Below, `L+x` means `[esp+x]` with all four registers pushed (L = entry - 0x260).
- Callers: 1 (Update). Callees (all `this = &ThePaths` 0x96F050 except ArePathsLoaded): ArePathsLoaded 0x595380; CPathFind::StartNewInterior 0x44de80 (`(int32)`); CPathFind::AddNodeToNewInterior 0x450e90 (thiscall, hidden `CNodeAddress*` result pointer pushed last, then `float x, float y, float z, bool dontWander, int8 con0..con5`; callee pops 0x2C; returns the result pointer in EAX); CPathFind::AddInteriorLink 0x44ded0 (`(int32 a, int32 b)`, links both directions); CPathFind::FindNearestExteriorNodeToInteriorNode 0x450f30 (hidden result ptr, `(int32 interiorNode)`); CPathFind::AddInteriorLinkToExternalNode 0x44df30 (`(int32 interiorNode, CNodeAddress ext)` — ext passed as one dword); 0x44df60 (proposed CPathFind::RemoveInteriorLinks, `(int32 node)`, see its own section); CPathFind::CompleteNewInterior 0x452270 (`(CNodeAddress* out = NULL)`).
- Globals: `0x96F050 ThePaths`; R `0x96F854` = `ThePaths.m_pPathNodes[area]` (ThePaths + 0x804), CPathNode stride 0x1C with `int16 pos[3]` at +8/+0xA/+0xC (fixed point, x8); constants f32 `0x858c48` = 0.125, `0x85eed4` = 9.0, `0x858fec` = 1.0e9, `0x858b3c` = 3.0.
- Fields of `this`: `+0xC int8 m_id` (movsx), `+0xE m_groupType`, `+0x10 m_interiors[8]`, `+0xB8 m_pathSetupComplete` (written 1 at the end).
- Fields of Interior_c used (offsets absolute in Interior_c; names are descriptive, see the Interior_c layout):
  - `+0x14 m_box` (tEffectInterior*): `+5 m_door`, `+6 m_lDoorStart`, `+8 m_rDoorStart`, `+0xA m_tDoorStart` (int8, "> -1" means the exit exists).
  - `+0x40C int8 numGotoPts`; goto points: point `k` position (x,y,z) at `+0x414 + 0x10*k` (stride 0x10).
  - Exit records, 4 x 0x20 bytes at `+0x510 + 0x20*e` (e = 0 door, 1 left, 2 top, 3 right — order as processed below): `+0x512 + 0x20e int8 linkA`, `+0x513 + 0x20e int8 linkB` (goto point indices), `+0x514 + 0x20e CVector innerPos`, `+0x524 + 0x20e CVector outerPos`. (Filled by CalcExitPts 0x5924a0.)
  - Written: `+0x3EC CNodeAddress exteriorNode` (dword), `+0x3F0 CNodeAddress m_nodeAddress` (dword), `+0x3F4 CVector exteriorNodePos`, `+0x400 CVector m_position`.
- Behaviour:
  1. `if (!ArePathsLoaded()) return;`
  2. `ThePaths.StartNewInterior((int32)(int8)m_id)`. Locals: `nodeBase = 0` (L+0x18), `node = 0` (EDI, the index the next AddNodeToNewInterior will get), `firstNode[8]` (L+0xC0).
  3. Phase 1 — goto-point ladders. For `i = 0..7`: `in = m_interiors[i]`; if null continue (firstNode[i] left unwritten). `firstNode[i] = nodeBase`. `n = (int8)in->numGotoPts`. If `n > 2` (signed):
     - add node for point 0 (dontWander 0, all con = -1) [index `node`], `node++`; add node for point 1 [index `node`]; `AddInteriorLink(node, nodeBase)`; `node++`.
     - for `k = 2; k < n; k++`: add node for point k [index `node`]; `AddInteriorLink(node, nodeBase + k - 2)`; if `k` is odd (`k % 2 != 0`, signed remainder) `AddInteriorLink(node, nodeBase + k - 1)`; `node++`.
     Always (also when `n <= 2`): `nodeBase += n`.
  4. Phase 2 — exits. `node = nodeBase` (EDI reset to the sum of counts), `prev = node - 1` (EBX; invariant `prev == node - 1`, saved at L+0x24). Dangling list: `cnt = 0` (L+0x20), `dPos[16]` CVector at L+0x1A0, `dNode[16]` int at L+0x160, `dUsed[16]` bool at L+0x34. For `j = 0..7`: `in = m_interiors[j]`; if null continue. `haveMain = 0` (L+0x17). Then four exit blocks in this order — e=0 (test `box->m_door`), e=1 (`m_lDoorStart`), e=2 (`m_tDoorStart`), e=3 (`m_rDoorStart`) — each only if its box byte is `> -1`:
     a. Inner node: `AddNodeToNewInterior(innerPos_e, dontWander 0, con -1 x6)` → `addr` (CNodeAddress, dword). Its index is `node`; remember `inner_e = node` (e0: L+0x30, e1: L+0x4C, e2: L+0x28, e3: L+0x48).
     b. Links to goto points: for `x` in (linkA_e, linkB_e): e=0 tests `(int8)x > -1`; e=1..3 test `x != 0xFF` (byte compare, so -2..-128 pass for e>0): `AddInteriorLink(node, firstNode[j] + (int8)x)`.
     c. Main exit: e=0 always: `in->m_position = innerPos_0; in->m_nodeAddress = addr; haveMain = 1`. e=1, e=2: only if `!haveMain`, then also `haveMain = 1`. e=3: only if `!haveMain`, and haveMain is NOT set (no effect, last block).
     d. Outer node: `node++; prev++`; `AddNodeToNewInterior(outerPos_e, dontWander = (e == 0 ? (m_groupType != 0) : 0), con -1 x6)` [index `node`]; `AddInteriorLink(node, prev)` (outer to inner).
     e. `ext = FindNearestExteriorNodeToInteriorNode(node)`. If `ext.areaId == 0xFFFF`: append to the dangling list: `dPos[cnt] = outerPos_e; dNode[cnt] = node; dUsed[cnt] = 0; cnt++` (no bound check) and skip f.
     f. Else: `in->exteriorNode (+0x3EC) = ext`; `p = &ThePaths.m_pPathNodes[ext.areaId][ext.nodeId]`; `in->exteriorNodePos = ((float)p.x * 0.125f, (float)p.y * 0.125f, (float)p.z * 0.125f)` (fild int16 → fmul m32 → fstp f32, each component). If `m_groupType == 0` (HOUSE): recompute `X,Y,Z` the same way (Z stored to an f32 temp), `dx = X - outer.x` (stored to an f32 temp), `dy = Y - outer.y`, `dz = Z - outer.z`, and the squared length in this exact order — e=0: `(dz*dz + dy*dy) + dx*dx`; e=1,2,3: `(dx*dx + dz*dz) + dy*dy`. If `d <= 9.0f` (`fcomp m32; test ah,0x41; jp`: less or equal passes, NaN fails): `AddInteriorLinkToExternalNode(node, ext)`.
     g. `node++; prev++`.
     After the four blocks, if `in->numGotoPts == 0` (byte test): link the inner exit nodes together: if door: (if left: `Link(inner_0, inner_1)`; if top: `Link(inner_0, inner_2)`; if right: `Link(inner_0, inner_3)`). Then if top: (if left: `Link(inner_2, inner_1)`; if right: `Link(inner_2, inner_3)`). (left-right is never linked directly.)
  5. Phase 3 — pair dangling outer nodes. For `a = 0 .. cnt-1`: if `dUsed[a]` continue. `best = 1.0e9f` (x87, from f32 const), `bi = -1`. For `b = a+1 .. cnt-1` ascending (unrolled by 4 while at least 4 remain, then one by one — same order and results): if `!dUsed[b]`: `dx = dPos[a].x - dPos[b].x; dy = dPos[a].y - dPos[b].y; dz = dPos[a].z - dPos[b].z; d = (dz*dz + dy*dy) + dx*dx`; `t = (f32)d` (fst to L+0x1C); if `d < best` (x87 compare of the unrounded d with best; NaN/equal fail): `best = t` (reloaded from the f32 temp), `bi = b`.
     Then if `best < 3.0f` (fcomp m32 0x858b3c; NaN fails): `AddInteriorLink(dNode[a], dNode[bi]); dUsed[a] = 1; dUsed[bi] = 1`. Else: `ThePaths.RemoveInteriorLinks(dNode[a])` (0x44df60).
  6. `ThePaths.CompleteNewInterior(NULL); m_pathSetupComplete = 1`.
- Edge cases / oddities:
  - Phase 1 adds `n` to `nodeBase` even when `n <= 2` added no nodes; Phase 2 then starts numbering at `nodeBase`, so with any interior having 1 or 2 goto points the function's node indices no longer match the indices AddNodeToNewInterior hands out (links point at wrong nodes). Keep as is.
  - The dangling arrays hold 16 entries with no bound check (up to 32 possible: 8 interiors x 4 exits); `dPos[16]` would reach the return address.
  - Exterior links (step f distance test) exist only for HOUSE groups; other groups record +0x3EC/+0x3F4 but never call AddInteriorLinkToExternalNode, and rely on phase 3.
  - Exit order is door, left, top, right (box bytes +5, +6, +0xA, +8), i.e. not the record order of the box fields.
- gta-reversed: plugin::Call stub, no code.
- Open questions: none affecting behaviour; the role names of the exit records (door/left/top/right) are inferred from which tEffectInterior byte gates each block (+5/+6/+0xA/+8), the records themselves are at +0x510/+0x530/+0x550/+0x570.

## 0x5965e0 InteriorGroup_c::SetupHousePeds  (catalogue: specified/REA, port level 18)
- Convention: `__thiscall` (this = ECX: InteriorGroup_c*), no args, `ret` (gr: int32; EAX on return is not meaningful, SetupPeds ignores it). SEH frame (handler 0x83d0cb).
- Callers: 1 (SetupPeds 0x596890). Callees: CStreaming::StreamPedsForInterior 0x40bbb0, CStreaming::LoadAllRequestedModels 0x40ea10, AllocateMatrix 0x54f560, UpdateMatrix 0x54f1b0, `CMatrix*CVector` 0x59c890, `_rand` 0x821b1e (x4 sites), `_ftol` 0x821b40 (x2), CPopCycle::PickGangToCreateMembersOf 0x60f8d0 (returns ePedType in EAX), CPopulation::ChooseGangOccupation 0x611550 (cdecl `(int gangIndex)`, returns model id or -1), CStreaming::FindMIPedSlotForInterior 0x407fb0, CPopulation::AddPed 0x612710, CScriptsForBrains::StartOrRequestNewStreamedScriptBrainWithThisName 0x46ced0 (`this = &CTheScripts::ScriptsForBrains` 0xA90CF0, args `(const char* name = "house" @0x866d4c, CEntity* ped, uint8 attachType = 3)`), CPed::SetCharCreatedBy 0x5e47e0, SetPedDecisionMakerType 0x600b50, CTask::operator new 0x61a5a0, CTaskInteriorBeInHouse ctor 0x674fc0 (`thiscall(InteriorGroup_c*)`), CTaskManager::SetTask 0x681af0, InteriorGroup_c::RemovePed 0x594e30.
- Globals: R `0x858b14` f32 = 1/32768, `0x858628` f32 = 100.0, `0xC0BC68 CPopCycle::m_pCurrZoneInfo`, `0xA9B0C8 ms_modelInfoPtrs`, `0xBB3D9C g_interiorMan.m_InteriorPedsAliveState[16]` (indexed by the loop counter).
- Fields: `+8 m_pEntity`, `+0x36 m_numPeds`, `+0x38 m_peds[16]`.
- Behaviour:
  1. `StreamPedsForInterior(0); LoadAllRequestedModels(0); m_numPeds = 0`.
  2. `pos = *m_pEntity->m_matrix * CVector(0,0,0)` (allocate/update the matrix first if NULL) = entity position.
  3. RNG A at 0x596665: `a = _ftol((float)(rand() & 0xFFFF) * f32[0x858b14] * f32[0x858628](100.0))`; `count = (a <= 50) ? 2 : 1`. `isGang = 0`. (The masked rand value of RNG A stays in the stack slot `[esp+0x18]`, see edge cases.)
  4. If `CPopCycle::m_pCurrZoneInfo != NULL`:
     - `gang = CPopCycle::PickGangToCreateMembersOf()`. If `gang != 0` and `CPopulation::ChooseGangOccupation(gang - 7) > -1` (signed `jle -1` test):
       - `isGang = 1`. RNG B at 0x5966c7: `b = _ftol((float)(rand() & 0xFFFF) * 1/32768 * 100.0)`. If `b > 15`: `isGang = 0` (count stays from step 3).
       - Else RNG C at 0x5966f5: raw `r = rand()` (NOT masked); `count = (r >= 0x3FFF) ? 4 : 3`.
  5. For `i = 0; i < count; i++`:
     a. Model: if `isGang`: `model = CPopulation::ChooseGangOccupation(gang - 7)`. Else if `count == 1`: raw `r = rand()` (0x59673d, not masked): `model = FindMIPedSlotForInterior(r >= 0x3FFF ? 1 : 0)`. Else: `model = FindMIPedSlotForInterior(i)`.
     b. `pedType = ms_modelInfoPtrs[model]->m_nPedType` (+0x28); `slot = (int8)m_numPeds`; `m_peds[slot] = CPopulation::AddPed(pedType, model, pos, false)`.
     c. If non-null: `m_numPeds++`; `ScriptsForBrains.StartOrRequestNewStreamedScriptBrainWithThisName("house", ped, 3)`; `ped->SetCharCreatedBy(2)`; `ped->m_pIntelligence->SetPedDecisionMakerType(7)`; `task = new(0x18) CTaskInteriorBeInHouse(this)` (NULL on failed alloc); `ped->m_pIntelligence->m_TaskMgr.SetTask(task, 4, 0)`.
     d. Always: `if (g_interiorMan.m_InteriorPedsAliveState[i] == 0) RemovePed(m_peds[slot])` (RemovePed of NULL is a no-op).
- Edge cases: the `gang` value used in the loop is reloaded from the stack slot at `[esp+0x18]`, which holds `gang` only when `m_pCurrZoneInfo != NULL`; otherwise it holds the masked rand of RNG A — but then `isGang == 0` and the value is unused. Gang check: `gang` from PickGangToCreateMembersOf is a ped type (gangs start at 7). The "dead ped" check uses the loop index `i`, not the slot. RNG consumption: A always; B and C only on the gang path; one raw rand per iteration only when `!isGang && count == 1`.
- gta-reversed: plugin::Call stub.

## 0x596830 InteriorGroup_c::UpdatePeds  (catalogue: specified/REA, port level 19)
- Convention: `__thiscall` (this = ECX: InteriorGroup_c*), no args, void; may end in a tail `jmp` to UpdateOfficePeds 0x594e90.
- Callers: 1 (Update 0x5968e0). Callees: CPed::IsPointerValid 0x5e4220, RemovePed 0x594e30, UpdateOfficePeds 0x594e90 (tail).
- Globals: R `0xBB3DC2 bool g_interiorMan.m_ArePedsEnabled` (0xBAF670 + 0x4752).
- Fields: `+0x30 CEntryExit* m_EnEx`, `+0x78 CPed* m_pedsToRemove[16]`, `+0xE m_groupType`.
- Behaviour:
  1. If `m_EnEx == 0` or `!m_ArePedsEnabled`: return.
  2. For `i = 0..15` ascending: if `p = m_pedsToRemove[i]` non-null: `if (CPed::IsPointerValid(p)) RemovePed(p);` then `m_pedsToRemove[i] = 0` (cleared whether valid or not).
  3. If `m_groupType == 2` (OFFICE): tail call `UpdateOfficePeds()`. Other types: return.
- gta-reversed: plugin::Call stub.

## 0x596890 InteriorGroup_c::SetupPeds  (catalogue: specified/REA, port level 19)
- Convention: `__thiscall` (this = ECX: InteriorGroup_c*), no args, void, `ret`.
- Callers: 1 (Update 0x5968e0). Callees: SetupHousePeds 0x5965e0, SetupShopPeds 0x594c10, SetupOfficePeds 0x594bf0.
- Globals: R `0xBB3DC2 g_interiorMan.m_ArePedsEnabled`.
- Fields: `+0x30 m_EnEx`, `+0xE m_groupType`, `+0xB9 int8 m_updatePeds` (written).
- Behaviour: if `m_EnEx == 0` or `!m_ArePedsEnabled` return (m_updatePeds unchanged). Else switch on `m_groupType`: 0 → `SetupHousePeds()`; 1 → `SetupShopPeds()`; 2 → `SetupOfficePeds()`; any other value (e.g. 0xFF) → nothing. In all four cases `m_updatePeds = 1`.
- gta-reversed: plugin::Call stub.

## 0x5968e0 InteriorGroup_c::Update  (catalogue: specified/REA, port level 20)
- Convention: `__thiscall` (this = ECX: InteriorGroup_c*), no args; ends with tail `jmp` to ReferenceAnims 0x5950d0 (void).
- Callers: 1 (InteriorManager_c::Update 0x598f50). Callees: CalcIsVisible 0x595200, SetupPaths 0x595590, SetupPeds 0x596890, UpdatePeds 0x596830, ReferenceAnims 0x5950d0.
- Fields: `+0xB8 int8 m_pathSetupComplete`, `+0xB9 int8 m_updatePeds` (both read as bytes, nonzero = true).
- Behaviour:
  1. `CalcIsVisible()`.
  2. If `!m_pathSetupComplete`: `SetupPaths()`; if still `!m_pathSetupComplete` go to step 4.
  3. (paths complete) If `!m_updatePeds`: `SetupPeds()`.
  4. If `m_updatePeds`: `UpdatePeds()`.
  5. Tail call `ReferenceAnims()`.
  Note: in step 4 reached from step 2 (paths not complete) UpdatePeds still runs if m_updatePeds was already set.
- gta-reversed: plugin::Call stub.

## 0x597fe0 InteriorGroup_c::Constructor  (catalogue: specified/REA, port level 1)
- `__thiscall` (this = ECX), no args; calls ListItem_c::ListItem_c 0x4a8db0 (sets `+0 m_pPrev = 0`, `+4 m_pNext = 0`; its body hops through 0x401c62 for the first store); returns `this` in EAX; `ret`. No other field is initialised.
- Callers: InteriorManager_c ctor 0x5984c0 (via eh_vector_constructor_iterator, function pointer).
- gta-reversed: commented-out hook; `= default`.

## 0x597ff0 InteriorGroup_c::Destructor  (catalogue: specified/REA, port level 1)
- `__thiscall` (this = ECX); `jmp 0x4a8dc0` (ListItem_c destructor, which is a bare `ret`). No effect.
- Callers: InteriorManager_c ctor (EH unwind) / dtor via eh_vector_destructor_iterator (function pointer).
- gta-reversed: commented-out hook; `= default`.
