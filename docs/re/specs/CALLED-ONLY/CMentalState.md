# CMentalState (CALLED-ONLY) — RE-SPEC-REST

Layout (gr `MentalHealth.h`, size 0x14): +0x0 u8 m_AngerAtPlayer, +0x1 u8 m_LastAngerAtPlayer, +0x4 CTaskTimer m_AngerTimer
(+0x4 start ms, +0x8 interval, +0xC started u8, +0xD stopped u8), +0x10 u8 m_pedHealth, +0x11 u8 m_oldPedHealth,
+0x12 u8 m_vehicleHealth, +0x13 u8 m_oldVehicleHealth. Embedded in CPedIntelligence at +0x174 (gr PedIntelligence.h:57).

## 0x421050 CMentalState::IncrementAnger  (catalogue: specified/REA, port level 0)
- NAME CORRECTION: ECX is a **CPedIntelligence*** (fields at +0x174..+0x181 = m_mentalState), i.e. this is
  `CPedIntelligence::IncrementAngerAtPlayer(uint8 anger)` (gr PedIntelligence.cpp:979 has that name and C++ for 0x421050; gr's
  `CMentalState::IncrementAnger` wrapper, MentalHealth.cpp:5, passes a CMentalState* to the same address = wrong `this`, a gr error).
- `__thiscall`, u8/int anger at [esp+4] (only DL = low byte used), `ret 4`. Stub 0x421050 jumps to the .HOODLUM body 0x156FC20.
  Callers: CCarCtrl::SlowCarDownForPedsSectorList 0x425440, CEventHandler::ComputeVehicleCollisionResponse 0x4BD6A0,
  ComputePedCollisionWithPedResponse 0x4BDB80, CCollisionEventScanner::ScanForCollisionEvents 0x604500.
- Behaviour (inlined CTaskTimer::IsOutOfTime on the anger timer):
  1. If `started (+0x180) == 0` → return (0x156FC28).
  2. If `stopped (+0x181) != 0`: `start (+0x178) = CTimer::m_snTimeInMilliseconds [0xB7CB84]` — read through a protection idiom at
     0x156FC34 (`stc; sbb` of [0x15605C0] and [0x741FD8] gives 0xB7CB84) — and `stopped = 0`.
  3. `now = [0xB7CB84]`; if `now >= start + interval (+0x17C)` (unsigned, `cmp; sbb al,al; inc al`, 0x156FC65): `start = now;
     interval = 3000 (0xBB8); started = 1; anger (+0x174) += (u8)arg` (wraps mod 256). Else nothing.
- gta-reversed: gr's IncrementAngerAtPlayer matches the bytes (IsOutOfTime / Start(3000) / +=).

## 0x6008A0 CMentalState::Process(const CPed& ped)  (catalogue: specified/REA, port level 9)
- `__thiscall` (ECX = CMentalState* → EBP), ped at [esp+4] (ESI), `ret 4`; SEH frame (state var [esp+0x40]). Caller:
  CEventScanner::ScanForEvents 0x607E30. Callees: `_ftol` 0x821B40, CEventEditableResponse ctor 0x4AC450 / dtor 0x4AC480,
  CEventGroup::Add 0x4AB420 (thiscall on ped->intelligence(+0x47C)+0x68, args (CEvent*, bool 0)).
- Events (local CEventEditableResponse with its vtable overwritten; event types from the vtables' GetEventType slots):
  vt 0x86CBE8 → type 0x34 EVENT_HEALTH_REALLY_LOW; vt 0x86CBA0 → 0x35 EVENT_HEALTH_LOW; vt 0x86CC30 → 0x32 EVENT_LOW_ANGER_AT_PLAYER;
  vt 0x86CC78 → 0x33 EVENT_HIGH_ANGER_AT_PLAYER. Each is constructed, added with `Add(&ev, false)`, then destroyed.
- Behaviour:
  1. `pedHealth (+0x10) = (u8)_ftol(ped.m_fHealth (+0x540))` (truncation, low byte stored).
  2. If `ped.flags (+0x46C) & 0x100` (bInVehicle) and `ped.m_pVehicle (+0x58C) != NULL`: `vehicleHealth (+0x12) =
     (u8)_ftol(vehicle.m_fHealth (+0x4C0))` (low byte only: 1000.0 → 232).
  3. Not in vehicle (0x600908): if `oldPedHealth (+0x11) >= 50` (u8): if `pedHealth < 10` → add HEALTH_REALLY_LOW; else if
     `pedHealth < 50` → add HEALTH_LOW. Else (old < 50): if `old >= 10 && pedHealth < 10` → add HEALTH_REALLY_LOW.
  4. In vehicle (0x600994): compares the u8 health values with 600 and 300 (`movzx; cmp eax, 0x258 / 0x12C`): u8 values are always
     < 300, so the path always reaches 0x6009F5 `jl` → no event. ORIGINAL BUG (dead code): the vehicle health events
     (old >= 600: new < 300 → REALLY_LOW, new < 600 → LOW; old >= 300 && new < 300 → REALLY_LOW) can never fire. Keep.
  5. Anger (0x600A43): if `lastAnger (+1) <= 3`: if `anger (+0) > 6` → add HIGH_ANGER; else if `anger > 3` → add LOW_ANGER.
     Else if `lastAnger <= 6 && anger > 6` → add HIGH_ANGER.
  6. `lastAnger = anger; oldPedHealth = pedHealth; oldVehicleHealth = vehicleHealth` (0x600AFF-0x600B14). Note step 2 leaves
     vehicleHealth unchanged when not in a vehicle.
- gta-reversed: `plugin::CallMethod<0x6008A0>` only (MentalHealth.cpp:9).
