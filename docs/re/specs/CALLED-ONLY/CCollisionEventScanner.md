# CCollisionEventScanner (CALLED-ONLY) - full specs (RE-SPEC-FINAL)

`CCollisionEventScanner` is 1 byte: +0 `m_bAlreadyHitByCar` (bool). It lives inside CPedIntelligence (gr `m_collisionScanner`).
Ped fields used (raw offsets; gr names where known): +0x14 m_matrix, +0x1C entity flags, +0x8C m_fMass, +0xD8 m_fDamageIntensity,
+0xDC m_pDamageEntity, +0xE0 m_vecLastCollisionImpactVelocity, +0xEC m_vecLastCollisionPosn, +0xF8 m_nPieceType (u16),
+0xFC m_pAttachedTo, +0x128 (dword written, see case 4), +0x46C ped flags (bit 0 bIsStanding, bit 8 bInVehicle), +0x47C m_pIntelligence,
+0x480 m_pPlayerData, +0x4E0 acquaintance block (thiscall `CAcquaintance::GetAcquaintances` 0x608970), +0x550/+0x554 a 2-D vector
(dotted with the impact velocity; gr name not verified), +0x568 (dword, must be 0 in the train test), +0x584 (entity pointer compared in
case 4), +0x598 m_nPedType. Entity type = `byte +0x36 & 7` (1 building, 2 vehicle, 3 ped, 4 object).

## 0x604500 CCollisionEventScanner::ScanForCollisionEvents  (catalogue: specified/REA, port level 18)
- Convention: `__thiscall void (CCollisionEventScanner* this = ECX, CPed* victim [esp+4], CEventGroup* group [esp+8])`, `ret 8`; SEH
  frame (state slot [esp+0x12C] in the case bodies) for the stack-allocated events. Inside the body victim = ESI, other entity = EDI.
  The tail is reached through a .text detour: `0x60515F: nop; jmp 0x401CE1` which does `ecx = victim->m_pPlayerData (+0x480); jmp 0x605165`
  (port: plain code).
- Caller: CPedIntelligence::ProcessFirst 0x6073A0. Callees (in order of use): CTaskManager::GetSimplestActiveTask 0x6819D0, CTask::IsGoToTask
  0x61A360 (cdecl), 0x406DA0 (thiscall on a CVector, ST0 = squared length, see note), CPed::KillPedWithCar 0x5F0360,
  CPed::IsPlayer 0x5DF8F0, CPlaceable::GetHeading 0x441DB0, CEntity::GetColModel 0x535300, 0x4119D0 (`out = v * (1.0f/k)`),
  CMatrix::operator*(CVector) 0x59C890, VectorSub 0x40FE60 (`out = a - b`, cdecl (out, a, b)), CGeneral::LimitRadianAngle 0x53CB50,
  CVector::Normalise 0x59C910, CVector::DotProduct 0x40FDB0, 0x40FE90 (`out = k * v`, cdecl (out, float k, v)),
  CPedGeometryAnalyser::ComputePedHitSide(CPed*, CPhysical*) 0x5F3640, CPedDamageResponseCalculator ctor 0x4AD3F0 / dtor 0x4AD420 /
  ComputeDamageResponse 0x4B5AC0, CEventDamage ctor 0x4AD830 / AffectsPed 0x4B35A0 / dtor 0x4AD960, CPedIntelligence::GetMoveStateFromGoToTask
  0x601D70, event ctors CEventVehicleCollision 0x4AC840, CEventPlayerCollisionWithPed 0x5FEE40, CEventPedCollisionWithPlayer 0x5FED40,
  CEventPedCollisionWithPed 0x4AC990, CEventObjectCollision 0x4ACCF0, CEventBuildingCollision 0x4ACF00, CEventAcquaintancePedHate 0x420E70,
  CEventSoundQuiet 0x5E05B0, CEventGroup::Add 0x4AB420 (thiscall (event, 0)), RwHelper::GetEventGlobalGroup 0x4ABA50, CPed::Say 0x5EFFE0,
  0x421050 (thiscall on `ped->m_pIntelligence`, arg 1; catalogue CMentalState::IncrementAnger), CAcquaintance::GetAcquaintances 0x608970,
  CPedType::GetPedFlag 0x608830 (cdecl), CEntity::GetIsStatic 0x4633E0, 0x420800 (`max`: returns a if a > b else b, cdecl (a, b), ST0),
  CPed::GetLocalDirection 0x5DEF60, `_ftol` 0x821B40, CWeapon::GenerateDamageEvent 0x73A530, CEntity::GetIsTypePhysical 0x4DA030,
  CPedClothesDesc::GetIsWearingBalaclava 0x5A7950. Event destructors: 0x4AC920, 0x4ACA70 (after setting the vtable to 0x86C9B0 or 0x86C970
  as listed), 0x4ACDD0, 0x4ACFF0, 0x4AF890 (vtable 0x858E68), 0x5DEA00 (port: drop the event).
  Note on 0x406DA0: it returns `(x*x + y*y) + z*z` of `this` (squared length; no sqrt, 0x406DA0-0x406DBE; proposed name CVector::SquaredMagnitude). So "speed" below is the SQUARED
  move speed.
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84; .rdata 0.0f [0x858B50], 0.1f [0x858B1C], 20.0f [0x858BA4], -1.0f [0x858C1C],
  3.1415927f [0x858CB8], 0.0001f [0x858FC4], 0.01f [0x86CD7C], 0.0025f [0x86C920] and 0.0025f [0x86C924] (same value), 40.0f [0x858A10],
  30.0f [0x858CA4]; tweakable .data floats 3.0f [0x8D239C], 1.0f [0x8D23A0], 0.04f [0x8D23A4], 10.0f [0x8D23A8], 2.0f [0x8D23AC],
  1.0f [0x8D23B0] (read from memory each time); timers 0xC0B1B0 (u32, last "bump" sound) and 0xC0B1B4 (u32, last "moving object" sound).
