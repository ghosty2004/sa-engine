# CObject: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x59f180 CObject::Render  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Entity/Object/Object.cpp:566` (rbf `CObject_Render_DetachedPartRendering`): with the fix on, gr applies the editable materials (`SetEditableMaterialsCB`), disables culling around `CEntity::Render()` and resets the materials.
  - Original bytes: `if (dword [this+0x140] & 0x2000000)` (bDoNotRender) return (0x59f189). If `m_nRefModelIndex (+0x14a, i16) != -1 && byte [this+0x13c] == 3 (temporary) && (byte [this+0x140] & 0x80)` (bChangesVehColor, `test cl,cl; jns`): `CVehicleModelInfo::ms_pRemapTexture (0xb4e47c) = this->m_pRemapTexture (+0x16c)`; `ms_modelInfoPtrs[ref]->SetVehicleColour(col[0] +0x14c, col[1] +0x14d, col[2] +0x14e, col[3] +0x14f)` (0x4c84b0, bytes zero-extended). Then tail-jump to `CEntity::Render` (0x534310). No material setup, no cull-mode change, no reset.
  - Verdict: CONFIRMED. The fix path is not in the original (detached car parts render with whatever material colours are current).
  - Class: keep-original (take the `!fix` path).

## 0x59fb50 CObject::DoBurnEffect  (gr_deviations = 3)
Rows checked: 3 (CONFIRMED 3, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Entity/Object/Object.cpp:888/889/890` (rng-helper, float): `fRandX/Y/Z = CGeneral::GetRandomNumberInRange(box.m_vecMin.c, box.m_vecMax.c)`.
  - Original bytes: sizes precomputed and stored f32: `sz = max.z - min.z`, `sy = max.y - min.y`, `sx = max.x - min.x` (0x59fb8d-0x59fbb1, z first). Per particle: `x = (float)rand() * (1/32767)[0x858c7c] * sx + min.x` (0x59fbe0-0x59fbfb), then y with sy (0x59fbff-0x59fc1a), then z with sz (0x59fc1e-0x59fc47); i.e. `t*size + min` with t from a raw `fild` of rand() (no mask). Then `CMatrix::operator*(m_matrix, &v)` (0x59c890) and `g_fx.m_SmokeII3expand (0xa9ae24)->AddParticle(&pos, &(0,0,0.02f), 0, &FxPrtMult_c(0,0,0,1,1,1,0.4f), -1.0f, 1.2f, 0.6f, 0)` (0x4aa440).
  - Verdict (all three): CONFIRMED (gr lerp `max*t + min*(1-t)` vs original `size*t + min`, cat14 finding 1).
  - Class: keep-original.
- Found in passing (keep-original): particle count `n = _ftol(sx * sy * sz * m_fBurnDamage(+0x178) * 0.05f [0x858c28])` (0x59fbb1-0x59fbc9, x87 order sx*sy*sz*burn*0.05). gr divides by 20.0f; `*0.05f` and `/20` can round differently, which changes the count near integer boundaries.

## 0x5a1340 CObject::Explode  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Entity/Object/Object.cpp:1181/1182` (rng-helper): `m_vecMoveSpeed.x/y += GetRandomNumberInRange(-0.0256F, 0.0256F);`
  - Original bytes: `m_vecMoveSpeed.z (+0x4c) += 0.5f [0x858b8c]` first (0x5a13e0-0x5a13e6), then `m_vecMoveSpeed.x (+0x44) += (float)((rand() & 0xFF) - 128) * 0.0002f [0x85a7d0]` (0x5a13e9-0x5a1409) and the same for y (+0x48, 0x5a140c-0x5a1431). The random term is one of 256 discrete values from -0.0256 to +0.0254 (low 8 bits of rand(), not a scaled rand()).
  - Verdict (both): CONFIRMED (completely different mapping from rand() than gr's float lerp).
  - Class: keep-original.

## 0x5a1840 CObject::TryToFreeUpTempObjects  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Entity/Object/Object.cpp:1253` (pool-iteration): `for (auto& obj : GetObjectPool()->GetAllValid()) if (obj.IsTemporary() && !obj.IsVisible()) { Remove; delete; --numObjects; }`
  - Original bytes: object pool 0xb7449c, DESCENDING loop (`dec edi` first, 0x5a1861, until `edi == 0`, 0x5a18a5), and at the top of every iteration `if (numObjects (EBX) <= 0) break` (`test ebx,ebx; jle 0x5a18a9`, 0x5a1868). Per slot: skip free/null; `[obj+0x13c] == 3` (temporary); `!CEntity::IsVisible()` (0x536bc0); `CWorld::Remove` (0x563280), `vtbl[0](1)`, `--numObjects`.
  - Verdict: CONFIRMED. Two differences: (a) the original walks from the highest slot down; (b) it stops as soon as `numObjects` objects were freed, while gr keeps deleting every invisible temporary object (its decrement has no effect on the loop). So gr frees more objects and different ones.
  - Class: keep-original.

## 0x5a18b0 CObject::DeleteAllTempObjects  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Entity/Object/Object.cpp:1264` (pool-iteration): `for (auto& obj : GetObjectPool()->GetAllValid()) if (obj.IsTemporary()) { CWorld::Remove(&obj); delete &obj; }`
  - Original bytes: object pool 0xb7449c, ASCENDING loop `edi = 0 .. size-1` (size cached in a local before the loop, signed `jle` guard; `inc edi; cmp edi,eax; jl`, 0x5a18fe-0x5a1907), skip free/null; `if (byte [obj+0x13c] == 3)` (temporary) `CWorld::Remove(obj)` (0x563280) then virtual destructor `vtbl[0](1)` (0x5a18f8).
  - Verdict: REFUTED (same ascending order).
  - Class: harmless.

## 0x5a1910 CObject::DeleteAllMissionObjects  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Entity/Object/Object.cpp:1274` (pool-iteration): delete objects with `IsMissionObject()`.
  - Original bytes: ascending loop over the object pool (`inc edi; ... jl 0x5a1930`, 0x5a1962-0x5a196b, size cached), skip free/null; type byte `[obj+0x13c]` == 2 or 6 (0x5a193f-0x5a194b) -> `CWorld::Remove`, `vtbl[0](1)`.
  - Verdict: REFUTED (ascending in the original too).
  - Class: harmless.

## 0x5a1980 CObject::DeleteAllTempObjectsInArea  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Entity/Object/Object.cpp:1284` (pool-iteration): delete temporary objects with `DistanceBetweenPointsSquared(pos, point) < sq(radius)`.
  - Original bytes: ascending loop (`inc ecx; cmp ecx,edx; jl 0x5a19b0`, 0x5a1a71-0x5a1a82, size cached in a local), skip free/null, type `[obj+0x13c] == 3`; `d2 = ((p.z-pos.z)^2 + (p.y-pos.y)^2) + (p.x-pos.x)^2` (each square as `(a-b)*(a-b)` from two loads, z first, 0x5a1a0c-0x5a1a3d); delete if `radius*radius > d2` (`fcompp`, `test ah,0x41; jne` skip, 0x5a1a3f-0x5a1a4e; NaN does not delete).
  - Verdict: REFUTED (ascending in the original too).
  - Class: harmless.
