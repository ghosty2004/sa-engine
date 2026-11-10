# CShadows (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Shadows.cpp` (pin 20d8d2a). RwRenderStateSet = `call [RwEngineInstance 0xC97B24 + 0x20]`.

## 0x707B40 CShadows::StoreShadowForPedObject  (gr_deviations = 4)
Original: `__cdecl void (CPed* ped [esp+4], float dispX, float dispY, float frontX, float frontY, float sideX, float sideY)`, plain `ret`.
- Rows Shadows.cpp:364 ("well, in R* world it is") and :368 ("we really should fix this"): comments only.
  Original bytes: `ped->GetBonePosition(&pos, BONE 0, false)` 0x5E4280 (0x707B49-0x707B54) with no type check. Verdict: REFUTED (no
  behavioural content). Class: harmless.
- Row :369 `assert(ped->GetIsTypePed() || ped->GetIsTypeObject())`: the original has no check (0x707B40-0x707B54). Verdict: REFUTED as a
  release-behaviour difference (assert only). Class: harmless (a port must not reject CCutsceneObject callers, 0x5B1E00).
- Row :403 NOTSA (`isPlayerPed || g_fx.GetFxQuality() >= FX_QUALITY_VERY_HIGH` as the last StoreShadowToBeRendered argument):
  original passes only `FindPlayerPed(-1) == ped` (`sete cl` 0x707BB9, pushed at 0x707C37 as the bool `drawOnBuildings` argument).
  Verdict: CONFIRMED. Class: keep-original.
Found in passing: distance test `if (!(d2 < MAX_DISTANCE_PED_SHADOWS_SQR [0xC4B6B0])) return` (`test ah,5; jp`, 0x707BA4; NaN returns,
gr's `>=` lets NaN through - harmless); d2 = `(b.y - c.y)^2 + (b.x - c.x)^2` (y term first, 0x707B78). Strength: if
`sqrt(d2) < MAX_DISTANCE_PED_SHADOWS [0x8D5240] * 0.5f` the raw DWORD at 0xB7C4E8 (m_nShadowStrength plus the next u16 in the high half)
is pushed (only its low 16/8 bits are consumed by the callee), else `_ftol((1.0f - (d - h) / h) * (int)(i16)strength)` - same formula as
gr's CalculateShadowStrength. Callee arguments match gr (type 1, gpShadowPedTex 0xC403E4, (b.x + dispX, b.y + dispY, b.z), front, side,
intensity, r = g = b = intensity, 4.0f, false, 1.0f, nullptr, isPlayer).

## 0x707FA0 CShadows::RenderExtraPlayerShadows  (gr_deviations = 1)
Row Shadows.cpp:523 NOTSA (`|| lightToPlyrDist == 0.f` added to the skip test):
- Original bytes (0x708089-0x7080F2): `dist = |light.pos - FindPlayerCoors()|` (CVector::Magnitude 0x4082C0); skip only if
  `!(dist < light.radius)` (`fcom [ebp-0x16]; test ah,5; jp` 0x708097). No zero test; `1.0f / dist` is computed at 0x7080F2-0x7080F9
  (dist == 0 gives inf and NaN direction components).
- Verdict: CONFIRMED. Class: keep-original.
Found in passing (CONFIRMED, keep-original): the strength is NOT gr's CalculateShadowStrength form. Original: `s = (5 * (i16)strength) / 8`
(signed division rounding toward zero, `lea eax,[eax+eax*4]; cdq; and edx,7; add; sar 3`, 0x7080B2-0x7080D4); if `dist < radius * 0.5f`
use `s`, else `_ftol((1.0f - ((dist + dist) - radius) / radius) * s)` (0x7080CB-0x7080EB); gr computes `(1 - (dist - r/2) / (r/2)) * s`
(different rounding). Early-outs match gr (strength word 0xB7C4EA == 0, no player vehicle, model 0x1B9, appearance 2, vehicle subtype
+0x590 in {9, 8, 5}).

## 0x70A960 CShadows::RenderStoredShadows  (gr_deviations = 1)
Row Shadows.cpp:758 ("Originally renderstates are still set even though there are no shadows to be rendered ... we early-out here"):
- Original bytes: unconditionally `uiTempBufferIndicesStored = uiTempBufferVerticesStored = 0` (0x70A97A/0x70A980) and sets
  (8 ZWRITEENABLE, 0), (6 ZTESTENABLE, 1), (12 VERTEXALPHAENABLE, 1), (14 FOGENABLE, 0), (2 TEXTUREADDRESS, 3 CLAMP), (20 CULLMODE, 1 NONE),
  (30 ALPHATESTFUNCTIONREF, 0), (9 TEXTUREFILTER, 2 LINEAR) (0x70A977-0x70A9DD); only then reads ShadowsStoredToBeRendered (0xC403DC).
  With 0 shadows it jumps to the epilogue 0x70B6D8, which still sets (12, 0), (8, 1), (6, 1), (2, 1 WRAP), (20, 2 BACK) and writes
  `ShadowsStoredToBeRendered = 0` (0x70B71A).
- gr returns before any state change when the count is 0, so the render state after the call differs (FOG left as before instead of 0,
  ALPHATESTFUNCTIONREF / TEXTUREFILTER / ZWRITE / ... not forced).
- Verdict: CONFIRMED. Class: keep-original (later renderers observe the state).
