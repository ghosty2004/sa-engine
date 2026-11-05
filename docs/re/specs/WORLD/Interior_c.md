# Interior_c (procedural interior room) - full specs

Source of truth: the 1.0 US disassembly (bundles of RE-SPEC-1). gta-reversed has only `plugin::Call` stubs
for every function in this file (`.Reversed = false`; ctor/dtor hooks commented out), so all behaviour
below is read from the bytes. gr's header gives a size of 0x794 and a few field names; most of the
middle of the object (0x3EC..0x58F) is unnamed in gr and is described here.

## Conventions used in this file

- `R16` = `(f32)(int)(rand() & 0xFFFF) * f32[0x858b14]` with `rand` = MSVC CRT `_rand` 0x821b1e (LCG
  `s = s*0x343FD + 0x269EC3; return (s >> 16) & 0x7FFF`, per-thread state) and f32[0x858b14] =
  3.0517578125e-05 (1/32768). Emitted as `fild m32; fmul m32 [0x858b14]`. The scale that follows is either
  `fimul m32 <int>` (written `R16 * (int)n`) or `fmul m32 <f32 constant>` (written `R16 * 4.0f`). The result
  goes through `_ftol` 0x821b40 (truncation toward zero; low dword used). This is NOT
  `CGeneral::GetRandomNumberInRange`; reproduce the inline form exactly.
- Where `CGeneral::GetRandomNumberInRange(int min, int max)` is called, it is the ORIGINAL helper 0x407180
  (`cdecl`): `min + _ftol((f32)(rand() & 0xFFFF) * f32[0x858b14] * (int)(max - min))` (one rand(); range
  min .. max-1). gr's helper uses a different formula (docs/re/cat14-gr-deviations.md finding 1); the port
  must call the original formula.
- `srand` = 0x821b11 (CRT). Interior_c::Init reseeds the CRT generator before furnishing, so furniture
  layouts are deterministic per interior, and the global rand() stream is changed by every Init.
- Tile grid: `u8 tiles[30][30]` at `this+0x68`, element (x, y) at `this + 0x68 + x*30 + y`. x runs
  0..width-1 (m_box +2), y runs 0..depth-1 (m_box +3). All tile bound checks are signed int compares
  against the zero-extended u8 width/depth.
- Tile status values seen in this class: 0 empty; 1 returned for out-of-range coordinates; 2 occupied
  / reserved; 3 walkway ring; 4 walkway with a goto point; 5 furniture (floor); 6 tall furniture; 7 door
  approach; 8 door strip; 9 window strip (furniture allowed in front only with canPlaceInFrontOfWindow);
  10 window tile covered by furniture (9 + status 5); 11 no-go rectangle. (Names are descriptive; the
  numbers are what the code compares.)
- "x87" lines list operations in the exact instruction order. Precision control is 24-bit in-game.

## Layout (size 0x794)

| off | size | type | meaning | gr name |
|---|---|---|---|---|
| 0x000 | 8 | prev/next | ListItem_c links (pool list in InteriorManager_c +0x3CA0) | m_pPrev/m_pNext |
| 0x008 | 4 | int32 | interior id (InteriorManager_c::Update: `(u32)(x*y*z of entity pos) + fxId`) | m_interiorId |
| 0x00C | 4 | InteriorGroup_c* | owning group | m_pGroup |
| 0x010 | 4 | int32 | area code; low byte copied to entity +0x2F | m_areaCode |
| 0x014 | 4 | tEffectInterior* | room description = C2dEffect + 0x10: +0 u8 type (0 shop, 1 office, 2 lounge, 3 bedroom, 4 kitchen; 6 accepted by GetBoundingBox; 0x63 disables srand), +1 group, +2 width, +3 depth, +4 height, +5 s8 door (bottom wall x), +6/+7 lDoor start/end, +8/+9 rDoor, +0xA/+0xB tDoor, +0xC/+0xD lWindow, +0xE/+0xF rWindow, +0x10/+0x11 tWindow (all s8, 0xFF = none), +0x12 noGoLeft[3], +0x15 noGoBottom[3], +0x18 noGoWidth[3], +0x1B noGoDepth[3], +0x1E u8 seed, +0x1F u8 status (wealth rating for furniture choice), +0x20 f32 rot (degrees) | m_box |
| 0x018 | 0x40 | RwMatrix | box-local to world (right +0x18, flags +0x24, up +0x28, at +0x38, pos +0x48) | m_matrix |
| 0x058 | 4 | int32 | not used by these functions | field_58 |
| 0x05C | 0xC | TList_c<FurnitureItem> | placed furniture (head +0x5C, tail +0x60, count +0x64); items come from g_furnitureMan.m_FurnitureList (0xBAD3EC); item: +8 CEntity*, +0xC u16 tileX, +0xE u16 tileY | m_list |
| 0x068 | 900 | u8[30][30] | tile grid | field_68 |
| 0x3EC | 4 | CNodeAddress | exterior path node of the last exit that found one (ctor writes 0xFFFF to the u16 at +0x3EC) | field_3EC/3EE |
| 0x3F0 | 4 | CNodeAddress | path node of the main exit's inner node (ctor writes 0xFFFF to the u16 at +0x3F0) | m_nodeAddress |
| 0x3F4 | 0xC | CVector | exterior node position | field_3F4.. |
| 0x400 | 0xC | CVector | main exit inner position (IsVisible distance test) | m_position |
| 0x40C | 1 | s8 | number of goto points (max 16) | field_40C |
| 0x40D | 1 | s8 | number of interior infos (max 16) | m_interiorInfosCount |
| 0x410 | 0x100 | 16 x {u8 tileX, u8 tileY, pad[2], CVector pos} | goto points (AddGotoPt) | field_412.. (gr misaligned) |
| 0x510 | 0x80 | 4 x {u8 ?, u8 ?, s8 a, s8 b, CVector p0 (+4), pad (+0x10), CVector p1 (+0x14)} | exit records: bottom door, left, top, right (CalcExitPts) | gap436 |
| 0x590 | 0x200 | InteriorInfo_t[16] | {s8 Type, bool IsInUse, pad, CVector Pos (+4), CVector Dir (+0x10), CEntity* (+0x1C)} | m_interiorInfos |
| 0x790 | 1 | s8 | shop furniture group (FurnishShop) | m_furnitureGroupId |
| 0x791 | 1 | s8 | style / furniture id chosen by GetRandomId (house rooms; office desk) | m_furnitureId |
| 0x792 | 1 | s8 | office chair furniture id | field_792 |
| 0x793 | 1 | - | not seen | field_793 |

Related statics: 0xbb3dc8 u8 (Office_PlaceEdgeFillers once-flag, never reset), 0x8d0948 int32 (shop
shelf-info spacing counter, initial 2, never reset), 0xbb3de4 int32 (read by Shop_PlaceEdgeUnits, never
written by the exe).

Callee conventions used across the file (each specified below): PlaceFurniture 0x592aa0
`(Furniture_c*, x, y, f32 z, mode, rot, int* outW, int* outD, u8 skipTileCheck)` -> entity or 0;
PlaceFurnitureOnWall 0x593120 (14 args); PlaceFurnitureInCorner 0x593340 (12 args); PlaceObject 0x5934e0;
FurnitureManager_c::GetFurniture 0x5911e0 `(group, sub, i16 id, u8 wealth)` and GetRandomId 0x591220
`(group, sub, u8 wealth)` on g_furnitureMan 0xBAB380 (outside this file).

## 0x591360 Interior_c::Destructor  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall` (this = ECX), void, plain `ret`. SEH frame (handler 0x83cf48).
- Callers: fnref from InteriorManager_c ctor 0x5984c0 (eh vector ctor unwind) and dtor 0x598580 (eh vector destructor). Callees: TList_c dtor 0x4a8de0 on `this+0x5C` (empty `ret`), ListItem_c dtor 0x4a8dc0 (empty `ret`).
- Behaviour: no observable effect (both called destructors are empty). Port as a no-op.
- gta-reversed: hook commented out (`= default`). Open questions: none.

## 0x5913b0 Interior_c::GetFurnitureEntity  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), arg `[esp+4]` CEntity* entity; returns EAX FurnitureEntity_c* (or null); `ret 4`.
- Callers: 1 (0x598090 InteriorManager_c::GetBoundingBox). Callees: none. Globals: none.
- Fields: `this+0x5C` TList_c<FurnitureEntity_c>::head. Item layout: +0 prev, +4 next, +8 CEntity* entity, +0xC u16 tileX, +0xE u16 tileY.
- Behaviour: `item = this->list.head; while (item) { if (item->entity == entity) return item; item = item->next; } return null;` (head to tail, pointer equality only).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x5913e0 Interior_c::IsPtInside  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*); `[esp+4]` const CVector* pt; `[esp+8..0x10]` CVector bias passed BY VALUE (3 f32: bx, by, bz); returns AL bool (1 inside / 0 outside); `ret 0x10`.
- Callers: 4 (Interior_c::IsVisible 0x5929f0, InteriorGroup_c::FindClosestInteriorInfo 0x594a50, InteriorManager_c::GetVectorsInterior 0x5983d0, GetPedsInterior 0x598620). Callees: none.
- Globals: f32 [0x858b8c] = 0.5.
- Fields: `this+0x18` RwMatrix m_matrix: right = +0x18/+0x1C/+0x20, up = +0x28/+0x2C/+0x30, at = +0x38/+0x3C/+0x40, pos = +0x48/+0x4C/+0x50. `this+0x14` tEffectInterior* m_box: u8 width +2, depth +3, height +4.
- Behaviour (exact x87 order):
  1. `dx = (f32)(pt.x - pos.x)` (fld pos.x; fsubr pt.x; fstp f32 temp), `dy = (f32)(pt.y - pos.y)`, `dz = (f32)(pt.z - pos.z)`; each rounded to f32 on store.
  2. Axis 1: `d = right.y*dy; d += right.x*dx; d += right.z*dz; d = |d|`. `lim = (f32)(int)width * 0.5f + bx` (fild of zero-extended u8, fmul 0.5, fadd bias.x). If `lim < d` (fcompp ST0=lim vs ST1=d; `test ah,5; jnp`) return 0. If equal or unordered (NaN) it continues (counts as inside).
  3. Axis 2: same with `up` (+0x2C*dy + +0x28*dx + +0x30*dz), depth (+3), by.
  4. Axis 3: same with `at` (+0x3C*dy + +0x38*dx + +0x40*dz), height (+4), bz.
  5. return 1.
  Note the product order per axis: the .y term is computed first, then .x is added, then .z.
