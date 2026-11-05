# CAnimBlendHierarchy: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x4CF270 CAnimBlendHierarchy::CAnimBlendHierarchy()  (gr_deviations = 1)
Row AnimBlendHierarchy.cpp:36 (notsa: `m_hashKey = 0`).
- Original bytes (0x4CF270-0x4CF28E, returns this): +0x4 m_pSequences = 0; +0x8 m_nSeqCount (w) = 0; +0xA m_bIsCompressed = 0;
  +0xB m_bKeepCompressed = 0; +0xC m_nAnimBlockId = -1 (d); +0x10 m_fTotalTime = 0; +0x14 m_Link = 0.
  +0x0 m_hashKey is NOT written.
- Verdict: CONFIRMED (gr zeroes m_hashKey).
- Class: harmless (the only static instances, `ms_aAnimations` 0xB4EA40, live in zeroed .bss; the hash is always set by
  SetName before use).

## 0x4CF800 CAnimBlendHierarchy::MoveMemory  (gr_deviations = 1)
Row AnimBlendHierarchy.cpp:173 (`NOTSA_UNREACHABLE("Unused Function")`).
- Reachability: no static callers in the catalogue (dead in practice), but the original has a real body.
- Original bytes (`__thiscall bool()`, return AL; +0x4 m_pSequences (CAnimBlendSequence[], stride 0xC), +0x8 m_nSeqCount i16):
  1. `seqs = m_pSequences`; null -> return false (0x4CF804-0x4CF811).
  2. If `seqs[0].flags (byte +0x4) & 8` (UsingExternalMemory: one block shared by all sequences, 0x4CF812-0x4CF81D):
     `old = seqs[0].m_Frames (+0x8)`; null -> return false; `new = CMemoryMgr::MoveMemory(old)` (0x72F500, cdecl - in this build it
     just returns its argument); if `new == old` return false; else for i in 0..m_nSeqCount-1 (signed compare, count re-read
     each iteration): `seqs[i].m_Frames += (new - old)`; return true (0x4CF81F-0x4CF861).
  3. Else: `moved = false`; for i in 0..m_nSeqCount-1: `if (CAnimBlendSequence::MoveMemory(&seqs[i]) /*0x4D1150*/) moved = true`;
     return moved (0x4CF862-0x4CF892).
- Verdict: CONFIRMED (gr has no implementation).
- Class: keep-original (port the body; observable result in this build: path 2 always returns false).
