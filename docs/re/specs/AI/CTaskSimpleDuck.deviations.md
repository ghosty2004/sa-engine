# CTaskSimpleDuck: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 groups J2-J4 (helper B).

## 0x6923F0 CTaskSimpleDuck::ControlDuckMove  (gr_deviations = 1)
Original bytes (`__thiscall void (this, CVector2D moveDir [esp+4..+0xB])`): `+0x1B = 1` (0x6923F8) then returns early if the raw dword `+0x1C`
(m_MoveCmd.x) equals 0x3F800000 or 0xBF800000 (integer compares, 0x6923F3 / 0x692407); then the y step limit `k = ms_fTimeStep * 0.07 [0x858CA8]`
(0x69240F-0x692415), `d = moveDir.y - m_MoveCmd.y`; `d <= k` (`fcom; test ah,0x41; jne`), else-branch `-k` test (`test ah,5; jp`), and the
`|moveDir.y| ... 0.1 [0x858B1C]` test at 0x692457-0x692468.
Row TaskSimpleDuck.cpp:170 (`std::abs(m_MoveCmd.x) == 1.f`, "Originally checked if either -1 or 1"):
- Original bytes: two exact bit-pattern compares of +0x1C with +1.0f / -1.0f (0x6923F3, 0x692402-0x692407).
- Verdict: REFUTED - `fabs(x) == 1.0f` is true exactly for the bit patterns 0x3F800000 / 0xBF800000 (NaN and other values compare false in both).
  The port may keep either form; keep the integer compares to stay byte-exact. Class: harmless.
