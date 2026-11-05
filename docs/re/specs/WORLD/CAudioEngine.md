# CAudioEngine (WORLD) - full specs

## 0x507400 CAudioEngine::StopPoliceScanner  (catalogue: specified/REA, port level 9)
- Convention: `__thiscall void (CAudioEngine* this = ECX (unused), uint8/bool stopSound [esp+4])`, `ret 4`.
- Callers: CGameLogic::RestorePlayerStuffDuringResurrection 0x442060. Callees: `CAEPoliceScannerAudioEntity::StopScanner`
  0x4E6DC0 (static, cdecl(bool)).
- Behaviour: `CAEPoliceScannerAudioEntity::StopScanner(arg)` - the whole argument dword is forwarded; caller cleanup via
  `pop ecx` (ECX is clobbered on return).
- gr: `CAudioEngine::StopPoliceScanner(uint8 a1) { CAEPoliceScannerAudioEntity::StopScanner(a1); }` - matches.
