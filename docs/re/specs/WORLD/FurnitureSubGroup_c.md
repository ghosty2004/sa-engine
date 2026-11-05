# FurnitureSubGroup_c - full specs

Source of truth: the 1.0 US disassembly. gr has `plugin::CallMethodAndReturn` stubs only
(`.Reversed = false`) for all three functions.

## Layout

| class | off | size | type | gr name |
|---|---|---|---|---|
| FurnitureSubGroup_c (0x1C) | 0x0/0x4 | 8 | prev/next | ListItem_c |
| | 0x8 | 4 | int32 | m_SubGroupId |
| | 0xC | 0xC | TList_c<Furniture_c> (head +0xC, tail +0x10, count +0x14) | m_Furnitures |
| | 0x18 | 1 | bool | m_bCanPlaceInFrontOfWindow |
| | 0x19 | 1 | bool | m_bIsTall |
| | 0x1A | 1 | bool | m_bCanSteal |
| Furniture_c (0x14) | 0x0/0x4 | 8 | prev/next | ListItem_c |
| | 0x8 | 2 | int16 | m_nModelId |
| | 0xA | 2 | int16 | m_nId |
| | 0xC | 1 | int8 | m_nWidthX (tiles) |
| | 0xD | 1 | int8 | m_nWidthY (tiles) |
| | 0xE | 1 | u8 | m_nWealthMin |
| | 0xF | 1 | u8 | m_nWealthMax |
| | 0x10..0x12 | 3 | bool | m_bCanPlaceInFrontOfWindow, m_bIsTall, m_bCanSteal |
| | 0x13 | 1 | int8 | m_nMaxAng |

Furniture_c storage: static array at 0xBAE1F8 (256 x 0x14), next index `u16 g_currFurnitureId` at 0xBAB378.

RNG notation: `R16 = (f32)(int)(rand() & 0xFFFF) * f32[0x858b14] (1/32768)`, `_rand` 0x821b1e, `_ftol` 0x821b40.

## 0x590ee0 FurnitureSubGroup_c::GetFurniture  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall` (this = ECX: FurnitureSubGroup_c*), `[esp+4] int16 id` (only the low word is read), `[esp+8] uint8 wealth` (low byte); returns `Furniture_c*` in EAX; `ret 8`.
- Callers: 2 (FurnitureGroup_c::GetFurniture 0x591130, FurnitureManager_c::GetFurniture 0x5911e0). Callees: 0x4a8ec0 (TList_c::GetNumItems: `return list->m_cnt` at +8), 0x4a9010 (TList_c get item by offset: `(fromHead=1, n)` walks `n` times via `m_pNext` from the head, returns NULL if it runs off), `_rand` 0x821b1e, `_ftol` 0x821b40.
- Globals: `0x858b14` f32 = 1/32768.
- Fields: `this+0xC TList_c<Furniture_c>` (head +0xC, tail +0x10, count +0x14). Furniture_c: `+4 m_pNext`, `+0xA int16 m_nId`, `+0xE uint8 m_nWealthMin`, `+0xF uint8 m_nWealthMax`.
- Behaviour:
  1. If `id > -1` (signed 16-bit compare `cmp cx,-1; jle`): walk the list from head; return the first item with `m_nId == id` (16-bit compare); NULL if none.
  2. Else (`id <= -1`): `w = wealth`. The test `if ((uint32)w == 0xFFFFFFFF)` (0x590f1d, `movzx eax,bl; cmp eax,-1`) can never be true, so the "any wealth" branch at 0x590f22..0x590f61 is dead code. (It would do: `n = count; r = _rand()&0xFFFF; k = _ftol((float)n * ((float)r * f32[0x858b14]))` — x87 order `fild r; fmul [0x858b14]; fild n; fmul st(1)` — then `return GetItemOffset(1, k)`.)
  3. Live branch: `c = 0`; for each item from head: if `w >= item->m_nWealthMin && w <= item->m_nWealthMax` (unsigned byte compares) `c++`.
  4. RNG at 0x590f87: `r = _rand() & 0xFFFF; k = _ftol((fild r) * f32[0x858b14] * c (fimul int32))`.
  5. Walk the list again from head with `j = 0`: for each item in the wealth range: if `j == k` return item, else `j++`. Return NULL if not found (e.g. `c == 0`).
- Edge cases: the dead branch must not be "fixed" to make 0xFF mean "any wealth"; with wealth 0xFF only items whose range includes 255 qualify. The rand() call happens even when `c == 0`.
- gta-reversed: plugin::Call stub (uint8 wealth parameter, consistent with the dead compare).