- Behaviour:
  1. `other = victim->m_pDamageEntity`; if null, or `victim->m_fDamageIntensity == 0.0f` (`test ah,0x44; jnp`, NaN continues), go to 10.
  2. `task = victim->m_pIntelligence->taskManager(+4).GetSimplestActiveTask()`; `ms = (task && IsGoToTask(task)) ? task->dword[+8] : 1`
     (victim move state, called `ms` below).
  3. Switch on `(other->type) - 1` (jump table 0x6052E4: 0x605102, 0x60459D, 0x604B56, 0x604F70); type 0 or > 4 -> step 9.
  4. Building (0x605102): `group->Add(CEventBuildingCollision(piece, intensity, other, &impactVel, &collPos, ms), 0)` (piece = (u16)+0xF8,
     intensity = +0xD8 as float bits, impactVel = &victim+0xE0, collPos = &victim+0xEC). -> 9.
  5. Vehicle (0x60459D):
     a. If `this->m_bAlreadyHitByCar != 0` -> 9.
     b. `sp = other->m_vecMoveSpeed (+0x44).SquaredMagnitude()` (0x406DA0, stored f32); `thr = other->dword[+0x4C4] ? [0x86C924] : [0x86C920]`
        (both 0.0025f).
     c. Train kill (0x6045D8): if `other->dword[+0x594] == 6` && `other->byte[+0x40] & 4` && `victim->flags(+0x1C) bit 4` && victim bIsStanding
        && `victim->dword[+0x568] == 0` && `sp > 0.0001f` (strict): `victim->KillPedWithCar(other, 15.0f, false)`.
     d. If `sp > thr` (strict; else e):
        - `d = victim->m_fDamageIntensity`; if bIsStanding: `t = impactVel.x*v550 + impactVel.y*v554` (x87; `v550/v554` = victim +0x550/+0x554);
          if `t < 0`: `d2 = t*victim->m_fMass + d`; `d = (d2 < 0) ? 0.0f : d2` (0x60467D: `0 > d2` stores 0, NaN keeps d2).
        - If victim IsPlayer (0x6046A9):
          `if (d > 20.0f) d = 20.0f`; `h = other->GetHeading()` (stored f32); `bmin = other->GetColModel()->bbox.min` (+0), `bmax = +0xC`
          (each GetColModel called once, values copied); `c = (bmin + bmax)` per component (f32 sums: z, y, x in that order) then
          `c = 0x4119D0(c, 2.0f)` (= c * (1.0f/2.0f)); `cw = other->m_matrix * c` (0x59C890); `rel = VectorSub(cw, victimPos)` (victimPos =
          matrix pos or placement pos); `a = atan2(-rel.x, rel.y)` (fchs + fpatan, x87 value); `diff = LimitRadianAngle(h - a)` (argument
          stored f32 first; result stored f32); `boxAng = atan2(bmax.x - bmin.x, bmax.y - bmin.y)` (stored f32);
          `dir = Normalise(VectorSub(victimPos, otherPos))` (dir = victim - vehicle position; Normalise in place on the local).
          Then (0x604822): `ad = |diff|`; if `ad < boxAng` OR `PI - boxAng < ad` (both strict) -> `dot = (dir.y*mv.y + dir.z*mv.z) + dir.x*mv.x`
          (mv = other->m_vecMoveSpeed); else if `diff <= 0` (or NaN): `dot = DotProduct(other->matrix.right, mv)`; additionally if `sp > 0.01f`
          and `dot < 0.1f` it evaluates `DotProduct(other->matrix.right, victim->matrix.up) < 0` but the only effect is a dead store
          (0x6048D7 writes ebx into the boxAng slot) - dot is unchanged; else (`diff > 0`): `dot = DotProduct(-1.0f * other->matrix.right, mv)`.
          If `dot > 0.1f` (strict): `victim->KillPedWithCar(other, d, false)`. -> 9.
        - Not player: if `victim->m_pAttachedTo` exists and is a vehicle (type 2) -> damage path (f) [0x604925]; else
          `victim->KillPedWithCar(other, victim->m_fDamageIntensity, false)` (the RAW intensity, not d; 0x6049FB). -> 9.
     e. `sp <= thr` (or NaN, 0x604A17): if attachedTo is a vehicle -> damage path (f); else if victim IsPlayer -> 9; else
        `group->Add(CEventVehicleCollision(piece, intensity, other, &impactVel, &collPos, victim->m_pIntelligence->GetMoveStateFromGoToTask(), 0), 0)`.
     f. Damage path (two copies, 0x604942 / 0x604A32; identical): `side = ComputePedHitSide(victim, other)`;
        `calc = CPedDamageResponseCalculator(other, victim->m_fDamageIntensity, 49 /*0x31*/, 3 /*torso*/, false)`;
        `ev = CEventDamage(other, CTimer::m_snTimeInMilliseconds, 49, 3, side, false, (victim->flags46C >> 8) & 1 /*bInVehicle*/)`;
        `if (ev.AffectsPed(victim)) calc.ComputeDamageResponse(victim, &ev.m_damageResponse (ev+0x38), true)`; destroy ev, calc. The event is
        not added to any group.
  6. Ped (0x604B56): `oms = other->m_pIntelligence->GetMoveStateFromGoToTask()`. Event args for all ped events below:
     `(piece, intensity, X, &vel, &collPos, ms, oms)`.
     - victim IsPlayer: `group->Add(CEventPlayerCollisionWithPed(piece, intensity, other, &impactVel, ...), 0)`; `other->Say(0x1C, 0, 1.0f, 0, 0, 0)`.
     - else other IsPlayer: `group->Add(CEventPedCollisionWithPlayer(... other ...), 0)`; `0x421050(victim->m_pIntelligence, 1)`;
       `victim->Say(0x1C, 0, 1.0f, 0, 0, 0)`; if `GetAcquaintances(&victim+0x4E0, 4) & GetPedFlag(other->m_nPedType)`:
       `group->Add(CEventAcquaintancePedHate(other), 0)`.
     - else: `group->Add(CEventPedCollisionWithPed(... other ...), 0)`.
     Mirror (0x604D3E): only if `other->m_pDamageEntity == null`: `v = impactVel * -1.0f [0x858C1C]` (per component, stored f32);
     `og = &other->m_pIntelligence->+0x68` (other's event group):
     - other IsPlayer: `og->Add(CEventPlayerCollisionWithPed(piece, intensity, victim, &v, &collPos, ms, oms), 0)` (no Say);
     - else victim IsPlayer: `og->Add(CEventPedCollisionWithPlayer(..., victim, &v, ...), 0)`; `0x421050(other->m_pIntelligence, 1)`;
       if `GetAcquaintances(&other+0x4E0, 4) & GetPedFlag(victim->m_nPedType)`: `og->Add(CEventAcquaintancePedHate(victim), 0)` (no Say);
     - else `og->Add(CEventPedCollisionWithPed(..., victim, &v, ...), 0)`.
     (piece and intensity are the VICTIM's +0xF8/+0xD8 in the mirror too; ms/oms keep the same order.) -> 9.
  7. Object (0x604F70): `st = victim bIsStanding`; `d = intensity`; if st && `!other->GetIsStatic()`: `t = impactVel.x*v550 + impactVel.y*v554`;
     if `t < 0`: `d = 0x420800(0.0f, t*mass + d)` (max, NaN -> second). `thr = victim->m_pPlayerData ? [0x8D23AC] : [0x8D23B0]`.
     If `d > thr` && `!other->GetIsStatic()` && st && `other != victim->ptr[+0x584]` && !(`other->m_pAttachedTo != null && other->m_pAttachedTo
     == other`) (0x605000-0x605034):
       `dmg = (d / thr) * [0x8D23A8]` (stored f32); `dir2 = (-impactVel.x, -impactVel.y)` (CVector2D local);
       `ld = victim->GetLocalDirection(&dir2)`; `CWeapon::GenerateDamageEvent(victim, victim->m_pDamageEntity, 54 /*0x36*/, _ftol(dmg), 3, ld)`;
       `victim->dword[+0x128] = victim->m_pDamageEntity`.
     else `group->Add(CEventObjectCollision(piece, intensity, other, &impactVel, &collPos, ms), 0)`. -> 9.
  8. (none)
  9. Tail (0x605165): `pd = victim->m_pPlayerData`; if pd && (damage entity type is 1 or 4, re-read through +0xDC) &&
     `GetIsWearingBalaclava(pd->m_pPedClothesDesc (+4))`:
     - `e = victim->m_pDamageEntity`; if `GetIsTypePhysical(e)` && `e->byte[+0x40] & 0x20`: require `|e->m_vecTurnSpeed.z (+0x58)| > [0x8D23A4]`
       (strict) and `now > [0xC0B1B4] + 2000` (unsigned `jbe` skip), then `range = 40.0f` (immediate) and `[0xC0B1B4] = now`; else skip.
     - else: require `victim->m_fDamageIntensity > [0x8D23A0]` and `now > [0xC0B1B0] + 1000` (unsigned); `range = (intensity > [0x8D239C]) ?
       40.0f [0x858A10] : 30.0f [0x858CA4]`; `[0xC0B1B0] = now`; continue only if `range > 0` (always).
     - Emit: `GetEventGlobalGroup()->Add(CEventSoundQuiet(victim, range, 0xFFFFFFFF, &CVector(0,0,0)), 0)`.
  10. Always (also on the early exit of step 1): `this->m_bAlreadyHitByCar = 0`.
- Edge cases / oddities: m_bAlreadyHitByCar is only ever cleared here, so 5a only skips when something set it since the previous scan;
  the two vehicle thresholds are identical; the right/up DotProduct test in 5d has no effect (dead store); the non-player vehicle kill uses
  the raw damage intensity while the clamped `d` is only used for players; the `other == other->m_pAttachedTo` self test is effectively
  never true; the bump-sound branch runs only when the player wears the balaclava (as the bytes say).
- gta-reversed: `plugin::CallMethod` wrapper only (CollisionEventScanner.cpp:5). Open questions: names of victim +0x550/+0x554, +0x568,
  +0x584 and +0x128 (offsets are exact; semantics unverified).
