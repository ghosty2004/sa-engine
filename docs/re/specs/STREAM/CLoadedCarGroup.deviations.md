# CLoadedCarGroup: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/LoadedCarGroup.cpp` (pin 20d8d2a). Layout: `int16 m_models[23]`
(0x2E bytes), unused entries = -1. Vehicle model info: +8 refCount i16, +0x4D vehicleClass i8, +0x52 frq i16.

## 0x611BD0 CLoadedCarGroup::RemoveMember  (gr_deviations = 1)
Row LoadedCarGroup.cpp:38 (`notsa::remove_first`).
- Original bytes: search all 23 entries (not stopping at -1) for `(int)m_models[i] == modelIndex`
  (0x611BE0-0x611BEC); not found -> return. Found at i: if `i < 22` shift words i+1..22 down by one
  (`rep movsw`, count 22 - i); then `m_models[22] = -1` (0x611C09). `ret 4`.
- Verdict: REFUTED (same effect as remove_first + back = -1).
- Class: harmless.

## 0x611C50 CLoadedCarGroup::PickRandomCar  (gr_deviations = 5)
Rows :54 (notsa::accumulate), :61 (rng-helper), :70 / :71 ("originally the last value was used" +
NOTSA_UNREACHABLE), :89 ("originally an array was created"), :104 (`#ifdef FIX_BUGS` GetAllModels vs
m_models).
- Original bytes:
  1. `count` = number of leading entries with `m_models[i] >= 0` (stops at the first negative,
     0x611C60-0x611C6C). count == 0 -> return -1.
  2. bOnlyPickNormalCars (arg 2): fill a local int16[23] with -1, append every one of the first `count`
     models whose class is 0, 1, 2 or 8 (signed byte, negative classes excluded, 0x611CCE-0x611CDE) and
     add its frq to `sum`. Otherwise `sum` = frq sum of the first `count` models (0x611D00-0x611D14).
  3. `sum == 0` -> return -1 immediately, no rand() call (0x611D16-0x611D1C).
  4. Up to 10 tries: `w = _ftol((rand() & 0xFFFF) * (1/32768.f) * (float)sum)` (sum converted once with
     fild/fstp f32, 0x611D22-0x611D4C). Walk from element 0 of the chosen list (local array or
     m_models): `while (w > frq(model)) { w -= frq; next }` (0x611D80-0x611D9F), no end bound.
     Accept when not suppressed (0x46A810), not blocked by script (0x46A890), not being phased out
     (0x407F80), and (`!bNotTooManyInTheWorld` or `refCount < 3`). Return the model; after 10 rejections
     return -1.
- :54 Verdict REFUTED (same int sum). Class harmless.
- :61 Verdict CONFIRMED (helper formula; gr picks w with lerp(0, sum-1, rand/32767)). Class keep-original.
- :70/:71 Verdict REFUTED: with `w <= sum - 1` and non-negative frq the walk always stops inside the list
  in both versions. The gr comment is inaccurate: the original does not stop at the last entry, it keeps
  walking past it (reading -1 entries) if the weights were inconsistent. Class harmless.
- :89 Verdict REFUTED (the gr filter accepts the same four classes 0, 1, 2, 8). Class harmless.
- :104 Verdict REFUTED: the original sums and walks only the first `count` models, which is gr's FIX_BUGS
  branch; the `#else` branch is not the original. Class harmless.
- Found in passing (CONFIRMED, keep-original): step 3. gr does not test `sum == 0`; with a non-empty list
  whose frequencies are all 0 it consumes rand() up to 10 times and may return a model, the original
  returns -1 without calling rand().
