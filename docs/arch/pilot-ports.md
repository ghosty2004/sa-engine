# CORE pilot ports (ARCH-1)

The first ports on the ADR-001 foundation, in `crates/sa-core`. State `implemented` in
`catalog/state.csv` for 25 of them (the two GetRandomNumberInRange rows are `L1` from MATH-2, whose
implementation the sa-core hooks call; the L1 specs for them are MATH-2's) (Rust code exists and is registered with `#[hook]`; not verified until
the main session accepts L1 and INF-8 runs L2). Installed by sa-hook only with `SA_HOOK_PORTS` set
(ADR-001 §4); they start in original mode unless named there.

Source column: **bytes** = ported from the 1.0 US disassembly (capstone over `re/gta/1.0-US/gta_sa.exe`),
with gta-reversed (`re/gta-reversed` @ 20d8d2a) as a reading aid only. Deviations column: where
gta-reversed differs and the port follows the original. Rows marked *new* are not in
`catalog/gr-deviations.csv` (found while porting; candidates for `gr-deviations-notes.csv`).

## Functions

| address | name | conv | port (sa-core) | source | deviations followed (original kept) | verification so far |
|---|---|---|---|---|---|---|
| 0x407180 | CGeneral::GetRandomNumberInRange_int32 | cdecl | `general::get_random_number_in_range_i32` (arithmetic: sa-math, MATH-2) | bytes | gr-deviations `General.h:119` / `General.cpp:16` (rng-helper, keep-original): original = `min + _ftol((rand() & 0xFFFF) * (1/32768) * (max - min))` under PC=24; gr = `lerp(min, max-1, rand()/32767)` | L1 (agent run) 1,000,512 cases, 0 mismatches; Unicorn = CPU on 200,512 cases |
| 0x41BD90 | CGeneral::GetRandomNumberInRange&lt;float&gt; | cdecl, ST0 | `general::get_random_number_in_range_f32` (returns `X87`, exact ST0; arithmetic: sa-math, MATH-2) | bytes | gr-deviations `General.h:104` (keep-original): original = `(max - min) * (rand() * (1/32767)) + min`, x87 PC=24, extended exponent; gr = `max*t + min*(1-t)` | L1 1,003,364 cases (12,370 SNaN cases decided by the CPU), 0 mismatches, ST0 compared bit-exact (80 bits); Unicorn = CPU on 200,727 non-SNaN cases |
| 0x5E45E0 | CPedPool::New | thiscall | `pool::ped_pool_new` → `pool_new` | bytes | *new*: search starts at `m_nFirstFree + 1` (gr `FindFreeSlot` starts at `m_nFirstFree`); wrap-around restarts at 0 once, then returns null with `m_nFirstFree = 0`; no `DoFill(0xCD)` (gr NOTSA debug fill writes the whole slot); reuse counter `(b+1) & 0x7F` merged by xor as at +0x29 | L1 1,000,148 cases, 0 mismatches |
| 0x6E2A50 | CVehiclePool::New | thiscall | `pool::vehicle_pool_new` | bytes | as CPedPool::New (slot 0xA18 = CHeli) | L1 1,000,148, 0 mismatches |
| 0x5A1C20 | CObjectPool::New | thiscall | `pool::object_pool_new` | bytes | as CPedPool::New (slot 0x19C = CCutsceneObject); the `m_nFirstFree = 0` store is in a protection chunk at 0x405B24 that jumps back to 0x5A1C3D | L1 1,000,148, 0 mismatches |
| 0x404910 | CPedPool::GetAtRef | thiscall, ret 4 | `pool::ped_pool_at_ref` | bytes | *new*: no bounds check on `ref >> 8` (gr checks `IsIndexInBounds`); the map byte is compared whole with `(uint8)ref` | L1 1,000,060, 0 mismatches (incl. negative indices reading before the map) |
| 0x404870 | CObjectPool::GetAt | thiscall, ret 4 | `pool::object_pool_at` | bytes (.HOODLUM body 0x156DE60) | none beyond gr's asserts; valid iff map byte bit 7 clear (`cmp byte,0; jns`) | L1 1,000,030, 0 mismatches |
| 0x5E4720 | CPed::operator new-anon | cdecl | `pool::ped_operator_new` | bytes | `mov ecx, ms_pPedPool; jmp CPedPool::New`: the port calls 0x5E45E0 through `orig!` (its hook decides) | host tests only |
| 0x5E4760 | CPed::operator delete-anon | cdecl | `pool::ped_operator_delete` → `pool_delete` | bytes | gr-deviations `Pool.h:261` (FIX_BUGS null check, keep-original): no null check; *new*: no `DoFill(0xDD)`; `m_nFirstFree = min(idx, m_nFirstFree)` signed | host tests only |
| 0x54FF60 | CPools::GetPedRef | cdecl | `pool::get_ped_ref` → `pool_ref` | bytes | `(idx << 8) + map[idx]`, idx by signed division (magic 0x41EE7CA7, sar 9) | host tests; magic division = `/ 0x7C4` on 9M inputs |
| 0x54FF90 | CPools::GetPed | cdecl | `pool::get_ped` → `pool_at_ref` | bytes | inlined GetAtRef, no bounds check | host tests |
| 0x54FFC0 | CPools::GetVehicleRef | cdecl | `pool::get_vehicle_ref` | bytes | magic 0x06572EC3, sar 6 (= `/ 0xA18`) | host tests |
| 0x54FFF0 | CPools::GetVehicle | cdecl | `pool::get_vehicle` | bytes | no bounds check | host tests |
| 0x550020 | CPools::GetObjectRef | cdecl | `pool::get_object_ref` | bytes | magic 0x13E22CBD, sar 5 (= `/ 0x19C`) | host tests |
| 0x550050 | CPools::GetObject | cdecl | `pool::get_object` | bytes | no bounds check | host tests |
| 0x5617E0 | CTimer::Initialise | cdecl | `timer::initialise` | bytes | *new*: divider = **low dword** of the QPF frequency / 1000 (`mul` by 0x10624DD3, `shr 6`); gr divides the 64-bit value. Entry `xor eax,eax; jmp 0x401220` (chunk stores `m_UserPause`). QPF through the call site 0x56186C | host tests |
| 0x5618C0 | CTimer::Shutdown | cdecl | `timer::shutdown` | bytes | none | host tests |
| 0x5618D0 | CTimer::UpdateVariables | cdecl | `timer::update_variables` → `step` | bytes | *new*: `nonClipped = _ftol((float)nonClipped + delta)` (gr: `nonClipped += (uint32)delta`; differs e.g. at 2^24+1 with delta 0.5); clipped step `_ftol(delta)` clamped with an **unsigned** compare to 300; all ops x87 PC=24 with exact `fild` of the u32 globals (+2^32 when bit 31 set); min/max clamps with the original compare/branch semantics (`test ah,5`/`0x41`) | host tests (determinism-mode step: 40 ms → step 2.0) |
| 0x5619D0 | CTimer::Suspend | cdecl | `timer::suspend` | bytes | `++count <= 1` unsigned | host tests |
| 0x561A00 | CTimer::Resume | cdecl | `timer::resume` | bytes | 64-bit wrapping `now - pause + start` | host tests |
| 0x561A40 | CTimer::GetCyclesPerMillisecond | cdecl | `timer::get_cycles_per_millisecond` | bytes | none | host tests |
| 0x561A50 | CTimer::GetCyclesPerFrame | cdecl | `timer::get_cycles_per_frame` | bytes | `_ftol((float)divider * 20.0f)` (tail jump into `_ftol`; EDX = high dword not reproduced, callers use EAX) | host tests |
| 0x561A80 | CTimer::GetCurrentTimeInCycles | cdecl | `timer::get_current_time_in_cycles` | bytes | low dwords only; QPC through the call site 0x561A88 (virtual in determinism mode) | host tests |
| 0x561AA0 | CTimer::Stop | cdecl | `timer::stop` | bytes | none | host tests |
| 0x561AD0 | CTimer::GetIsSlowMotionActive | cdecl, AL | `timer::get_is_slow_motion_active` | bytes | `scale < 1.0` via `fcomp; test ah,5; jp` (NaN → false) | host tests |
| 0x561AF0 | CTimer::StartUserPause | cdecl | `timer::start_user_pause` | bytes | none | host tests |
| 0x561B00 | CTimer::EndUserPause | cdecl | `timer::end_user_pause` | bytes | none | host tests |

