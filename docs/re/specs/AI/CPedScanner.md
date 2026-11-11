# CPedScanner (AI) - RE-SPEC-FINAL group A

## 0x5FFF20 CPedScanner::GetClosestPedInRange  (catalogue: specified/REA, port level 0)
- `__thiscall CPed* (const CPedScanner* this)`: `return this->m_pClosestEntityInRange` (+0x4C), plain `ret`. Callers: CCheat::MayhemCheat 0x439B20,
  CTaskSimpleSitIdle::ProcessPed 0x637A60, CTaskComplexWander::ScanForBlockedNode 0x671EF0.
- gta-reversed: PedScanner.cpp:11 - matches. Open questions: none.
