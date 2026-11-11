# CInterestingEvents (AI) - RE-SPEC-3 group I4

Layout (gr InterestingEvents.h; global g_InterestingEvents): +0x00 m_Events[8] (TInterestingEvent {i32 type, u32 time,
CEntity* entity}, stride 0xC), +0x60 m_nPriorities[29] (u8), +0x7E m_nDelays[29] (u16), +0xB8 m_nEndsOfTime[29] (u32),
+0x12C m_nFlags (u8: bit0 enabled, bit1 "behind-camera check", bit2, bit3 "short end time"), +0x130 m_CurrentFrameCounter (u32),
+0x134 m_nLastScanTimeUpdate, +0x138 m_fRadius (f32), +0x13C m_vecCenter (CVector), +0x148 vec148 (CVector, view direction),
+0x154 m_nInterestingEvent (i8).

## 0x6023A0 CInterestingEvents::CInterestingEvents  (catalogue name: CInterestingEvents::Constructor; catalogue: specified/REA, port level 0)
- `__thiscall void (this)`, plain `ret`. EAX = this on return (`mov eax,ecx` at 0x6023A0; not used by the caller).
- Callers: `__init_g_InterestingEvents` 0x8514E0 (static init).
- Behaviour (in byte order):
  1. `m_nFlags = (m_nFlags & 0xFE) | 0x0E` (0x6023A2-0x6023B3): read-modify-write - bits 4..7 keep their old value (0 in .bss).
  2. `m_nInterestingEvent = -1` (0xFF, 0x6023B9); `m_nLastScanTimeUpdate = 0` (0x6023C0); `m_fRadius = 30.0f` (imm 0x41F00000,
     0x6023C6); `m_CurrentFrameCounter = CTimer::m_FrameCounter [0xB7CB4C] - 1` (0x6023D0-0x6023D7).
  3. For the 8 events: type = 0, time = 0, entity = 0 (0x6023E5-0x6023F1).
  4. For k = 0..28: `m_nEndsOfTime[k] = 0`, `m_nDelays[k] = 2000`, `m_nPriorities[k] = 5` (0x602403-0x602418).
  5. Overrides (0x60241A-0x602556), final table (type: priority, delay ms):
     0: 5, 2000 | 1-3: 1, 5000 | 4-7: 2, 3000 | 8-9: 4, 3000 | 10: 5, 6000 | 11: 6, 6000 | 12: 6, 8000 | 13: 6, 5000 | 14: 5, 6000 |
     15-16: 9, 6000 | 17: 8, 6000 | 18: 7, 6000 | 19: 6, 5000 | 20: 7, 6000 | 21: 8, 8000 | 22: 9, 5000 | 23-25: 9, 6000 |
     26: 9, 8000 | 27-28: 10, 4000.
- Not written: m_vecCenter (+0x13C) and vec148 (+0x148) (zero only because the object is in .bss).
- gta-reversed: InterestingEvents.cpp:23 (`.Reversed = false`): same table (checked entry by entry); gr sets `m_nFlags = 0` first
  and zero-initialises the two vectors (harmless for the static object). Open questions: none.

## 0x602590 CInterestingEvents::Add  (catalogue: specified/REA, port level 5)
- `__thiscall void (this, int32 type [esp+4], CEntity* entity [esp+8])`, `ret 8`. Body split by the no-CD build (flag test at
  0x401F31-0x401F38).
- Callers: 27 (CEventHandler::ComputeSexyVehicleResponse 0x4B9AA0, ComputeKnockOffBikeResponse 0x4B9FF0, ComputeDamageResponse
  0x4C0170, ComputePedEnteredVehicleResponse 0x4C1590, CTaskComplexChat::CreateFirstSubTask, ...).
- Callees: CWorld::FindPlayerPed 0x56E210 (arg -1), CVector::NormaliseAndMag 0x59C970 (ST0), CWorld::GetIsLineOfSightClear
  0x56A490, CEntity::CleanUpOldReference 0x571A00, CEntity::RegisterReference 0x571B70.