Not ported on purpose: `CTimer::Update` (0x561B10) stays sa-hook's passthrough with the frame boundary
(determinism mode); `rand`/`srand` (0x821B1E/0x821B11, CRT, excluded) are called through `orig!`
(`sa_core::crt`, with an owned MSVC LCG for phases B/C).

## Globals used (all `.bss` in 1.0 US, initial value 0)

`CPools::ms_pPedPool` 0xB74490, `ms_pVehiclePool` 0xB74494, `ms_pObjectPool` 0xB7449C; `CTimer` block
0xB7CB28–0xB7CB89 (names and addresses as in gta-reversed `Timer.h`, confirmed by the disassembly and by
`catalog/rwsets.csv` for 0x5617E0/0x5618D0); `CSpecialFX::bSnapShotActive` 0xC7C714.

## What INF-8 (L2) must check

For each function above, in-process original-vs-Rust with `SA_HOOK_PORTS=orig` and per-function
toggles: return value (EAX, or AL for 0x561AD0, ST0 80-bit for 0x41BD90) and the read/write sets below.

- Pools (0x5E45E0, 0x6E2A50, 0x5A1C20, 0x404910, 0x404870, 0x5E4720, 0x5E4760, 0x54FF60–0x550050):
  pool header (`+0x0C m_nFirstFree`) and the byte map; nothing else may be written (gr's NOTSA fills
  must not appear).
- `GetRandomNumberInRange` int/float: return value and the main thread's `_holdrand` (one `rand()` per
  call), and in determinism mode the `randcalls` hash with the hook on Rust vs. original (caller
  attribution 0x407187 / 0x41BD96).
- CTimer: the 0xB7CB28–0xB7CB89 block. `UpdateVariables` and `Initialise` write several fields per call;
  `GetCurrentTimeInCycles`/`Suspend`/`Resume` read the clock, so compare them in determinism mode where
  the clock is virtual. `Initialise` runs once at start-up: needs a record from process start with
  `SA_HOOK_PORTS=CTimer::Initialise`.
- Hook-off check (L3) for each, once L3 exists.

## Known limits of the evidence so far

- L1 for the pool functions covers sizes ≤ 1,260 slots (scratch is 0x500 bytes since MATH-2) and indices whose map bytes
  lie in the compared scratch bytes (including reads before the map); larger pools exercise the same instructions.
- The extra code ranges allowed for 0x407180 (0x40810B chunk), 0x5A1C20 (0x405B24 chunk, 0x5A1C3D rest)
  and 0x404870 (.HOODLUM body) are not covered by the oracle's SSE scan; their disassembly has no SSE
  instruction (integer moves/jumps only).
- `GetRandomNumberInRange` L1 treats `rand()` as an input (the image's `rand` is replaced by a stub that
  returns a scratch value); `rand` itself is the CRT's and is not ported.
