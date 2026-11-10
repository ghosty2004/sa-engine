# CTaskComplexFollowPointRoute: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x669090 CTaskComplexFollowPointRoute::CreateTask  (gr_deviations = 1)
Original bytes (`__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH): EDI = 0 at entry (0x6690AC);
compare chain on taskType (0x6690AE-0x6691DC): 900 (0x384), 200 (0xC8), 203, 704 (0x2C0), 903 (0x387), 926 (0x39E) are cases;
1302 (0x516, TASK_FINISHED) -> `xor edi,edi` -> 0 (0x6691E2); every other value jumps to 0x669322 and returns EDI = 0; allocation
failure -> 0x66931E (EAX = 0).
Row TaskComplexFollowPointRoute.cpp:164 (`NOTSA_UNREACHABLE();`):
- Original: unknown task types return nullptr (`jne 0x669322` at 0x6690D6 / 0x6691DC).
- Verdict: CONFIRMED (gr aborts). Class: keep-original.