- Globals: TheCamera.ActiveCam (byte 0xB6F081), TheCamera.m_aCams[i].Source (0xB6F338 + i*0x238), CTimer::m_FrameCounter
  0xB7CB4C, CTimer::m_snTimeInMilliseconds 0xB7CB84.
- Behaviour:
  1. Return if `!(m_nFlags & 1)` (0x401F31) or entity == 0 (0x6025A8).
  2. `cam = TheCamera.m_aCams[ActiveCam].Source` (copied as 3 dwords, 0x6025B2-0x6025E1).
  3. If `m_CurrentFrameCounter != CTimer::m_FrameCounter` (0x6025DF): store the frame counter (0x6025ED);
     `player = FindPlayerPed(-1)`; `pp = player->m_matrix ? &matrix->pos (+0x30) : &placement.pos (+4)` (0x6025FA-0x602609);
     `vec148 = (pp.x - cam.x, pp.y - cam.y, pp.z - cam.z)` (each f32, 0x60260C-0x602645); `vec148.z = 0` (0x602648);
     `m = vec148.NormaliseAndMag()`; if `m == 0.0f [0x858B50]` (exact equality; `test ah,0x44; jp` skips on not-equal and on
     NaN, 0x602657-0x602662) `vec148 = player->m_matrix->forward (+0x10)` (NO null check of m_matrix here, 0x602664).
     Then `r = m_fRadius`: `t.x = r*vec148.x`, `t.y = r*vec148.y` (f32 stores), `t.z = r*vec148.z` (kept in st0);
     pp re-read (same matrix/placement choice, 0x602682-0x6026A2); `m_vecCenter = (t.x + pp.x, t.y + pp.y, t.z + pp.z)` (0x6026A5-0x6026D9).
  4. Range (0x6026DC-0x60271B): `r2 = m_fRadius * m_fRadius`; ep = entity position (matrix or placement);
     `dx = center.x - ep.x; dy = center.y - ep.y; d2 = dy*dy + dx*dx` (that addition order); return if `d2 > r2`
     (`fcomp st(3); test ah,0x41; je`; NaN continues).
  5. If `m_nFlags & 2` (0x602721): `camDot = -((cam.y*v.y + cam.z*v.z) + cam.x*v.x)` (v = vec148; `fchs` at 0x602751);
     `eDot = (v.z*ep.z + v.y*ep.y) + v.x*ep.x` (0x602758-0x602774); return if `eDot + camDot < 0.0f [0x858B50]`
     (`test ah,5; jnp` = strictly less; equal / NaN continue, 0x602776-0x602785).
  6. `if (!CWorld::GetIsLineOfSightClear(&cam, &ep, buildings 1, vehicles 0, peds 0, objects 0, dummies 0, seeThrough 1,
     cameraIgnore 0))` return (0x602795-0x6027B3).
  7. Slot search (0x6027B9-0x602827): `pNew = m_nPriorities[type]` (zero-extended), `now = m_snTimeInMilliseconds`,
     `newReady = now > m_nEndsOfTime[type]` (unsigned). For i = 0..7 (ev = &m_Events[i]):
     `isCur = (i == (int32)(int8)m_nInterestingEvent)`; `oldReady = now > (u32)m_nDelays[ev->type] + ev->time` (unsigned, 32-bit
     wrapping add; computed for every slot, so m_nDelays is indexed with ev->type even for empty slots);
     - if `ev->entity == 0`: `ev->type = 0; ev->entity = 0`; take slot i (0x602833);
     - else if `ev->type == 0`: take slot i (0x602801-0x602803; no priority or timing test for type-0 events);
     - else if `(pNew < m_nPriorities[ev->type] && !oldReady)` -> next slot; else if `!newReady || isCur` -> next slot; else take.
     If no slot is taken, return.
  8. Take (0x602840-0x602860): if ev->entity CleanUpOldReference(ev->entity, &ev->entity); `ev->type = type; ev->entity = entity;
     ev->time = now`; `entity->RegisterReference(&ev->entity)`.
  9. If `m_nFlags & 8`: `m_nEndsOfTime[type] = now + (m_nDelays[type] >> 1)` (0x602872-0x60287E) else `m_nEndsOfTime[type] = now`
     (0x60288C).
