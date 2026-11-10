# CPedAttractorManager: gta-reversed deviation notes (AI, group I2)

Manager layout: 10 SArray<CPedAttractor*> (16 bytes each: +0 allocator, +4 first, +8 last, +0xC end) at +0x00..+0x90, one per
attractor type 0..9 (seat, ATM, stop, pizza, shelter, trigger-script, look-at, scripted, park, step); 2dEffect `fx->+0xC` = effect
type (3 = EFFECT_ATTRACTOR), `fx->+0x34` = attractor type.

## 0x5EB690 CPedAttractorManager::IsPedRegisteredWithEffect<CPedAttractor>  (gr_deviations = 2)
- Original bytes (`__thiscall bool (this, CPed* ped, fx, entity, const SArray* arr)`, `ret 0x10`): for each attractor `a` in arr (in
  order): if `a->+4 == fx` and `a->+8 == entity` and `a->IsRegisteredWithPed(ped)` 0x5EB4C0 -> return 1; else 0 (0x5EB698-0x5EB6E1).
- Row PedAttractorManager.cpp:35 (hook comment): Verdict REFUTED (not a behavioural statement). Class: harmless.
- Row PedAttractorManager.h:115 (NOTSA: uses GetRelevantAttractor): GetRelevantAttractor (h:149) applies the same three tests in the
  same order -> Verdict REFUTED (same result). Class: harmless.

## 0x5EBB00 CPedAttractorManager::HasEmptySlot  (gr_deviations = 1)
Row cpp:149 (`assert(isa<C2dEffectPedAttractor>(fx)); // NOTE: Original function just returned false`).
- Original bytes: if `fx == 0` or `fx->+0xC != 3` return 0 (0x5EBB04-0x5EBB10); attractor type `fx->+0x34 > 9` -> return 1; else
  `a = FindAssociatedAttractor(fx, entity, &array[type])` 0x5EB6F0; return `a ? a->HasEmptySlot() 0x5EAF10 : 1`.
- Verdict: CONFIRMED (the original returns false for a null / non-attractor effect; gr asserts and in release would continue).
  Class: keep-original.

## 0x5EBBA0 CPedAttractorManager::HasQueueTailArrivedAtSlot  (gr_deviations = 1)
Row cpp:129 (same assert note).
- Original bytes: null / non-attractor fx -> 0; type > 9 or no attractor -> 1; `n = queue size ((+0x24 - +0x20) >> 2)`; tail =
  GetTailOfQueue 0x5EB5B0 (null -> 1); if tail == GetHeadOfQueue 0x5EB590: return `tail TaskMgr.FindActiveTaskByType(0xEB USE_ATTRACTOR)
  != 0`; else `t = FindActiveTaskByType(0xEA WAIT_AT_ATTRACTOR)`; return `t && t->+0x10 == n - 1` (0x5EBC14-0x5EBC83).
- Verdict: CONFIRMED for the null/non-attractor guard (returns false); the rest matches gr. Class: keep-original.

## 0x5EE190 CPedAttractorManager::GetPedAttractorManager  (gr_deviations = 1)
Row cpp:54 (`NOTSA: They used new`).
- Original bytes: if `[0xC09840]` null: `p = operator new(0xA0)` 0x82119A; `[0xC09840] = p ? CPedAttractorManager(p) 0x5EDF10 : 0`; return
  `[0xC09840]` (SEH frame; never freed).
- Verdict: CONFIRMED (heap singleton vs gr function-static). Class: harmless (same lifetime/behaviour; a port may use a static).

## 0x5EFCA0 CPedAttractorManager::RegisterPedWithAttractor  (gr_deviations = 2)
- Original bytes (`__thiscall CPedAttractor* (this, CPed* ped, fx, entity, int32 moveState)`, `ret 0x10`): if
  `CScripted2dEffects::GetIndex(fx) < 0` 0x6F9F60 and `!CEventAttractor::IsEffectActive(entity, fx)` 0x4AF460 -> return 0; if
  `IsPedRegisteredWithEffect(ped, fx, entity)` 0x5EBD70 -> return 0; if `fx->+0xC != 3` or `fx->+0x34 > 9` -> return 0; else
  `RegisterPed<T>(ped, fx, entity, moveState, &array[type])` (jump table 0x5EFDF0; 0x5EF980 for the base type) and return it.
- Row cpp:186 (assert note): Verdict CONFIRMED (the original returns null for a non-attractor effect - after the two earlier tests;
  gr asserts first). Class: keep-original.
- Row cpp:195 (notsa::cast helper): Verdict REFUTED. Class: harmless.
