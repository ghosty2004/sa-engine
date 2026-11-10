# CTagManager (POP) - gta-reversed deviation notes (group P4)

Source: `re/gta-reversed/source/game_sa/TagManager.cpp` (pin 20d8d2a). ms_tagDesc 0xA9A8C0 (tTagDesc stride 8: +0 CEntity* Entity,
+4 u8 Alpha), ms_numTags 0xA9AD70 (i32), ms_numTagged 0xA9AD74.

## 0x49CC60 CTagManager::ShutdownForRestart  (gr_deviations = 1)
Original (stub `jmp 0x156E900`, `__cdecl void ()`): `for (i = ms_numTags; i-- > 0;) ms_tagDesc[i].Alpha = 0` (byte 0xA9A8C4 + 8i, descending,
skipped when ms_numTags == 0); `ms_numTagged = 0`. ms_numTags is NOT reset.
Row TagManager.cpp:44 (comment "leave it as it was originally"): the `#if 0` block is not compiled.
- Verdict: REFUTED. Class: keep-original.
Row TagManager.cpp:46 (ReversibleBugFix CTagManager_MissingTagCountResetOnShutdown): inside the same `#if 0`, never compiled; the original
does not reset ms_numTags either.
- Verdict: REFUTED (dead code in gr). Class: keep-original.
Found in passing: gr's active branch clears the Alpha of ALL MAX_TAGS entries, the original only of the first ms_numTags entries
(0x156E910). Entries past ms_numTags are not saved or rendered, so this is harmless unless ms_numTags later grows over stale entries.

## 0x49CD30 CTagManager::SetAlpha(RpAtomic*, uint8)  (gr_deviations = 1)
Original: `movzx ax, byte [esp+8]; mov [esp+8], eax; jmp CVisibilityPlugins::SetUserValue 0x732380` (tail call; the dword written back holds
the caller's upper EAX half in bits 16-31; SetUserValue's parameter is 16-bit).
Rows TagManager.cpp:143 (assert) and :150 (ReversibleBugFix GenericCrashing): both belong to the CEntity overload SetAlpha(CEntity&, uint8)
(0x49CEC0), not to this function; 0x49CD30 has no corresponding code.
- Verdict: REFUTED (row mapped to the wrong overload). Class: harmless.

## 0x49CF90 CTagManager::GetAlpha(CEntity*)  (gr_deviations = 1)
Original (`__cdecl uint8 (CEntity* e)`): if `e->m_pRwObject (+0x18)`: tail call `CVisibilityPlugins::GetUserValue(atomic)` 0x7323A0.
Else search `i = ms_numTags - 1 .. 0` (descending) for `ms_tagDesc[i].Entity == e` and return `ms_tagDesc[i].Alpha`; when not found (or
ms_numTags == 0) EAX = 0 and it executes `mov al, [eax + 4]` (0x49CFC0-0x49CFC2): a read from address 4, i.e. an access violation.
Row TagManager.cpp:121 (assert IsTag): no check in the original. Verdict: REFUTED (debug only). Class: harmless.
Row TagManager.cpp:128 (ReversibleBugFix GenericCrashing: return 0 when no desc):
- Original bytes: 0x49CFC0 reads [4] (crash).
- Verdict: CONFIRMED (gr default build returns 0). Class: keep-original (the not-found path is a crash in the original; ports must not
  silently return 0 - record as a divergence if a guard is added).

## 0x49D160 CTagManager::GetNearestTag  (gr_deviations = 1)
Original (`__cdecl CEntity* (const CVector* p)`): best = FLT_MAX (0x7F7FFFFF), idx = -1; for i = ms_numTags-1 .. 0 (descending): pos = entity
matrix ? &matrix->pos : &placement.pos; `d = (p.y - pos.y)^2 + (p.x - pos.x)^2` (y term first, 0x49D197-0x49D1A9); if `d < best` (strict,
NaN fails): best = d, idx = i. Return `ms_tagDesc[idx].Entity` (0x49D1C9).
Row TagManager.cpp:210 (BUGFIX: return nullptr when there are no tags):
- Original bytes: with ms_numTags == 0, idx = -1 and the function returns the dword at 0xA9A8B8 (just before ms_tagDesc; .bss, 0 in the
  image), so the original also returns 0 unless that dword was written.
- Verdict: CONFIRMED (gr returns literal nullptr; the original returns the dword at 0xA9A8B8). Class: keep-original.
Found in passing (CONFIRMED, keep-original): the original iterates downwards with a strict `<`, so on equal distances the HIGHER index wins;
gr iterates upwards and keeps the lower index.

## 0x5D3DA0 CTagManager::Load  (gr_deviations = 1)
Original (`__cdecl bool ()`): `CGenericGameStorage::SaveDataToWorkBuffer(&ms_numTags, 4)` (0x5D1270, a save call in Load); then for
`i = 0; i < ms_numTags (unsigned); i++`: `LoadDataFromWorkBuffer(&ms_tagDesc[i].Alpha, 1)` (0x5D1300); return AL = 1.
Row TagManager.cpp:241 (`#if 0` FIX_BUGS block, not compiled): Verdict: REFUTED. Class: harmless.
Row TagManager.cpp:242 (comment in the same block): Verdict: REFUTED. Class: keep-original.
Row TagManager.cpp:246 ("original also saves into buffer instead of loading"):
- Original bytes: 0x5D3DA8 calls SaveDataToWorkBuffer with ms_numTags, exactly as gr's active branch.
- Verdict: REFUTED (gr matches). Class: keep-original. Note: gr's Load returns void; the original returns 1 in AL.