## 0x590fd0 FurnitureSubGroup_c::GetRandomId  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall` (this = ECX: FurnitureSubGroup_c*), `[esp+4] uint8 wealth` (low byte); returns int32 in EAX (furniture id sign-extended from int16, or -1); `ret 4`.
- Callers: 2 (FurnitureGroup_c::GetRandomId 0x591170, FurnitureManager_c::GetRandomId 0x591220). Callees: 0x4a8ec0, 0x4a9010 (see GetFurniture), `_rand` 0x821b1e, `_ftol` 0x821b40.
- Globals: `0x858b14` f32 = 1/32768.
- Fields: as GetFurniture (list at +0xC; item `+0xA m_nId`, `+0xE/+0xF` wealth min/max).
- Behaviour:
  1. `if ((uint32)(uint8)wealth == 0xFFFFFFFF)` at 0x590fd9 is never true: the branch 0x590fe1..0x591025 is dead (it would pick `GetItemOffset(1, _ftol((float)count * ((float)(rand()&0xFFFF) * 1/32768)))` and return `(int16)item->m_nId` without a NULL check).
  2. `c` = number of items with `wealthMin <= wealth <= wealthMax` (unsigned byte compares, walk from head).
  3. RNG at 0x59104b: `k = _ftol((float)(rand() & 0xFFFF) * f32[0x858b14] * c)` (fild, fmul m32, fimul m32int).
  4. Second walk from head with `j = 0`: for items in range, if `j == k` return `(int32)(int16)item->m_nId` (movsx); else `j++`.
  5. Not found: return -1 (`or eax, -1`).
- gta-reversed: plugin::Call stub.

## 0x5c00c0 FurnitureSubGroup_c::AddFurniture  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall` (this = ECX: FurnitureSubGroup_c*), args `[esp+4] uint16 modelId`, `[esp+8] int16 id`, `[esp+0xC] uint8 wealthMin`, `[esp+0x10] uint8 wealthMax`, `[esp+0x14] uint8 maxAng`; returns bool in AL; `ret 0x14`. The function reuses the modelId and id argument slots as scratch (f32 and int32 temporaries).
- Callers: 1 (FurnitureGroup_c::AddFurniture 0x5c0230). Callees: 0x4a8df0 (TList_c::AddItem, insert at head: `item->prev = 0; item->next = head; if (head) head->prev = item; else tail = item; head = item; cnt++`), `_ftol` 0x821b40 (x2).
- Globals: R/W `0xbab378 uint16 FurnitureManager_c::g_currFurnitureId`; W `0xbae1f8 Furniture_c g_furnitureStore[256]` (stride 0x14; gr: FurnitureManager_c static furniture array); R `0xa9b0c8 CModelInfo::ms_modelInfoPtrs[]`; R `0x858b8c` f32 = 0.5; R `0x858b38` f32 = 0.02 (0x3ca3d70a).
- Fields: this `+0xC TList_c<Furniture_c>`, `+0x18 bool m_bCanPlaceInFrontOfWindow`, `+0x19 bool m_bIsTall`, `+0x1A bool m_bCanSteal`. Furniture_c (0x14): `+8 int16 m_nModelId`, `+0xA int16 m_nId`, `+0xC int8 m_nWidthX`, `+0xD int8 m_nWidthY`, `+0xE m_nWealthMin`, `+0xF m_nWealthMax`, `+0x10/+0x11/+0x12` copies of the subgroup flags, `+0x13 m_nMaxAng`. CBaseModelInfo `+0x14 CColModel*`; CColModel bounding box `+0 min (x,y,z)`, `+0xC max (x,y,z)`.
- Behaviour:
  1. `n = g_currFurnitureId`; if `n >= 0x100` (unsigned) return 0. `g_currFurnitureId = n + 1` (16-bit). `f = &store[n]` (`0xbae1f8 + n*0x14`).
  2. `f->m_nModelId = modelId; f->m_nId = id`.
  3. `col = ms_modelInfoPtrs[modelId]->m_pColModel`.
     - If `this->m_bCanSteal != 0`: `wx = col.max.x - col.min.x` (fld max.x; fsub min.x), `wy = col.max.y - col.min.y` (fld max.y; fsub min.y).
     - Else: `wx = col.max.x + f32[0x858b8c](0.5)`, `wy = col.max.y + 0.5` (the min corner is not used).
  4. `wy` is stored to an f32 temporary (rounding to f32). `wx` stays on the x87 stack.
  5. X width: `ix = _ftol(wx)` (truncation; `wx` kept on the stack via `fld st(0)`); `d = wx - (float)ix` (fild ix; fsubr st(1)); `if (!(d < 0.02f))` (fcomp m32 0x858b38; `test ah,5; jnp` skips only when exactly C0 is set, i.e. d < 0.02; equal, greater and NaN all increment) `ix++` — the increment is 8-bit (`inc cl`). `f->m_nWidthX = (int8)ix`.
  6. Y width: same with the f32 `wy`: `iy = _ftol(wy); d = wy - (float)iy` (fild iy; fsubr m32 wy); `if (!(d < 0.02f)) iy++` (8-bit). `f->m_nWidthY = (int8)iy`.
  7. `f->m_nWealthMin = wealthMin; f->m_nWealthMax = wealthMax; f->m_nMaxAng = maxAng; f->+0x10 = this->+0x18; f->+0x11 = this->+0x19; f->+0x12 = this->+0x1A`.
  8. `TList_c::AddItem(&this->list, f)` (head insert). Return 1.
- Edge cases: no NULL check on model info or col model. The width is a ceiling with a 0.02 tolerance (e.g. 1.01 -> 1, 1.03 -> 2). Store write order: +0xC is written before +0xD/+0xF/+0xE/+0x13/+0x10/+0x11/+0x12 (no observable effect).
- gta-reversed: plugin::Call stub.
