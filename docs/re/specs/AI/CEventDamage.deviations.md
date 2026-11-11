# CEventDamage: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 helper A (groups J1/J6).

## 0x4ADC10 CEventDamage::ComputeBodyPartToRemove  (gr_deviations = 1)
Original bytes (`__thiscall void (this, int32* boneFrameId [esp+4])`, `ret 4`): `*out = 0` (0x4ADC18); `w = this+0x1C` (m_weaponType); if w > 0x33
(unsigned, 0x4ADC24) return. Byte index table 0x4ADD0C (52 entries) into dword table 0x4ADCFC:
- w in {0..15, 46} -> 0x4ADC38: if m_damageResponse.m_bForceDeath (this+0x41) `*out = 2` (HEAD).
- w in {24..27, 30, 31, 33, 34, 38} -> 0x4ADC50: if this+0x41: `p = this+0x20` (m_pedPieceType); if p-5 > 4 (unsigned) return; table 0x4ADD40:
  p=5 -> 3, p=6 -> 4, p=7 -> 7, p=8 -> 8, p=9 -> 2 (0x4ADC71-0x4ADCA5).
- w in {16, 35, 36, 51} -> 0x4ADCB2: if this+0x41: local array {3, 8, 2, 4, 7} (0x4ADCBD-0x4ADCDD); `i = CGeneral::GetRandomNumberInRange(0, 5)`
  0x407180 (cdecl int version, pushes 5 then 0, 0x4ADCB9); `*out = arr[i]` (0x4ADCEA).
- all other w <= 0x33 -> return with *out = 0.
Row EventDamage.cpp:350 (`rng-helper`, `boneArray[CGeneral::GetRandomNumberInRange(0, 5)]`):
- Original bytes: 0x4ADCE5 calls the original integer helper 0x407180, `min + ftol((rand() & 0xFFFF) * (1/32768) * (max-min))`
  (docs/re/cat14-gr-deviations.md finding 1); gr's template helper computes `lerp(min, max-1, rand()/32767.0f)`, a different index for the
  same rand() value. Case sets and the table contents above match gr cpp:314-380.
- Verdict: CONFIRMED. Class: keep-original (port must call the original helper 0x407180 semantics).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B3FC0 CEventDamage::ComputeDamageAnim  (gr_deviations = 1)
Original bytes: `__thiscall void (this, CPed* ped [esp+4], bool)`; only the row's site is re-checked here.
Row EventDamage.cpp:764 (`playerData->m_nHitAnimDelayTimer = CTimer::GetTimeInMS() + 2500; // originally 2500.0f`):
- Original bytes (0x4B447C-0x4B44AC): `now = CTimer::m_snTimeInMilliseconds [0xB7CB84]`; if `now <= pd->m_nHitAnimDelayTimer (+0x28)` (unsigned `jbe`) or ped state (+0x530) == 0x32
  -> m_nAnimID (+0x2C) = 0xBF (0x4B4455); else `pd+0x28 = _ftol((float)now + 2500.0f [0x85AB04])` where the u32 -> x87 conversion adds 4294967296.0f [0x858C54] when the
  sign bit is set (`fild` + fix-up), and the sum is rounded by the in-game 24-bit x87 precision control before `_ftol` 0x821B40 truncation.
  The two sibling sites (0x4B440B-0x4B4428 for weapon type 0x1F: + 2500.0f [0x85AB04]; 0x4B4430-0x4B444D otherwise: + 1500.0f [0x85AB00]) use the same float path.
- Verdict: CONFIRMED - gr's integer `now + 2500` differs from the original once `now` exceeds 2^24 ms (about 4.66 hours of game time), where the float sum is rounded to a
  multiple of 2 (and coarser later). Class: keep-original (port must emulate the f32/24-bit rounding).
