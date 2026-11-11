# CTaskSimpleSwim: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 groups J2-J4 (helper B).

## 0x688A90 CTaskSimpleSwim::ProcessControlInput  (gr_deviations = 2)
Row TaskSimpleSwim.cpp:943 (`fRotationX = std::clamp(fRotationX, -1.0f, 1.0f); // originally min/max used`):
- Original bytes: `t = (asin(camFront.z) 0x821E70 - +0x24) * 10.0 [0x85862C]` (0x688C76-0x688C81), then `t = 0x420800(-1.0f, t)` (returns arg1 if
  `arg1 > arg2` else arg2, i.e. max(-1, t), NaN -> t) and `t = 0x404330(1.0f, t)` (returns arg2 if `arg1 >= arg2` or unordered, else arg1, i.e.
  min(1, t), NaN -> t) (0x688C8A-0x688CA0). Both helpers return in ST0.
- Verdict: REFUTED - for all inputs, including NaN (passes through both) and the bounds, `std::clamp(t, -1, 1)` gives the same value. Class: harmless
  (port may call the same helpers to stay byte-exact).
Row TaskSimpleSwim.cpp:976 (`// BUG: it should be m_fTimeCanRun <= 0.1f`, code `if (m_fTimeCanRun <= 0.0f) m_fTimeCanRun = 0.1f`):
- Original bytes: `fld [playerData+0x18]; fcomp 0.0 [0x858B50]; test ah,0x41; jp skip` (0x688F3E-0x688F4C) -> stores 0.1f (immediate 0x3DCCCCCD,
  0x688F4E) when the value is < 0 or == 0; > 0 and NaN skip. gr's `<= 0.0f` matches (NaN false).
- Verdict: REFUTED (bug note only; the original has the same "bug"). Class: keep-original.

## 0x68AA70 CTaskSimpleSwim::ProcessEffects  (gr_deviations = 3)
Context: swim state 4 (SWIM_UNDERWATER_SPRINTING, `cmp ax,4`, 0x68AC27). `oxygen = 5` (0x68AC33); if `ped->IsPlayer()`: `x = playerData(+0x480)->+0x44
(m_fBreath) / CStats::GetFatAndMuscleModifier(8)` (0x559AF0, `fdivr`), `x *= 100.0 [0x858628]`, `x = 100.0 [0x858628] - x`, `x *= 0.33333334 [0x859040]`,
`oxygen = _ftol(x)` 0x821B40 (truncation, signed int) (0x68AC41-0x68AC6B). Then `if (CGeneral::GetRandomNumberInRange(0, 100) 0x407180 >= oxygen)`
(signed `jge`, 0x68AC78) skip; else the bubble position is the translation of matrix index 3 of the clump's hierarchy:
`RpHAnimHierarchyGetMatrixArray(GetAnimHierarchyFromSkinClump(ped->m_pRwClump)) + 0xF0` (0x68AC84-0x68AC8F; 3*0x40 + 0x30).
Row TaskSimpleSwim.cpp:660 (`assert(ped->GetPlayerData())`):
- Original bytes: no check, `mov esi,[edi+0x480]` then dereference (0x68AC41, 0x68AC4E). Verdict: REFUTED (debug assert only). Class: harmless.
Row TaskSimpleSwim.cpp:663 (gr `CGeneral::GetRandomNumberInRange(0, 100)` and `(unsigned)` compare):
- Original bytes: calls the original 0x407180 (int range) and compares SIGNED (`cmp eax, esi; jge`, 0x68AC78). gr's helper uses a different formula
  (docs/re/cat14-gr-deviations.md finding 1) and its compare is unsigned; additionally gr computes `/ 3.0f` where the original multiplies by
  0.33333334 [0x859040] (can differ in the last bit before truncation), and gr's `static_cast<uint32>` of a negative value differs from `_ftol`
  (negative oxygen: original never emits; gr's unsigned value is huge).
- Verdict: CONFIRMED. Class: keep-original (use 0x407180, `* 0.33333334f`, `_ftol`, signed compare).
Row TaskSimpleSwim.cpp:665 (FIX_BUGS: gr uses BONE_JAW instead of the original "air bubbles from ass"):
- Original bytes: matrix array index 3 (+0xF0) of the hierarchy (0x68AC8F), not the jaw bone.
- Verdict: CONFIRMED (gr changes the emission point). Class: keep-original.
