# CObjectData: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x5a2d00 CObjectData::SetObjectData  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/ObjectData.cpp:170` (rbf `CObjectData_SetObjectData_UninitializedPhysics`): on the "no object info" path gr also copies `m_fElasticity` and `m_fBuoyancyConstant` from the default entry.
  - Original bytes: `if (ms_modelInfoPtrs[model]->m_nObjectInfoIndex (+0x10, i16) == -1)` (0x5a2d11) -> 0x5a2e23: `obj+0x8c (m_fMass) = obj+0x90 (m_fTurnMass) = 99999.0f (0x47c34f80)`; `dword [obj+0x40] = (flags & ~0x2) | 0x80000c` (0x5a2e3b-0x5a2e3e: clears bApplyGravity, sets 0x4, 0x8 and 0x800000); `obj+0x160 (m_pObjectInfo) = &ms_aObjectInfo[0] (0xbb4a90)`; return. Elasticity (+0x9c), buoyancy (+0xa0) and air resistance (+0x98) are not written on this path.
  - Verdict: CONFIRMED. With the fix on, gr writes two fields the original leaves untouched (they keep whatever the object had, e.g. constructor values).
  - Class: keep-original (take the `!fix` branch).

## 0x5b5360 CObjectData::Initialise  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/ObjectData.cpp:62` (bugfix-comment): `if (line[0] == '#') continue; // NOTSA: Skip comments so we can use a sanity check for sscanf.`
  - Original bytes: the line filter is only `';'` -> skip (0x5b5476), `'\0'` -> skip (0x5b547e), `'*'` -> stop (0x5b5486). A `'#'` line is processed as data: the current free slot (ESI, starting at index 5 = 0xbb4c20, stride 0x50) is zeroed (`rep stosd` 20 dwords, 0x5b549b), `sscanf` (0x8220ad, format 0x868dc8) fills it, the sscanf return value is ignored, then `CModelInfo::GetModelInfo(modelName, 0)` (0x4c5940) returns null for a `#...` name and the line is skipped (0x5b557f) without advancing the slot.
  - Verdict: CONFIRMED (the original has no `#` handling), but the only effect is scratch writes into the next free slot, which the next data line zeroes again (or which stays unreferenced after the last line). No observable difference unless a model name starts with `#`.
  - Class: harmless.
- gr `ObjectData.cpp:70` (fix-bugs-comment): `sscanf_s(...) // FIX_BUGS: Sized string read` wrapped in `VERIFY`.
  - Original bytes: plain `sscanf` (0x8220ad) with unsized `%s` into 256-byte stack buffers (modelName, effectName); return value not checked.
  - Verdict: CONFIRMED in mechanism (bounded vs unbounded read, plus gr's VERIFY on the count), no difference for well-formed data (tokens < 256 chars).
  - Class: harmless (port: unbounded semantics are only relevant for malformed files; do not abort on a short count).
