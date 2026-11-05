# CTimer helpers catalogued under PHYS, full specs

## 0x541f40 CTimer::UpdateTimeStep (proposed; catalogue: unknown/REA, port level 0)
Name evidence: body is exactly gta-reversed's inline `CTimer::UpdateTimeStep(float ts) { ms_fTimeStep =
std::max(ts, 0.00001f); }` (Timer.h:85; no address in gr); only caller is `CPhysical::ApplySpeed` 0x547b80
(6 sites), which temporarily rescales the time step.
- Convention / signature: `__cdecl void (float ts [esp+4])`, plain `ret`. Clobbers EAX (fnstsw).
- Callers: 1 function (0x547b80, 6 sites). Callees: none.
- Globals: `CTimer::ms_fTimeStep` 0xb7cb5c (written); constant 1e-5f at 0x858c14 (bits 0x3727c5ac).
- Behaviour: `fld ts; fcomp [0x858c14]; test ah,0x41; jp`: if `ts > 1e-5f` OR `ts` is NaN, store `ts` (raw
  dword copy); otherwise (`ts <= 1e-5f`) store the dword 0x3727c5ac (1e-5f).
- Edge cases: NaN is stored as is (gr's `std::max(ts, 1e-5f)` = `ts < 1e-5f ? 1e-5f : ts` also keeps NaN).
  `ts == 1e-5f`, 0.0 and -0.0 store 1e-5f.
- gta-reversed: inline helper, same result. Open questions: none.
