# CTaskComplexKillCriminal: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x68BE70 CTaskComplexKillCriminal::CTaskComplexKillCriminal  (catalogue name: Constructor; gr_deviations = 2)
Original bytes (`__thiscall this (CPed* criminal [esp+4], bool randomize [esp+8])`, `ret 8`, SEH): CTaskComplex ctor 0x61A3B0;
`vtbl = 0x870A00`; `m_Criminal (+0xC) = criminal`; `m_Cop (+0x10) = 0`; `m_TimeToGetOutOfCar (+0x14) = 3.0f` (imm 0x40400000);
`m_Randomize (+0x18) = randomize`; bytes +0x19, +0x1A, +0x1E = 0 (+0x1B..+0x1D are NOT written) (0x68BE8F-0x68BEC2). If criminal != 0
(0x68BEC5): if `criminal->IsPlayer()` 0x5DF8F0, or `m_nPedType (+0x598)` == 6 (COP), == 0x12 (MEDIC), == 0x13 (FIREMAN), or `>= 0x18`
(signed: MISSION1..MISSION8 and above), or `m_nCreatedBy (byte +0x484) == 2` (mission ped) -> `m_Criminal = 0` (no reference was
registered); else `criminal->RegisterReference(&m_Criminal)` 0x571B70 (0x68BEC7-0x68BEF6). Returns this.
Row TaskComplexKillCriminal.cpp:46 (`notsa::contains({ PED_TYPE_COP, PED_TYPE_MEDIC, PED_TYPE_FIREMAN, PED_TYPE_MISSION1 }, ...)`):
- Original: `cmp eax,0x18; jge` at 0x68BEE7 rejects every ped type >= 24 (MISSION1..MISSION8), gr only MISSION1 (24). Also the original
  tests the ped type before IsCreatedByMission (order is harmless, no side effects).
- Verdict: CONFIRMED (gr accepts MISSION2..MISSION8 criminals; the original rejects them). Class: keep-original.
Row TaskComplexKillCriminal.cpp:52 (NOTSA copy ctor used by Clone 0x68CE50):
- Original Clone 0x68CE50: `operator new(0x20)`; if non-null `CTaskComplexKillCriminal(p, this->m_Criminal, false)` 0x68BE70
  (0x68CE83-0x68CE8B). gr's copy ctor delegates with `(o.m_Criminal, false)`: identical.
- Verdict: REFUTED. Class: harmless.
