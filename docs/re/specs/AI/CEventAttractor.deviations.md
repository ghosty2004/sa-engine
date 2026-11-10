# CEventAttractor (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

Source: `re/gta-reversed/source/game_sa/Events/EventAttractor.cpp` (pin 20d8d2a). RNG: original
`CGeneral::GetRandomNumberInRange(int min, int max)` 0x407180 (cdecl) = `min + _ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * (max - min))`
(`fild; fmul [0x858B14]; fimul (max-min); call _ftol 0x821B40`, 0x407190-0x4071B1); gr's helper is `(int)lerp(min, max-1, rand()/32767.0f)`
(docs/re/cat14-gr-deviations.md finding 1): different value for the same rand().

## 0x4AF4B0 CEventAttractor::AffectsPed  (gr_deviations = 3)
Original: `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, AL = 0/1. Order of tests (each failing test returns 0 at 0x4AF654):
`ped->IsAlive()` 0x5E0170 (0x4AF4BD); if `GetEventType()` (vtbl+4) == 0x1B then `m_entity` (+0x18) != 0 (0x4AF4CE-0x4AF4DB); if GetEventType()
== 0x1B (called again) then `IsEffectActive(m_entity, m_2dEffect)` 0x4AF460 (0x4AF4E5-0x4AF4FF); `ped->m_pIntelligence (+0x47C)->FindTaskByType(0x4BF)`
0x600EE0 must be null (0x4AF505-0x4AF517); cop block: if `ped->m_nPedType (+0x598) == 6` and GetEventType() == 0x1B (third call):
`FindPlayerWanted(-1)` 0x56E230 `->m_nWantedLevel (+0x2C) == 0`, `m_2dEffect->m_nAttractorType (+0x34 byte) == 5`,
`CPopulation::PedMICanBeCreatedAtThisAttractor(ped->m_nModelIndex (movsx +0x22), m_2dEffect + 0x38)` 0x6110E0 (0x4AF51D-0x4AF567).
Then (0x4AF56D-0x4AF5A8): `t = TaskManager(+0x47C +4).GetActiveTask()` 0x681720; `if (t && !m_bAvoidLookingAtAttractor (+0x1C) && t->GetTaskType()
(vtbl+0x10) == 0xE9) return 0`; then, UNCONDITIONALLY, `if (m_2dEffect->m_nAttractorType == 4 && !CWeather::IsRainy() 0x4ABF50) return 0`.
Then `if (m_bAvoidLookingAtAttractor) return 1` (0x4AF5AE-0x4AF5B3).
- EventAttractor.cpp:53 `GetRandomNumberInRange(0, 100) >= m_nPedExistingProbability`: original `r = GetRandomNumberInRange(0, 100)` 0x407180 (0x4AF5B9-0x4AF5BD);
  `if (r >= (u8)m_2dEffect[+0x35]) return 0` (signed `cmp eax,edx; jge`, 0x4AF5C5-0x4AF5CE). Verdict: CONFIRMED (gr helper formula). Class: keep-original.
- EventAttractor.cpp:55 `GetRandomNumberInRange(0, 100) >= field_36`: original 0x4AF5D4-0x4AF5E9, `if (r >= (u8)m_2dEffect[+0x36]) return 1`.
  Verdict: CONFIRMED. Class: keep-original.
- EventAttractor.cpp:58 `time = GetRandomNumberInRange(2000, 4000)`: original only if `!g_ikChainMan.IsLooking(ped)` 0x6181A0 (ECX = 0xC15448,
  0x4AF5EB-0x4AF5F8): `time = GetRandomNumberInRange(0x7D0, 0xFA0)` 0x407180 (0x4AF5FB-0x4AF605) is drawn BEFORE the matrix transform;
  `point = m_entity->GetMatrix() 0x411990 * m_2dEffect->m_Pos (CMatrix::operator* 0x59C890, vector at m_2dEffect+0 passed by pointer)`;
  `g_ikChainMan.LookAt(0x85B1FC "CEventAttractor" tag, ped, 0, time, -1, &point, 0, 0x3E800000 = 0.25f, 500, 3, 0)` 0x618970 (0x4AF629-0x4AF64E);
  then return 0 (falls to 0x4AF654). Verdict: CONFIRMED (helper). Class: keep-original.
- Found in passing (no marker), CONFIRMED, keep-original: gr EventAttractor.cpp:44-49 skips the shelter/rain test when the active task is null
  or m_bAvoidLookingAtAttractor is set (`!activeTask || avoid || (type != USE_EFFECT && (shelter -> rainy))`); the original evaluates the
  shelter test (0x4AF598-0x4AF5A8) for every ped that passed the 0xE9 test, so a ped without an active task (or with avoid set) near a shelter
  attractor in dry weather returns 0 in the original but may return 1 / look in gr.
