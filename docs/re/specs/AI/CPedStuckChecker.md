# CPedStuckChecker (AI) - RE-SPEC-AI-2 (lead)

Layout (gr PedStuckChecker.h, size 0x10): +0 m_lastNonStuckPoint CVector, +0xC u16 (gr `m_radius`; the bytes use it as a frame
counter, proposed name `m_frameCount`), +0xE u16 m_state (0 NONE, 1 STUCK, 2 WAS_STUCK/pushed). Constructed inline at 0x607292.

## 0x602C00 CPedStuckChecker::TestPedStuck  (catalogue: specified/REA, port level 9)
- Convention: `__thiscall bool (this = ECX: CPedStuckChecker* (kept in EBP), CPed* ped [esp+4], CEventGroup* events [esp+8])`, `ret 8`,
  SEH frame (only for the stack CEventStuckInAir). Locals relative to B = esp after `push edi` (entry - 0xB4): B+0x10 cnt f32 (later z2),
  B+0x14 z1, B+0x18 V CVector, B+0x24 D CVector, B+0x30 colEntity CEntity*, B+0x34 tmp CVector, B+0x40 CEventStuckInAir, B+0x50 colPoint1
  (CColPoint; pos.z at B+0x58), B+0x7C colPoint2 (pos.z at B+0x84). "pos" = ped matrix ? &matrix->pos (+0x14 -> +0x30) : &placement.pos (+4),
  re-evaluated at every use.
- Callers: CPedIntelligence::ProcessFirst 0x6073A0. Callees: CPed::IsPlayer 0x5DF8F0, CPedIntelligence::GetTaskSwim 0x601070 /
  GetTaskJetPack 0x601110 / GetTaskClimb 0x601180, VectorSub 0x40FE60 (out = a - b), VectorAdd 0x40FE30 (out = a + b, unnamed in the
  catalogue: proposed name `operator+(CVector,CVector)`), CVector::Magnitude 0x406DA0 (thiscall, x*x+y*y+z*z then sqrt; catalogue
  FUN_00406da0, NAME PROPOSAL), CWorld::ProcessVerticalLine 0x5674E0, CWorld::GetIsLineOfSightClear 0x56A490, CPlaceable::SetPosn 0x4241C0,
  CVector::Normalise 0x59C910, CPhysical::ApplyMoveForce(CVector) 0x5429F0, CGeneral::GetRadianAngleBetweenPoints 0x53CBE0,
  CGeneral::LimitRadianAngle 0x53CB50, CPlaceable::SetOrientation 0x439A80, CEventGroup::GetEventOfType 0x4AB650, CEventStuckInAir ctor
  0x4B1490, CEventGroup::Add 0x4AB420, CEventStuckInAir dtor 0x4B1520 (catalogue FUN_004b1520, NAME PROPOSAL `CEventStuckInAir::~CEventStuckInAir`).
- Globals: CTimer::ms_fTimeStep 0xB7CB5C (R), CTimer::m_FrameCounter 0xB7CB4C (R).
- Ped fields: +0x1C entity flags (bit0 bUsesCollision, bit4 bIsStuck), +0x20 low byte of m_nRandomSeed, +0xB9 m_nNumEntitiesCollided u8,
  +0xBC m_apCollidedEntities[0], +0xD8 m_fDamageIntensity, +0xDC m_pDamageEntity, +0xE8 m_vecLastCollisionImpactVelocity.z, +0xFC
  m_pAttachedTo, +0x46C ped flags (bit0 bIsStanding, bit1 bWasStanding), +0x470 bit 0x200000 (third flag dword, gr name not checked),
  +0x47C m_pIntelligence, +0x530 m_nPedState, +0x558 m_fCurrentRotation, +0x55C m_fAimingRotation. Entity type = byte +0x36 & 7
  (1 building, 3 ped).
