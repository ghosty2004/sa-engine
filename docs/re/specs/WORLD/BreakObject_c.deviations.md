# BreakObject_c: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x59d570 BreakObject_c::SetGroupData  (gr_deviations = 10)
Rows checked: 10 (CONFIRMED 10, REFUTED 0, UNCLEAR 0).
Rows: gr `source/game_sa/BreakObject_c.cpp` lines 152, 153, 154 (`m_Velocity.c += GetRandomNumberInRange(-r, r)`), 157, 158, 159 (`m_Velocity.c = GetRandomNumberInRange(-r, r)`), 162 (`m_RotationSpeed = GetRandomNumberInRange(3.0f, 6.0f)`), 164, 165, 166 (`m_RotationAxis.c = GetRandomNumberInRange(-1.0f, 1.0f)`), all rng-helper (float overload).
- Original bytes. Every draw is inline: `t = (float)rand() * 3.0518509447574615e-05f [0x858c7c]` (= 1/32767; `rand` 0x821b1e, `fild` of the raw int, NO `& 0xFFFF` mask here). Per group (stride 0x74, loop `i = 0 .. m_NumBreakGroups(+4)-1` ascending): copy 16 dwords of `*matrix` (0x59d599), `CalcGroupCenter(group)` (0x59d190), then:
  - `vecVelocity == null` (0x59d5b7): `lo = -r` (`fchs`, stored f32); for x, y, z in that order: `v.c = (r - lo) * t + lo` (`fld r; fsub lo; fmul t; fadd lo`, 0x59d5e5-0x59d5f3 etc.), stored to +0x40/+0x44/+0x48.
  - `vecVelocity != null`: copy the vector to +0x40; `if (r == 0.0f)` (ordered equal, `test ah,0x44; jnp`, 0x59d677) skip; else for x, y, z: `v.c = ((r - lo) * t + v.c) + lo` (`fmul st1; fadd [v.c]; fadd lo`, 0x59d6b0-0x59d6c0): the old component is added BEFORE `lo`.
  - `m_RotationSpeed (+0x60) = t * 3.0f [0x858b3c] + 3.0f` (0x59d738-0x59d74a).
  - `m_RotationAxis.c (+0x64/+0x68/+0x6c) = (t + t) - 1.0f [0x858624]` (`fadd st0,st0; fsub 1.0`, 0x59d764-0x59d76c), x, y, z order.
  - `RwV3dNormalize(&axis, &axis)` (0x7ed9b0, return value popped), `m_bStoppedMoving (+0x4c) = 0`.
- Verdict (all 10 rows): CONFIRMED. gr's float helper is `lerp = max*t + min*(1-t)` with t = rand()/32767 (cat14 finding 1); the original evaluates `(max-min)*t + min` (and for the `+=` case `(max-min)*t + old + min`), so results differ in rounding. RNG call count and order match.
  - Class: keep-original.
