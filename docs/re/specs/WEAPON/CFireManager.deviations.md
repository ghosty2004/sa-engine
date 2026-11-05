# CFireManager (WEAPON) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/FireManager.cpp` (pin 20d8d2a).

## 0x53AF00 CFireManager::Update  (gr_deviations = 6)
Original: `__thiscall void ()` (ECX = gFireManager), aligned frame. `t = (float)rand() * 3.0518509e-05f [0x858C7C]` (raw rand).
Riot block (0x53AF42-0x53B17E), entered if `CGameLogic::LaRiotsActiveHere()` and `now/500 != prevNow/500` (unsigned division by 500: `mul 0x10624DD3`, `edx >> 5`).
Row FireManager.cpp:377 (rng-helper, angle `GetRandomNumberInRange(0.0f, TWO_PI)`).
- Original bytes: `a = t * 6.2831855f [0x858CBC]` (stored f32, 0x53AF75-0x53AF8E).
- Verdict: CONFIRMED (inline formula, not gr's helper). Class: keep-original.
Row FireManager.cpp:378 (rng-helper, distance `GetRandomNumberInRange(35.0f, 60.0f)`).
- Original bytes: `d = t * 25.0f [0x858FE8] + 35.0f [0x85F070]` (second rand), then offset `(sin(a)*d, cos(a)*d, 10.0f)` added to the
  camera position, z replaced by `FindGroundZFor3DCoord(p, &hit, &hitEntity)`.
- Verdict: CONFIRMED. Class: keep-original.
Row FireManager.cpp:393 (inlined, "On the Android version it's inlined").
- Original bytes: the 0.2 threshold is an f64 constant (0x862D08) and the FX name is the string at 0x863A54 ("riot_smoke").
- Verdict: REFUTED (comment only). Class: harmless.
Row FireManager.cpp:402 (rng-helper `StartFire(point, GetRandomNumberInRange(0.5f, 2.5f), ...)`).
- Original bytes: `CGeneral::GetRandomNumberInRange(0.5f, 2.5f)` (0x41BD90, real helper) then `StartFire(point, size, 1, null, 30000, 5, 1)`
  (0x539F00).
- Verdict: CONFIRMED (helper formula). Class: keep-original.
- Found in passing (CONFIRMED, keep-original): the smoke/fire decision is reached only if `hitEntity != null && (hitEntity->+0x1E & 1)
  && hit && point.z > 0.5f` (0x53B0AD-0x53B0DE); gr has none of these gates. Camera forward = matrix ? matrix.forward :
  `(-sin(heading 0xB6F038), cos(heading), 0)`. Then `dot(fwd, normalise(cam - point)) > 0.2` (f64 compare) -> smoke; else
  `rand() >= 0x3FFF` -> StartFire; else smoke (same as gr). Smoke: `g_fxMan.CreateFxSystem("riot_smoke", &point, null, 1)` + PlayAndKill.
Row FireManager.cpp:437 (original-comment "R* also does some weird vector point calculation ... probably not used").
- Original bytes: an accumulator vector at [esp+0x40..0x48] is initialised at 0x53B45A from [esp+0x64..0x6C] (NOT zero) and
  `+= fire.pos * ceil(strength)` for each grouped fire (0x53B4ED-0x53B537; ceil = CRT 0x823820 on the f64 strength, then `_ftol2`
  for the integer sum). No later read of these three locals was identified, but the stack offsets after 0x53B548 were not fully
  tracked.
- Verdict: UNCLEAR (likely dead; open question 0x53B45A / later reads). Class: harmless if dead.
Row FireManager.cpp:451 (rng-helper `fColorMult = GetRandomNumberInRange(0.6f, 1.0f)`).
- Original bytes (0x53B594-0x53B5D5): `m = t * 0.39999998f [0x862D50] + 0.6f [0x858CC8]` (stored f32); shadow colour components
  `_ftol2((float)baseColour.b/g/r [0x8CCFF2/1/0] * m)`.
- Verdict: CONFIRMED (inline formula, constant 0.39999998f). Class: keep-original.
