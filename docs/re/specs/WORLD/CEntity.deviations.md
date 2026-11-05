# CEntity: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x532b70 CEntity::HasPreRenderEffects  (gr_deviations = 1)
Rows checked: 4 (CONFIRMED 0, REFUTED 4, UNCLEAR 0).
- gr `source/game_sa/Entity/Entity.cpp:761` (bugfix-comment, matched on the word "fixed"): "fixed model IDs, in defauld.ide" before `contains({MODEL_MISSILE, MODEL_GRENADE, MODEL_MOLOTOV}, id)`.
  - Original bytes: the same three constants are compared inline, `cmp si,0x159` (345), `0x156` (342), `0x158` (344) (0x532c04-0x532c1f), placed between the MI_BRIEFCASE and MI_BEACHBALL tests. All tests are pure equality compares joined by OR, so the position in the chain does not matter.
  - Verdict: REFUTED (the comment means "hard-coded IDs"; no behaviour change).
  - Class: harmless.
- gr `Entity.cpp:731`, `:762`, `:781` (notsa-helper `notsa::contains`, deviates=no): REFUTED, equivalent to the original's chains of `movzx MI_x; cmp eax, edx; je` (0x532bb9-0x532c97, 0x532cd1-0x532cf4). Class: harmless. Note: the original zero-extends the u16 MI_* globals (`movzx`) and sign-extends the model index (`movsx`), so an unset MI (0xFFFF) never matches.

## 0x5332c0 CEntity::SetRwObjectAlpha  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Entity/Entity.cpp:913` (notsa-unreachable): `default: NOTSA_UNREACHABLE_CASE(type);` in the switch on the RwObject type.
  - Original bytes: `obj = this->m_pRwObject (+0x18)`; null -> return (0x5332c5). `type = byte [obj]`: 1 (atomic) -> `geometry = atomic->geometry (+0x18); geometry->flags (+8) |= 0x40; RpGeometryForAllMaterials(geometry, SetCompAlphaCB 0x533280, alpha)` (0x5332e6-0x5332f8); 2 (clump) -> `RpClumpForAllAtomics(clump, SetAtomicAlpha 0x533290, alpha)` (0x5332d0-0x5332db); any other type -> return (`jne 0x533300`, 0x5332ce).
  - Verdict: CONFIRMED. The original silently does nothing for other RwObject types; gr aborts/UB.
  - Class: keep-original (no-op default).

## 0x533410 CEntity::GetRandom2dEffect  (gr_deviations = 1)
Rows checked: 2 (CONFIRMED 1, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Entity/Entity.cpp:990` (rng-helper): `return numCandidates ? candidates[CGeneral::GetRandomNumberInRange(0u, numCandidates)] : nullptr;`
  - Original bytes: candidates collected in ascending effect order (max 32, `cmp ebx,0x20; jge` skip, 0x533478) exactly like gr; if `n > 0`: `idx = _ftol((float)(rand() & 0xFFFF) * (1/32768) [0x858b14] fimul n)` (`fimul dword` multiplies by the integer count, 0x533494-0x5334b0), return `candidates[idx]` (0x5334b5).
  - Verdict: CONFIRMED (cat14 finding 1: same range 0..n-1, different mapping from rand()).
  - Class: keep-original.
- gr `Entity.cpp:980` (notsa-helper `notsa::cast`, deviates=no): REFUTED; the original calls `CPedAttractorManager::HasEmptySlot(GetPedAttractorManager(), effect, this)` (0x5ee190, 0x5ebb00) only when `mustBeFree`, same as gr. Effect type test: `movzx byte [fx+0xc]` vs the full int argument. Class: harmless.

## 0x534030 CEntity::DeleteRwObject  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Entity/Entity.cpp:353` (define-branch `#ifdef SA_SKINNED_PEDS`, lines 353-357): remove skin animation from the clump's atomics before `RpClumpDestroy`.
  - Original bytes: clump branch (type 2) always does it: `a = RwHelper::GetFirstAtomic(clump)` (0x734820); if `a && RpSkinGeometryGetSkin(a->geometry (+0x18))` (0x7c7550) then `RpClumpForAllAtomics(clump, AtomicRemoveAnimFromSkinCB 0x734b90, 0)`; then `RpClumpDestroy` (0x534060-0x534092).
  - Verdict: REFUTED. SA_SKINNED_PEDS is defined in gr's build (`source/CMakeLists.txt:256`), so gr runs the same code; the port must keep it unconditionally.
  - Class: harmless.
- Found in passing (keep-original): atomic branch (type 1). The original reads `frame = atomic->object.parent (+4)` and calls `RpAtomicDestroy(atomic)` then `RwFrameDestroy(frame)` unconditionally (0x534045-0x534050); gr destroys both only if the frame is non-null. Then (as gr): `m_pRwObject = 0`, `RemoveRef`, `CStreaming::RemoveEntity(m_pStreamingLink (+0x28))`, link = 0, `gBuildings (0xb71804)--` for buildings.

## 0x571b70 CEntity::RegisterReference  (gr_deviations = 3)
Rows checked: 3 (CONFIRMED 3, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Entity/Entity.cpp:1698` (pool-iteration): ped pool `GetAllValid()` calling `PruneReferences()` until `CReferences::pEmptyList` is non-null.
- gr `Entity.cpp:1706` (pool-iteration): same for the vehicle pool.
- gr `Entity.cpp:1715` (pool-iteration): same for the object pool.
  - Original bytes: if `pEmptyList (0xba1768)` is null: ped pool (0xb74490, stride 0x7c4), then vehicle pool (0xb74494, stride 0xa18), then object pool (0xb7449c, stride 0x19c); each loop is DESCENDING (`flags[esi-1]` read, `dec esi`, 0x571bdb/0x571c26/0x571c69), skips free/null slots, calls `CEntity::PruneReferences` (0x571a90) and stops all three loops as soon as `pEmptyList != null` (0x571bfc, 0x571c47, 0x571c8a). Then, if `pEmptyList` is non-null: `ref = pEmptyList; pEmptyList = ref->next; ref->next = this->m_pReferences (+0x24); this->m_pReferences = ref; ref->m_ppEntity (+4) = entity` (0x571c97-0x571cab).
  - Verdict (all three): CONFIRMED. The original prunes the highest slots first; gr the lowest. This changes which entities' stale references get pruned before the free list refills, and so the free-list contents/order (observable through later reference allocation and through which stale references survive).
  - Class: keep-original.
- Early-out (matches gr): building (`byte [this+0x36] & 7 == 1`) with neither flag 0x4400000 in dword +0x1c set and `byte [this+0x2e]` (IPL index) == 0 -> return (0x571b73-0x571b8a). Duplicate check walks `m_pReferences` comparing `ref->m_ppEntity` with the argument.
