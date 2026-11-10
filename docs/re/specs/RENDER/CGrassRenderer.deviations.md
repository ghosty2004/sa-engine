# CGrassRenderer (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/GrassRenderer.cpp` (pin 20d8d2a). PPTriPlant (stride 0x5C): +0 V1, +0xC V2, +0x18 V3, +0x24
Center, +0x30 model_id (u16), +0x32 num_plants (u16), +0x34 scale.x, +0x38 scale.y, +0x3C texture, +0x40..+0x43 color (r,g,b,a),
+0x44 intensity (u8), +0x48 seed (f32), +0x4C scale_var_xy, +0x50 scale_var_z, +0x54 wind_bend_scale, +0x58 wind_bend_var.
CRT: srand 0x821B11 (stores the seed in the per-thread data +0x14), rand 0x821B1E (`s = s*0x343FD + 0x269EC3; return (s >> 16) & 0x7FFF`),
_ftol 0x821B40 (truncation), floor 0x8219F0 (double).

## 0x5DAD00 CGrassRenderer::DrawTriPlants  (gr_deviations = 2)
Original: `__cdecl int (PPTriPlant* plants [esp+4], int count [esp+8], RpAtomic** tab [esp+0xC])`, returns EAX = 1 (0x5DB082), plain `ret`.
Rows GrassRenderer.cpp:71 (`#if FIX_BUGS std::mt19937 randomGen(*(uint32*)&plant.seed) #else std::mt19937(static_cast<uint32>(plant.seed))`
+ `uniform_real_distribution<float>(0,1)`) and :79 ("Original implementation used srand() ...; //srand(...)").
- Original bytes (per plant, after the distance cull):
  1. `srand(_ftol(plant.seed))` (0x5DADB7-0x5DADCD) - reseeds the GLOBAL CRT generator.
  2. Per instance j = 0..num_plants-1 (count-down counter, 0x5DAEE6/0x5DB042):
     `randX = (float)rand() * 3.0518509e-05f` (f32 [0x858C7C] = 0x38000100 = 1/32767, stored f32, 0x5DAEF0-0x5DAF03);
     `randY` likewise (0x5DAF07-0x5DAF22); `GenPointInTriangle(&pos, plant, V2, V3, randX, randY)` (0x5DAF38);
     if `dist(camPos, pos) < m_closeDist - 2.0f` (0x5DAF76-0x5DAF89; strict; equal/NaN render): call rand() three times (0x5DAF8B-0x5DAF95)
     and skip; else `RwFrameTranslate(frame, &pos, 0)`, then
     `xy = ((float)rand() * (1/32767) * scale_var_xy) + scale.x` -> `right.x *= xy`, `up.y *= xy` (0x5DAFB3-0x5DAFD6, xy kept in x87);
     `at.z *= ((float)rand() * (1/32767) * scale_var_z) + scale.y` (0x5DAFE0-0x5DAFFD);
     `w = ((float)rand() * (1/32767) * wind_bend_var + 1.0f) * (m_windBending * wind_bend_scale)` stored to at.x and at.y (0x5DB007-0x5DB033);
     RwMatrixUpdate, `atomic->renderCallBack(atomic)` (`call [ebx+0x48]`, 0x5DB03C).
- Verdict row :71: CONFIRMED - both gr branches replace the CRT `srand/rand` sequence with std::mt19937 + uniform_real_distribution,
  so every plant position/scale/bend differs from the original. Class: keep-original (seed with `srand(_ftol(seed))`, draw with
  CRT rand() and the factor 1/32767 in the order above).
- Verdict row :79: CONFIRMED - the comment is accurate and gr removed the srand. Class: keep-original. The global-RNG side effect is
  contained by the caller CPPTriPlantBuffer::Flush 0x5DB0C0 (`r = rand()` before, `srand(r)` after), so after Flush the global state
  equals gr's; inside DrawTriPlants the original draws from the reseeded global generator (no other consumer runs in between).
Found in passing (CONFIRMED, keep-original): the fade alpha is `_ftol(clamp0(_ftol(clamp255(_ftol(floor((double)(((farDist + 20.0f) - camDist)
* (int)a * 0.05f)))))))` (0x5DADED-0x5DAE58; `* 0.05f` [0x858C28], not gr's `/ 20.0f`; clamp at 255 also catches NaN, clamp at 0 is `> 0` keep).