- Behaviour. RESET = {m_frameCount = 0; m_state = 0}. TAIL = step 9.
  1. If !(ped flags bit0 bUsesCollision) or ped->m_pAttachedTo != 0 or m_nPedState == 0x36 or 0x37: RESET, return 0 (0x602C25-0x602C53,
     0x603459).
  2. If bIsStanding or bWasStanding: RESET -> TAIL (0x602C59-0x602C70, 0x602F6E).
  3. Continue only if (m_pDamageEntity != 0 and m_fDamageIntensity > 0.0f [0x858B50]) or (ped->IsPlayer() and entity bIsStuck); otherwise
     RESET -> TAIL (0x602C76-0x602CA9). If GetTaskSwim() or GetTaskJetPack() or GetTaskClimb() is non-null: RESET -> TAIL (0x602CAF-0x602CE2).
  4. D = (0,0,0), colEntity = 0. n = m_frameCount: if n > 10000 (unsigned u16) or n == 0: m_frameCount = 1, m_state = 0, m_lastNonStuckPoint
     = pos (0x602D41-0x602D69). Else D = pos - m_lastNonStuckPoint, m_frameCount = n + 1 (0x602D08-0x602D3B).
  5. t = (0.01f [0x858C58] > ms_fTimeStep) ? 0.01f : ms_fTimeStep (`test ah,0x41; jne`, 0x602D6C-0x602D87). cnt = (float)m_frameCount
     (fild, stored f32 at B+0x10). If cnt > 50.0f [0x858B40] / (4.0f [0x858B90] * t) (0x602D9D-0x602DB8) and cnt * 0.01f [0x858C58] >
     |D| (Magnitude 0x406DA0, 0x602DBA-0x602DD4) -> STUCK (step 7).
  6. Else, k = m_nNumEntitiesCollided: k == 0 -> TAIL. k >= 2 (0x602DDC-0x602E3A): t recomputed as in step 5; if cnt > 50.0f [0x858B40] /
     (t + t) and cnt * 0.004f [0x859CE0] > |D.z| -> STUCK, else TAIL. k == 1 -> PUSH-OUT (step 8).
  7. STUCK (0x602E40): if ((u8 randomSeed + m_FrameCounter + 3) & 7) != 0: m_state = 1 (0x602FC1-0x602FCB; also a dead store of ped->+0x480
     into a local) -> TAIL. Else (one frame in 8): V = pos with V.z = pos.z + 1.0f [0x858624]; hit = ProcessVerticalLine(&V, pos.z -
     1.0f [0x858624], &colPoint1, &colEntity, 1, 1, 0, 1, 0, 0, nullptr) (0x602E57-0x602EBB). No hit -> TAIL (m_state unchanged).
     On hit: if (ped +0x470 & 0x200000) and !(colPoint1.z + 1.0f < pos.z) (`test ah,5; jp`, NaN counts as not less) -> skip to (*).
     If m_fDamageIntensity > 0.0f [0x858B50] and m_pDamageEntity is a ped and impactVelocity.z > 0.3f [0x858C24] -> skip to (*).
     Else V.z = colPoint1.z + 1.0f [0x858624]; SetPosn(&V); clear +0x470 bit 0x200000 if set (0x602F35-0x602F61).
     (*) bIsStanding = 1 (0x602F67); RESET -> TAIL.
  8. PUSH-OUT (0x602FD8): e = m_apCollidedEntities[0]; require e != 0 and type(e) == 1 (building), t as step 5, cnt > 50.0f [0x858B40] /
     (t + t) and cnt * 0.004f [0x859CE0] > |D.z|; otherwise TAIL.
     V = (-D.y, D.x, 1.0f); V.Normalise() (0x603048-0x603066).
     z1: hit = ProcessVerticalLine(&(pos + V), -20.0f (imm 0xC1A00000), &colPoint1, &colEntity, 1, 0, 0, 0, 0, 0, nullptr); if hit and
     GetIsLineOfSightClear(pos, pos + V, 1, 1, 0, 1, 0, 0, 0): z1 = colPoint1.z, else z1 = 5001.0f (imm 0x459C4800) (0x60307A-0x603103).
     z2: same with pos - V and colPoint2; miss/blocked value 5002.0f (imm 0x459C5000) (0x603103-0x6031A1).
     choice: if z1 > 5000.0f [0x86CD78] and z2 > 5000.0f -> -1 (0x6031A1-0x6031C5). Else if (pos.z - 1.0f < z1) and (z1 < 5000.0f
     [0x86CD78]) and ((z2 < 5001.0f [0x86CD74]) or (z1 < z2)) -> 1 (0x6031CD-0x60321F; all `test ah,5; jp/jnp` = strict less, NaN false).
     Else if (pos.z - 1.0f < z2) and (z2 < 5001.0f [0x86CD74]) -> 2 (0x603226-0x603259). Else 0.
     If choice is 1 or 2: unless (+0x470 & 0x200000) or the damage-by-ped test of step 7 (0.3f [0x858C24]) holds: SetPosn(choice == 2 ?
     &colPoint2.pos : &colPoint1.pos); pos.z += 1.0f [0x858624] (in place, 0x6032A3-0x6032D4).
     If z1 < z2: V = V * -1.0f [0x858C1C] (0x6032D7-0x603324).
     ApplyMoveForce((V.x * 4.0f [0x858B90], V.y * 4.0f [0x858B90], 4.0f)) (CVector by value, 0x603328-0x60336F); the code also leaves
     V.z = 1.0f in the local.
     If choice > -1 (signed u16 compare, includes 0): off = V * 0.25f [0x858C84] (with V.z = 1.0f); SetPosn(pos + off) (0x60337E-0x6033CD);
     h = GetRadianAngleBetweenPoints(V.x, V.y, 0, 0); ped->m_fCurrentRotation = h; h = LimitRadianAngle(h); m_fCurrentRotation =
     m_fAimingRotation = h; SetOrientation(0, 0, h) (0x6033D2-0x603413).
     m_state = 2 (0x603418) -> TAIL.
  9. TAIL (0x602F76): if m_state == 0 return 0. If events->GetEventOfType(0x42) exists and its f32 at +0xC > 1.0f [0x858624]
     (`test ah,0x41; jne` -> <= / NaN adds the event): clear bIsStanding, return 0 (0x602F80-0x602FBC). Else construct
     CEventStuckInAir(ped) on the stack, events->Add(&ev, false), destroy it, return 1 (0x603423-0x603455).
- Edge cases: m_state persists between calls unless a RESET path runs (step 6 "TAIL" keeps a previous 1/2). Step 5 compares the counter
  against 50/(4t) but step 6/8 use 50/(2t). In step 8 the -20.0f is passed as the vertical line's end Z (absolute), not a distance.
- gta-reversed: absent (catalogue name from CAT-5); gr only has the class layout. gr calls +0xC `m_radius`; it is a frame counter.
- Open questions: the meaning of ped +0x470 bit 0x200000 (tested 0x602ECB/0x60325E, cleared 0x602F5C) and of event 0x42's +0xC field
  (read 0x602FA1) were not resolved by name.
