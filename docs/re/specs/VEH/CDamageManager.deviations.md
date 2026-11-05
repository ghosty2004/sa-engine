# CDamageManager: gta-reversed deviation notes (VEH)

Source: `re/gta-reversed/source/game_sa/DamageManager.cpp` (pin 20d8d2a). Layout (size 0x18): +0x00
`m_fWheelDamageEffect` f32, +0x04 `m_nEngineStatus` u8, +0x05 `m_anWheelsStatus[4]` u8, +0x09 `m_aDoorsStatus[6]` u8,
+0x0F padding, +0x10 `m_nLightsStatus` u32 (2 bits per light), +0x14 `m_nPanelsStatus` u32 (4 bits per panel, panel
p at bits 4p..4p+3; 4 = windscreen, 5 = front bumper, 6 = rear bumper).

RNG in this file: the original never calls CGeneral::GetRandomNumberInRange here. It calls CRT `rand()` (0x821B1E)
and tests low bits (`rand() & 7`, `rand() & 1`), or inlines `_ftol((rand() & 0xFFFF) * (1/32768.f) * k)`.
gr's helper is `(int)lerp(min, max-1, rand()/32767.f)` (docs/re/cat14-gr-deviations.md finding 1), which picks
different outcomes for the same rand() value and makes the top value almost impossible.

## 0x6A0520 CDamageManager::Init  (gr_deviations = 1)
Row DamageManager.cpp:47 (original-comment "R* used memset here").
- Original bytes: `rep stosd` of 6 zero dwords over `this` (0x6A0523-0x6A052C), i.e. all 0x18 bytes including
  `m_fWheelDamageEffect` and the padding byte +0x0F; returns `this` in EAX (0x6A052E). No callers in the catalogue
  (inlined copies elsewhere).
- gr: `m_fWheelDamageEffect = 0; ResetDamageStatus()` (engine 0, wheels 0, doors 0, lights 0, panels 0). Same field
  values; only the padding byte and the EAX return differ.
- Verdict: REFUTED (no behavioural difference in fields).
- Class: harmless (a port may return `this` and clear the whole struct to match exactly).

## 0x6C2320 CDamageManager::ProgressDoorDamage  (gr_deviations = 2)
Convention: `__thiscall bool (CDamageManager* this, u8 door [esp+4] (movzx), CAutomobile* veh [esp+8])`, `ret 8`.
Row DamageManager.cpp:194 (rng-helper `GetRandomNumberInRange(0, 8)`).
- Original bytes: `door >= 6` -> return false (0x6C232C-0x6C2331). `s = m_aDoorsStatus[door]`; s == 4 -> false;
  s > 4 -> fall to the store with s unchanged and return true (0x6C233D `ja 0x6C2360`). Jump table 0x6C2394 =
  {0x6C2346, 0x6C2346, 0x6C2375, 0x6C237C}: s 0/1 -> `veh->m_doors[door].Open(0.0f)` (`this` for CDoor::Open 0x6F4790 =
  veh + (door*3 + 0xB7)*8 = veh + 0x5B8 + door*0x18; arg pushed as dword 0), new = 2; s 2 -> new = 3; s 3 ->
  `if (rand() & 7) return false;` (0x6C237C-0x6C2383) else new = 4. Store `m_aDoorsStatus[door] = new` (re-checked
  `0 <= door < 6`, 0x6C2360-0x6C2369), return true.
- Verdict: CONFIRMED (original: NOTPRESENT when `(rand() & 7) == 0`, probability 1/8 on the low bits; gr: when
  `(int)(rand()*7/32767.f) == 0`, i.e. rand() <= 4680).
- Class: keep-original.
Row DamageManager.cpp:203 (original-comment "this original code replaced by SetDoorStatus").
- Original bytes: a single bounds-checked byte store after the switch (0x6C2360-0x6C2369). gr's per-case
  SetDoorStatus writes the same byte. Status values > 4 are stored back unchanged and return true in the original;
  gr's switch has no default and also returns true without a change.
- Verdict: REFUTED.
- Class: harmless.

## 0x6C23C0 CDamageManager::ProgressPanelDamage  (gr_deviations = 2)
Convention: `__thiscall bool (CDamageManager* this, u8 panel [esp+4])`, `ret 4`.
Rows DamageManager.cpp:150 (`GetRandomNumberInRange(0, 2)`) and :155 (`GetRandomNumberInRange(0, 8)`).
- Original bytes: `s = (m_nPanelsStatus >> (panel*4)) & 0xF`. If s == 2: panel == 4 (windscreen) ->
  `if (rand() & 1) return false` (0x6C23E2-0x6C23E9); other panels -> `if (rand() & 7) return false` (0x6C23F2-0x6C23F9).
  If s == 3 -> return false. Otherwise (and after a passing roll) the 4-bit field becomes `(u8)(s + 1)` (0x6C2412-0x6C242C,
  other bits kept), return true.
- Verdict :150: CONFIRMED (original 50 % on rand() bit 0; gr returns 1 only for rand() == 32767, so it almost always
  progresses the windscreen).
- Verdict :155: CONFIRMED (original `rand() & 7`, gr `rand() <= 4680`).
- Class: keep-original (both).

## 0x6C25D0 CDamageManager::FuckCarCompletely  (gr_deviations = 1)
Convention: `__thiscall void (CDamageManager* this, bool bDontDetachWheel [esp+4])`, `ret 4`.
Row DamageManager.cpp:59 (rng-helper, `SetWheelStatus(GetRandomNumberInRange(MAX_CARWHEELS), MISSING)`).
- Original bytes: if the arg byte is 0: `r = _ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * -3.0f
  [0x859024])` (0x6C25DD-0x6C25FB; r in {0, -1, -2}), `m_anWheelsStatus[-r] = 2` (`mov byte [this - r + 5], 2`).
  So only wheels 0, 1, 2 can be removed; wheel 3 never. Then doors 0..5 = 4 (dword 0x04040404 at +9, word at +0xD).
  Then 3 iterations of an inlined ProgressPanelDamage on panel 5 (bits 20-23) then panel 6 (bits 24-27), each:
  s == 2 -> `if ((rand() & 7) == 0) s+1` ; s == 3 -> nothing; else s+1 (0x6C2620-0x6C2685). Finally
  `m_nLightsStatus = 0`, `m_nPanelsStatus = 0`, `m_nEngineStatus = 250` (0x6C2687-0x6C2690). (The bumper progression
  only consumes rand() calls; its result is wiped.)
- Verdict: CONFIRMED (gr picks wheel `(int)(rand()*3/32767.f)`, 0..3 with 3 only at rand() == 32767; original
  `-trunc(rand()*-3/32768)`, 0..2). The rest of gr's body matches (doors, 6 rolls, lights 0, panels 0, engine 250).
- Class: keep-original.

## 0x6C26F0 CDamageManager::GetCarNodeIndexFromDoor  (gr_deviations = 1)
Convention: `__cdecl int32 (int32 door [esp+4])`, plain `ret` (static).
Row DamageManager.cpp:575 (`NOTSA_UNREACHABLE_CASE`, "originally return -1").
- Original bytes: `eax = -1`; `if ((u32)door > 5)` return -1 (0x6C26F4-0x6C26FA); table 0x6C2728: 0 -> 0x10, 1 -> 0x11,
  2 -> 0x0A, 3 -> 0x08, 4 -> 0x0B, 5 -> 0x09.
- Verdict: CONFIRMED (gr aborts / assumes unreachable; original returns -1).
- Class: keep-original (return -1 for any other value, including negatives).
