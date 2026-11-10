# CRenderer (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Renderer.cpp` (pin 20d8d2a). Counters: ms_nNoOfVisibleLods 0xB76840, ms_nNoOfVisibleEntities
0xB76844, ms_nNoOfVisibleSuperLods 0xB76838, ms_nNoOfInVisibleEntities 0xB7683C; arrays 0xB748F8, 0xB75898, 0xB74830, 0xB745D8.

## 0x553910 CRenderer::PreRender  (gr_deviations = 8)
- Rows :278 / :281 / :284 / :287 (asserts on the counters): deviates=no, debug only. REFUTED, harmless.
- Rows :294 / :295 / :307 / :308 (NOTSA hack comparing the alpha-list callback with gr's hooked RenderEntity as well as 0x732B40).
  Original bytes: alpha list walk `link = [0xC88130]` until `link == 0xC88134` (0x553999-0x5539CB), underwater alpha list
  `[0xC88188]` until `0xC8818C` (0x5539CD-0x5539FA); `link = [link+0x10]`; when `[link+4] == 0x732B40` (CVisibilityPlugins::RenderEntity):
  `entity = [link]`, `entity->flags(+0x1C) &= 0xFFFDFFFF` (clears bit 17, m_bOffscreen), `entity->vtbl[0x44/4]()` (PreRender).
  Verdict (all four): REFUTED - gr's extra comparison only accepts gr's own copy of the same callback; behaviour is identical.
  Class: harmless.
- Found in passing (CONFIRMED, keep-original): each of the four array loops re-reads its counter global after every PreRender call
  (`mov eax, [0xB76840]; inc esi; cmp esi, eax`, e.g. 0x55392C-0x553934); gr's `for_each(GetVisibleLodPtrs())` fixes the span
  length at the start. If a PreRender changes a counter, the original visits a different set. Then CHeli::SpecialHeliPreRender
  0x6C4750 and a tail `jmp` to CShadows::RenderExtraPlayerShadows 0x707FA0 (same as gr).

## 0x553AA0 CRenderer::RenderEverythingBarRoads  (gr_deviations = 2)
- Row :352 (assert): REFUTED, harmless.
- Row :360 (`// todo: R* nice check | or we missed smth here?` inside `if (vehicle || (ped && GetClumpAlpha != 255))`).
  Original bytes: entity type `[e+0x36] & 7`: building (1) whose `byte [modelinfo+0x13] & 1` (road) -> skip (0x553B07-0x553B1E).
  Type 2 (vehicle) -> 0x553B47; type 3 (ped) with `GetClumpAlpha([e+0x18]) != 0xFF` (0x732B20) -> 0x553B47; anything else ->
  RenderOneNonRoad 0x553260. At 0x553B47: if type != 2 (i.e. the translucent PED) -> 0x553BA9 = InsertEntityIntoSortedList
  0x734570. Vehicles: boat (`[e+0x590] == 5`, 0x553B50): sorted if cam mode (`word [0xB6F1A8 + 0x238 * byte [0xB6F081]]`) == 0xE
  (WHEELCAM), or (== 0x10 (1STPERSON) and LookDirection 0x50AE90 != 3 and != 0), or GetClumpAlpha != 0xFF; else underwater list
  0x733D90. Non-boat: `[e+0x40] & 0x8000000` (bTouchingWater) -> underwater list, else sorted (0x553BB5). If the insert returns 0
  -> RenderOneNonRoad.
  Verdict: CONFIRMED - gr LOGIC ERROR: in gr a translucent ped enters the block but `if (entity->GetIsTypeVehicle())` fails, so
  bInserted stays false and the ped is rendered immediately via RenderOneNonRoad; the original inserts it into the sorted alpha list
  (distance-sorted, rendered later). Class: keep-original.
- Found in passing (CONFIRMED, keep-original):
  1. Distance passed to the insert functions is `sqrt` of the squared components of `ms_vecCameraPosition (0xB76870) - pos`, with a
     DIFFERENT summation order per path: underwater list `(dz*dz + dx*dx) + dy*dy` (0x553BE8-0x553BF8), sorted list
     `(dz*dz + dy*dy) + dx*dx` (0x553C29-0x553C39); gr uses DistanceBetweenPoints (x, y, z order). pos = matrix pos (+0x14 -> +0x30)
     or placement pos (+4).
  2. The entity loop re-reads ms_nNoOfVisibleEntities each iteration (0x553C5A), the LOD loop re-reads ms_nNoOfVisibleLods (0x553CBD).
  3. zShift: `old = cam->zShift (+0x90)`; EndUpdate; `cam->zShift = -100.0f [0x8CD814] + cam->zShift` (fld const, fadd field,
     0x553C7D-0x553C8E; note 0x8CD814 is in .data, i.e. mutable); BeginUpdate; LODs; EndUpdate; restore; BeginUpdate. Same as gr
     apart from the operand order of the addition (no numeric effect).