- gta-reversed: InterestingEvents.cpp:88 is `plugin::CallMethod` followed by dead reference code that differs from the bytes:
  (a) step 5 is executed when bit1 is SET (gr tests `!m_b2`); (b) step 9 branches are swapped (gr: m_b8 -> `time`);
  (c) gr lacks the "type == 0 -> take" shortcut; (d) gr sums dot products x+y+z, the original (z+y)+x / (y+z)+x. Port from the bytes.
- Open questions: none.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x6028A0 CInterestingEvents::GetInterestingEvent  (catalogue: specified/REA, port level 0)
- `__thiscall Event* ()`, `ret`. Layout: 8 events of 12 bytes at +0 {+0 type, +4 u32 time, +8 CEntity*}, +0x60 u8 priority[type], +0x7E u16 duration[type], +0x12C flags,
  +0x154 int8 last chosen index. now = CTimer::m_snTimeInMilliseconds [0xB7CB84].
- If flags bit 2 and last != -1 (0xFF) and events[last].entity != 0 and `now < events[last].time + duration[events[last].type]` (unsigned `jb`) -> return &events[last]
  (0x6028A7-0x6028E1).
- Else best = -1, bestPri = 0; for i = 0..7 (0x6028F6-0x60293B): if entity != 0 and `now < time + duration[type]` (unsigned): p = priority[type]; if `p > bestPri` (signed)
  -> choose; else `(rand() & 0xFFFF) < 0x80` (`_rand` 0x821B1E, called only in this case) -> choose. choose: bestPri = p, best = i. Then +0x154 = (u8)best; return best == -1 ?
  0 : &events[best].

## 0x856880 CInterestingEvents::Destructor  (catalogue: specified/REA, port level 1)
- atexit-style `void ()` for the global instance at 0xC0B058: for 8 slots `p = 0xC0B060 + 12*i` (events[i].entity): if `*p` `CEntity::CleanUpOldReference(*p, p)` 0x571A00
  then `*p = 0` (0x856890-0x8568A6).

## 0x605A30 CInterestingEvents::ScanForNearbyEntities  (catalogue: specified/REA, port level 6)
- Convention: `__thiscall void (this = ECX: CInterestingEvents*)`, plain `ret`; frame `push ebp; mov ebp,esp; and esp,-8; sub esp,0x28`.
- Callers: CGame::Process 0x53BEE0 (this = g_InterestingEvents 0xC0B058). Callees: CWorld::FindPlayerPed(-1) 0x56E210,
  CVector::NormaliseAndMag 0x59C970 (returns ST0), CRT floor(double) 0x8219F0 + _ftol 0x821B40 (truncation of the floored value),
  CWorld::ClearScanCodes 0x563470, CInterestingEvents::Add 0x602590 (thiscall on 0xC0B058, (type, entity)).
- Globals: byte [0xC0B184] (= g_InterestingEvents+0x12C, read through the ABSOLUTE address, not this), CTimer::m_snTimeInMilliseconds
  0xB7CB84, m_FrameCounter 0xB7CB4C, TheCamera.ActiveCam byte 0xB6F081 and cam Source vectors 0xB6F338 + 0x238*ActiveCam,
  CWorld::ms_nCurrentScanCode 0xB7CD78 (u16, R/W), CWorld::ms_aRepeatSectors 0xB992B8 (16x16 entries of 3 list heads, stride 12:
  +0 vehicles, +4 peds).
