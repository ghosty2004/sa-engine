# CVehicleAnimGroup: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x6E3B00 CVehicleAnimGroup::GetGroup  (gr_deviations = 1)
Row VehicleAnimGroupData.cpp:134 (`NOTSA_UNREACHABLE(); // Shouldn't ever get to this point`).
- Original bytes (`__thiscall AssocGroupId(AnimationId animId [esp+4])`, `ret 4`, return EAX sign-extended byte):
  1. `idx = animId - 351`; if `(uint32)idx > 39` skip to 3 with `bit = animId` unchanged (0x6E3B04-0x6E3B0F).
  2. Byte table 0x6E3C58 -> jump table 0x6E3C0C writes `bit` into the argument slot: idx 0-3 -> 0, 4-5 -> 1, 6-7 -> 2, 8-9 -> 3,
     10-11 -> 4, 12 -> 3, 13-15 -> 5, 16-17 -> 6, 18-19 -> 7, 20-21 -> 8, 22-23 -> 9, 24-25 -> 10, **26 (animId 377
     ANIM_ID_UNKNOWN_26) -> case 0x12 = jump straight to 3 with `bit = 377`**, 27-28 -> 11, 29-30 -> 12, 31-32 -> 13,
     33-34 -> 14, 35 -> 15, 36-37 -> 16, 38-39 -> 17.
  3. `mask = _ftol2(pow(2.0 [f64 0x86F990], (double)(int32)bit))` (fild, CRT pow 0x822130, truncation to int64, low dword used)
     (0x6E3BE4-0x6E3BF3). For bit 0..31 this is `1 << bit`; for bit 32..62 the low dword is 0; for bit >= 63 the int64
     conversion gives the indefinite value 0x8000000000000000, low dword 0; for negative bit the result is 0.
  4. `return (m_animFlags (+0x4) & mask) ? (int8)m_ucSecondGroup (+0x1) : (int8)m_ucFirstGroup (+0x0)` (0x6E3BF8-0x6E3C09).
- Verdict: CONFIRMED - gr's switch has the same case -> flag mapping, but for any other animId (ANIM_ID_UNKNOWN_26 = 377 and every
  id outside 351..390) it is unreachable/UB; the original returns m_ucFirstGroup when the mask is 0 (ids >= 32 incl. 377), and
  for animId 0..31 tests flag bit `animId`.
- Class: keep-original.

## 0x6E3D10 CVehicleAnimGroup::ComputeAnimDoorOffsets  (gr_deviations = 1)
Row VehicleAnimGroupData.cpp:193 (`default: NOTSA_UNREACHABLE()` in the doorId -> animId lambda).
- Original bytes (`__thiscall CVector*(CVector* ret [esp+4], eVehAnimDoorOffset door [esp+8])`, `ret 8`, returns ret in EAX):
  1. `o = this + 0x40 + door*12` (no range check). If `(o.x*o.x + o.y*o.y) + o.z*o.z == 0.0f` (exact; NaN -> not equal) compute,
     else go to 4 (0x6E3D10-0x6E3D47).
  2. animId by door (jump table 0x6E3E08): 0 -> 359, 1 -> 361, 2 -> 363, 3 -> 373, 4 -> 375, 5 -> 378, 6 -> 379;
     **door > 6 (unsigned) -> animId = door** (0x6E3D8B reads the door argument).
  3. `g = this->GetGroup(animId)` (0x6E3B00); `h = GetAnimAssociation(g, animId)->m_BlendHier`; `seq = h->m_pSequences`;
     `UncompressAnimation(h)`; if `(int16)seq->m_FramesNum > 0`: `o = last frame's Trans` (frame stride 0x20 if flags bit1 else 0x14;
     +0x14..+0x1C copied as 3 dwords) (0x6E3D8F-0x6E3DE7).
  4. `*ret = o` (3 dwords) (0x6E3DEB-0x6E3DFE).
- Verdict: CONFIRMED (out-of-range door ids are UB in gr; the original uses the door value as the anim id).
- Class: keep-original.
