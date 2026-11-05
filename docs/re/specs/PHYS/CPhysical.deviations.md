# CPhysical: gta-reversed deviation notes (PHYS)

Rows from `catalog/gr-deviations.csv`, checked against the original bytes (RE-SPEC-1, writer B).
RNG background (docs/re/cat14-gr-deviations.md finding 1): gr's float `GetRandomNumberInRange(min, max)` is
`max*t + min*(1-t)` with `t = rand() * (1/32767)`; its int version is `(int)lerp(min, max-1, t)`. The original
float helper 0x41bd90 computes `(max-min)*t + min`; several call sites below inline `rand()` instead.

## 0x542260 CPhysical::Constructor  (gr_deviations = 1)
- Physical.cpp:128 `m_nContactSurface = SURFACE_DEFAULT; // NOTSA; If not initialize, it will go out of bounds
  in the SurfaceInfos_c`
  - Original bytes: the constructor writes +0xb8 (`m_nFakePhysics`) and +0xb9 (`m_nNumEntitiesCollided`) at
    0x542373/0x542379 but never writes +0xba (`m_nContactSurface`); no instruction in the constructor body touches
    `[esi+0xba]`.
  - Verdict: CONFIRMED (gr adds an initialisation). The port keeps the original: the byte keeps whatever the
    pool slot held (callers set it on the first collision).
  - Class: keep-original.

## 0x542560 CPhysical::RemoveAndAdd  (gr_deviations = 0; 1 row)
- Physical.cpp:621 "identical, so reinterpret the node pointers (the original code did the same)."
  - Original bytes: the node-reuse path 0x54268d..0x5426d8 moves the existing `CPtrNodeDoubleLink` between
    sector lists without any type distinction (lists are plain head pointers).
  - Verdict: REFUTED (comment describes the original correctly; full spec in CPhysical.md).
  - Class: harmless.

## 0x5454c0 CPhysical::ApplyFriction-self  (gr_deviations = 1)
- Physical.cpp:1755 `float fRandom = CGeneral::GetRandomNumberInRange(-0.2f, 0.2f);` (8 spark iterations)
  - Original bytes: loop 0x5458c0..0x545959 (EDI = 8 down to 1): `r = (float)rand()` (0x821b1e, `fild`),
    `r *= 3.0518509e-5f` (0x858c7c), `r *= 0.4f` (0x858ee8), `r -= 0.2f` (0x858cc4), then the spark origin
    `cp.point + cross * r` and `Fx_c::AddSparks` 0x49f040 (g_fx 0xa9ae00).
  - Verdict: CONFIRMED. Original value is `(t*0.4f) - 0.2f`; gr's `0.2f*t + (-0.2f)*(1-t)` rounds differently.
    rand() call count and order are the same (one per iteration).
  - Class: keep-original.

## 0x546670 CPhysical::ProcessShiftSectorList  (gr_deviations = 1)
- Physical.cpp:2127 `if (!GetIsTypeVehicle() || 1.5f <= 0.0f) { // R* nice check`
  - Original bytes: 0x546c19 type test (`(byte +0x36 & 7) == 2`), then 0x546c28 `fld [0x8cd7d0]` (a `.data`
    float, initial 1.5f) `fcomp 0.0f; test ah,0x41; jne -> return` (i.e. skip when the global `<= 0` or NaN).
    The shift multipliers above also read a `.data` float 0x8cd7d4 (1.5f) at 0x546b12/0x546b8e/0x546b9a/0x546ba4.
    `catalog/rwsets.csv` lists no writer of either address.
  - Verdict: REFUTED (gr folds two never-written globals into the literal 1.5f; same behaviour). A port should
    still read 0x8cd7d0/0x8cd7d4 through `globals!` to stay exact if anything patches them.
  - Class: harmless.

## 0x546ff0 CPhysical::PositionAttachedEntity  (gr_deviations = 1)
- Physical.cpp:2272 `float randomNumber = CGeneral::GetRandomNumberInRange(-1.0f, 1.0f);`
  - Original bytes: 0x5475f7..0x547606 `push 1.0f; push -1.0f; call 0x41bd90; fstp` (after
    `DettachAutoAttachedEntity` 0x5446a0 and the `bDisableCollisionForce` (+0x40 & 4) test at 0x5475ed).
  - Verdict: CONFIRMED (finding 1: 0x41bd90 returns `(max-min)*t + min = 2t - 1`, gr returns
    `1*t + (-1)*(1-t)`, different rounding). Port calls 0x41bd90 via `orig!`/its port.
  - Class: keep-original.

## 0x548680 CPhysical::ApplyCollision-2  (gr_deviations = 2)
- Physical.cpp:2708 and Physical.cpp:2734 `CStats::IncrementStat(STAT_COST_OF_PROPERTY_DAMAGED,
  static_cast<float>(CGeneral::GetRandomNumberInRange(30, 60)));`
  - Original bytes: 0x548a50 (first site) and 0x548c7f (second site, jumps to 0x548a67): `rand()` 0x821b1e;
    `cdq; idiv 30; edx += 30` -> `rand() % 30 + 30` (30..59); `fild`; pushed as float with stat id 0x7d (125).
  - Verdict: CONFIRMED for both rows. The original uses `rand() % 30 + 30`, not a range helper; gr's
    `(int)lerp(30, 59, rand()/32767)` maps the same rand() value to a different number.
  - Class: keep-original.
- Note (not a row): a third inline `rand() % 100` at 0x548fb3 exists in the same function; check gr's
  counterpart when porting.

## 0x54a2c0 CPhysical::ApplySoftCollision-2  (gr_deviations = 1; 2 rows)
- Physical.cpp:3339 "BUG: Game should be checking if entity is ped or not." (`entityPed->m_standingOnEntity ==
  this`)
  - Original bytes: 0x54a2cf..0x54a301: if `entity.flags(+0x40) & 0x10` and `!(this.flags & 0x60)`: mass factor
    10.0f and `cmp [ebp+0x568], edi` (entity's `CPed::m_standingOnEntity` slot compared with `this`) with no type
    check on `entity`.
  - Verdict: REFUTED (gr keeps the original unchecked read; the comment only documents the original bug).
  - Class: keep-original (keep the unchecked read of entity+0x568).
- Physical.cpp:3418 `CStats::IncrementStat(STAT_COST_OF_PROPERTY_DAMAGED, ... GetRandomNumberInRange(30, 60))`
  - Original bytes: 0x54a5c2 `rand()`, `cdq; idiv 30; edx += 30`, `fild`, stat 0x7d.
  - Verdict: CONFIRMED (same as ApplyCollision). Class: keep-original.
- Note (not a row): inline `rand() % 100` at 0x54a959.