- Fields: +0x130 last frame counter, +0x134 last scan time, +0x138 f32 radius, +0x13C CVector centre, +0x148 CVector direction.
- Behaviour:
  1. If ([0xC0B184] & 1) == 0 return. now = ms time; if now - this+0x134 < 500 (unsigned `jb`) return; this+0x134 = now.
  2. player = FindPlayerPed(-1) (no NULL check). If this+0x130 != m_FrameCounter: this+0x130 = m_FrameCounter; c = camera Source;
     p = player pos (matrix+0x30 or placement+4); dir (+0x148/+0x14C/+0x150) = (p.x - c.x, p.y - c.y, p.z - c.z) (each fsub stored
     f32), then +0x150 = 0 (0x605B06); if dir.NormaliseAndMag() == 0.0f [0x858B50] (`test ah,0x44; jp`: NaN does not count as 0):
     dir = player->matrix->forward (+0x10..+0x18 of the matrix; no NULL check on the matrix here). Then with r = +0x138: tx = r*dir.x
     (stored f32), ty = r*dir.y (stored f32), tz = r*dir.z (kept in ST0); centre = (tx + p.x, ty + p.y, tz + p.z) (p re-fetched).
  3. Sector bounds (each computed twice by the bytes, once for the test and once for the value): f(v) = _ftol(floor((double)(v *
     [0x858B38]=0.02f + [0x858B34]=60.0f))) where v is built in x87 as (a - b) then *0.02 then +60, stored as a double for floor.
     x0 = max(f(centre.x - r), 0) (`test eax,eax; jle` -> 0); y0 = max(f(centre.y - r), 0); x1 = min(f(r + centre.x), 0x77);
     y1 = min(f(r + centre.y), 0x77) (`cmp eax,0x77; jge` -> 119). Note x1/y1 load r first then add the centre component.
  4. Advance scan code: if ms_nCurrentScanCode >= 0xFFFF { ClearScanCodes(); code = 1 } else code+1; store; player+0x2C = code.
  5. If y0 > y1 return. For y = y0..y1 (outer, 0x605E7E), for x = x0..x1 (inner): s = sector[((y & 15) << 4) + (x & 15)]:
     - peds list (s+4), for each node (entity = node[0], next = node[1] read before processing): skip if ped+0x2C == code; else
       ped+0x2C = code; skip if ped+0x530 (m_nPedState) == 0x37 (PEDSTATE_DEAD); e = (ped+0x46C & 0x100) && ped+0x58C != NULL ?
       vehicle : ped; type = ped+0x598: 6 (COP) -> Add(5, e); 20 (CRIMINAL) -> Add(6, e); 22 (PROSTITUTE) -> Add(4, e); 7..16 (gangs)
       -> Add(7, e); else nothing.
     - vehicles list (s+0): skip if veh+0x2C == code; else veh+0x2C = code; skip if veh+0x40 & 0x20000000 (bRenderScorched);
       skip if veh+0x460 (driver) == NULL; style = u8 veh+0x3B9 (autopilot driving style); skip if style == 0 or 6; else
       Add(0xE = 14, veh).
- gta-reversed: plugin::CallMethod stub plus an unexecuted draft (InterestingEvents.cpp:148). Draft errors vs bytes: it multiplies by
  50.0 where the original multiplies by 0.02 (then asserts against GetSectorX, so its asserts would fire); it uses m_pVehicle even when
  NULL (the original falls back to the ped); it reads the flag via `m_b1` (the original reads the absolute global). Otherwise it
  matches.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x602240 CInterestingEvents::sub_602240 (proposed: CPedIntelligence::AddInterestingEntity)  (catalogue: unknown/REA, port level 2)
- Name correction: operates on `CPedIntelligence::m_apInterestingEntities[3]` (+0x288/+0x28C/+0x290; gr PedIntelligence.h:65, size 0x294), not on CInterestingEvents. No callers / data refs (dead out-of-line copy).
- `__thiscall void (this: CPedIntelligence*, CEntity* entity [esp+4])`, `ret 4`.
- Behaviour: shift the history down with reference bookkeeping, newest at index 0: for k = 2, 1 (esi = +0x290 then +0x28C, `ebx = 2` 0x60224B): `if e[k]` -> CleanUpOldReference(e[k], &e[k]) 0x571A00 (0x602257); `e[k] = e[k-1]` (0x602261); `if e[k]` -> RegisterReference(e[k], &e[k]) 0x571B70 (0x602266). Then `if e[0]` -> CleanUpOldReference(e[0], &e[0]) (0x602282); `e[0] = entity`; `if entity` -> RegisterReference(entity, &e[0]) (0x602292). e[2]'s old entity is dropped; no duplicate check.