- Edge cases: NaN coordinates pass every test (inside). The caller-provided `[esp+4]` slot is reused as an int temp after the pointer is read (no effect on caller).
- gta-reversed: plugin::Call only (gr's default `bias = {}`). Open questions: none.

## 0x5914d0 Interior_c::CalcMatrix  (catalogue: specified/REA, port level 3)
- Convention / signature: `__thiscall`; `[esp+4]` CVector* translation (box position in the entity's model space; Init passes the effect position); void; `ret 4`.
- Callers: Init 0x593bf0. Callees: RwMatrixRotate 0x7f1fd0, RwMatrixTranslate 0x7f2450, InteriorGroup_c::GetEntity 0x594bd0, RwMatrixMultiply 0x7f18b0.
- Fields: `this+0x18` RwMatrix m_matrix (flags at +0x24); `this+0x14` m_box (f32 rot at +0x20); `this+0xC` InteriorGroup_c* m_pGroup.
- Behaviour:
  1. Set m_matrix to identity: right=(1,0,0), up=(0,1,0), at=(0,0,1), pos=(0,0,0); `flags |= 0x20003` (rwMATRIXTYPEORTHONORMAL | rwMATRIXINTERNALIDENTITY), other flag bits kept; pad words untouched.
  2. `RwMatrixRotate(&m, &axis{0,0,1}, m_box->rot, rwCOMBINEREPLACE (0))` (angle passed as the raw f32 from box+0x20, degrees per RW convention).
  3. `RwMatrixTranslate(&m, translation, rwCOMBINEPOSTCONCAT (2))`.
  4. `e = m_pGroup->GetEntity(); rw = e->m_pRwObject (+0x18)`. If rw: `RwMatrixMultiply(&m, &m, &((RwFrame*)rw->parent (+4))->modelling (+0x10))`; else `RwMatrixMultiply(&m, &m, NULL)`.
- Edge cases: the null-matrix call when the entity has no RwObject is what the original does (it would fault inside RW); keep it or record the divergence. dst and first source alias (&m, &m), as in the original.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591590 Interior_c::Furnish  (catalogue: specified/REA, port level 18)
- Convention / signature: `__thiscall` (this = ECX), void, plain `ret` (or tail jumps).
- Callers: none found (dead code; Init 0x593bf0 has its own inlined copy of this switch). Callees: FurnishShop 0x59a790 (arg 0), FurnishOffice 0x599af0, FurnishLounge 0x597740, FurnishBedroom 0x593fc0, FurnishKitchen 0x5970b0 (tail `jmp`s).
- Behaviour: `switch (m_box->type (u8 +0)) { 0: FurnishShop(0); 1: FurnishOffice(); 2: FurnishLounge(); 3: FurnishBedroom(); 4: FurnishKitchen(); default: nothing }`.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x5915d0 Interior_c::Unfurnish  (catalogue: specified/REA, port level 3)
- Convention / signature: `__thiscall` (this = ECX), void, plain `ret`.
- Callers: Exit 0x592230 (tail jump). Callees: CWorld::FindPlayerPed 0x56e210 (-1), CPed::GetEntityThatThisPedIsHolding 0x5e02e0, CWorld::Remove 0x563280, entity vtable slot 0 (scalar deleting destructor, arg 1), TList_c::RemoveItem 0x4a8e30, TList_c::AddItem 0x4a8df0 (head insert).
- Globals: u16 [0xbb4a70] CObject::nNoTempObjects (RW), u32 [0xb7cb84] CTimer::m_snTimeInMilliseconds, list at 0xbad3ec = g_furnitureMan (0xBAB380) + 0x206C = m_FurnitureList (free pool of FurnitureItem).
- Fields: `this+0x5C` TList_c<FurnitureItem> (head +0x5C, tail +0x60, count +0x64); item: +4 next, +8 CObject* entity.
- Behaviour: `item = head; while (item) { next = item->next;`
  1. `p = FindPlayerPed(-1)`. If `p && (h = p->GetEntityThatThisPedIsHolding()) && h == item->entity && (h->m_nType (byte +0x36) & 7) == 4 (object) && (h->objectFlags (+0x140) & 0x2000) (bIsLiftable)`: the player is carrying it, so hand it over: `CObject::nNoTempObjects++; h->m_nObjectType (+0x13C) = 3 (OBJECT_TEMPORARY); h->removalTime (+0x150) = CTimer::m_snTimeInMilliseconds + 99999999 (0x5F5E0FF)`.
  2. Otherwise: `CWorld::Remove(item->entity)`; then if `item->entity != null` call its virtual destructor `vtable[0](1)` (delete).
  3. `item->entity = null; this->list.RemoveItem(item); g_furnitureMan.m_FurnitureList.AddItem(item);` `item = next; }`
- Edge cases: CWorld::Remove is called before the null check (null entity would fault in Remove). FindPlayerPed is called once per item.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591680 Interior_c::CheckTilesEmpty  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall`; args `[esp+4]` int x, `[esp+8]` int y, `[esp+0xC]` int w (extent along x), `[esp+0x10]` int d (extent along y), `[esp+0x14]` u8 allowType9; returns AL bool; `ret 0x14`.
- Callers: FindEmptyTiles 0x591c50, PlaceFurniture 0x592aa0. Callees/globals: none.
- Fields: `this+0x14` m_box (width u8 +2, depth u8 +3); tile grid `u8 tiles[30][30]` at `this+0x68`, element (x,y) at `this + 0x68 + x*30 + y`.
- Behaviour: return 0 if `x < 0`, or `y < 0`, or `x + w > width`, or `y + d > depth` (all signed int compares; width/depth zero-extended). Then `for (i = 0; i < w; i++) for (j = 0; j < d; j++) { t = tiles[x+i][y+j]; if (t != 0) { if (!allowType9) return 0; if (t != 9) return 0; } }` and return 1. Outer loop over x ascending, inner over y ascending. w <= 0 or d <= 0 returns 1 after the bounds checks.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591700 Interior_c::SetTilesStatus  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall`; `[esp+4]` int x, `[esp+8]` int y, `[esp+0xC]` int w, `[esp+0x10]` int d, `[esp+0x14]` int status (only the low byte is stored), `[esp+0x18]` u8 force; void; `ret 0x18`.
- Callers: 15 sites in 12 functions (SetCornerTiles x8, PlaceFurniture, all Furnish*/Office_*/Shop_* placers). Callees/globals: none.
- Fields: m_box width/depth (`+0x14` -> +2/+3); tiles at `this+0x68 + x*30 + y`.
- Behaviour: silently return if `x < 0`, `y < 0`, `x + w > width` or `y + d > depth` (signed). Then for `i = 0..w-1` (x, ascending) and inside it `j = 0..d-1` (y, ascending), with `t = tiles[x+i][y+j]`:
  1. if `t == 9 && status == 5`: tile = 10; next tile.
  2. else if `force != 0`: if `t` is 5, 7 or 8, leave it; otherwise tile = (u8)status.
  3. else (not forced): if `t == 3`: if `status == 3` **return immediately** (the rest of the rectangle is left unprocessed, earlier tiles stay written); if `status == 4` tile = 4; any other status leaves it. Else if `t == 0` tile = (u8)status. Else leave it.
- Edge cases: the early `return` on (t==3, status==3) is original behaviour (partial writes). w <= 0 does nothing.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x5917c0 Interior_c::SetCornerTiles  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall`; `[esp+4]` int corner, `[esp+8]` int len, `[esp+0xC]` int status, `[esp+0x10]` u8 force (pushed on as the full dword); void; `ret 0x10`.
- Callers: FurnishLounge (9 sites). Callees: SetTilesStatus 0x591700 (x, y, w, d, status, force).
- Behaviour (W = m_box width +2, D = depth +3, re-read before each call; two L-shaped strips, first call then second):
  - corner 1: `SetTilesStatus(0, 0, len, 1, status, force)`; `SetTilesStatus(0, 0, 1, len, status, force)`.
  - corner 0: `SetTilesStatus(0, D-1, len, 1, ...)`; `SetTilesStatus(0, D-len, 1, len, ...)`.
  - corner 2: `SetTilesStatus(W-len, 0, len, 1, ...)`; `SetTilesStatus(W-1, 0, 1, len, ...)`.
  - corner 3: `SetTilesStatus(W-len, D-1, len, 1, ...)`; `SetTilesStatus(W-1, D-len, 1, len, ...)`.
  - any other corner: nothing.
  (corner 0 = x 0 / y max; 1 = origin; 2 = x max / y 0; 3 = x max / y max.)
- gta-reversed: plugin::Call only. Open questions: none.

## 0x5918e0 Interior_c::GetTileStatus  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall`; `[esp+4]` int x, `[esp+8]` int y; returns EAX int (u8 zero-extended); `ret 8`. Part of the body is displaced to 0x401b9b (protection chunk: `movzx eax, byte [ecx+eax+0x68]; jmp 0x59190e`), semantics are inline.
- Callers: AddGotoPt, FurnishLounge (x4), Shop_FurnishEdges (x2). Callees/globals: none.
- Behaviour: if `x >= width` or `y >= depth` or `x < 0` or `y < 0` (in that order, signed) return 1; else return `tiles[x*30 + y]` (`this+0x68`).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591920 Interior_c::GetNumEmptyTiles  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall`; `[esp+4]` int x, `[esp+8]` int y, `[esp+0xC]` int dir, `[esp+0x10]` int len; returns EAX int count; `ret 0x10`.
- Callers: Office_PlaceEdgeFillers, Office_PlaceEdgeDesks, Shop_PlaceEdgeUnits. Callees/globals: none.
- Behaviour: counts how many consecutive lines of `len` tiles, all exactly 0, can be stepped through starting at (x, y).
  1. `count = 0; step = (dir == 3 || dir == 0) ? -1 : +1`.
  2. If `dir == 3 || dir == 1` (lines run along y, stepping in x): `cx = x; loop { for (k = 0; k < len; k++) { ty = y + k; if (cx >= width || ty >= depth || cx < 0 || ty < 0) return count; if (tiles[cx][ty] != 0) return count; } count++; cx += step; }`
  3. Otherwise (dir 0, 2 or any other value; lines run along x, stepping in y): `cy = y; loop { for (k = 0; k < len; k++) { tx = x + k; if (tx >= width || cy >= depth || tx < 0 || cy < 0) return count; if (tiles[tx][cy] != 0) return count; } count++; cy += step; }` (dir 0 steps -1, dir 2 and others +1).
  Bounds checks in that order, signed; width/depth (m_box +2/+3) re-read per tile.
- Edge cases: `len <= 0` never reaches a bounds check, so the loop never terminates (original hang; callers pass positive lengths).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591b20 Interior_c::GetRandomTile  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall`; `[esp+4]` int status, `[esp+8]` int* outX, `[esp+0xC]` int* outY; returns EAX = the chosen y (gr declares int32; callers that use it get y); `ret 0xC`.
- Callers: InteriorGroup_c::SetupShopPeds 0x594c10. Callees: `_rand` 0x821b1e (x2 per iteration), `_ftol` 0x821b40 (x2).
- Globals: f32 [0x858b14] = 3.0517578125e-05 (1/32768).
- Behaviour: loop (do-while):
  1. 0x591b35: `x = _ftol((f32)(int)(rand() & 0xFFFF) * 1/32768 * (int)width)`: `fild` int, `fmul m32 [0x858b14]`, `fimul m32 width` (width = u8 m_box+2 zero-extended, re-read every iteration), `_ftol` truncation. Range 0..width-1.
  2. 0x591b63: `y = _ftol((rand() & 0xFFFF) * 1/32768 * (int)depth)` (depth = m_box+3), same instruction sequence. rand() for x is always drawn before rand() for y.
  3. `s = (x < width && y < depth && x >= 0 && y >= 0) ? tiles[x*30 + y] : 1`.
  4. repeat while `s != status`.
  Then `*outX = x; *outY = y`; EAX = y.
- Edge cases: no iteration limit; if no tile has `status` the loop never ends (original). Each iteration consumes exactly 2 rand() values. This is NOT CGeneral::GetRandomNumberInRange; it is the inline `rand()&0xFFFF * (1/32768) * n` form, which must be reproduced exactly.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591bd0 Interior_c::GetTileCentre  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall`; `[esp+4]` f32 tx, `[esp+8]` f32 ty (tile coordinates, callers pass ints converted to float or fractional values), `[esp+0xC]` CVector* out; returns EAX = return value of RwV3dTransformPoints (= out); `ret 0xC`.
- Callers: 6 (AddGotoPt, AddInteriorInfo, AddPickups, CalcExitPts x8, GetBoundingBox x4, InteriorGroup_c::SetupShopPeds). Callees: RwV3dTransformPoints 0x7edd90 (out, out, 1, &this->m_matrix (+0x18)).
- Globals: f32 [0x858b8c] = 0.5.
- Behaviour (x87, each line one expression, stored f32):
  1. `out->x = (f32)(-(int)width) * 0.5f + tx + 0.5f` (fild -width; fmul 0.5; fadd tx; fadd 0.5).
  2. `out->y = (f32)(-(int)depth) * 0.5f + ty + 0.5f`.
  3. `out->z = (f32)(-(int)height) * 0.5f` (height = m_box +4).
  4. `RwV3dTransformPoints(out, out, 1, (RwMatrix*)(this+0x18))` (local box space -> world).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591c50 Interior_c::FindEmptyTiles  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall`; `[esp+4]` int w, `[esp+8]` int d, `[esp+0xC]` int* outX, `[esp+0x10]` int* outY; returns AL bool; `ret 0x10`.
- Callers: FurnishBedroom (x5), FurnishLounge (x3). Callees: `_rand` 0x821b1e (x2 per try), `_ftol` 0x821b40 (x2), CheckTilesEmpty 0x591680.
- Globals: f32 [0x858b14] = 1/32768.
- Behaviour: `for (try = 0; try < 100; try++)`:
  1. 0x591c74: `x = _ftol((f32)(rand() & 0xFFFF) * 1/32768 * (int)(width - w))` (`fild`, `fmul m32`, `fimul m32` of the int `width - w`; width re-read each try).
  2. 0x591ca4: `y = _ftol((rand() & 0xFFFF) * 1/32768 * (int)(depth - d))`.
  3. `if (CheckTilesEmpty(x, y, w, d, 1)) { *outX = x; *outY = y; return 1; }`
  After 100 failures return 0 (outputs untouched).
- Edge cases: `width - w` may be 0 or negative (x 0 or <= 0); CheckTilesEmpty rejects negative coordinates. Always 2 rand() per try.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591d20 Interior_c::AddGotoPt  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall`; `[esp+4]` int x, `[esp+8]` int y (tile), `[esp+0xC]` f32 fx, `[esp+0x10]` f32 fy (fractional offsets); void; `ret 0x10`.
- Callers: FurnishKitchen (x4), FurnishLounge (x4), Office_FurnishEdges (x4), Shop_FurnishAisles (x6). Callees: GetTileStatus 0x5918e0, GetTileCentre 0x591bd0.
- Fields: `this+0x40C` s8 m_numGotoPts (gr field_40C); goto-point array at `this+0x410`, 16 entries of 0x10 bytes: `+0 u8 tileX, +1 u8 tileY, +2..3 unused, +4 CVector pos` (so x at 0x410+16n, y at 0x411+16n, pos at 0x414+16n). (gr's field_412..gap436 cover this area.)
- Behaviour:
  1. If `(s8)this[0x40C] >= 16` return.
  2. Accept the tile if (`x < width && y < depth && x >= 0 && y >= 0 && tiles[x][y] == 3`); otherwise call `GetTileStatus(x, y)` and accept only if it returns 7. Not accepted: return.
  3. `GetTileCentre((f32)x + fx, (f32)y + fy, &p)` (ty is computed first: `fild y; fadd fy`; then `fild x; fadd fx`; both stored f32).
  4. `n = (s8)this[0x40C]`; `pt[n].pos = p; pt[n].x = (u8)x; pt[n].y = (u8)y`.
  5. If `x >= 0 && y >= 0 && x+1 <= width && y+1 <= depth`: if `tiles[x][y]` is 3 or 0, set it to 4 (inlined `SetTilesStatus(x, y, 1, 1, 4, 0)`).
  6. `this[0x40C]++` (also when step 5 is skipped).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591e40 Interior_c::AddInteriorInfo  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall`; `[esp+4]` int type (low byte stored), `[esp+8]` f32 tx, `[esp+0xC]` f32 ty, `[esp+0x10]` int dir, `[esp+0x14]` CEntity* entity; returns AL bool; `ret 0x14`.
- Callers: 11 functions (Bedroom/Kitchen/Lounge/Office/Shop placers). Callees: GetTileCentre 0x591bd0, RwV3dTransformVectors 0x7eddf0.
- Globals: f32 [0x858c98] = 0.8.
- Fields: `this+0x40D` s8 m_interiorInfosCount; `this+0x590` InteriorInfo_t m_interiorInfos[16] (0x20 each: s8 Type +0, bool IsInUse +1, CVector Pos +4, CVector Dir +0x10, CEntity* EntityIgnoredCollision +0x1C); `this+0x18` RwMatrix.
- Behaviour:
  1. If `(s8)count >= 16` return 0.
  2. `GetTileCentre(tx, ty, &pos)`; `pos.z = pos.z + 0.8f` (fld, fadd m32, fstp).
  3. `d = (0, 0, 0)`. If `dir != -1`: dir 3 -> d.x = -1.0; dir 1 -> d.x = 1.0; dir 2 -> d.y = 1.0; dir 0 -> d.y = -1.0; other values keep zero; then `RwV3dTransformVectors(&d, &d, 1, &m_matrix)` (also for the zero vector). dir == -1 skips the transform.
  4. `i = &infos[count]`: `Type = (u8)type; Pos = pos; Dir = d; IsInUse = 0; EntityIgnoredCollision = entity;` (written in that order), `count++`, return 1.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x591f90 Interior_c::AddPickups  (catalogue: specified/REA, port level 19)
- Convention / signature: `__thiscall` (this = ECX), void, plain `ret`.
- Callers: Init 0x593bf0 (box types 2 and 3 only). Callees: `_rand` 0x821b1e, `_ftol` 0x821b40, GetTileCentre 0x591bd0, CPickups::GenerateNewOne 0x456f20, CPickups::GenerateNewOne_WeaponType 0x457380.
- Globals: u32 [0xb7cb84] CTimer::m_snTimeInMilliseconds; u32 [0xbb3dc4] g_interiorMan.m_TimeLastPickupsGenerated (+0x4754, read only here); u16 [0x8cd59c] MI_MONEY; f32 [0x858b14] 1/32768, [0x858628] 100.0, [0x85a708] -40.0, [0x866d14] -15.0, [0x858b8c] 0.5.
- Notation: `R16 = (f32)(rand() & 0xFFFF) * 1/32768` (fild, fmul m32 [0x858b14]).
- Behaviour:
  1. If `(u32)(now - m_TimeLastPickupsGenerated) < 180000` (0x2BF20, unsigned `jb`) return.
  2. `placed = 0; for (tries = 0; placed < 1 && tries < 100; tries++)` (test `placed < 1` first, then body, then `tries++`, loop while `tries < 100`):
     a. 0x591fca: `x = _ftol(R16 * (int)(width - 1))` (fimul). 0x591ffd: `y = _ftol(R16 * (int)(depth - 1))`.
     b. If not (`x < width && y < depth && x >= 0 && y >= 0`) next try. `t = tiles[x][y]`; if t is not 0, 3 or 4, next try.
     c. `GetTileCentre((f32)x, (f32)y, &pos)` (ty = fild y converted first, then tx).
     d. 0x592087: `k = _ftol(R16 * 100.0f)` (fmul m32, not fimul).
     e. If `k < 75`: money. 0x5920af: `a = _ftol(R16 * -40.0f)` (range -39..0); `CPickups::GenerateNewOne(pos (by value), MI_MONEY (zero-extended u16), 8, 10 - a, 0, 0, NULL)` (args after the vector: modelId, pickupType 8, ammo/amount, moneyPerDay 0, isEmpty 0, message null; caller pops 0x24).
     f. Else: weapon. `pos.z = pos.z + 0.5f` (stored f32). 0x59211e: `k2 = _ftol(R16 * 100.0f)`; weapon = `k2 < 40 ? 5 : k2 < 80 ? 22 : k2 < 90 ? 4 : 25`. 0x59216a: `a = _ftol(R16 * -15.0f)`; `CPickups::GenerateNewOne_WeaponType(pos, weapon, 3, 3 - a, 0, NULL)` (caller pops 0x20).
     g. `placed++` (only after a pickup call; the return value is ignored).
- Edge cases: RNG use per try: 2 rand() when the tile is rejected; 4 (money) or 5 (weapon) when accepted. Does not write m_TimeLastPickupsGenerated.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x5921d0 Interior_c::Constructor  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall` (this = ECX), no args, returns EAX = this; plain `ret`. Has an SEH frame (handler 0x83cf68) for the member unwind.
- Callers: InteriorManager_c::InteriorManager_c 0x5984c0 (as `eh vector constructor` fnref). Callees: ListItem_c ctor 0x4a8db0 (`this->prev = 0; this->next = 0`, via displaced chunk 0x401c62), TList_c ctor 0x4a8dd0 on `this+0x5C` (`head = tail = count = 0`).
- Behaviour: 1. prev/next (+0/+4) = 0. 2. `this+0x5C/+0x60/+0x64` = 0. 3. `*(u16*)(this+0x3EC) = 0xFFFF`; `*(u16*)(this+0x3F0) = 0xFFFF` (CNodeAddress m_nodeAddress area id = -1; gr field_3EC / m_nodeAddress). Nothing else is initialised (tile grid, matrix, infos stay as the containing object had them, i.e. zero for the static g_interiorMan in .bss).
- gta-reversed: constructor hook commented out; `= default` in gr does NOT set +0x3EC/+0x3F0 (gr differs: original writes 0xFFFF to both). Open questions: none.

## 0x592230 Interior_c::Exit  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall` (this = ECX), void, plain `ret` (via tail jump to Unfurnish).
- Callers: InteriorGroup_c::Exit 0x595290. Callees: CPickups::RemovePickUpsInArea 0x456d30 (cdecl, 6 f32), Interior_c::Unfurnish 0x5915d0 (tail `jmp`).
- Globals: f32 [0x858b40] = 50.0.
- Behaviour: `p = m_matrix.pos (+0x48/+0x4C/+0x50)` copied to locals; computed in this order, each fld/fadd|fsub/fstp f32: `z+50, z-50, y+50, y-50, x+50, x-50`; call `CPickups::RemovePickUpsInArea(x-50, x+50, y-50, y+50, z-50, z+50)`; then `Unfurnish()`.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x5922c0 Interior_c::FindBoundingBox  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall`; `[esp+4]` int x, `[esp+8]` int y, `[esp+0xC]` int* minX, `[esp+0x10]` int* maxX, `[esp+0x14]` int* minY, `[esp+0x18]` int* maxY, `[esp+0x1C]` int* visited (int[30*30], index x*30+y); void; `ret 0x1C`. (gr names the five pointers a3..a7; the order is minX, maxX, minY, maxY, visited as proven by the compares below.) Part of the body is displaced to 0x40a4a3 (`movzx ecx, byte [edi+edx+0x68]; jmp 0x592307`).
- Callers: itself (3 recursive sites), GetBoundingBox 0x593db0. Callees: itself.
- Behaviour: 4-neighbour flood fill over tiles whose status is 5, growing the box. Neighbours are processed in this fixed order (recursion is depth first):
  1. (x-1, y): requires `x > 0`, `x-1 < width`, `y < depth`, `x-1 >= 0`, `y >= 0`, `tiles[x-1][y] == 5`, `visited[(x-1)*30+y] == 0`. Then `visited[...] = 1; if (x-1 < *minX) *minX = x-1;` recurse `(x-1, y, same pointers)`.
  2. (x, y+1): requires `y < 29`, `x < width`, `y+1 < depth`, `x >= 0`, `y+1 >= 0`, tile == 5, visited == 0. Then mark; `if (y+1 > *maxY) *maxY = y+1;` recurse `(x, y+1, ...)`.
  3. (x+1, y): requires `x < 29`, `x+1 < width`, `y < depth`, `x+1 >= 0`; then if `y < 0` the function returns at once (skipping step 4 too; step 4 would skip anyway); tile == 5, visited == 0. Then mark; `if (x+1 > *maxX) *maxX = x+1;` recurse `(x+1, y, ...)`.
  4. (x, y-1): requires `y > 0`, `x < width`, `y-1 < depth`, `x >= 0`, `y-1 >= 0`, tile == 5, visited == 0. Then mark; `if (y-1 < *minY) *minY = y-1;` and continue at step 1 with `y = y-1` (tail call compiled as a loop; same effect as recursing).
  Width/depth = m_box +2/+3 (u8), all compares signed. The start tile itself is not marked or tested here (the caller does that).
- Edge cases: recursion depth up to the number of status-5 tiles (max 900 frames of 0x2C bytes).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x5924a0 Interior_c::CalcExitPts  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall` (this = ECX), void, plain `ret`.
- Callers: Init 0x593bf0 (after furnishing). Callees: GetTileCentre 0x591bd0 (2 per door, up to 8).
- Globals: f32 [0x858b8c] = 0.5, f32 [0x858c84] = 0.25; immediate f32 -0.25 (0xBE800000).
- Fields: m_box: s8 door +5 (bottom wall, x position), s8 lDoor +6/+7, rDoor +8/+9, tDoor +0xA/+0xB; width +2, depth +3. Goto points (see AddGotoPt): `n = (s8)this[0x40C]`, `gx(k) = (s8)this[0x410+16k]`, `gy(k) = (s8)this[0x411+16k]` (read signed here). Exit records: 4 x 0x20 bytes at `this+0x510` (bottom), `+0x530` (left), `+0x550` (top), `+0x570` (right): `+2 s8 a`, `+3 s8 b` (goto-point indices or -1), `+4 CVector p0` (inner point), `+0x14 CVector p1` (outer point); bytes +0, +1, +0x10..0x13, +0x20.. untouched.
- Behaviour (sections in this order; a section whose door start byte is <= -1 (signed `jle` against 0xFF) is skipped entirely and its record keeps whatever it held before):
  1. Bottom door (`s = (s8)box[5]`): `D = (f32)s - 0.5f` (fild, fsub, stored f32).
     - if `n <= 2`: `a = b = -1`; `p0 = GetTileCentre(D, 0.0f)`.
     - else: `i = 0; while (i < n) { if ((f32)gx(i) > D) break; i += 2; }` (n re-read each step; a NaN compare continues). Then `if (i == 0) { a = 0; b = -1; } else if (i == n_start) { a = (s8)(n - 2); b = -1; } else { a = i; b = i - 2; }` (n_start = count read at section entry; when n is odd and no break, i = n+1 lands in the last branch: a = n+1, b = n-1). `p0 = GetTileCentre(D, (f32)gy(0) - 0.25f)`.
     - always: `p1 = GetTileCentre(D, -0.25f)`.
  2. Left door (`s = (s8)box[6]`, `len = (s8)box[7] - s`): `D = (f32)len * 0.5f + s - 0.5f` (fild len; fmul 0.5; fiadd s; fsub 0.5; stored f32).
     - if `n <= 2`: `a = b = -1`; `p0 = GetTileCentre(0.0f, D)`.
     - else: `if ((f32)gy(0) > D) { a = 0; b = -1; } else if ((f32)gy(1) < D) { a = 1; b = -1; } else { a = 0; b = 1; }` (NaN: first test false, second false -> a=0, b=1). `p0 = GetTileCentre((f32)gx(0) - 0.25f, D)`.
     - always: `p1 = GetTileCentre(-0.25f, D)`.
  3. Top door (`s = (s8)box[0xA]`, len from +0xB): D as in 2.
     - if `n <= 2`: `a = b = -1`; `p0 = GetTileCentre(D, (f32)(depth - 1))` (no 0.25 offset).
     - else: `i = 1; while (i < n) { if ((f32)gx(i) > D) break; i += 2; }`. `if (i == 0) {a = 1; b = -1;} /*unreachable*/ else if (i == n_start) { a = (s8)(n - 1); b = -1; } else { a = i; b = i - 2; }`. `p0 = GetTileCentre(D, (f32)gy(1) + 0.25f)`.
     - always: `p1 = GetTileCentre(D, (f32)(depth - 1) + 0.25f)`.
  4. Right door (`s = (s8)box[8]`, len from +9): D as in 2.
     - if `n <= 2`: `a = b = -1`; `p0 = GetTileCentre((f32)(width - 1), D)`.
     - else: `if ((f32)gy(n-2) > D) { a = n-2; b = -1; } else if ((f32)gy(n-1) < D) { a = n-1; b = -1; } else { a = n-1; b = n-2; }`. `p0 = GetTileCentre((f32)gx(n-2) + 0.25f, D)`.
     - always: `p1 = GetTileCentre((f32)(width - 1) + 0.25f, D)`.
  All GetTileCentre arguments are computed in x87 and stored as f32 before the call; the record's a/b bytes are written before p0, p0 before p1.
- Edge cases: stale exit records for absent doors (pooled Interior_c objects are reused). With n > 2 the code assumes goto points come in x-sorted pairs; it reads gx/gy of indices up to n-1 only.
- gta-reversed: plugin::Call only (gr has no field names for 0x410..0x58F). Open questions: none.

## 0x5929f0 Interior_c::IsVisible  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall` (this = ECX), no args; returns AL bool; plain `ret`.
- Callers: InteriorGroup_c::CalcIsVisible 0x595200. Callees: IsPtInside 0x5913e0.
- Globals: TheCamera (0xb6f028): `+0x14` CMatrix* m_matrix (0xb6f03c), `+0x4` placement position (0xb6f02c); f32 [0x858628] = 100.0.
- Fields: `this+0x14` m_box (s8 door +5), `this+0x400` CVector m_position.
- Behaviour:
  1. `cam = TheCamera.m_matrix ? TheCamera.m_matrix->pos (matrix+0x30) : TheCamera.m_placement.pos (0xb6f02c)`; copied to a local CVector.
  2. `if (IsPtInside(&cam, CVector(5.0f, 5.0f, 0.0f))) return 1;`
  3. `if ((s8)m_box->door <= 0) return 0;`
  4. `dx = cam.x - m_position.x; dy = cam.y - m_position.y` (kept in x87, not stored); `d2 = dy*dy + dx*dx`; return `d2 < 100.0f` (fcomp with [0x858628]; equal or NaN -> 0).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x592aa0 Interior_c::PlaceFurniture  (catalogue: specified/REA, port level 4)
- Convention / signature: `__thiscall`; `[esp+4]` Furniture_c* f, `[esp+8]` int x, `[esp+0xC]` int y (tile of the footprint's min corner), `[esp+0x10]` f32 z (height offset), `[esp+0x14]` int mode (0 = hang from ceiling, 1 = floor with tile check and tile marking, 2 = floor without tiles, other = no translation), `[esp+0x18]` int rot (0..3, quarter turns; 4 also reaches here from Office_PlaceEdgeFillers), `[esp+0x1C]` int* outW, `[esp+0x20]` int* outD, `[esp+0x24]` u8 skipTileCheck (only skips the CheckTilesEmpty test of step 3); returns EAX = the created CBuilding* (item->entity) or 0 (gr declares void; callers use the value); `ret 0x24`. SEH frame (handler 0x83d001), 0x2A8 bytes of locals (up to 9 CMatrix).
- Callers: PlaceFurnitureOnWall, PlaceFurnitureInCorner, FurnishBedroom, Kitchen_FurnishEdges (x7), FurnishKitchen, FurnishLounge, Office_PlaceEdgeFillers, Office_PlaceDesk (x2), Shop_Place3PieceUnit (x6), Shop_PlaceCounter (x3).
- Callees: TList_c::GetNumItems 0x4a8ec0, CheckTilesEmpty 0x591680, CMatrix::SetUnity 0x59ae70, CMatrix::SetTranslate(CVector) 0x59af40 (sets identity rotation and the position), **CGeneral::GetRandomNumberInRange(int,int) 0x407180**, CMatrix::RotateZ 0x59b390, operator*(CMatrix,CMatrix) 0x59be30, CMatrix::operator= 0x59bbc0, ~CMatrix 0x59acd0, CMatrix(RwMatrix*, bool) 0x59c050, TList_c::RemoveHead 0x4a8e70, CBuilding operator new 0x404090 (0x38 bytes, building pool), CBuilding ctor 0x403e00, entity vtable +0x18 (SetModelIndex), CPlaceable::SetMatrix 0x54f610, CWorld::Add 0x563220, TList_c::AddItem 0x4a8df0, SetTilesStatus 0x591700.
- Globals: pool list 0xbad3ec (g_furnitureMan.m_FurnitureList); f32 [0x858b8c] = 0.5, [0x8595ec] = 0.017453292 (deg->rad), [0x858fe4] = 1.5707964 (pi/2).
- Fields: Furniture_c: u16 modelId +8, u8 widthX +0xC, u8 widthY +0xD, bool canPlaceInFrontOfWindow +0x10, bool isTall +0x11, u8 maxAng +0x13. Interior: m_box width/depth/height (+2/+3/+4), m_areaCode +0x10, m_matrix +0x18, list +0x5C.
- Behaviour:
  1. If `pool.count <= 0`: `*outW = 0; *outD = 0;` return 0.
  2. Footprint: if `rot == 1 || rot == 3`: `w = f->widthY; d = f->widthX;` else `w = f->widthX; d = f->widthY`. (Unrotated sizes X = widthX, Y = widthY are kept separately.)
  3. If `mode == 1 && skipTileCheck == 0` and `!CheckTilesEmpty(x, y, w, d, f->canPlaceInFrontOfWindow)`: `*outW = *outD = 0`, return 0.
  4. Random wobble matrix `mRand`:
     - if `f->maxAng == 0`: `mRand.SetUnity()`.
     - else: `a = 0.5f - (f32)Y * 0.5f; b = 0.5f - (f32)X * 0.5f` (fild; fmul 0.5; fsubr 0.5; stored f32). `mA.SetUnity(); mA.SetTranslate(CVector(b, a, 0))`. `mB.SetUnity(); r = CGeneral::GetRandomNumberInRange(-maxAng, maxAng)` at **0x592c15** (cdecl, original helper 0x407180: `r = min + _ftol((f32)(rand() & 0xFFFF) * f32[0x858b14] (1/32768) * (int)(max - min))`, i.e. `-maxAng + _ftol(R16 * (2*maxAng))`, range -maxAng .. maxAng-1; one rand()); `mB.RotateZ((f32)r * 0.017453292f)`. `mC.SetUnity(); mC.SetTranslate(CVector(-b, -a, 0))`. `mRand = (mC * mB) * mA`.
  5. `CMatrix mInt(&m_matrix, false)`; `mRot.SetUnity(); mRot.RotateZ((f32)rot * 1.5707964f)` (fild rot; fmul m32).
  6. Rotation offset (ints): `ox = oy = 0`; rot 1: `ox = w-1`; rot 2: `ox = w-1, oy = d-1`; rot 3: `oy = d-1`.
  7. Translation (x87, kept on the stack, then added to mRot.pos):
     - mode 0: `tz = (f32)height * 0.5f - z`; mode 1 or 2: `tz = (f32)(-height) * 0.5f + z`;
     - modes 0/1/2: `ty = (f32)(-depth) * 0.5f + oy + y + 0.5f` (fild -depth; fmul 0.5; fiadd oy; fiadd y; fadd 0.5); `tx = (f32)(-width) * 0.5f + ox + x + 0.5f` (same order);
     - `mRot.pos.x = mRot.pos.x + tx; mRot.pos.y += ty; mRot.pos.z += tz` (each fld m32; fadd st(1); fstp m32).
     - other modes: no translation.
  8. `mFinal = (mInt * mRot) * mRand`.
  9. `item = pool.RemoveHead()`; if null: destroy matrices, `*outW = *outD = 0`, return 0.
  10. `bld = new CBuilding()` (null if the pool is full; not checked); `item->entity = bld; bld->SetModelIndex(f->modelId); bld->SetMatrix(mFinal); bld->m_nAreaCode (+0x2F) = (u8)m_areaCode; bld->flags (+0x1C) |= 0x10000 (m_bDontCastShadowsOn); bld->flags |= 0x400000 (m_bIsTempBuilding); CWorld::Add(bld);`
  11. `item->tileX = (u16)x; item->tileY = (u16)y; this->list.AddItem(item)` (head insert).
  12. If `mode == 1`: `SetTilesStatus(x, y, w, d, f->isTall ? 6 : 5, 1)`.
  13. `*outW = w; *outD = d;` return bld.
- Edge cases: the only RNG use is the GetRandomNumberInRange call in step 4 (only when maxAng != 0) - it must be the original 0x407180 formula, not gr's lerp (cat14 finding 1). No null check on `new CBuilding`.
- gta-reversed: plugin::Call only (declared `void`, but EAX carries the entity). Open questions: none.

## 0x593120 Interior_c::PlaceFurnitureOnWall  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall`; 14 stack args, `ret 0x38`: `[+4]` int groupId, `[+8]` int subGroupId, `[+0xC]` int furnitureId (passed as i16), `[+0x10]` f32 z, `[+0x14]` int mode (PlaceFurniture mode), `[+0x18]` int wall (0..3, -1 = random), `[+0x1C]` int pos (along the wall, -1 = random), `[+0x20]` int offset (distance from the wall), then 6 optional out pointers (each null-checked): `[+0x24]` int* outWall, `[+0x28]` int* outPos, `[+0x2C]` int* outX, `[+0x30]` int* outY, `[+0x34]` int* outW, `[+0x38]` int* outD. Returns EAX = PlaceFurniture's result (entity) or 0. Part of the prologue is displaced to 0x4015b9 (`mov ecx, offset g_furnitureMan; jmp 0x593146`).
- Callers: FurnishBedroom (x8), Kitchen_FurnishEdges (x4), FurnishLounge (x10). Callees: FurnitureManager_c::GetFurniture 0x5911e0 (thiscall on g_furnitureMan 0xBAB380), `_rand` 0x821b1e, `_ftol` 0x821b40, PlaceFurniture 0x592aa0.
- Globals: f32 [0x858b14] = 1/32768, f32 [0x858b90] = 4.0.
- Behaviour:
  1. `f = g_furnitureMan.GetFurniture(groupId, subGroupId, (i16)furnitureId, m_box->+0x1F (u8, gr m_status, the wealth rating))`. If `f == null` return 0.
  2. `tries = (wall == -1 || pos == -1) ? 100 : 1`. Loop `i = 0 .. tries-1`:
     a. If the ORIGINAL wall arg is -1: 0x5931a9 `wall = _ftol((f32)(rand() & 0xFFFF) * 1/32768 * 4.0f)` (fmul m32, not fimul; range 0..3); the value overwrites the wall arg slot (so it is also what outWall gets). Else wall = the arg.
     b. If `wall != 1 && wall != 3` (0, 2 and any other value): if the original pos arg is -1: 0x5931f6 `pos = _ftol(R16 * (int)(width - f->widthX))` (fimul). `x = pos`; `y = (wall == 2) ? offset : depth - f->widthY - offset`.
     c. If `wall == 1 || wall == 3`: if the original pos is -1: 0x593256 `pos = _ftol(R16 * (int)(depth - f->widthX))`. `y = pos`; `x = (wall == 1) ? offset : width - f->widthY - offset`.
     (R16 = `(f32)(rand() & 0xFFFF) * f32[0x858b14]`; the random pos also overwrites the pos arg slot.)
     d. `e = PlaceFurniture(f, x, y, z, mode, wall /*as rot*/, &w, &d, 0)`. If `e != 0`: write (each only if its pointer is non-null) `*outWall = wall, *outPos = pos, *outX = x, *outY = y, *outW = w, *outD = d`; return e.
  3. After `tries` failures return 0 (outputs untouched).
- Edge cases: RNG per iteration: 1 draw for a random wall (first), then 1 for a random pos; a fixed pos with a random wall still loops up to 100 times. Width/depth are m_box +2/+3.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x593340 Interior_c::PlaceFurnitureInCorner  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall`; 12 stack args, `ret 0x30`: `[+4]` int groupId, `[+8]` int subGroupId, `[+0xC]` int furnitureId (as i16), `[+0x10]` f32 z, `[+0x14]` int mode, `[+0x18]` int corner (0..3, -1 = random), `[+0x1C]` int offset, out pointers (each null-checked): `[+0x20]` int* outCorner, `[+0x24]` int* outX, `[+0x28]` int* outY, `[+0x2C]` int* outW, `[+0x30]` int* outD. Returns EAX = PlaceFurniture's result or 0.
- Callers: FurnishLounge. Callees: FurnitureManager_c::GetFurniture 0x5911e0 (on g_furnitureMan 0xBAB380), `_rand` 0x821b1e, `_ftol` 0x821b40, PlaceFurniture 0x592aa0.
- Globals: f32 [0x858b14] = 1/32768, [0x858b90] = 4.0.
- Behaviour:
  1. `f = g_furnitureMan.GetFurniture(groupId, subGroupId, (i16)furnitureId, m_box->+0x1F)`; `tries = (corner == -1) ? 20 : 1`. If `f == null` return 0.
  2. `x = y = corner` (initial register values, only visible for a fixed corner outside 0..3). Loop `i = 0 .. tries-1`:
     a. If the original corner is -1: 0x5933bb `corner = _ftol((f32)(rand() & 0xFFFF) * 1/32768 * 4.0f)` (fmul m32), written back to the corner arg slot.
     b. corner 1: `x = offset; y = 0`. corner 3: `x = width - f->widthY - offset; y = depth - f->widthX` (no offset on y - original asymmetry). corner 2: `x = width - f->widthX; y = offset`. corner 0: `x = 0; y = depth - f->widthY - offset`. Other values: x, y unchanged.
     c. `e = PlaceFurniture(f, x, y, z, mode, corner /*rot*/, &w, &d, 0)`. If `e`: write (null-checked) `*outCorner = corner, *outX = x, *outY = y, *outW = w, *outD = d`; return e.
  3. Return 0.
- Edge cases: one rand() per iteration only for a random corner. The corner-3 y formula ignores `offset` (corner 0 and 1/2 do use it for one axis); keep as is.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x5934e0 Interior_c::PlaceObject  (catalogue: specified/REA, port level 15)
- Convention / signature: `__thiscall`; `[esp+4]` u8 isStealable (byte compare), `[esp+8]` Furniture_c* f, `[esp+0xC]` f32 offX, `[esp+0x10]` f32 offY, `[esp+0x14]` f32 offZ, `[esp+0x18]` f32 rotZ (degrees); returns EAX CObject* (null on failure); `ret 0x18`. SEH frame (handler 0x83d047) for 3 local CMatrix + temp + the `new`.
- Callers: Bedroom_AddTableItem, FurnishBedroom (x5), Kitchen_FurnishEdges (x4), Lounge_AddTV (x2), Lounge_AddHifi, FurnishLounge (x3). (PlaceFurniture does NOT use it.)
- Callees: TList_c::GetNumItems 0x4a8ec0 (`return count (+8)`), CMatrix(RwMatrix*, bool) 0x59c050, CMatrix::SetUnity 0x59ae70, CMatrix::RotateZ 0x59b390, operator*(CMatrix,CMatrix) 0x59be30, CMatrix::operator= 0x59bbc0, ~CMatrix 0x59acd0, TList_c::RemoveHead 0x4a8e70, operator new 0x5a1ee0 (0x17C), CObject::CObject(int modelId, bool create) 0x5a1d70, CPlaceable::SetMatrix 0x54f610, CEntity vtable +0x10 (SetIsStatic), CWorld::Add 0x563220, `_ftol` 0x821b40 (x2), TList_c::AddItem 0x4a8df0, InteriorManager_c::HasInteriorHadStealDataSetup 0x5982b0, InteriorManager_c::FindStealableObjectId(int,int,CVector) 0x5982f0 (x2), CWorld::Remove 0x563280, CEntity vtable +0 (deleting dtor), TList_c::RemoveItem 0x4a8e30.
- Globals: CModelInfo::ms_modelInfoPtrs 0xa9b0c8; furniture item pool list 0xbad3ec (g_furnitureMan.m_FurnitureList); g_interiorMan 0xBAF670: m_ObjectCount [0xbb3a18] (+0x43A8), m_Objects[32] at 0xbb3a1c, stride 0x1C (entity +0, modelId +4 (0xbb3a20), interiorId +8 (0xbb3a24), CVector pos +0xC (0xbb3a28), bool wasStolen +0x18 (0xbb3a34)); f32 [0x858b8c] = 0.5, f32 [0x8595ec] = 0.017453292 (deg->rad).
- Fields: f->m_nModelId (u16 +8); `this+0x10` m_areaCode (low byte used); `this+0x8` m_interiorId; `this+0x18` RwMatrix; `this+0x5C` furniture list; item (FurnitureItem): +8 CObject*, +0xC u16 tileX, +0xE u16 tileY.
- Behaviour:
  1. `bbMinZ = ms_modelInfoPtrs[f->modelId]->m_pColModel (+0x14)->boundBox.min.z (+8)` (min.x/min.y are also copied to locals but unused).
  2. Local position (x87, each stored f32): `lx = (f32)(-width) * 0.5f + offX`; `ly = (f32)(-depth) * 0.5f + offY`; `lz = (f32)(-height) * 0.5f + offZ - bbMinZ`.
  3. If `pool.count <= 0` return null (before building any matrix).
  4. `CMatrix mInt(&m_matrix, false)`; `CMatrix mObj` (attach fields zeroed); `mObj.SetUnity(); mObj.RotateZ(rotZ * 0.017453292f)`; `mObj.pos.x = mObj.pos.x + lx; mObj.pos.y += ly; mObj.pos.z += lz` (fld/fadd/fstp each); `CMatrix mFinal = mInt * mObj` (via temp + operator=).
  5. `item = pool.RemoveHead()`; if null: destroy matrices, return null.
  6. `obj = new CObject(f->modelId, false)` (null if allocation failed, not checked afterwards); `item->entity = obj; obj->SetMatrix(mFinal); obj->m_nAreaCode (+0x2F) = (u8)m_areaCode; obj->flags (+0x1C) |= 0x10000 (m_bDontCastShadowsOn); obj->m_nObjectType (+0x13C) = 5; obj->SetIsStatic(true) (vtable +0x10, arg 1); CWorld::Add(obj);`
  7. `item->tileX = (u16)_ftol(offX); item->tileY = (u16)_ftol(offY); this->list.AddItem(item)` (head insert).
  8. If `isStealable == 0` return obj.
  9. `obj->objectFlags (+0x140) |= 0x2000` (bIsLiftable).
     - If `g_interiorMan.HasInteriorHadStealDataSetup(this)`: `id = FindStealableObjectId(m_interiorId, f->modelId, CVector(lx, ly, lz))`. If `id >= 0 && m_Objects[id].wasStolen`: the object was stolen before: `CWorld::Remove(obj); if (obj) obj->vtable[0](1); item->entity = null; list.RemoveItem(item); pool.AddItem(item);` return null. Otherwise call `FindStealableObjectId` again with the same arguments and, if `>= 0`, set `m_Objects[id].entity = obj`; return obj.
     - Else: `k = m_ObjectCount; m_Objects[k] = { obj, f->modelId, m_interiorId, (lx, ly, lz), false }; m_ObjectCount++;` (NO bound check against 32: original overflow if more than 32 stealable objects are registered) return obj.
- Edge cases: stealable-object positions are stored in the interior's local box coordinates (lx, ly, lz), not world coordinates; FindStealableObjectId compares them exactly.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x593910 Interior_c::ResetTiles  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX), void, plain `ret`.
- Callers: Interior_c::Init 0x593bf0. Callees: none (SetTilesStatus logic is inlined). Globals: none.
- Fields: tile grid `u8 tiles[30][30]` at `this+0x68` (x*30 + y); m_box (`this+0x14`, tEffectInterior*): u8 width +2, depth +3; s8 pairs (start,end): lDoor +6/+7, rDoor +8/+9, tDoor +0xA/+0xB, lWindow +0xC/+0xD, rWindow +0xE/+0xF, tWindow +0x10/+0x11; noGoLeft[3] +0x12, noGoBottom[3] +0x15, noGoWidth[3] +0x18, noGoDepth[3] +0x1B.
- Behaviour:
  1. Zero the grid: `rep stosd` of 0xE1 dwords (900 bytes) at `this+0x68`.
  2. Six edge strips, in this order. For each: `s = (u8)box[start]`; skip if `s == 0xFF`; `len = (s8)box[end] - (s8)s`; skip if `len <= 0`; then the bounds check below; then for each tile of the strip (ascending), `if (t != 3 && t == 0) t = value` (i.e. only tiles still 0 are written):
     - lDoor (+6/+7): column x = 0, y = s .. s+len-1, value 8. Skip if `s < 0` (signed), `width < 1`, or `s + len > depth`.
     - rDoor (+8/+9): column x = width-1, y = s .. s+len-1, value 8. Skip if `width-1 < 0`, `s < 0`, or `s + len > depth`.
     - tDoor (+0xA/+0xB): row y = depth-1, x = s .. s+len-1, value 8. Skip if `s < 0`, `depth-1 < 0`, or `s + len > width`.
     - lWindow (+0xC/+0xD): as lDoor, value 9.
     - rWindow (+0xE/+0xF): as rDoor, value 9.
     - tWindow (+0x10/+0x11): as tDoor, value 9.
  3. No-go rectangles, `i = 0..2` ascending: `x = (s8)noGoLeft[i]`, `y = (s8)noGoBottom[i]`; skip if the raw byte x == 0xFF or y == 0xFF; `w = (s8)noGoWidth[i]`, `d = (s8)noGoDepth[i]`; skip if `x < 0`, `y < 0`, `x + w > width`, `y + d > depth`, or `w <= 0`; then `for a in 0..w-1 (x) for b in 0..d-1 (y): if tile == 0 → tile = 11 (0x0B)`.
  Each step is exactly `SetTilesStatus(x, y, w, d, value, 0)` on a grid where earlier strips win on overlap (value 8/9/11 never triggers the 3/4/5 special cases).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x593bf0 Interior_c::Init  (catalogue: specified/REA, port level 20)
- Convention / signature: `__thiscall`; `[esp+4]` const CVector* pos (the effect position in model space); returns EAX = 1 always; `ret 4`. SEH frame (handler 0x83d068) guarding a local CMatrix.
- Callers: InteriorManager_c::Update 0x598f50 (after setting m_box, m_interiorId, m_areaCode, m_pGroup). Callees: CalcMatrix 0x5914d0, ResetTiles 0x593910, InteriorGroup_c::GetEntity 0x594bd0, CPlaceable::AllocateMatrix 0x54f560, CSimpleTransform::UpdateMatrix 0x54f1b0, CMatrix copy ctor 0x59bcf0 (copies 0x40 bytes, attach fields +0x40/+0x44 = 0), `_ftol` 0x821b40 (x6) / srand 0x821b11, FurnishShop 0x59a790, FurnishOffice 0x599af0, FurnishLounge 0x597740, FurnishBedroom 0x593fc0, FurnishKitchen 0x5970b0, CalcExitPts 0x5924a0, InteriorManager_c::HasInteriorHadStealDataSetup 0x5982b0, AddPickups 0x591f90, ~CMatrix 0x59acd0.
- Globals: g_interiorMan 0xBAF670; [0xbb3dac] = g_interiorMan.m_EnEx (CEntryExit*, +0x473C); [0xbb3914] = m_InteriorCount (+0x42A4, RW); [0xbb3918] = m_InteriorIds[64] (+0x42A8, W).
- Fields: m_box (+0x14): type +0, seed u8 +0x1E. `this+0x8` m_interiorId, `+0xC` m_pGroup, `+0x40C` s8 goto count, `+0x40D` s8 info count.
- Behaviour:
  1. `CalcMatrix(pos); ResetTiles();`
  2. `e = m_pGroup->GetEntity(); if (!e->m_matrix (+0x14)) { e->AllocateMatrix(); e->m_placement (+4).UpdateMatrix(e->m_matrix); }` `CMatrix m(*e->m_matrix)` (local copy; position m.pos = +0x30).
  3. Seeding, only if `m_box->type != 0x63` (99):
     - if `enex = g_interiorMan.m_EnEx` is non-null: `a = _ftol(enex->m_fEntranceZ (+0x18)); a *= _ftol(enex->+0x14 (m_recEntrance 4th float, gr 'bottom')); a *= _ftol(enex->+0x8 (m_recEntrance.left)); b = _ftol(m.pos.z); b *= _ftol(m.pos.y); b *= _ftol(m.pos.x); srand(a + b + m_box->seed)` (int32 wrap-around multiplications; ftol call order exactly as listed).
     - else: `srand(_ftol(m.pos.z * m.pos.y * m.pos.x + (f32)(int)m_box->seed))` (x87: fld z; fmul y; fmul x; fiadd seed; one ftol).
  4. `this[0x40C] = 0; this[0x40D] = 0;`
  5. `switch (m_box->type) { 0: FurnishShop(0); 1: FurnishOffice(); 2: FurnishLounge(); 3: FurnishBedroom(); 4: FurnishKitchen(); }` (other types: nothing).
  6. `CalcExitPts();`
  7. `if (!g_interiorMan.HasInteriorHadStealDataSetup(this) && g_interiorMan.m_InteriorCount < 64) { m_InteriorIds[m_InteriorCount] = this->m_interiorId; m_InteriorCount++; }`
  8. If `m_box->type` is 2 or 3: `AddPickups()`.
  9. Destroy the local CMatrix; return 1.
- Edge cases: srand reseeds the global CRT generator, so every furnishing sequence is deterministic per interior; the RNG stream after Init differs between the EnEx and no-EnEx paths. The int products overflow silently (imul).
- gta-reversed: plugin::Call only. Open questions: none.

## 0x593db0 Interior_c::GetBoundingBox  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall`; `[esp+4]` FurnitureEntity_c* fe, `[esp+8]` CVector* out (array of 4 CVector, 0x30 bytes); returns AL bool; `ret 8`. EBP frame, stack aligned to 8, 0xE24 bytes of locals (900-int visited array + 4 ints + radius copy).
- Callers: InteriorManager_c::GetBoundingBox 0x598090. Callees: FindBoundingBox 0x5922c0, GetTileCentre 0x591bd0 (x4).
- Globals: f32 [0x8d22b0] CPedGeometryAnalyser::ms_fPedNominalRadius (0.35 at start; read once, copied to a local), f32 [0x858b8c] = 0.5.
- Fields: m_box->type (u8 +0); fe->m_tileX (u16 +0xC), fe->m_tileY (u16 +0xE).
- Behaviour:
  1. If `m_box->type` is not 0, 1 or 6 return 0 (nothing written).
  2. `x = fe->tileX; y = fe->tileY` (zero-extended). `int visited[900] = {0}` (rep stosd 0x384 dwords); `visited[x*30+y] = 1`. `minX = maxX = x; minY = maxY = y`.
  3. `FindBoundingBox(x, y, &minX, &maxX, &minY, &maxY, visited)`.
  4. `r = ms_fPedNominalRadius` (f32 local). Four corners, in this order, each via `GetTileCentre(tx, ty, &out[i])` (x87, results stored f32 as the args):
     - out[0]: `ty = (f32)maxY + r + 0.5f`; `tx = (f32)minX - 0.5f - r`.
     - out[1]: `ty = (f32)minY - 0.5f - r`; `tx = (f32)minX - 0.5f - r`.
     - out[2]: `ty = (f32)minY - 0.5f - r`; `tx = (f32)maxX + r + 0.5f`.
     - out[3]: `ty = (f32)maxY + r + 0.5f`; `tx = (f32)maxX + r + 0.5f`.
     (ty is computed before tx each time; the operation order is exactly `int + r + 0.5` and `int - 0.5 - r`.)
  5. return 1.
- Edge cases: the flood fill only spreads over status-5 tiles; the start tile itself need not be 5.
- gta-reversed: plugin::Call only. Open questions: none.

## 0x593f10 Interior_c::Bedroom_AddTableItem  (catalogue: specified/REA, port level 16)
- Convention / signature: `__thiscall` (this = ECX), `CObject* Bedroom_AddTableItem(int32 groupId [esp+4], int32 subGroupId [esp+8], int32 side [esp+0xC], int32 tileX [esp+0x10], int32 tileY [esp+0x14], int32 rot [esp+0x18])`, `ret 0x18`. Returns EAX = PlaceObject result.
- Callers: 1 (FurnishBedroom 0x593FC0, 2 sites). Callees: 0x5911E0 FurnitureManager_c::GetFurniture (thiscall on &g_furnitureMan 0xBAB380, `(int32 group, int32 subGroup, int16 id, u8 wealth)`); 0x5934E0 Interior_c::PlaceObject (`(u8 isStealable, Furniture_c*, f32 x, f32 y, f32 z, f32 rotZ)`, `ret 0x18`).
- Globals: f32 [0x858B8C]=0.5, f32 [0x85991C]=90.0, 0xBAB380 g_furnitureMan.
- Fields: `this+0x14` m_box: `+0x1F` u8 m_status (used as the wealth rating).
- Behaviour:
  1. `fx = (f32)tileX; fy = (f32)tileY`.
  2. if side == 0 or side == 2: `fx = fx + 0.5`; else if side == 1 or 3: `fy = fy + 0.5`; other values: no change. (Compare order: 0, 2, 1, 3.)
  3. `f = GetFurniture(groupId, subGroupId, -1, m_box->status)` (id pushed as dword -1).
  4. `rotZ = (f32)rot * 90.0`; `y = fy + 0.5`; `x = fx + 0.5` (each f32-rounded).
  5. return `PlaceObject(1, f, x, y, 0.5, rotZ)`.
- Edge cases: `rot` is not masked (used raw). No null check on `f` (PlaceObject receives it as is).
- gta-reversed: plugin::Call stub; gr arg names a2..a7 match the order above.
- Open questions: none.

## 0x593fc0 Interior_c::FurnishBedroom  (catalogue: specified/REA, port level 17)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, `ret`, void. Frame `sub esp,0x20` + push ebx/ebp/esi/edi; locals below are relative to ESP after those pushes ("base"): `wall` [base+0x10], `pos` [base+0x14], `ty` [base+0x18], `tx` [base+0x1C], `a` [base+0x20], `b` [base+0x24] (b also reused as rand scratch).
- Callers: 2 (Interior_c::Init 0x593BF0 when m_box->type == 3; Interior_c::Furnish 0x591590 tail call). Callees: 0x591220 FurnitureManager_c::GetRandomId (this 0xBAB380, `(group, subGroup, u8 wealth)`); 0x591700 SetTilesStatus `(x, y, w, d, status, force)` x8; 0x593120 PlaceFurnitureOnWall x8 sites (`(group, subGroup, int16 id, f32 z, int32 a6, int32 wall, int32 pos, int32 offset, int32* outWall, int32* outPos, int32* outX, int32* outY, int32* outW, int32* outD)`, `ret 0x38`; wall/pos = -1 means random; returns the value PlaceFurniture returned, i.e. the placed entity pointer, or 0); 0x591E40 AddInteriorInfo x2; 0x593F10 Bedroom_AddTableItem x2; 0x591C50 FindEmptyTiles x5 (`(int32 w, int32 d, int32* outX, int32* outY)`, `ret 0x10`, returns AL); 0x5911E0 GetFurniture x6; 0x5934E0 PlaceObject x5; 0x592AA0 PlaceFurniture x1; 0x407180 CGeneral::GetRandomNumberInRange(int) x1; 0x821B1E rand x8; 0x821B40 _ftol x10.
- Globals: 0xBAB380 g_furnitureMan; f32 [0x858B14]=1/32768, [0x858628]=100.0, [0x858B34]=60.0, [0x858BA4]=20.0, [0x859CE4]=-30.0, [0x858B8C]=0.5; immediate f32 0x3D4CCCCD = 0.05f.
- Fields: `this+0x14` m_box: `+2` W, `+3` D, `+5` door (s8), `+0x1F` status (u8 wealth). `this+0x791` m_furnitureId (written, then read sign-extended). Furniture_c `+0xC` widthX, `+0xD` widthY.
- Behaviour (original order):
  1. `this[0x791] = (u8)GetRandomId(3, 1, status)`. `SetTilesStatus((s8)door - 1, 0, 2, 2, 7, 0)`.
  2. Bed: `bed = PlaceFurnitureOnWall(3, 0, -1, 0.0, 1, -1, -1, 0, &wall, &pos, NULL, NULL, NULL, NULL)` (EBP keeps the result).
  3. Bedside table 1: `if (pos > 0)` (signed; `pos`/`wall` are uninitialised stack if step 2 failed: original bug): `if PlaceFurnitureOnWall(3, 1, id, 0.0, 1, wall, pos - 1, 0, NULL x6)`: by `wall` (compare order 1, 3, 0, 2) set (type, dir, infoX, infoY, tileX, tileY):
     - 1: (3, 2, 1, pos-1, 2, pos-1)
     - 3: (4, 2, W-2, pos-1, W-3, pos-1)
     - 0: (3, 1, pos-1, D-2, pos-1, D-3)
     - 2: (4, 1, pos-1, 1, pos-1, 2)
     - other: all six = current value of local `b` [base+0x24] (uninitialised).
     Then `AddInteriorInfo(type, (f32)infoX, (f32)infoY, dir, (CEntity*)bed)`; `SetTilesStatus(tileX, tileY, 1, 1, 2, 0)`.
  4. Bedside table 2 (always attempted, even when `pos <= 0` or step 2 failed): `if PlaceFurnitureOnWall(3, 1, id, 0.0, 1, wall, pos + 2, 0, NULL x6)`: by wall (order 1, 3, 0, 2):
     - 1: (type 4, dir 0, infoX 1, infoY pos+2, tile (2, pos+2))
     - 3: (3, 0, W-2, pos+2, (W-3, pos+2)) (type 3 comes from EAX still holding `wall`)
     - 0: (4, 3, pos+2, D-2, (pos+2, D-3))
     - 2: (3, 3, pos+2, 1, (pos+2, 2))
     - other: all = local `b`.
     `AddInteriorInfo(type, (f32)infoX, (f32)infoY, dir, bed)`; `SetTilesStatus(tileX, tileY, 1, 1, 2, 0)`.
  5. `PlaceFurnitureOnWall(3, 3, id, 0.0, 1, -1, -1, 0, NULL x6)`; `PlaceFurnitureOnWall(3, 2, id, 0.0, 1, -1, -1, 0, NULL x6)` (results ignored).
  6. `r = ftol((rand() & 0xFFFF) * 1/32768 * 100.0)` (rand at 0x5942AB). Let `T() = PlaceFurnitureOnWall(2, 6, -1, 0.0, 1, -1, -1, 0, &wall, NULL, &tx, &ty, NULL, NULL)`.
     - r < 25: `if T(): Bedroom_AddTableItem(2, 8, wall, tx, ty, wall)`.
     - 25 <= r < 50: `if T(): Bedroom_AddTableItem(2, 3, wall, tx, ty, wall)`.
     - 50 <= r < 75: `if T(): Bedroom_AddTableItem(2, 8, wall, tx, ty, wall)`; then (regardless) `if T(): Bedroom_AddTableItem(2, 3, wall, tx, ty, wall)`.
     - r >= 75: nothing.
  7. Clutter threshold `p` from status (unsigned): `>= 75`: `p = ftol(r1 * 20.0)` (rand 0x5943C9); `>= 50`: `p = 20 - ftol(r1 * -30.0)` (rand 0x5943F2); else `p = GetRandomNumberInRange(50, 100)` (0x407180 at 0x594422, cdecl args pushed 100 then 50; original formula `50 + ftol((rand()&0xFFFF)*(1/32768)*50)`). `r1 = (rand() & 0xFFFF) * 1/32768`.
  8. Five clutter attempts, each `q = ftol((rand()&0xFFFF) * 1/32768 * k)`, then `if q < p && FindEmptyTiles(w, d, &a, &b)`: `PlaceObject(0, GetFurniture(8, s, -1, status), (f32)a + 0.5, (f32)b + 0.5, 0.05f, 0.0)`; `SetTilesStatus(a, b, sw, sd, 2, 0)`:
     | rand at | k | FindEmptyTiles w,d | s | SetTilesStatus w,d |
     |---|---|---|---|---|
     | 0x59442C | 60.0 | 2,2 | 2 | 2,2 |
     | 0x5944CD | 100.0 | 1,1 | 5 | 1,1 |
     | 0x59456A | 100.0 | 1,1 | 4 | 1,1 |
     | 0x594607 | 100.0 | 1,1 | 3 | 1,1 |
     | 0x5946A4 | 100.0 | 2,2 | 6 | 1,1 (original marks only 1x1 although it searched 2x2) |
     y is computed first (`fild b; fadd 0.5`), then x. No null check on the GetFurniture result.
  9. Rug: `f = GetFurniture(8, 1, -1, status)`; `ty = ftol((f32)D*0.5 - (f32)f->widthY*0.5)`; `tx = ftol((f32)W*0.5 - (f32)f->widthX*0.5)` (`fsubp st(1)` = first minus second; ty computed first); `PlaceFurniture(f, tx, ty, 0.0, 0, 0, &tmp1, &tmp2, 0)`.
- Edge cases / original bugs (keep): uninitialised `wall`/`pos` used when the bed fails; switch default uses uninitialised `b`; table 2 is tried even when pos <= 0; the 5th clutter item marks 1x1 tiles for a 2x2 search; `bed` (possibly 0) passed as the info's ignored-collision entity.
- gta-reversed: plugin::Call stub only.
- Open questions: none on control flow.

## 0x596930 Interior_c::Kitchen_FurnishEdges  (catalogue: specified/REA, port level 16)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no stack args, plain `ret`. gr declares `CObject*` but the function is effectively void (EAX = whatever the last call left; FurnishKitchen does not use it). Frame: `push ebp; mov ebp,esp; and esp,-8; sub esp,0x228`, then push ebx/ebp/esi/edi; offsets below are from ESP after those 4 pushes ("base").
- Callers: 1 (FurnishKitchen 0x5970B0). Callees: 0x5911E0 FurnitureManager_c::GetFurniture (this = &g_furnitureMan 0xBAB380, `(int32 group, int32 subGroup, int16 id, u8 wealth)`) x8; 0x592AA0 PlaceFurniture x7 (returns the placed entity pointer or 0; gr arg order `(Furniture_c* f, int32 a2, int32 a3, f32 a4, int32 a5, int32 a6, int32* a7, int32* a8, u8 a9)`; a2/a3 = tile x/y, a6 = rotation 0..3; returns nonzero when placed); 0x593120 PlaceFurnitureOnWall x4 (`(group, subGroup, int16 id, f32 z, int32 a6, int32 wall, int32 pos, int32 offset, int32* outWall, int32* outPos, int32* outX, int32* outY, int32* outW, int32* outD)`, wall/pos -1 = random, returns the placed entity or 0); 0x591700 SetTilesStatus x2 `(x, y, w, d, status, force)`; 0x591E40 AddInteriorInfo x1; 0x5934E0 PlaceObject x4 `(u8 isStealable, Furniture_c*, f32 x, f32 y, f32 z, f32 rotZ)`; 0x407180 CGeneral::GetRandomNumberInRange(int) x2 (original formula, see finding 1); 0x821B1E rand x9 sites; 0x821B40 _ftol x7 sites.
- Globals: 0xBAB380 g_furnitureMan; f32 consts [0x858B14]=1/32768 (3.0517578125e-05), [0x858B8C]=0.5, [0x858CE8]=1.5, [0x858B50]=0.0, [0x858BA4]=20.0, [0x859CE4]=-30.0, [0x859B60]=-50.0, [0x858628]=100.0; immediates f32 0x3F866666 = 1.05f, 45.0, 315.0, 90.0, 270.0, 0.5.
- Fields: `this+0x14` m_box (tEffectInterior*): `+2` width W, `+3` depth D, `+7` lDoorEnd (s8), `+9` rDoorEnd (s8), `+0xD` lWindowEnd (s8), `+0xF` rWindowEnd (s8), `+0x10` tWindowStart (s8, 0xFF = none), `+0x1F` status (u8, used as wealth). `this+0x791` s8 m_furnitureId (style id; passed sign-extended as the furniture id).
- Locals: `W1 = W-1` [base+0x20]; `D1 = D-1` [base+0x10]; `cornerL` u8 [base+0x1A], `cornerR` u8 [base+0x1B] (both 0); out ints `o24` [base+0x24], `o2C` [base+0x2C] (PlaceFurniture a7/a8), `o34` [base+0x34] = outWall, `o28` [base+0x28] = outPos (PlaceFurnitureOnWall); worktop slot arrays (32 entries each, no bound check): `Y[]` f32 at base+0x38, `X[]` f32 at base+0xB8, `Rot[]` f32 at base+0x138, `Mark[]` 32 dwords at base+0x1B8 (zeroed with `rep stosd`); `count` [base+0x1C].
- Behaviour (original order). `id = (s8)this[0x791]`, `wl = m_box->status`.
  1. `f = GetFurniture(4, 7, id, wl)`. `if PlaceFurniture(f, 0, D1, 0.0, 1, 1, &o24, &o2C, 0) != 0: cornerL = 1`. `if PlaceFurniture(f, W1, D1, 0.0, 1, 0, &o24, &o2C, 0) != 0: cornerR = 1` (same f).
  2. `L = max_s8(lDoorEnd, lWindowEnd)` (signed byte compare, `lDoorEnd` kept when greater), sign-extended, then `L = (L <= 0) ? 0 : L`. `R = max_s8(rDoorEnd, rWindowEnd)`, same clamp.
     - R > 0: `r = R`; `l = (L > 0) ? L : 0`.
     - R == 0 and L > 0: `l = L; r = 0`.
     - R == 0 and L == 0: `l = GetRandomNumberInRange(0, D1)` (0x407180, cdecl, pushed `D1` then `0`; called at 0x596A26), then `r = GetRandomNumberInRange(0, D1)` (0x596A30). Formula of the original: `min + ftol((rand() & 0xFFFF) * (1/32768) * (max-min))`, so 0..D1-1.
  3. `if l > 0: SetTilesStatus(0, 0, 1, l, 2, 0)`. `if r > 0: SetTilesStatus(W1, 0, 1, r, 2, 0)`.
  4. `PlaceFurniture(GetFurniture(4, 0, id, wl), 0, l, 0.0, 1, 1, &o24, &o2C, 0)`; `PlaceFurniture(GetFurniture(4, 2, id, wl), W1, r, 0.0, 1, 3, &o24, &o2C, 0)` (results ignored).
  5. `a8 = (tWindowStart == 0xFF) ? -1 : (s8)tWindowStart`. `if PlaceFurnitureOnWall(4, 3, id, 0.0, 1, 0, a8, 0, &o34, &o28, 0, 0, 0, 0) != 0`: `o28 += 1`; `AddInteriorInfo(5, (f32)o28, (f32)D - 1.5, 2, NULL)` (y = fild D, fsub 1.5; x = fild o28).
  6. `PlaceFurnitureOnWall(4, s, id, 0.0, 1, -1, -1, 0, &o34, &o28, 0, 0, 0, 0)` for s = 5, then 4, then 6 (results ignored).
  7. `f = GetFurniture(4, 1, id, wl)`; zero Mark[0..31]; `count = 0`.
     - Loop A, ascending `y = l+1; y < D1; y++`: `if PlaceFurniture(f, 0, y, 0.0, 1, 1, &o24, &o2C, 0)`: `X[count]=0.5; Rot[count]=90.0; Y[count]=(f32)y + 0.5; count++`.
     - Loop B (only if W1 > 1), ascending `x = 1; x < W1; x++`: `if PlaceFurniture(f, x, D1, 0.0, 1, 0, ...)`: `Rot[count]=0.0; X[count]=(f32)x+0.5; Y[count]=(f32)D1+0.5; count++`.
     - Loop C, ascending `y = r+1; y <= D1-1; y++`: `if PlaceFurniture(f, W1, y, 0.0, 1, 3, ...)`: `Rot[count]=270.0; X[count]=(f32)W1+0.5; Y[count]=(f32)y+0.5; count++`.
  8. `f10 = GetFurniture(4, 10, -1, wl)`. `n = rand()` (0x596D8B, raw). If `n < 0x3FFF && cornerL`: `PlaceObject(1, f10, 0.5, (f32)D - 0.5, 1.05f, 45.0)`. Else if `cornerR`: `PlaceObject(1, f10, (f32)W - 0.5, (f32)D - 0.5, 1.05f, 315.0)`. (So with n < 0x3FFF but no left corner, the right corner is still tried.) The rand is consumed in all cases.
  9. If `count <= 0`: return. Else `cf = (f32)count` (stored f32).
     - `i1 = ftol((f32)(rand() & 0xFFFF) * 1/32768 * cf)` (rand at 0x596E2A; x87: fild, fmul [0x858B14], fmul cf, _ftol truncation). `Mark[i1] = 0x3F800000` (1.0f bits).
     - `i2 = ftol((rand()&0xFFFF) * 1/32768 * cf)` (0x596E5B). If `Mark[i2] == 0x3F800000` (integer compare): retry loop `k = 0; while k < 30: i2 = ftol(rand()... * cf) (0x596E95); k++; if Mark[i2] != 1.0f break`. After the loop `if k == 30: i2 = -1`. (Bug kept: a free slot found on the 30th retry is also discarded.) With count == 1, i2 always ends -1.
  10. Clutter chance `p` from `wl` (unsigned): `wl >= 75`: `p = ftol(r * 20.0)` (rand 0x596ED6, 0..19); `50 <= wl < 75`: `p = 20 - ftol(r * -30.0)` (0x596F01, 20..49); `wl < 50`: `p = 50 - ftol(r * -50.0)` (0x596F2B, 50..99); `r = (rand() & 0xFFFF) * 1/32768`. ftol truncates toward zero, so the negative products give -29..0 / -49..0.
  11. Ascending `i = 0; i < count; i++` (count re-read each iteration): if `(f32)Mark[i] == 0.0` (x87 fcomp with [0x858B50]; skipped when not equal or unordered): `q = ftol((rand()&0xFFFF) * 1/32768 * 100.0)` (0x596F88); if `q < p`: `n = rand()` (0x596FAF, raw); `fc = GetFurniture(8, n < 0x3FFF ? 3 : 4, -1, wl)`; if `fc != NULL`: `PlaceObject(0, fc, X[i], Y[i], 1.05f, Rot[i])` (not stealable). Note slot i2 is not marked, so it can also receive clutter.
  12. `if i1 != -1` (always true): `PlaceObject(1, GetFurniture(4, 8, -1, wl), X[i1], Y[i1], 1.05f, Rot[i1])`.
  13. `if i2 != -1`: `PlaceObject(1, GetFurniture(4, 9, -1, wl), X[i2], Y[i2], 1.05f, Rot[i2])`.
- RNG summary: 0x407180 x2 only when both side runs are zero; raw rand at 0x596D8B, 0x596FAF; scaled `ftol((rand()&0xFFFF)*(1/32768)*k)` at 0x596E2A, 0x596E5B, 0x596E95 (loop), one of 0x596ED6/0x596F01/0x596F2B, and 0x596F88 per unmarked slot.
- Edge cases / original bugs: arrays have 32 entries but `count` is not bounded (loops A+B+C can exceed 32 on large rooms and overwrite the next array / Mark); 30th-retry discard; i2's slot not excluded from clutter; f/f10/GetFurniture results passed to PlaceFurniture/PlaceObject without null checks except in step 11. `Mark` is an int array read as f32 in step 11.
- gta-reversed: plugin::Call stub only.
- Open questions: none on control flow. (The meaning of PlaceFurniture a4/a5 and PlaceFurnitureOnWall a5..a9 belongs to those functions' specs.)

## 0x5970b0 Interior_c::FurnishKitchen  (catalogue: specified/REA, port level 17)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, `ret`, void.
- Callers: 2 (Interior_c::Init 0x593BF0 when m_box->type == 4; Interior_c::Furnish 0x591590 tail call). Callees: 0x591700 SetTilesStatus `(x, y, w, d, status, force)` x5 sites; 0x591D20 AddGotoPt `(int32 x, int32 y, f32, f32)` (`ret 0x10`) x4; 0x591220 FurnitureManager_c::GetRandomId (this = 0xBAB380, `(int32 group, int32 subGroup, u8 wealth)`); 0x596930 Kitchen_FurnishEdges; 0x5911E0 FurnitureManager_c::GetFurniture; 0x592AA0 PlaceFurniture; 0x821B40 _ftol x2.
- Globals: 0xBAB380 g_furnitureMan; f32 [0x858B8C]=0.5.
- Fields: `this+0x14` m_box: `+2` width W, `+3` depth D, `+5` door (s8), `+0x1F` status. Writes `this+0x791` (u8, m_furnitureId). Furniture_c `+0xC` widthX (u8), `+0xD` widthY (u8).
- Behaviour:
  1. `SetTilesStatus((s8)door - 1, 0, 2, 1, 7, 0)` (2x1 tiles in front of the door, status 7).
  2. Ascending `x = 1; x <= W-2; x++` (skipped if W-2 < 1): `SetTilesStatus(x, D-2, 1, 1, 3, 0)`; `SetTilesStatus(x, 0, 1, 1, 3, 0)`.
  3. Ascending `y = 0; y <= D-2; y++` (skipped if D-2 < 0): `SetTilesStatus(1, y, 1, 1, 3, 0)`; `SetTilesStatus(W-2, y, 1, 1, 3, 0)`. (Ring of status-3 tiles one tile in from the walls; note row y=0 rather than y=1 on the x loop.)
  4. `AddGotoPt(1, 1, 0.0, 0.0)`; `AddGotoPt(1, D-2, 0.0, 0.0)`; `AddGotoPt(W-2, 1, 0.0, 0.0)`; `AddGotoPt(W-2, D-2, 0.0, 0.0)`.
  5. `this[0x791] = (u8)GetRandomId(4, 0, status)` (low byte of EAX).
  6. `Kitchen_FurnishEdges()`.
  7. `f = GetFurniture(8, 1, -1, status)`; `ty = ftol((f32)D * 0.5 - (f32)f->widthY * 0.5)`; `tx = ftol((f32)W * 0.5 - (f32)f->widthX * 0.5)` (x87: fild, fmul 0.5 each, `fsubp st(1)` = first minus second, then `_ftol` truncation toward zero; ty is computed first). `PlaceFurniture(f, tx, ty, 0.0, 0, 0, &tmpA, &tmpB, 0)` (out ints are stack locals, discarded). No null check of `f` before reading its width bytes.
- RNG: none directly (GetRandomId / Kitchen_FurnishEdges consume rand()).
- gta-reversed: plugin::Call stub only.
- Open questions: none.

## 0x597240 Interior_c::Lounge_AddTV  (catalogue: specified/REA, port level 16)
- Convention / signature: `__thiscall` (this = ECX), `void Lounge_AddTV(int32 corner [esp+4], int32, int32, int32)`, `ret 0x10`. Only arg 1 is read; args 2..4 are ignored. Returns EAX = second PlaceObject result (callers ignore it).
- Callers: 1 (FurnishLounge 0x597740). Callees: 0x591E40 AddInteriorInfo; 0x5911E0 FurnitureManager_c::GetFurniture x2 (on 0xBAB380); 0x5934E0 PlaceObject x2; 0x821B1E rand x1.
- Globals: f32 [0x858624]=1.0, [0x858B8C]=0.5, [0x858CA0]=2.0, [0x858CB4]=45.0, [0x85991C]=90.0.
- Fields: `this+0x14` m_box: `+2` width W, `+3` depth D, `+0x1F` status.
- Behaviour: `r = corner & 3`. Four f32 locals are set per case: TV position (tx, ty) and a second item position (sx, sy); then an info is added:
  | corner | AddInteriorInfo(type 0, x, y, dir -1, ent NULL) | (tx, ty) | (sx, sy) |
  |---|---|---|---|
  | 0 | (1.0, D-2.0) | (0.5, D-0.5) | (1.5, D-0.5) |
  | 2 | (W-2.0, 1.0) | (W-0.5, 0.5) | ((W-0.5)-1.0, 0.5) |
  | 1 | (1.0, 1.0) | (0.5, 0.5) | (0.5, 1.5) |
  | 3 | (W-2.0, D-2.0) | (W-0.5, D-0.5) | (W-0.5, (D-0.5)-1.0) |
  Compare order 0, 2, 1, 3. All values computed in x87 from `fild` of the byte and the f32 constants, stored f32 (exact).
  Then, for every corner value (also unmatched ones, which skip AddInteriorInfo):
  1. `f = GetFurniture(2, 3, -1, status)`; `rz = (f32)((f32)r * 90.0)` (stored f32); `PlaceObject(1, f, tx, ty, 0.5, rz + 45.0)` (TV, diagonal into the corner).
  2. `n = rand()` (0x821B1E, raw, no mask). `sub = (n < 0x3FFF) ? 7 : 9` (signed compare). `f2 = GetFurniture(2, sub, -1, status)`.
  3. `PlaceObject(1, f2, sx, sy, 0.5, rz)`.
- Edge cases / bugs: for corner outside 0..3, tx/sx/sy are uninitialised stack locals and ty is the raw `corner` int reinterpreted as f32; the objects are still placed (keep-original). The TV is always stealable (arg 1 = 1).
- RNG: one raw `rand()` at 0x5973D3, threshold 0x3FFF (rand() in 0..0x7FFF), so subgroup 7 has probability 16383/32768.
- gta-reversed: plugin::Call stub only (declares 4 args; behaviour uses only the first).
- Open questions: none.

## 0x597430 Interior_c::Lounge_AddHifi  (catalogue: specified/REA, port level 16)
- Convention / signature: `__thiscall` (this = ECX), `CObject* Lounge_AddHifi(int32 side [esp+4], int32 tileX [esp+8], int32 tileY [esp+0xC], int32 unused [esp+0x10])`, `ret 0x10`. Returns EAX = PlaceObject result. The 4th argument is never read.
- Callers: 1 (FurnishLounge 0x597740). Callees: 0x5911E0 FurnitureManager_c::GetFurniture (on 0xBAB380), 0x5934E0 Interior_c::PlaceObject.
- Globals: f32 [0x858B8C]=0.5, f32 [0x85991C]=90.0.
- Fields: `this+0x14` m_box `+0x1F` status (wealth).
- Behaviour:
  1. `fx = (f32)tileX; fy = (f32)tileY; r = side & 3`.
  2. side 0 or 2: `fx += 0.5`; side 1 or 3: `fy += 0.5`; else unchanged.
  3. `f = GetFurniture(2, 8, -1, m_box->status)`.
  4. `rotZ = (f32)r * 90.0`; `y = fy + 0.5`; `x = fx + 0.5`.
  5. return `PlaceObject(1, f, x, y, 0.5, rotZ)`.
- gta-reversed: plugin::Call stub (declares 4 int args, correct count).
- Open questions: none.

## 0x5974e0 Interior_c::Lounge_AddChairInfo  (catalogue: specified/REA, port level 3)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), `void Lounge_AddChairInfo(int32 side [esp+4], int32 pos [esp+8], CEntity* ent [esp+0xC])`, `ret 0xC`. Every path tail-ends in `call 0x591E40` followed by `ret 0xC` (EAX = whatever AddInteriorInfo returned; callers ignore it).
- Callers: 1 (FurnishLounge 0x597740, 3 sites). Callees: 0x591E40 Interior_c::AddInteriorInfo (thiscall `(int32 type, f32 x, f32 y, int32 dir, CEntity* ent)`, `ret 0x14`).
- Globals: f32 [0x858624]=1.0, f32 [0x858B8C]=0.5.
- Fields: `this+0x14` m_box (tEffectInterior*): `+2` width (u8), `+3` depth (u8).
- Behaviour:
  1. `dir = (side - 2) & 3` (computed for all cases).
  2. Dispatch on `side` (compare order 0, 2, 1, 3):
     - side 0: `y = (f32)(i32)(depth - 1) - 1.0`; `x = (f32)pos + 0.5`; `AddInteriorInfo(1, x, y, dir, ent)`.
     - side 2: `x = (f32)pos + 0.5`; `AddInteriorInfo(1, x, 1.0, dir, ent)`.
     - side 1: `y = (f32)pos + 0.5`; `AddInteriorInfo(1, 1.0, y, dir, ent)`.
     - side 3: `y = (f32)pos + 0.5`; `x = (f32)(i32)(width - 1) - 1.0`; `AddInteriorInfo(1, x, y, dir, ent)`.
     - any other side: nothing, return.
  Each float is one x87 `fild`, one `fadd`/`fsub` with the f32 constant, stored as f32 (exact for all byte-range ints).
- Edge cases: the side-0/3 paths overwrite the caller's `ent` argument slot with `depth-1` / `width-1` after `ent` has already been pushed (no observable effect). Info type is always 1.
- gta-reversed: plugin::Call stub only (`.Reversed = false`). gr names the args `(a2, a3, entityIgnoredCollision)`.
- Open questions: none.

## 0x5975c0 Interior_c::Lounge_AddSofaInfo  (catalogue: specified/REA, port level 3)
- Convention / signature: `__thiscall` (this = ECX), `void Lounge_AddSofaInfo(int32 side [esp+4], int32 pos [esp+8], CEntity* ent [esp+0xC])`, `ret 0xC`. Returns void (EAX = last AddInteriorInfo result or `side` when no case matched).
- Callers: 1 (FurnishLounge 0x597740). Callees: 0x591E40 Interior_c::AddInteriorInfo x2 per side (`(type, f32 x, f32 y, dir, ent)`, `ret 0x14`).
- Globals: f32 [0x858624]=1.0, f32 [0x858B8C]=0.5.
- Fields: `this+0x14` m_box: `+2` width, `+3` depth (u8).
- Behaviour: `dir = (side - 2) & 3`. `p = (f32)((f32)pos + 0.5)` (stored to the `pos` stack slot as f32); the second seat uses `p2 = (f32)(p + 1.0)` loaded back from that f32 slot. Dispatch order 0, 2, 1, 3:
  - side 0: `y = (f32)(depth-1) - 1.0` (f32). `AddInteriorInfo(1, p, y, dir, ent)`; then `AddInteriorInfo(1, p2, y, dir, ent)`.
  - side 2: `AddInteriorInfo(1, p, 1.0, dir, ent)`; `AddInteriorInfo(1, p2, 1.0, dir, ent)`.
  - side 1: `AddInteriorInfo(1, 1.0, p, dir, ent)` (the type argument is pushed from EAX, which equals `side` = 1); `AddInteriorInfo(1, 1.0, p2, dir, ent)`.
  - side 3: `x = (f32)(width-1) - 1.0`. `AddInteriorInfo(1, x, p, dir, ent)`; `AddInteriorInfo(1, x, p2, dir, ent)`.
  - other: nothing.
- Edge cases: gr names arg1 `sitType`, it is the wall side (0..3), same as Lounge_AddChairInfo. Values are small ints, all float ops exact.
- gta-reversed: plugin::Call stub only.
- Open questions: none.

## 0x597740 Interior_c::FurnishLounge  (catalogue: specified/REA, port level 17)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, `ret`, void. Frame `sub esp,0x28` + push ebx/ebp/esi/edi; locals relative to ESP after the pushes ("base"): `wall`/`corner` [base+0x10], `pos` [base+0x14], `tvCorner` [base+0x18] (later reused as loop var / tile x), `perim` [base+0x1C] (later rand scratch / tile coord), `cy` [base+0x20], `cx` [base+0x24], `hy` [base+0x28], `hx` [base+0x2C], `cd` [base+0x30], `cw` [base+0x34].
- Callers: 2 (Interior_c::Init 0x593BF0 when m_box->type == 2; Interior_c::Furnish 0x591590 tail call). Callees: 0x591700 SetTilesStatus `(x, y, w, d, status, force)`; 0x593340 PlaceFurnitureInCorner (`(group, subGroup, int16 id, f32 z, int32 a6, int32 corner, int32 offset, int32* outCorner, int32* outX, int32* outY, int32* outW, int32* outD)`, corner -1 = random; returns the placed entity or 0); 0x597240 Lounge_AddTV; 0x5917C0 SetCornerTiles `(corner, len, status, force)`; 0x591D20 AddGotoPt; 0x591220 FurnitureManager_c::GetRandomId; 0x593120 PlaceFurnitureOnWall (same signature as in FurnishBedroom: `(group, sub, id, f32 z, a6, wall, pos, offset, outWall, outPos, outX, outY, outW, outD)`, returns the placed entity or 0); 0x5975C0 Lounge_AddSofaInfo; 0x5974E0 Lounge_AddChairInfo; 0x597430 Lounge_AddHifi; 0x591C50 FindEmptyTiles `(w, d, int32* outX, int32* outY)` -> AL; 0x5911E0 GetFurniture; 0x5934E0 PlaceObject; 0x592AA0 PlaceFurniture; 0x5918E0 GetTileStatus `(x, y)` -> EAX; 0x591E40 AddInteriorInfo; 0x821B1E rand x6; 0x821B40 _ftol x8.
- Globals: 0xBAB380 g_furnitureMan; f32 [0x858B14]=1/32768, [0x858BA4]=20.0, [0x859CE4]=-30.0, [0x859B60]=-50.0, [0x858B34]=60.0, [0x858628]=100.0, [0x858B8C]=0.5; immediate 0x3D4CCCCD = 0.05f.
- Fields: `this+0x14` m_box: `+2` W, `+3` D, `+5` door (s8), `+0x1F` status. `this+0x791` m_furnitureId (written). Furniture_c `+0xC` widthX, `+0xD` widthY.
- Behaviour (original order):
  1. `perim = (D + W) * 2`. `SetTilesStatus(door-1, 0, 2, 1, 7, 0)`; `SetTilesStatus(door-2, 0, 1, 1, 2, 0)`; `SetTilesStatus(door+1, 0, 1, 1, 2, 0)` (door = (s8)m_box[5]).
  2. `tvCorner = -1`. `if PlaceFurnitureInCorner(2, 2, -1, 0.0, 1, -1, 0, &corner, &cx, &cy, &cw, &cd)`: `Lounge_AddTV(corner, cx, cy, 0)`; `tvCorner = corner`.
  3. `SetCornerTiles(corner, 2, 2, 1)` — executed even when step 2 failed, with `corner` uninitialised (original bug).
  4. Ring: ascending `x = 1..W-2` (skip if W-2 < 1): `SetTilesStatus(x, D-2, 1, 1, 3, 0)`; `SetTilesStatus(x, 1, 1, 1, 3, 0)`. Ascending `y = 1..D-2` (skip if D-2 < 1): `SetTilesStatus(1, y, 1, 1, 3, 0)`; `SetTilesStatus(W-2, y, 1, 1, 3, 0)`. (Differs from FurnishKitchen, which uses row 0 and starts y at 0.)
  5. `AddGotoPt(1, 1, 0.0, 0.0)`; `AddGotoPt(1, D-2, ...)`; `AddGotoPt(W-2, 1, ...)`; `AddGotoPt(W-2, D-2, ...)`.
  6. `SetCornerTiles(c, 2, 2, 0)` for c = 0, 2, 1, 3 (block the corners while placing wall items).
  7. `id = (s8)(this[0x791] = (u8)GetRandomId(2, 0, status))`.
  8. Sofa: `e = PlaceFurnitureOnWall(2, 0, id, 0.0, 1, -1, -1, 0, &wall, &pos, NULL, NULL, NULL, NULL)`; if `e`: `Lounge_AddSofaInfo(wall, pos, e)`; `if PlaceFurnitureOnWall(2, 4, -1, 0.0, 1, wall, pos, 2, NULL x6) == 0`: `PlaceFurnitureOnWall(2, 4, -1, 0.0, 1, wall, pos, 3, NULL x6)` (table in front of the sofa at offset 2, else 3).
  9. Chair 1: `e = PlaceFurnitureOnWall(2, 1, id, 0.0, 1, -1, -1, 0, &wall, &pos, NULL x4)`; if `e`: `Lounge_AddChairInfo(wall, pos, e)`.
  10. Chair 2 only if `perim > 28` (signed): same call and same info.
  11. Reopen corners: for c in order 0, 2, 1, 3, `if c != tvCorner: SetCornerTiles(c, 2, 0, 0)`.
  12. Hi-fi: `if PlaceFurnitureOnWall(2, 6, -1, 0.0, 1, -1, -1, 0, &wall, NULL, &hx, &hy, NULL, NULL)`: `Lounge_AddHifi(wall, hx, hy, 0)`.
  13. `PlaceFurnitureOnWall(2, 5, -1, 0.0, 1, -1, -1, 0, NULL x6)`; `PlaceFurnitureOnWall(8, 0, -1, ...same)` twice (results ignored).
  14. Clutter threshold `p` (unsigned status): `>= 75`: `p = ftol(r*20.0)` (rand 0x597B8E); `>= 50`: `p = 20 - ftol(r*-30.0)` (0x597BB9); else `p = 50 - ftol(r*-50.0)` (0x597BE3). `r = (rand() & 0xFFFF) * 1/32768` (x87: fild, fmul [0x858B14], fmul const, _ftol truncation).
  15. Clutter, each `q = ftol((rand()&0xFFFF) * 1/32768 * k)`, `if q < p && FindEmptyTiles(w, d, &tx, &ty)`: `PlaceObject(0, GetFurniture(8, s, -1, status), (f32)tx + 0.5, (f32)ty + 0.5, 0.05f, 0.0)` (y computed first); `SetTilesStatus(tx, ty, sw, sd, 2, 0)`:
     | rand at | k | w,d | s | sw,sd | tx / ty slots |
     |---|---|---|---|---|---|
     | 0x597C0D | 60.0 | 2,2 | 2 | 2,2 | [base+0x1C] / [base+0x18] |
     | 0x597CAE | 100.0 | 1,1 | 5 | 1,1 | [base+0x18] / [base+0x1C] |
     | 0x597D4F | 100.0 | 1,1 | 4 | 1,1 | [base+0x18] / [base+0x1C] |
  16. Rug: `f = GetFurniture(8, 1, -1, status)`; `ty = ftol((f32)D*0.5 - (f32)f->widthY*0.5)`; `tx = ftol((f32)W*0.5 - (f32)f->widthX*0.5)` (`fsubp st(1)` = first minus second; ty first); `PlaceFurniture(f, tx, ty, 0.0, 0, 0, &cw, &cd, 0)`.
  17. Wall look-at infos, loop 1 ascending `x = 0; x < W-1` (W re-read each iteration; skipped if W-1 <= 0): `s0 = GetTileStatus(x, 0)`; `s1 = GetTileStatus(x, D-1)`; `if s0 == 0 || s0 == 2: AddInteriorInfo(2, (f32)x, 0.0, 2, NULL)`; `if s1 == 0 || s1 == 2: AddInteriorInfo(2, (f32)x, (f32)(D-1), 0, NULL)`.
  18. Loop 2 ascending `y = 1; y < D-2` (skipped if D-2 <= 1): `s0 = GetTileStatus(0, y)`; `s1 = GetTileStatus(W-1, y)`; `if s0 ∈ {0,2}: AddInteriorInfo(2, 0.0, (f32)y, 1, NULL)`; `if s1 ∈ {0,2}: AddInteriorInfo(2, (f32)(W-1), (f32)y, 3, NULL)`. (Ranges are asymmetric: x covers 0..W-2, y covers 1..D-3; keep.)
- RNG: only steps 14/15 here (scaled `ftol((rand()&0xFFFF)*(1/32768)*k)` form, 4 rand calls) plus raw rand inside Lounge_AddTV; callees PlaceFurnitureInCorner/PlaceFurnitureOnWall/FindEmptyTiles/GetRandomId consume more.
- Edge cases / original bugs (keep): uninitialised `corner` in step 3 when no TV; `wall`/`pos` stale from the previous call when passed on; no null check on GetFurniture results; the 2x2 clutter writes tx into [base+0x1C] and ty into [base+0x18] while the 1x1 ones use the opposite slots (no behavioural effect).
- gta-reversed: plugin::Call stub only.
- Open questions: none on control flow.

## 0x599210 Interior_c::Office_PlaceEdgeFillers  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 5 stack args: `[esp+4] int32 subGroupId` (-1 = pick by random roll), `[esp+8] int32 tileX`, `[esp+0xc] int32 tileY`, `[esp+0x10] int32 dir` (wall side 0..3), `[esp+0x14] int32` unused (never read). Returns EAX int32 = number of tiles to advance (see below); `ret 0x14`.
- Callers: 1 — 0x599770 Office_FurnishEdges (4 sites, always subGroupId = -1, arg5 = dir). Callees: 0x821b1e `rand`, 0x821b40 `_ftol`, 0x407180 CGeneral::GetRandomNumberInRange(int,int) (original helper), 0x591920 Interior_c::GetNumEmptyTiles, 0x5911e0 FurnitureManager_c::GetFurniture (this = &g_furnitureMan 0xbab380), 0x591e40 Interior_c::AddInteriorInfo, 0x592aa0 Interior_c::PlaceFurniture.
- Globals: 0xbb3dc8 `uint8 s_officeFillerOncePlaced` (unnamed .bss byte, initial 0; R/W; only referenced at 0x5992c1 and 0x59930e in the whole exe, so it is never reset: once set it stays 1 for the process lifetime). 0xbab380 g_furnitureMan (address only). Constants 0x858b14 f32 1/32768, 0x858628 f32 100.0.
- Fields: `this+0x14` tEffectInterior* (gr `m_box`): `+0x1f` u8 (gr `m_status`, passed as the furniture wealth/rating).
- Locals: `r` ([esp+0x10] after the prologue), `outX` ([esp+0x14]), `outY` ([esp+0x18]); `infoX` is stored in the tileY argument slot and `infoY` in the dir argument slot (the original tileX/tileY stay in ESI/EBP and arg slots 2 for the PlaceFurniture call; dir stays in EBX). Storage only, no behavioural effect.
- Behaviour (original order):
  1. `r = ftol((rand()@0x599219 & 0xFFFF) * f32[0x858b14] * f32[0x858628])` (the `and 0xffff` is displaced to 0x405bef; `fild`, `fmul 1/32768`, `fmul 100.0f`, `_ftol`). r in 0..99 (exact = (rand*100)>>15). Drawn unconditionally, before anything else.
  2. `scan = (dir == 2 || dir == 0) ? 1 : 2` (0x599240..0x599252).
  3. `n = GetNumEmptyTiles(tileX, tileY, scan, 1)` (0x599266). `if (n < 1)` (signed) → return 1 (0x599386).
  4. Front tile for the interior info: `infoX = tileX; infoY = tileY;` then `if dir==2: infoY = tileY+1; else if dir==0: infoY = tileY-1; else if dir==3: infoX = tileX-1; else if dir==1: infoX = tileX+1;` (other dir values: unchanged).
  5. `rot = dir` (EBX). Furniture selection:
     - `if (subGroupId != -1)` (0x5992b0) → `furn = GetFurniture(1, subGroupId, -1, box->status)` (0x599395..0x5993a8); go to 6.
     - else `if (r > 90 && s_once == 0)` (0x5992ba/0x5992c5): `furn = GetFurniture(1, 2, -1, box->status)`; `if (furn) AddInteriorInfo(7, (float)infoX, (float)infoY, (dir-2)&3, nullptr)` (0x599307; floats via `fild`/`fstp` f32; `lea ecx,[ebx-2]; and ecx,3`); then **unconditionally** `s_once = 1` (0x59930c, also when furn is null); go to 6.
     - else `if (r > 75)` (note: also reached when r > 90 but s_once != 0): `rot = CGeneral::GetRandomNumberInRange(0, 5)` called at 0x599321 — cdecl, pushes 5 then 0, i.e. min = 0, max = 5; original formula `0 + ftol((rand() & 0xFFFF) * (1/32768) * 5)` → 0..4 (value 4 is possible; it replaces the rotation passed to PlaceFurniture). Then `furn = GetFurniture(8, 0, -1, box->status)` (group 8, sub-group 0); go to 6. No interior info.
     - else `if (r > 25)`: `furn = GetFurniture(1, 3, -1, box->status)`; `if (furn) AddInteriorInfo(7, (float)infoX, (float)infoY, (dir-2)&3, nullptr)` (0x59937f); go to 6.
     - else (r <= 25): return 1 (0x599386). No furniture.
  6. `PlaceFurniture(furn, tileX, tileY, 0.0f, 1, rot, &outX, &outY, 0)` (0x5993c9; pushes: 0, &outY, &outX, rot, 1, 0 (= 0.0f), tileY, tileX, furn). Its return value is ignored. Return `outX` (the value PlaceFurniture wrote through its 7th argument; PlaceFurniture writes 0 there on failure).
- RNG summary: rand at 0x599219 (always, percent roll); rand inside 0x407180 called from 0x599321 (only on the r>75 branch). GetFurniture/PlaceFurniture may draw more (their specs).
- Edge cases / oddities:
  - `furn` is passed to PlaceFurniture without a null check on every path (the r>75 path and the subGroupId path do not even test it). Whether PlaceFurniture tolerates null is in its spec (it reads `furn+0xc/+0xd` after a count check at 0x592ac6).
  - Return 0 is possible (PlaceFurniture failure → outX = 0); the caller loops `x += ret`, so it retries the same tile with a new roll (terminates only by a later roll ≤ 25 or a success).
  - rotation 4 from the r>75 branch is passed through to PlaceFurniture.
  - arg5 is never read.
- gta-reversed: `plugin::CallMethodAndReturn<bool, 0x599210>` (not reversed; declared return `bool` is wrong — the original returns an int32 advance count). If ported from gr later, the 0x407180 call must use the original formula (cat14 finding 1).
- Open questions: none.

## 0x5993e0 Interior_c::Office_PlaceDesk  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 6 stack args: `[esp+4] int32 tileX`, `[esp+8] int32 tileY`, `[esp+0xc] int32 dir` (0..3), `[esp+0x10] int32` unused, `[esp+0x14] uint8` unused, `[esp+0x18] int32 deskId` (furniture id within group 1 / sub-group 0). Returns EAX int32: 1 if the desk could not be placed, 2 otherwise; `ret 0x18`.
- Callers: 0x5995b0 Office_PlaceEdgeDesks (args 4/5 = 0x46, 0), 0x599960 Office_PlaceDeskQuad (4 sites, same 0x46, 0). Callees: 0x5911e0 FurnitureManager_c::GetFurniture (this = &g_furnitureMan 0xbab380) ×2, 0x592aa0 Interior_c::PlaceFurniture ×2, 0x591e40 Interior_c::AddInteriorInfo, 0x591700 Interior_c::SetTilesStatus.
- Globals: 0xbab380 g_furnitureMan (address only); constant 0x858b8c f32 0.5.
- Fields: `this+0x14` tEffectInterior* (`+0x1f` u8 status, used as wealth); `this+0x792` int8 (gr `field_792`; office chair furniture id, set by FurnishOffice 0x599b94).
- Behaviour:
  1. `rot = (dir - 2) & 3` (ESI, kept for the whole function).
  2. Desk tile: `dX = tileX; dY = tileY; if (dir == 2) dY = tileY + 1; else if (dir == 1) dX = tileX + 1;`
  3. `desk = GetFurniture(1, 0, (int16)deskId, box->status)` (0x599434; pushes status, deskId, 0, 1).
  4. `obj = PlaceFurniture(desk, dX, dY, 0.0f, 1, rot, &outX, &outY, 0)` (0x599457; outX = [esp+0x18] local, outY = [esp+0x14] local). `if (obj == 0) return 1` (0x59945e → 0x599596). No null check on `desk` before the call.
  5. Chair tile (EBX/EBP = cX/cY used for PlaceFurniture), info position (iX/iY), and status tile (sX/sY), all start at tileX/tileY:
     - dir 2: `cX = iX = tileX + 1` (sX = tileX, sY = tileY, cY = iY = tileY).
     - dir 0: `cY = iY = sY = tileY + 1; sX = tileX + 1` (cX = iX = tileX).
     - dir 3: `cX = iX = sX = tileX + 1; cY = iY = tileY + 1` (sY = tileY).
     - dir 1: `sY = tileY + 1` (cX = iX = sX = tileX, cY = iY = tileY).
     - other: all unchanged.
  6. `chair = GetFurniture(1, 1, (int16)(int8)this[0x792], box->status)` (0x5994d6; `movsx ax, byte [edi+0x792]`, pushed as a dword whose low 16 bits are the sign-extended id).
  7. `chairObj = PlaceFurniture(chair, cX, cY, 0.0f, 1, rot, &outX, &outY, 1)` (0x5994f1; last arg 1 = skip the tile-empty test). Return value kept in EAX for step 9.
  8. Info position floats: `fx = (float)iX; fy = (float)iY` (each `fild` + `fstp` to an f32 slot). Then `if rot == 2: fy = fy + 0.5f; else if rot == 0: fy = fy - 0.5f; else if rot == 3: fx = fx - 0.5f; else if rot == 1: fx = fx + 0.5f;` (constant 0x858b8c 0.5f; `fld` f32, `fadd/fsub` f32, `fstp` f32).
  9. `AddInteriorInfo(6, fx, fy, (rot - 2) & 3, chairObj)` (0x599569; the last arg is the PlaceFurniture return of step 7, possibly 0). Note `(rot-2)&3 == dir & 3`.
  10. `SetTilesStatus(sX, sY, 1, 1, 2, 1)` (0x599582; pushes 1, 2, 1, 1, sY, sX). Return 2.
- Edge cases: args 4 and 5 are never read. The chair is placed with "skip tile test" = 1 even if its tiles are occupied. The chair result is not checked; AddInteriorInfo and SetTilesStatus run regardless. GetFurniture results are not null-checked before PlaceFurniture.
- gta-reversed: `plugin::CallMethodAndReturn<int32, 0x5993E0>` (not reversed).
- Open questions: none.

## 0x5995b0 Interior_c::Office_PlaceEdgeDesks  (catalogue: specified/REA, port level 6)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 5 stack args: `[esp+4] int32` unused (callers pass -1), `[esp+8] int32 tileX`, `[esp+0xc] int32 tileY`, `[esp+0x10] int32 dir` (wall side), `[esp+0x14] int32 side` (selects the desk offset pattern; callers pass the same value as dir). Returns EAX int32 = tiles consumed (≥ 1); `ret 0x14`.
- Callers: 1 — 0x599770 Office_FurnishEdges (4 loops). Callees: 0x821b1e `rand` ×4, 0x821b40 `_ftol` ×4, 0x591920 Interior_c::GetNumEmptyTiles, 0x5993e0 Interior_c::Office_PlaceDesk.
- Globals: constants 0x858b14 f32 1/32768, 0x858628 f32 100.0, 0x85a708 f32 -40.0, 0x858b18 f32 -2.0.
- Fields: `this+0x791` int8 (gr `m_furnitureId`; office desk id, set by FurnishOffice 0x599b78).
- Behaviour:
  1. `r1 = ftol((rand()@0x5995b8 & 0xFFFF) * f32[0x858b14] * f32[0x858628]=100.0f)` → 0..99.
  2. `r2 = ftol((rand()@0x5995df & 0xFFFF) * f32[0x858b14] * f32[0x85a708]=-40.0f)` → -39..0 (`_ftol` truncates toward zero, so r2 = -((rand*40)>>15)). `limit = 30 - r2` (30..69).
  3. `scan = (dir == 2 || dir == 0) ? 1 : 2`; `n = GetNumEmptyTiles(tileX, tileY, scan, 1)` (0x59962f). `if (n <= 1)` (signed `jg` test) → return 1.
  4. `half = n / 2` (signed, truncating: `cdq; sub eax,edx; sar 1`).
  5. `r3 = ftol((rand()@0x59964f & 0xFFFF) * 1/32768 * f32[0x858b18]=-2.0f)` → -1..0; `count = 2 - r3` (2 or 3). `if (half < count) count = half;` **else** a fourth roll replaces it: `r4 = ftol((rand()@0x599681 & 0xFFFF) * 1/32768 * -2.0f); count = 2 - r4`. (So the 4th rand is drawn only when half ≥ the first count; count is then not clamped to half.)
  6. `if (r1 > limit)` (signed) → return 1 (no desk; all 3 or 4 rolls already consumed).
  7. `pos = 0` (ESI). `if (count <= 0)` → return 1. `rot = (dir - 2) & 3`. Loop `count` times (counter in the dir arg slot, `dec ... jne`):
     - side 0: `pos += Office_PlaceDesk(tileX + pos, tileY - 1, rot, 0x46, 0, (int8)this[0x791])`
     - side 3: `pos += Office_PlaceDesk(tileX - 1, tileY + pos, rot, 0x46, 0, id)`
     - side 1: `pos += Office_PlaceDesk(tileX, tileY + pos, rot, 0x46, 0, id)`
     - side 2: `pos += Office_PlaceDesk(tileX + pos, tileY, rot, 0x46, 0, id)`
     - other: no call, pos unchanged.
     (Office_PlaceDesk returns 1 on failure, 2 on success.)
  8. Return `pos + 1`.
- RNG: rand at 0x5995b8, 0x5995df, 0x59964f always (once n > 1; the first two even when n ≤ 1), 0x599681 conditionally. All inline (`rand & 0xFFFF`, `* 1/32768`, `* k`, `_ftol`), no GetRandomNumberInRange helper. Products are exact in 24-bit precision.
- Edge cases: arg1 is never read. Office_PlaceDesk's 4th/5th args (0x46, 0) are unused by it. With an unknown `side`, the loop just spins `count` times.
- gta-reversed: `plugin::CallMethodAndReturn<int32, 0x5995B0>` (not reversed).
- Open questions: none.

## 0x599770 Interior_c::Office_FurnishEdges  (catalogue: specified/REA, port level 7)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, `ret`, void.
- Callers: 1 — 0x599af0 FurnishOffice. Callees: 0x591700 Interior_c::SetTilesStatus ×5 sites, 0x591d20 Interior_c::AddGotoPt ×4, 0x5995b0 Interior_c::Office_PlaceEdgeDesks ×4 sites, 0x599210 Interior_c::Office_PlaceEdgeFillers ×4 sites.
- Fields: `this+0x14` tEffectInterior*: `+2` u8 width W, `+3` u8 depth D, `+5` int8 door (gr `m_door`, `movsx`).
- Behaviour (SetTilesStatus args = (x, y, sizeX, sizeY, status, force); all loops ascending):
  1. `a = W - 3; b = D - 3;`
     - `for (i = 2; i <= a; i++) { SetTilesStatus(i, b, 1, 1, 3, 0); SetTilesStatus(i, 2, 1, 1, 3, 0); }`
     - `for (j = 2; j <= b; j++) { SetTilesStatus(2, j, 1, 1, 3, 0); SetTilesStatus(a, j, 1, 1, 3, 0); }`
  2. `AddGotoPt(2, 2, 0.5f, 0.5f); AddGotoPt(2, b, 0.5f, -0.5f); AddGotoPt(a, 2, -0.5f, 0.5f); AddGotoPt(a, b, -0.5f, -0.5f);` (args: tileX, tileY, f32 offX, f32 offY; immediates 0x3f000000 / 0xbf000000).
  3. Re-read the box: `wm1 = W - 1; dm1 = D - 1;` `SetTilesStatus(door - 2, 0, 4, 2, 7, 0)` (status 7 in front of the door).
  4. Desks (each call returns the advance; arg1 = -1, arg5 = same as dir):
     - `for (p = 1; p < wm1; p += Office_PlaceEdgeDesks(-1, p, 0, 2, 2));`
     - `for (p = 1; p < wm1; p += Office_PlaceEdgeDesks(-1, p, dm1, 0, 0));`
     - `for (p = 1; p <= D - 2; p += Office_PlaceEdgeDesks(-1, 0, p, 1, 1));`
     - `for (p = 1; p <= D - 2; p += Office_PlaceEdgeDesks(-1, wm1, p, 3, 3));`
  5. Fillers, same four walls and bounds in the same order: `Office_PlaceEdgeFillers(-1, p, 0, 2, 2)` (p < W-1), `(-1, p, D-1, 0, 0)` (p < W-1), `(-1, 0, p, 1, 1)` (p ≤ D-2), `(-1, W-1, p, 3, 3)` (p ≤ D-2).
  (Each loop tests its condition before the first call; the x-walls use `<` W-1, the y-walls use `<=` D-2, i.e. the same range 1..W-2 / 1..D-2.)
- Edge cases: Office_PlaceEdgeFillers can return 0 (PlaceFurniture failure), in which case the loop calls it again at the same p (new random roll each time).
- gta-reversed: `plugin::CallMethod<0x599770>` (not reversed).
- Open questions: none.

## 0x599960 Interior_c::Office_PlaceDeskQuad  (catalogue: specified/REA, port level 6)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 4 stack args: `[esp+4] int32` unused (caller passes -1), `[esp+8] int32 x`, `[esp+0xc] int32 y`, `[esp+0x10] int32 deskId`. Returns EAX = 6; `ret 0x10`.
- Callers: 1 — 0x599a30 Office_FurnishCenter. Callees: 0x5993e0 Interior_c::Office_PlaceDesk ×4, 0x591700 Interior_c::SetTilesStatus ×4.
- Globals / fields: none directly.
- Behaviour (in order; Office_PlaceDesk args = (tileX, tileY, dir, 0x46, 0, deskId), return values ignored):
  1. `Office_PlaceDesk(x, y - 2, 2, 0x46, 0, deskId)`
  2. `Office_PlaceDesk(x, y, 0, 0x46, 0, deskId)`
  3. `Office_PlaceDesk(x - 2, y, 0, 0x46, 0, deskId)`
  4. `Office_PlaceDesk(x - 2, y - 2, 2, 0x46, 0, deskId)`
  5. `SetTilesStatus(x - 3, y - 3, 6, 1, 3, 0)`; `SetTilesStatus(x - 3, y + 2, 6, 1, 3, 0)`; `SetTilesStatus(x - 3, y - 2, 1, 4, 3, 0)`; `SetTilesStatus(x + 2, y - 2, 1, 4, 3, 0)` (args: x, y, sizeX, sizeY, status, force) — a status-3 ring around the 4 desks.
  6. Return 6.
- gta-reversed: `plugin::CallMethodAndReturn<int32, 0x599960>` (not reversed).
- Open questions: none.

## 0x599a30 Interior_c::Office_FurnishCenter  (catalogue: specified/REA, port level 7)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, `ret`. EAX on return = `offY` below (gr declares int32; the only caller ignores it).
- Callers: 1 — 0x599af0 FurnishOffice. Callees: 0x599960 Interior_c::Office_PlaceDeskQuad.
- Fields: `this+0x14` tEffectInterior*: `+2` u8 width (tiles), `+3` u8 depth; `this+0x791` int8 desk id.
- Behaviour:
  1. `w6 = width - 6; d6 = depth - 6;` (int32). `nx = w6 / 6; ny = d6 / 6;` (signed division truncating toward zero, via the 0x2AAAAAAB multiply idiom). `offX = (w6 - 6*nx) / 2; offY = (d6 - 6*ny) / 2;` (remainder, then signed truncating /2 via `cdq; sub; sar 1`).
  2. `if (w6 <= 0 || d6 <= 0 || nx <= 0)` → return (ny is not tested here).
  3. Loops (outer over x ascending, inner over y ascending): `qx = offX + 6`; repeat nx times { `qy = offY + 6`; repeat ny times (skipped if ny ≤ 0) { `Office_PlaceDeskQuad(-1, qx, qy, (int8)this[0x791])`; `qy += 6` }; `qx += 6` }.
- gta-reversed: `plugin::CallMethodAndReturn<int32, 0x599A30>` (not reversed).
- Open questions: none.

## 0x599af0 Interior_c::FurnishOffice  (catalogue: specified/REA, port level 8)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, void. Ends with a tail `jmp 0x59a130` (Shop_FurnishCeiling, which is a bare `ret`).
- Callers: 0x593bf0 Interior_c::Init (box type 1), 0x591590 Interior_c::Furnish (tail call). Callees: 0x591700 SetTilesStatus ×4, 0x591220 FurnitureManager_c::GetRandomId ×2 (this = &g_furnitureMan 0xbab380), 0x599770 Office_FurnishEdges, 0x599a30 Office_FurnishCenter, 0x59a130 Shop_FurnishCeiling (tail jump).
- Fields: `this+0x14` tEffectInterior* (`+2` W, `+3` D, `+0x1f` status/wealth); writes `this+0x791` int8 (gr `m_furnitureId`, desk id) and `this+0x792` int8 (gr `field_792`, chair id).
- Behaviour:
  1. Corner blocks (status 2): `SetTilesStatus(0, 0, 2, 2, 2, 0); SetTilesStatus(0, D-2, 2, 2, 2, 0); SetTilesStatus(W-2, 0, 2, 2, 2, 0); SetTilesStatus(W-2, D-2, 2, 2, 2, 0);` (W/D re-read from the box before each call).
  2. `this[0x791] = (int8)GetRandomId(1, 0, box->status)` (0x599b70; stores AL).
  3. `this[0x792] = (int8)GetRandomId(1, 1, box->status)` (0x599b8d).
  4. `Office_FurnishEdges()`, `Office_FurnishCenter()`, then tail-jump to `Shop_FurnishCeiling()` (no-op).
- RNG: none inline; GetRandomId (0x591220) draws (its own spec).
- gta-reversed: `plugin::CallMethod<0x599AF0>` (not reversed).
- Open questions: none.

## 0x599bb0 Interior_c::Shop_Place3PieceUnit  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 5 stack args: `[esp+4] int32 subGroupBase`, `[esp+8] int32 tileX`, `[esp+0xc] int32 tileY`, `[esp+0x10] int32 rot` (0..3), `[esp+0x14] int32 length` (pieces in total, ≥ 2 expected). Returns AL = 1 (upper 24 bits of EAX = upper bits of the last PlaceFurniture return); `ret 0x14`. The arg1 and arg4 stack slots are reused as PlaceFurniture output cells.
- Callers: 1 — 0x599dc0 Shop_PlaceEdgeUnits (5 sites). Callees: 0x5911e0 FurnitureManager_c::GetFurniture ×3 per call (this = &g_furnitureMan 0xbab380), 0x592aa0 Interior_c::PlaceFurniture (`length` times).
- Fields: `this+0x790` int8 (gr `m_furnitureGroupId`, `movsx`; part of it is displaced to 0x408080), `this+0x14` box `+0x1f` status/wealth.
- Behaviour:
  1. `g = (int8)this[0x790]`. Pick the end piece and the matching other end:
     - `rot != 2 && rot != 3`: `endA = GetFurniture(g, base, -1, status)`; `endB = GetFurniture(g, base + 1, (uint16)endA->m_nId /*+0xa*/, status)`.
     - `rot == 2 || rot == 3`: `endA = GetFurniture(g, base + 1, -1, status)`; `endB = GetFurniture(g, base, (uint16)endA->m_nId, status)`.
     `endA->m_nId` is read with `mov dx, word [ebp+0xa]` after zeroing EDX, without a null check.
  2. `mid = GetFurniture(g, base + 2, (uint16)endA->m_nId, status)` (0x599c78).
  3. Placement (all with z = 0.0f, mode 1, rotation `rot`, skip-test 0; outputs `oX` → arg4 slot (7th PlaceFurniture arg), `oY` → arg1 slot (8th arg)):
     - `rot == 2 || rot == 0` (walk along x): `PlaceFurniture(endA, x, y, ..)`; `cx = x + oX`; `for k in 1..=length-2 (only if length-2 > 0): PlaceFurniture(mid, cx, y, ..); cx += oX;` then `PlaceFurniture(endB, cx, y, ..)`.
     - otherwise (rot 1/3, walk along y): `PlaceFurniture(endA, x, y, ..)`; `cy = y + oY`; `for k in 1..=length-2: PlaceFurniture(mid, x, cy, ..); cy += oY;` then `PlaceFurniture(endB, x, cy, ..)`.
     oX/oY are the footprint sizes PlaceFurniture wrote through its 7th/8th args on its last call (0 after a failed placement, so later pieces then land on the same tile). PlaceFurniture return values are ignored.
  4. Return 1.
- Edge cases: when `length < 2` the middle loop is skipped and still both ends are placed. Null furniture from GetFurniture crashes at the `+0xa` read (endA) or is handed to PlaceFurniture (mid/endB).
- gta-reversed: `plugin::CallMethodAndReturn<int8, 0x599BB0>` (not reversed).
- Open questions: none.

## 0x599dc0 Interior_c::Shop_PlaceEdgeUnits  (catalogue: specified/REA, port level 6)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 4 stack args: `[esp+4] int32 subGroupBase` (-1 = choose from a global), `[esp+8] int32 tileX`, `[esp+0xc] int32 tileY`, `[esp+0x10] int32 rot`. Returns EAX int32 = tiles consumed; `ret 0x10`.
- Callers: 0x59a1b0 Shop_FurnishEdges (3 loops), 0x59a590 Shop_FurnishAisles (2 loops). Callees: 0x591920 Interior_c::GetNumEmptyTiles, 0x821b1e `rand`, 0x821b40 `_ftol`, 0x599bb0 Interior_c::Shop_Place3PieceUnit.
- Globals: 0xbb3de4 `int32` (unnamed, R). It is the only reference to this address in the whole exe (no direct writer; the neighbouring statics 0xbb3de0/0xbb3de8 are set by static initialisers 0x84f800/0x84f820, not this one), so it keeps its image value 0. Constants 0x858b14 f32 1/32768, 0x859024 f32 -3.0.
- Behaviour:
  1. `scan = (rot == 2 || rot == 0) ? 1 : 2`; `n = GetNumEmptyTiles(tileX, tileY, scan, 1)` (0x599deb; n stored over the tileX arg slot). `if (n <= 1)` → return 1.
  2. `r = ftol((rand()@0x599e05 & 0xFFFF) * f32[0x858b14] * f32[0x859024]=-3.0f)` → -2..0 (truncation toward zero). `len = 2 - r` (2..4).
  3. `if (n == 3) len = 3; else if (n - len < 0) len = n; else if (n - len == 1) len = len - 1;` (signed; `sub ecx,esi; jns`).
  4. `if (subGroupBase != -1)`: `Shop_Place3PieceUnit(subGroupBase, tileX, tileY, rot, len)`.
     else, with `G = *(int32*)0xbb3de4`: `G > 50` → base 0; else `G > 25` → base 3; else `G > 10` → base 6; else base 9; `Shop_Place3PieceUnit(base, tileX, tileY, rot, len)`.
  5. Return `len` (regardless of placement success).
- Edge cases: since 0xbb3de4 is never written, the -1 path always uses base 9 in practice (keep the global read so a port stays faithful if something writes it through a pointer). Both callers always pass an explicit base (0/3/6/9), so the -1 path is only reachable from other code (none known).
- gta-reversed: `plugin::CallMethodAndReturn<int32, 0x599DC0>` (not reversed).
- Open questions: 0x599e56 — confirm at runtime that 0xbb3de4 is always 0 (static scan found no writer).

## 0x599ef0 Interior_c::Shop_PlaceCounter  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 1 stack arg `[esp+4] uint8 leftOfDoor` (read as a byte). Returns EAX int32 = tile x of the shopkeeper spot (see below); `ret 4`.
- Callers: 1 — 0x59a030 Shop_PlaceFixedUnits. Callees: 0x5911e0 FurnitureManager_c::GetFurniture ×2 (this = &g_furnitureMan 0xbab380), 0x821b1e `rand`, 0x821b40 `_ftol`, 0x592aa0 Interior_c::PlaceFurniture ×2, 0x591700 Interior_c::SetTilesStatus.
- Globals: constants 0x858b14 f32 1/32768, 0x858b90 f32 4.0.
- Fields: `this+0x14` box: `+5` int8 door (`movsx`), `+0x1f` status/wealth.
- Behaviour:
  1. `counter = GetFurniture(0, 12, -1, status)` (0x599f0d); `till = GetFurniture(0, 13, -1, status)` (0x599f28). Group 0 (shop), sub-groups 12/13; not null-checked.
  2. `rot = ftol((rand()@0x599f2f & 0xFFFF) * f32[0x858b14] * f32[0x858b90]=4.0f)` → 0..3.
  3. `door = (int8)box[5]` (re-read each time, `movsx`). Outputs `oX` ([esp+0x10]) / `oY` ([esp+0x14]).
     - leftOfDoor != 0: `px = door - 5`; `PlaceFurniture(counter, px, 1, 0.0f, 1, 0, &oX, &oY, 0)`; `SetTilesStatus(px - 1, 0, oX + 1, 1, 2, 0)`; `PlaceFurniture(till, door + 1, 0, 0.0f, 1, rot, &oX, &oY, 1)`; return `px + 2` (= door - 3).
     - leftOfDoor == 0: `px = door + 2`; `PlaceFurniture(counter, px, 1, 0.0f, 1, 0, &oX, &oY, 0)`; `SetTilesStatus(px, 0, oX + 1, 1, 2, 0)`; `PlaceFurniture(till, door - 2, 0, 0.0f, 1, rot, &oX, &oY, 1)`; return `px + 2` (= door + 4).
     (PlaceFurniture args: furniture, tileX, tileY, f32 z, mode, rotation, int* outX, int* outY, skipTileTest. The counter's return value is ignored; oX is its footprint x size, 0 if it failed, so the status strip is then 1 tile wide.)
- Edge cases: the till is placed with skip-test 1 and a random rotation; the rotation roll is drawn before the counter is placed. The return value does not depend on placement success.
- gta-reversed: `plugin::CallMethodAndReturn<int32, 0x599EF0>` (not reversed).
- Open questions: none.

## 0x59a030 Interior_c::Shop_PlaceFixedUnits  (catalogue: specified/REA, port level 6)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, void, `ret`. Uses one local dword at [esp+4] after `push ecx`, i.e. it is initialised with the value of ECX (= this) on entry.
- Callers: 1 — 0x59a790 FurnishShop. Callees: 0x591700 Interior_c::SetTilesStatus, 0x821b1e `rand` (raw, no scaling), 0x599ef0 Interior_c::Shop_PlaceCounter, 0x591e40 Interior_c::AddInteriorInfo ×2.
- Fields: `this+0x14` box: `+2` u8 width W, `+5` int8 door.
- Behaviour:
  1. `if (box[5] == 0xFF)` (byte compare, door = -1) → return.
  2. `SetTilesStatus(door - 1, 0, 2, 1, 7, 0)` (status 7 at the door).
  3. `right = W - door - 2; left = door - 2;` (door via `movsx`).
     - `right >= 6 && left >= 6`: `side = (rand()@0x59a075 < 0x3FFF)` (signed compare of the raw rand value, `setl`; no `& 0xFFFF`); `local = Shop_PlaceCounter(side)`.
     - `right >= 6 && left < 6`: `local = Shop_PlaceCounter(0)`.
     - `right < 6 && left >= 6`: `local = Shop_PlaceCounter(1)`.
     - `right < 6 && left < 6`: no counter; `local` keeps its entry value, the low 32 bits of `this` (pointer value).
  4. `fx = (float)local` (`fild` int32, `fstp` f32). `AddInteriorInfo(9, fx, 2.0f, 0, nullptr)` (0x59a0be; immediate 0x40000000); `AddInteriorInfo(10, fx, 0.0f, 2, nullptr)` (0x59a0ce; y pushed as integer 0 = 0.0f).
- Edge cases / original bug: in the "no room on either side" case both interior infos are still added, with x = (float)(int32)this (a heap/static address, e.g. ~1.2e7), producing far-away info points. A port must reproduce this (use the Interior_c address as the original does, or record it in the divergence ledger since our addresses differ). The rand roll is a raw `rand()` < 16383 (≈ 50 %), not via the 1/32768 scaling.
- gta-reversed: `plugin::CallMethod<0x59A030>` (not reversed).
- Open questions: 0x59a0a3 — the uninitialised-local path depends on the object's address; a port's address differs, so exact parity of those two infos needs a ledger decision.

## 0x59a130 Interior_c::Shop_FurnishCeiling  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, `ret` (void). Body is a single `ret` (0x59a130).
- Callers: 1 — 0x599af0 Interior_c::FurnishOffice reaches it by tail `jmp` (0x599ba9). Callees: none.
- Globals / fields: none.
- Behaviour: does nothing. ECX is left as is.
- gta-reversed: `plugin::CallMethod<0x59A130>` (not reversed). Nothing to differ.
- Open questions: none.

## 0x59a140 Interior_c::Shop_AddShelfInfo  (catalogue: specified/REA, port level 3)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 3 stack args: `[esp+4] int32 tileX`, `[esp+8] int32 tileY`, `[esp+0xc] int32 dir`; returns void (EAX garbage: 1 on the info path, else ftol result / untouched); `ret 0xc`.
- Callers: 1 — 0x59a590 Shop_FurnishAisles (0x59a6e9). Callees: 0x821b1e `rand`, 0x821b40 `_ftol`, 0x591e40 Interior_c::AddInteriorInfo.
- Globals: 0x8d0948 `int32 s_shopShelfInfoCounter` (unnamed; .data, initial value 2; R/W); constants 0x858b14 f32 = 1/32768 (3.0517578e-05), 0x858628 f32 = 100.0.
- Behaviour:
  1. `if (s_counter[0x8d0948] <= 1)` (signed, `cmp ...,1; jle`) → `s_counter++` (0x59a1a4) and return. No rand is drawn.
  2. Else `r = ftol((rand()@0x59a14d & 0xFFFF) * f32[0x858b14] * f32[0x858628])` (0x59a14d..0x59a16b; `fild`, `fmul 1/32768`, `fmul 100.0f`, `_ftol` truncation). r is in 0..99; the product is exact in 24-bit precision, so r = (rand*100) >> 15.
  3. `if (r <= 60)` (signed `cmp eax,0x3c; jle`) → `s_counter++`, return.
  4. Else `AddInteriorInfo(8, (float)tileX, (float)tileY, dir, nullptr)` (0x59a192; both coordinates via `fild` of the int args then `fstp` f32; the return value is ignored), then `s_counter = 1` (`xor eax,eax; inc eax; mov [0x8d0948],eax`), return.
- Edge cases: the counter is shared with Shop_FurnishEdges (0x59a1b0) and is never reset elsewhere (all 11 references in the exe are in these two functions), so its value carries over between shops for the whole process. After an info is added the counter is 1, so the next call is always skipped (counter becomes 2) — at least one tile gap between shelf infos.
- gta-reversed: `plugin::CallMethod<0x59A140>` (not reversed); gr has no RNG helper here.
- Open questions: none.

## 0x59a1b0 Interior_c::Shop_FurnishEdges  (catalogue: specified/REA, port level 7)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, void, `ret`.
- Callers: 1 — 0x59a790 FurnishShop. Callees: 0x821b1e `rand` ×6 sites, 0x821b40 `_ftol` ×6, 0x599dc0 Interior_c::Shop_PlaceEdgeUnits ×3 loops, 0x591e40 Interior_c::AddInteriorInfo ×3, 0x5918e0 Interior_c::GetTileStatus ×2, 0x591700 Interior_c::SetTilesStatus ×6 sites.
- Globals: 0x8d0948 `int32 s_shopShelfInfoCounter` (unnamed .data, initial 2; R/W; shared with Shop_AddShelfInfo 0x59a140, never reset). Constants 0x858b14 f32 1/32768, 0x858628 f32 100.0.
- Fields: `this+0x14` tEffectInterior*: `+2` W, `+3` D, `+6/+7` int8 lDoorStart/lDoorEnd, `+8/+9` int8 rDoorStart/rDoorEnd, `+0xa/+0xb` int8 tDoorStart/tDoorEnd (gr names; -1 = none).
- Helpers used below:
  - `pct() = ftol((rand() & 0xFFFF) * f32[0x858b14] * f32[0x858628]=100.0f)` → 0..99 (inline at each site).
  - `pickBase(r) = r > 50 ? 0 : r > 25 ? 3 : r <= 10 ? 9 : 6` (the last two via `setle; dec; and -3; add 9`).
  - `inRange(s, e, i) = s != -1 && i >= s && i <= e` (signed, `movsx`; the -1 test is a byte compare with 0xFF).
  - `shelf(xf, yf, dir)`: `if (C > 1) { if (pct() > 60) { AddInteriorInfo(8, xf, yf, dir, nullptr); C = 0; } } C++;` where C = *(int32*)0x8d0948 (no roll when C ≤ 1).
- Behaviour (W, D read once at entry):
  1. `b = pickBase(pct()@0x59a1cb)`; `for (p = 1; p < W - 1; p += Shop_PlaceEdgeUnits(b, p, D - 1, 0));` (back wall; loop tested first).
  2. `b = pickBase(pct()@0x59a244)`; `for (p = 1; p <= D - 2; p += Shop_PlaceEdgeUnits(b, 0, p, 1));`
  3. `b = pickBase(pct()@0x59a2bc)`; `for (p = 1; p <= D - 2; p += Shop_PlaceEdgeUnits(b, W - 1, p, 3));`
     (All three rolls are drawn even if their loop does not run.)
  4. Front row, `for (x = 1; x <= W - 2; x++)`: `if (!inRange(box[0xa], box[0xb], x)) shelf((float)x, (float)(D - 2), 2);` then always `if (GetTileStatus(x, 1) == 0) SetTilesStatus(x, 1, 1, 1, 2, 0);` (roll site 0x59a363). Note: the door range uses the t(op)Door fields and the info y is D-2 while the status row is y = 1, exactly as in the bytes.
  5. Side columns, `for (y = 2; y <= D - 3; y++)`: `if (!inRange(box[6], box[7], y)) shelf(1.0f, (float)y, 3);` (roll 0x59a41e) then `if (!inRange(box[8], box[9], y)) shelf((float)(W - 2), (float)y, 1);` (roll 0x59a48f). C is kept in EDX across the loop and written back after each step (same values as reading the global each time).
  6. `for (x = 2; x <= W - 3; x++) { if (GetTileStatus(x, 2) == 0) SetTilesStatus(x, 2, 1, 1, 2, 0); SetTilesStatus(x, D - 3, 1, 1, 3, 0); }`
  7. `for (y = 3; y <= D - 4; y++) { SetTilesStatus(2, y, 1, 1, 3, 0); SetTilesStatus(W - 3, y, 1, 1, 3, 0); }`
  8. `SetTilesStatus(3, 3, W - 6, 1, 3, 0)`.
  (SetTilesStatus args: x, y, sizeX, sizeY, status, force. Float coordinates come from `fild` int32 → f32.)
- Edge cases: AddInteriorInfo return values are ignored (it fails silently after 16 infos). The counter state persists across interiors and across calls of Shop_AddShelfInfo.
- gta-reversed: `plugin::CallMethod<0x59A1B0>` (not reversed).
- Open questions: none.

## 0x59a590 Interior_c::Shop_FurnishAisles  (catalogue: specified/REA, port level 7)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), no args, void, `ret`.
- Callers: 1 — 0x59a790 FurnishShop. Callees: 0x591d20 Interior_c::AddGotoPt ×6 sites, 0x821b1e `rand`, 0x821b40 `_ftol`, 0x599dc0 Interior_c::Shop_PlaceEdgeUnits ×2 sites, 0x59a140 Interior_c::Shop_AddShelfInfo, 0x591700 Interior_c::SetTilesStatus.
- Globals: constants 0x858b14 f32 1/32768, 0x858628 f32 100.0.
- Fields: `this+0x14` box `+2` W, `+3` D (read once).
- Behaviour:
  1. `cols = W - 6; rows = D - 7; top = D - 3;` `if (cols <= 0 || rows <= 0)` → return.
  2. `AddGotoPt(2, 3, -0.5f, -0.5f); AddGotoPt(2, top, -0.5f, 0.5f);` (args: tileX, tileY, f32 offX, f32 offY).
  3. `for (i = 0; i < cols; i++)` (ascending):
     - `p = 4`; `r = ftol((rand()@0x59a606 & 0xFFFF) * f32[0x858b14] * f32[0x858628]=100.0f)` (drawn every iteration, also when unused); `b = r > 50 ? 0 : r > 25 ? 3 : r <= 10 ? 9 : 6`.
     - `switch (i % 4)` (signed remainder idiom; i ≥ 0):
       - 0: `if (i != cols - 1) repeat rows times: p += Shop_PlaceEdgeUnits(b, i + 3, p, 3);`
       - 1: `repeat rows times: p += Shop_PlaceEdgeUnits(b, i + 3, p, 1);`
       - 2: `for (k = 0; k < rows; k++) Shop_AddShelfInfo(i + 3, k + 4, 3);`
       - 3: `SetTilesStatus(i + 3, 4, 1, rows, 3, 0); AddGotoPt(i + 3, 3, -0.5f, -0.5f); AddGotoPt(i + 3, top, -0.5f, 0.5f);`
     Note the "repeat rows times" loops run a fixed number of calls (`rows`), not "while p < limit"; each call returns the tiles consumed (≥ 1), so p can run past the room depth (Shop_PlaceEdgeUnits then sees no empty tiles and returns 1).
  4. `ex = cols + 3` (= W - 3); `AddGotoPt(ex, 3, 0.5f, -0.5f); AddGotoPt(ex, top, 0.5f, 0.5f);`
- gta-reversed: `plugin::CallMethod<0x59A590>` (not reversed).
- Open questions: none.

## 0x59a790 Interior_c::FurnishShop  (catalogue: specified/REA, port level 8)
- Convention / signature: `__thiscall` (this = ECX: Interior_c*), 1 stack arg `[esp+4] int32` of which only the low byte is used (`mov al, [esp+4]`) = furniture group id; void; `ret 4`.
- Callers: 0x593bf0 Interior_c::Init (box type 0, passes 0), 0x591590 Interior_c::Furnish. Callees: 0x591700 SetTilesStatus ×4, 0x59a030 Shop_PlaceFixedUnits, 0x59a1b0 Shop_FurnishEdges, 0x59a590 Shop_FurnishAisles.
- Fields: writes `this+0x790` int8 (gr `m_furnitureGroupId`) = arg low byte, unconditionally first. Reads box `+2` W, `+3` D, `+5` int8 door.
- Behaviour:
  1. `this[0x790] = (uint8)arg`.
  2. `door = (int8)box[5]`. `if (door - 1 < 6 && W - door < 6)` (signed) → return (room too small on both sides of the door). Otherwise:
  3. Corners, status 2: `SetTilesStatus(0, 0, 1, 1, 2, 0); SetTilesStatus(0, D - 1, 1, 1, 2, 0); SetTilesStatus(W - 1, 0, 1, 1, 2, 0); SetTilesStatus(W - 1, D - 1, 1, 1, 2, 0);`
  4. `Shop_PlaceFixedUnits(); Shop_FurnishEdges(); Shop_FurnishAisles();`
- Edge cases: with door = -1 the size test uses door-1 = -2 (< 6) and W+1, so it continues when W ≥ 5; Shop_PlaceFixedUnits then returns at once.
- gta-reversed: `plugin::CallMethod<0x59A790, Interior_c*, int32>` (not reversed).
- Open questions: none.