- Found in passing: the axis is normalised with `RwV3dNormalize` (0x7ed9b0), gr calls `CVector::Normalise()`. Whether these round identically was not checked (open question, 0x59d7bd). Class: keep-original (call RwV3dNormalize's port).

## 0x59d7f0 BreakObject_c::SetBreakInfo  (gr_deviations = 1)
Rows checked: 2 (CONFIRMED 1, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/BreakObject_c.cpp:201` (rng-helper): `group.m_FramesToLive = 256 + CGeneral::GetRandomNumberInRange(0, 32);`
  - Original bytes (per group, after allocating m_RenderInfo with `operator new(n*0x48)` at 0x59d889): `m_FramesToLive (+0x70) = 256 - ftol((float)(rand() & 0xFFFF) * (1/32768) [0x858b14] * -32.0 [0x866e64])` (rand 0x821b1e at 0x59d898, `_ftol` 0x821b40 at 0x59d8b6, `mov edx,0x100; sub edx,eax` at 0x59d8bb). Result 256..287.
  - Verdict: CONFIRMED (cat14 finding 1: same range, different rand-to-value mapping). Port: `256 - ftol(r * k * -32.0)`; ftol truncates toward zero, so this equals `256 + trunc(r*k*32)`.
  - Class: keep-original.
- gr `BreakObject_c.cpp:211` (bug-note, deviates=no): "BUG: (?) Compiler shows that invalid access can happen in next line".
  - Original bytes: `renderInfo = group.m_RenderInfo (+0x50) + group.m_NumTriangles (+0x4e, u16) * 0x48` (0x59d91b-0x59d959) with no bounds check, exactly like gr.
  - Verdict: REFUTED (comment only; gr keeps the original indexing).
  - Class: harmless.

## 0x59de40 BreakObject_c::DoCollisionResponse  (gr_deviations = 7)
Rows checked: 7 (CONFIRMED 7, REFUTED 0, UNCLEAR 0).
Notation: `t = (float)rand() * 3.0518509447574615e-05f [0x858c7c]` (1/32767, raw `fild`, no mask), `rand` = 0x821b1e.
- gr `source/game_sa/BreakObject_c.cpp:262/263/264` (rng-helper): `vecRand = { GetRandomNumberInRange(-1.0f, 1.0f) x3 }`.
  - Original bytes: each component `(t + t) - 1.0f [0x858624]` (`fadd st0,st0; fsub`, 0x59dea9-0x59df0c), x then y then z, then `RwV3dNormalize(&r, &r)` (0x7ed9b0).
  - Verdict: CONFIRMED (gr's lerp rounds differently, cat14 finding 1). Class: keep-original.
- gr `BreakObject_c.cpp:285` (rng-helper): `m_FramesToLive = 32 + GetRandomNumberInRange(0, 32)`.
  - Original bytes: `m_FramesToLive (+0x70) = 32 - _ftol((float)(rand() & 0xFFFF) * (1/32768)[0x858b14] * -32.0f [0x866e64])` (0x59dffc-0x59e026). Reached when `speed < 0.05f [0x858c28]` (ordered; 0x59dfe1-0x59dff0; `m_bStoppedMoving (+0x4c) = 1` first) and `m_JustFaces` (byte [this+0]) is set.
  - Verdict: CONFIRMED (integer helper mapping differs). Class: keep-original.
- gr `BreakObject_c.cpp:294/295` (rng-helper): `particlePos.x/y += GetRandomNumberInRange(-0.5f, 0.5f)`.
  - Original bytes: `p.x = (t + p.x) - 0.5f [0x858b8c]` (0x59e07a-0x59e097), then the same for y (0x59e09b-0x59e0b8); p is reloaded from the group position (+0x30) every particle (0x59e064).
  - Verdict: CONFIRMED (order of operations `(t + p) - 0.5` vs gr `p + lerp`). Class: keep-original.
- gr `BreakObject_c.cpp:299` (rng-helper): particle velocity `{ GetRandomNumberInRange(-0.15f, 0.15f), GetRandomNumberInRange(-0.15f, 0.15f), 0 }`.
  - Original bytes: `v.c = t * 0.3f [0x858c24] - 0.15f [0x858fcc]` for x (0x59e0bc-0x59e0db) and y (0x59e0df-0x59e125); 4 particles per call; draw order per particle: pos.x, pos.y, vel.x, vel.y (same as gr). AddParticle (0x4aa440) on `g_fx.m_SmokeII3expand` (0xa9ae24) with (pos, vel, 0, FxPrtMult_c(1,1,1,0.1,0.3,0,0.15), -1.0, 1.2, 0.6, 0).
  - Verdict: CONFIRMED. Class: keep-original.
- Found in passing: the original uses `RwV3dNormalize` (0x7ed9b0, twice: 0x59df10, 0x59df79) and `RwV3dLength` (0x7edac0, 0x59df42) where gr uses `CVector::Normalise` / `Magnitude`. Port must call the RW math ports; rounding equivalence was not checked (open question).
