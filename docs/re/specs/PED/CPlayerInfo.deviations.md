# CPlayerInfo: gta-reversed deviation notes (PED)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x5717F0 CPlayerInfo::SetPlayerSkin  (gr_deviations = 1)
Row PlayerInfo.cpp:120 (NOTSA: "They used `strcpy`, we use `_s` for safety").
- Original bytes (`__thiscall void(const char* name [esp+4])`, `ret 4`): `strncpy(m_szSkinName (+0x164), name, 32)` (CRT `_strncpy`
  0x821F40, 0x5717F8-0x571802) - **strncpy, not strcpy**: copies at most 32 bytes, zero-pads short names, leaves the buffer
  unterminated for names of >= 32 chars. Then inlined LoadPlayerSkin: if `m_pSkinTexture (+0x184)`: `RwTextureDestroy` (0x7F3820),
  `m_pSkinTexture = 0`; `m_pSkinTexture = CPlayerSkin::GetSkinTexture(m_szSkinName)` (0x6FFA10) (0x571807-0x571831).
- Verdict: CONFIRMED - gr `strcpy_s` aborts/invokes the invalid-parameter handler for names of >= 32 chars and does not zero-pad;
  the original truncates (unterminated). gr's comment about `strcpy` is itself wrong (catalogue evidence: callee is `_strncpy`).
- Class: keep-original (strncpy 32 semantics, including the missing terminator; GetSkinTexture then reads past the field into
  m_pSkinTexture... only for >= 32-char names).
