# CCarPathLink (POP) - RE-SPEC-FINAL group A

## 0x44DB00 CCarPathLink::OneWayLaneOffset  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall float (const CCarPathLink* this)`, result in ST0, plain `ret` (`push ecx` used as a 4-byte int scratch for `fild`).
- Fields: +0xA m_nPathNodeWidth (byte, read with `movzx` = UNSIGNED), +0xB bits 0-2 m_numOppositeDirLanes, bits 3-5 m_numSameDirLanes.
- Behaviour (0x44DB01-0x44DB5F):
  - if `opposite == 0`: `n = same`; return `0.5f - (float)n * 0.5f` (`fild n; fmul [0x858B8C]=0.5f; fsubr [0x858B8C]=0.5f`).
  - else if `same == 0` (`test al,0x38`): return `0.5f - (float)opposite * 0.5f` (same instruction pattern).
  - else: return `(float)width_u8 * [0x858EEC]=0.011574074f + [0x858B8C]=0.5f` (`fild; fmul; fadd`) - a PLUS, on the raw byte.
- gta-reversed: PathFind.h:102 (inline, not hooked) DIFFERS: gr returns `0.5 - same/2` when `opposite != 0`, `0.5 - width/5.4/2` (width = raw
  int8/16) when only `same != 0`, else `0.5 - opposite/2`. The bytes branch on `opposite == 0` first, use the opposite-lane count in the
  second case, and return `0.5 + raw*0.011574074` (raw unsigned, no /16, added not subtracted) when both counts are non-zero. Port the bytes.
- Open questions: none.
