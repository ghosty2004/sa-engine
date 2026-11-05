# cHandlingDataMgr: gta-reversed deviation notes (VEH)

Source: `re/gta-reversed/source/game_sa/cHandlingDataMgr.cpp` (pin 20d8d2a). `VehicleNames` 0x8D3978 = `char[210][14]`
(ends at 0x8D44F4).

## 0x5BD830 cHandlingDataMgr::LoadHandlingData  (gr_deviations = 2)
Convention: `__thiscall void (cHandlingDataMgr* this)`, plain `ret`. Locals 0xFC.
Original structure (for orientation): `SetDir("DATA")`, `OpenFile("HANDLING.CFG", "rb")`, `SetDir("")`; loop over
`CFileLoader::LoadLine` (0x536F80). A line equal to ";the end" (9-byte `repe cmpsb`, i.e. exact strcmp, 0x5BD89E)
ends the loop; ';' lines are skipped. First character '!' (bike), '$' (flying), '%' (boat), '^' (anim group), else
vehicle. Every section tokenises with the CRT tokenizer 0x82244B (strtok) using the delimiter string " \t" (0x85A510,
copied to a stack buffer) and a per-section field counter (BL) that selects the destination through a jump table:
token -> `atof` (0x823CEE) + `fstp dword` (double rounded to f32), `atol` (0x82258E) for integers, `sscanf(tok, "%x")`
(0x8220AD, format 0x86A94C) for the two flag words (vehicle: +0xCC model flags, +0xD0 handling flags, the latter also
copied to +0x78 transmission flags, 0x5BE3D2-0x5BE400). The name token goes to `FindExactWord(tok, VehicleNames, 14,
210)` (0x6F4F30) and the id is written to the record's +0. The vehicle record is `this + 0x14 + id*0xE0`
(0x5BE1E9-0x5BE1EF); after its last token `ConvertDataToGameUnits(record)` (0x5BE44C). Bike records are addressed as
`this + 0x8F54 + id*0x40` (= `&m_aBikeHandling[id - 162]`, no range check). The file is closed at the end (0x5BE46D).

Row cHandlingDataMgr.cpp:183 (notsa ScopeGuard): REFUTED / harmless (the original closes the file on both exits, the
";the end" path jumps to 0x5BE466 -> CloseFile).

Row cHandlingDataMgr.cpp:190 (FIX_BUGS "Sized string read", `sscanf_s(line, "%31s", name)`).
- Original bytes: there is no sscanf for the name; the first (non-marker) token from strtok is passed directly to
  FindExactWord (e.g. 0x5BD8FC-0x5BD90B, 0x5BE1D3-0x5BE1E2). FindExactWord 0x6F4F30 compares only
  `strlen(VehicleNames[i])` characters (strncpy into a 20-byte local, then strncmp), so a token that merely starts with
  a known name matches; when nothing matches it returns 210 (the count), not -1 (0x6F4F7F path, EAX = count).
- gr: reads a <= 31-char word with sscanf, uses gr's FindExactWord (returns -1 on miss and the line is skipped), and
  parses all fields with one `sscanf_s` format using `%f`, `%hhu`, `%c`, `%d`, `%x` with TAB separators
  (tHandlingData::InitFromData, cHandlingDataMgr.cpp:31).
- Verdict: CONFIRMED (tokenisation, delimiters " \t", float parsing via atof -> f64 -> f32, prefix name matching and
  the miss value 210 all differ).
- Class: keep-original.

Row cHandlingDataMgr.cpp:224 (`NOTSA_UNREACHABLE` when a line fails to parse).
- Original bytes: no error path. Missing tokens leave the remaining fields unchanged; extra tokens hit the jump-table
  default (`ja` past the last case, e.g. 0x5BE1C6 `cmp ecx, 0x23; ja 0x5BE430`). An unknown vehicle name gives id 210
  and the record is written at `this + 0x14 + 210*0xE0` (= start of m_aBikeHandling, overwriting it).
- Verdict: CONFIRMED (gr aborts; the original silently continues).
- Class: keep-original (unreachable with the shipped HANDLING.CFG).

## 0x6F4FD0 cHandlingDataMgr::GetHandlingId  (gr_deviations = 1)
Convention: `__thiscall int32 (cHandlingDataMgr* this /*unused*/, const char* name [esp+4])`, `ret 4`.
Row cHandlingDataMgr.cpp:283 (`NOTSA_UNREACHABLE("Can't find {}")`).
- Original bytes: for i = 0..209: `if (strncmp(name, VehicleNames[i], 14) == 0) return i` (0x6F4FE0-0x6F4FEE);
  falls out with i = 210 and returns 210 (0x6F4FFC).
- Verdict: CONFIRMED (gr aborts; original returns 210).
- Class: keep-original.
Found in passing (CONFIRMED, keep-original): gr uses `strcmp`, the original `strncmp(name, entry, 14)`; names of 14+
characters compare only the first 14.
