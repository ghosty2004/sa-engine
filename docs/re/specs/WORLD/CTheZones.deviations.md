# CTheZones: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x5728a0 CTheZones::CreateZone  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 1, REFUTED 0, UNCLEAR 1).
Args (cdecl, bytes): infoLabel +4, type +8 (dword), pos1 +0xc..+0x14, pos2 +0x18..+0x20, level +0x24 (byte used), textLabel +0x28.
- gr `source/game_sa/TheZones.cpp:263` (notsa-unreachable): the zone-array lambda has `NOTSA_UNREACHABLE()` for types other than NAVI/LOCAL_NAVI/MAP.
  - Original bytes: after preparing the labels, `if (type < 0) return` (signed, 0x572993); `if (type <= 1)` navigation zone (0x57299d); `else if (type == 3)` map zone (0x5729a3); any other type (2 = INFO, or > 3) returns without creating anything (`jne 0x572b52`).
  - Verdict: CONFIRMED. Original: no-op for types other than 0, 1, 3.
  - Class: keep-original.
- gr `TheZones.cpp:267` (original-comment): "Some vanilla code compares all 8 bytes, so we need to pad with zeros"; gr does `strncpy(dst, src, 7); dst[7] = 0` directly into the zone label (strncpy zero-pads).
  - Original bytes: each label is first uppercased IN PLACE in the caller's string (bytes 'a'..'z' minus 0x20, over the u16 strlen, 0x572906-0x572933 for infoLabel, 0x572946-0x572976 for textLabel), then `strncpy(stackbuf, src, 7)` (0x821f40) and `stackbuf[7] = 0` (0x57294b, 0x57298e). The stack buffers are copied into the zone with a byte loop that stops after the NUL (0x5729d0-0x5729d8, 0x5729f0-0x5729f8 for map zones; 0x572aa0/0x572ac0 for navigation zones): bytes after the NUL in the zone are NOT written.
  - Verdict: UNCLEAR. The zone label is zero-padded in the original only if those bytes were already zero. gr's CTheZones::Init (gr TheZones.cpp:226-227) fills both arrays with `CZone{}`; whether the original Init clears them was not checked here (open question: original CTheZones::Init, and the copy at 0x5729d0). Class: keep-original (copy up to and including the NUL only).
- Found in passing (keep-original unless noted):
  1. Caller strings are modified in place (uppercased) by the original; gr leaves the source untouched and uppercases the copy with `toupper`. CONFIRMED.
  2. m_nType (+0x1e) is written by the original: navigation zones get `type` (0 or 1, BL, 0x572ace), map zones get 3 (0x5729fe). gr's CreateZone never writes m_nType. CONFIRMED.
  3. Coordinates: the original swaps pos1/pos2 per axis as floats first (`if pos1.c > pos2.c` swap, 0x5728a0-0x5728fc), then converts each with `_ftol` (0x821b40, truncation) and stores the low 16 bits: X1 +0x10, Y1 +0x12, Z1 +0x14, X2 +0x16, Y2 +0x18, Z2 +0x1a. gr converts first, then `minmax`. Same result for finite in-range values (truncation is monotonic). Harmless.
  4. Navigation zones: `AssignZoneInfoForThisZone(oldCount)` (0x572180) is called before `TotalNumberOfNavigationZones` is incremented (0x572b40, 0x572b48); gr increments first. gr's AssignZoneInfoForThisZone skips the new index, so likely harmless; the original 0x572180 loop bound was not checked here.
  5. Order inside the zone: map zones `TotalNumberOfMapZones` (0xba1900) is incremented after the coordinates (0x572a5a), level byte at +0x1f.

## 0x572c40 CTheZones::FindZoneByLabel  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/TheZones.cpp:328` (original-comment): `assert(type == eZoneType::ZONE_TYPE_INFO); // Originally an 'if' returning '-1', but let's be safe`.
  - Original bytes: first copies `name` with an unbounded byte loop into an 8-byte zeroed stack buffer (`[esp+0xc]`,`[esp+0x10]` zeroed at 0x572c50/0x572c54, copy 0x572c60-0x572c68; a name of 8+ chars overflows the buffer). Then `if (type (full dword arg +8) != 2) return -1` (`sub eax,2; jne 0x572cb0`, 0x572c6e). Then `for (u16 i = 0; i < TotalNumberOfNavigationZones (0xba3794, u16, unsigned compare); i++)` compares the two dwords of the buffer with `NavigationZoneArray[i]` (stride 0x20) +0 and +4 (0x572c96-0x572ca8): an exact 8-byte compare of m_InfoLabel. Returns i, else -1 (`or ax,0xffff`).
  - Verdict: CONFIRMED. In a release build gr's assert vanishes and gr searches for any type; the original returns -1 for every type other than 2 (ZONE_TYPE_INFO). Also found in passing: gr compares `std::string_view` (bytes up to the NUL), the original compares all 8 label bytes against the zero-padded name (differs only if label bytes after the NUL are non-zero).
  - Class: keep-original.
