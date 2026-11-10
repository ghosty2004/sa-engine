# CPedGroupMembership (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x5F6950 CPedGroupMembership::GetObjectForPedToHold  (gr_deviations = 1)
- PedGroupMembership.cpp:270 `RandomChoiceFromList({MI_GANG_SMOKE, MODEL_INVALID, MI_GANG_DRINK}) /* Each has 1/3 chance (Originally 33/33/34) */`:
  Original (`__thiscall`/no-arg use, `this` unused, plain `ret`, returns EAX): `r = _ftol(((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) * 100.0f [0x858628])`
  (0..99, truncation, 0x5F6951-0x5F696F); `if (r < 33) return (u16)MI_GANG_SMOKE [0x8CD72C]` (zero-extended `movzx`, 0x5F6974-0x5F6979);
  `if (r < 66) return (u16)MI_GANG_DRINK [0x8CD728]` (0x5F6982-0x5F6987); else return -1 (0x5F6990). Signed compares.
- Verdict: CONFIRMED (gr: different RNG formula, equal thirds, and a different bucket order SMOKE / INVALID / DRINK; original SMOKE 33%, DRINK 33%,
  none 34%). Note also the zero-extension: an unset MI (0xFFFF) returns 65535, not -1. Class: keep-original.
