# CPedGroupDefaultTaskAllocators (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x5F7200 CPedGroupDefaultTaskAllocators::Get  (gr_deviations = 1)
- Tasks/Allocators/PedGroup/PedGroupDefaultTaskAllocators.h:33 `default: NOTSA_UNREACHABLE();`:
  Original: `__cdecl` (type at [esp+4]; uses ECX as scratch, so callers must not rely on ECX - "spoils ecx" per gr), plain `ret`.
  `eax = 0`; `if ((uint32)type > 5) return 0` (`cmp ecx,5; ja 0x5F7235`, 0x5F7204-0x5F7209); else `jmp [0x5F7238 + 4*type]`, table
  {0x5F7212, 0x5F7218, 0x5F721E, 0x5F7224, 0x5F7230, 0x5F722A}: 0 -> &ms_FollowAnyMeansAllocator 0xC09908, 1 -> &ms_FollowLimitedAllocator 0xC0990C,
  2 -> &ms_StandStillAllocator 0xC09910, 3 -> &ms_ChatAllocator 0xC09914, 4 (SIT_IN_LEADER_CAR) -> 0xC0991C, 5 (RANDOM) -> 0xC09918.
- Verdict: CONFIRMED (out-of-range type returns nullptr in the original; gr aborts). Class: keep-original. The case mapping itself matches gr.
