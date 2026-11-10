# CRoadBlocks (POP) - full specs

## 0x4619C0 CRoadBlocks::CreateRoadBlockBetween2Points  (catalogue: specified/REA, port level 16)
- Convention: `__cdecl void (CVector a [esp+4..+0xC], CVector b [esp+0x10..+0x18], int32/bool isGang [esp+0x1C])` (by value),
  plain `ret` (0x4629D5). Entry is `push -1; jmp 0x407465`, an obfuscated .text stub that pushes the SEH handler
  `[0x448906] ^ [0x1560868]` = 0x837C55 and jumps back to 0x4619C7 (no behaviour). Frame: 0x10C bytes of locals; offsets
  below ("L+x") are relative to ESP after the 4 register pushes (args at L+0x12C a, L+0x138 b, L+0x144 isGang).
- Callers: CRoadBlocks::GenerateRoadBlocks 0x4629E0 (5 sites; always isGang = 0 there).
- Callees (all `orig!`): CVector::Normalise 0x59C910 (thiscall), CWorld::FindPlayerCoors 0x56E010 (cdecl (CVector* out, int -1)),
  CPopulation::PickRiotRoadBlockCar 0x6144B0, CWorld::FindPlayerWanted 0x56E230, CWanted::AreArmyRequired 0x561F80 /
  AreFbiRequired 0x561F60 / AreSwatRequired 0x561F40 (thiscall -> AL), CStreaming::GetDefaultCopCarModel 0x407C50 (cdecl (0)),
  `_rand` 0x821B1E, `_ftol` 0x821B40, CMatrix::RotateY 0x59B2C0 / RotateZ 0x59B390 (thiscall (float)), CMatrix::SetTranslateOnly
  0x59AF80 (thiscall (x, y, z)), CMatrix::SetUnity 0x59AE70, CMatrix::SetTranslate 0x59AF40 (thiscall (x, y, z)),
  0x59C810 `Multiply3x3(CVector* out, const CVector* v, const CMatrix* m)` (cdecl; out.x = (m.right.y*v.y + m.right.z*v.z) +
  v.x*m.right.x, out.y = (m.fwd.y*v.y + m.fwd.x*v.x) + m.fwd.z*v.z, out.z = (m.up.y*v.y + m.up.x*v.x) + m.up.z*v.z, i.e. the
  TRANSPOSED rotation), CRoadBlocks::ClearSpaceForRoadBlockObject 0x461020 (cdecl (CVector, CVector) by value -> AL), CObject
  operator new 0x5A1EE0 (0x17C) + ctor 0x5A1D70 (thiscall (int32 model, bool create = 1)), CMatrix::operator= 0x59BBC0 (thiscall
  (const CMatrix*)), CWorld::Add 0x563220, CFireManager::StartFire(CEntity*) 0x53A050 (thiscall ECX = gFireManager 0xB71F80),
  CVehicle operator new 0x6E2D50 (0x988) + CAutomobile ctor 0x6B0A90 (thiscall (model, createdBy = 1, setupSuspension = 1)),
  CAutomobile::PlaceOnRoadProperly 0x6AF420, CMatrix::UpdateRwMatrix 0x59AD70 / CSimpleTransform::UpdateRwMatrix 0x54EF40,
  CCarCtrl::JoinCarWithRoadSystem 0x42F5A0, CAutomobile::SetTotalDamage 0x6A27F0 (thiscall (bool)), CVehicle::UsesSiren
  0x6D8470, CVisibilityPlugins::SetClumpAlpha 0x732B00, CDamageManager::GetCarNodeIndexFromDoor 0x6C26F0 (cdecl (door) -> EAX),
  CObject::DeleteAllTempObjectsInArea 0x5A1980 (cdecl (CVector by value, float)), CWorld::FindGroundZFor3DCoord 0x5696C0 (cdecl
  (x, y, z, bool* found, CEntity** = null) -> ST0), local CMatrix destructor 0x59ACD0 at exit.
  Virtual calls: entity vtbl +0x10 SetIsStatic(bool) (0x462194, 0x46226D); vehicle vtbl +0xD4 GetHeightAboveRoad -> ST0
  (0x46223A); +0xA8 BlowUpCarCutSceneNoExtras(1,1,1,1) (0x46232E); +0x6C OpenDoor(CPed* = 0, nodeIdx, door, 1.0f, 1)
  (0x4623F1, 0x462425); +0x0 scalar deleting destructor (1) (0x4624C4). (CAutomobile vtable 0x871120 slots 4/27/42/53 =
  0x403E20 / 0x6A6AE0 / 0x6B3BB0 / 0x6A62B0.)
- Globals: MI_ROADWORKBARRIER1 0x8CD57C (int16), MI_ROADBLOCKFUCKEDCAR1 0x8CD580, CModelInfo::ms_modelInfoPtrs 0xA9B0C8,
  CStreaming::ms_aInfoForModel load state 0x8E4CD0 + 20*model, CObject::nNoTempObjects 0xBB4A70 (u16, ++ per object),
  CTimer::m_snTimeInMilliseconds 0xB7CB84.
- Constants: 0x858B8C = 0.5f, 0x858C1C = -1.0f, 0x858B50 = 0.0f, 0x858CA0 = 2.0f, 0x858CC4 = 0.2f, 0x858BF4 = 128.0f,
  0x859CE0 = 0.004f, 0x858F44 = 0.002f, 0x859CDC = 0.006f, 0x859CD8 = 0.003f, 0x858C24 = 0.3f, 0x858CC8 = 0.6f,
  0x858B14 = 3.0517578e-05f (1/32768), 0x858B90 = 4.0f, 0x859CD4 = 0.94f, 0x858C80 = 5.0f, 0x858C28 = 0.05f;
  immediates 3.1415927f (0x40490FDB), 2.8f (0x40333333), 1.0f.
- Fields written: vehicle +0x36 status (bits 3-7), +0x3B7 cur lane, +0x3B8 next lane, +0x3BA mission, +0x3BB temp action,
  +0x3C8 autopilot m_speed (dword 0), +0x3D0 cruise speed, +0x428 (&= 0xE7: bIsLocked, bEngineOn cleared), +0x42A |= 0x10
  (bCreateRoadBlockPeds), +0x42C |= 0x40 (bNeverUseSmallerRemovalRange), +0x42D |= 0x80 (bSirenOrAlarm), +0x4AD
  m_nPedsPositionForRoadBlock, +0x4AE m_nNumPedsForRoadBlock, +0x4D0 m_nTimeTillWeNeedThisCar, +0x4D8 m_nTimeWhenBlowedUp,
  +0x4F8 m_nDoorLock, +0x490 m_pFire (read), +0x648 m_aCarNodes (read). Object +0x13C m_nObjectType = 3, +0x150 m_nRemovalTime,
  +0x164 m_pFire (read). CFire +0x21 m_RemovalDist.
- Behaviour (block by block):
  1. Geometry (0x4619DF-0x461B6E). Local CMatrix M at L+0x74 (attach fields L+0xB4/L+0xB8 zeroed, 0x4619E1).
     `dz = b.z - a.z; dy = b.y - a.y; dx = b.x - a.x` (each stored f32); `V = (dx, dy, dz)`;
     `len = sqrt((dx*dx + dy*dy) + dz*dz)` (0x461A42-0x461A5C, stored f32 L+0xBC).
     `mid = ((a.x + b.x) * 0.5f, (a.y + b.y) * 0.5f, (a.z + b.z) * 0.5f)` (L+0xD4/D8/DC). `V.Normalise()` (0x461AD8).
     `P = (V.y, -V.x, 0)`, `P.Normalise()` (0x461AF7). `w = FindPlayerCoors(-1) - mid`; `q = ((w.z*P.z) + (w.x*P.x)) + w.y*P.y`
     (x87 order 0x461B27-0x461B39); if `q < 0` (strict; equal/NaN keep) `P = P * -1.0f` component-wise (each fmul + fstp).
  2. Model (0x461B72-0x461C28). If isGang: `model = PickRiotRoadBlockCar()`; if `model == -1` return. Else: army required ->
     433 (0x1B1), else FBI -> 490 (0x1EA), else SWAT -> 427 (0x1AB), else `GetDefaultCopCarModel(0)` (each test re-calls
     FindPlayerWanted(-1)); then if `ms_aInfoForModel[model].m_nLoadState != 1` `model = GetDefaultCopCarModel(0)`; if
     `model == 523` (0x20B) return. (The gang model gets neither check.)
  3. Sizes (0x461C2D-0x461CA6): from `ms_modelInfoPtrs[model]->m_pColModel` (+0x14): bbox min (+0) -> Bmin, max (+0xC) -> Bmax.
     `W = (Bmax.x - Bmin.x) + 2.0f`, `Lg = (Bmax.y - Bmin.y) + 0.2f`, and if isGang `Lg += 0.5f`.
  4. Fill (0x461CAA-0x461D2E): `rem = len; n = 0`; for i = 0..4: `side[i] = (model != 433 && !isGang) ? (_rand() & 1) : 0`;
     size = side[i] ? W : Lg; `if (rem < size) break` (fcomp + `test ah,1`; NaN breaks); `rem -= size; n++; if (n == 5) break`.
  5. `gap = rem / (float)(n + 1)` (fild + fdivr, 0x461D39-0x461D49); `s = gap` (L+0x18). If n <= 0 go to step 8 (the x87 copy is
     popped at 0x4624DF). Else `halfLg = Lg * 0.5f` (L+0x1C), `adv = halfLg + gap` (L+0x14).
  6. Per car j = 0..n-1 (0x461D72-0x4624D5), `up = 0` (bl):
     a. If side[j]: `s = W * 0.5f + s`; `M.fwd = P`, `M.right = (P.y, -P.x, 0)`, `M.up = (0,0,1)`; `r = _rand() & 0xFF`;
        `ang = (r - 128.0f) * (isGang ? 0.004f : 0.002f)` (0x461DF3-0x461E28).
        Else: `s = halfLg + s`; if isGang `up = ((_rand() & 0xFF) < 64)` (0x461E43-0x461E50); `M.fwd = V`; if up: `M.right =
        (0,0,1)`, `M.up = (-V.y, V.x, 0)` else `M.right = (V.y, -V.x, 0)`, `M.up = (0,0,1)`; if `_rand() & 1`:
        `up ? M.RotateY(pi) : M.RotateZ(pi)` (0x461EEC-0x461F09); `r = _rand() & 0xFF`; `ang = (r - 128.0f) * (isGang ? 0.006f :
        0.003f)`. Both: `M.RotateZ(ang)` (0x461F53).
     b. `M.SetTranslateOnly(s*V.x + a.x, a.y + s*V.y, a.z + s*V.z)` (0x461F58-0x461FC0). `M.pos.z = (M.pos.z - (up ? Bmin.x :
        Bmin.z)) + 0.3f` (0x461FC5-0x461FF2). Then `s = adv + s` (0x461FFA-0x46200A) - also for side cars, so a side car
        advances W/2 + halfLg + gap (asymmetric, original behaviour).
     c. `C1 = Multiply3x3(Bmin, M) + M.pos`, `C2 = Multiply3x3(Bmax, M) + M.pos` (each component `out + pos`, 0x46200E-0x4620CA);
        if `!ClearSpaceForRoadBlockObject(C1, C2)` skip this car (0x4620EB).
     d. Burnt wreck instead of a car if isGang and (`up` or `(_rand() & 0xFF) < 64`) (0x4620F1-0x46210A): `o = new CObject(
        MI_ROADBLOCKFUCKEDCAR1, true)`; `*o->m_matrix = M`; position copied again from M.pos (into the matrix, or placement if no
        matrix); `o->SetIsStatic(false)`; `++nNoTempObjects`; `o->m_nObjectType = 3`; `o->m_nRemovalTime = now + 600000`
        (0x927C0); `CWorld::Add(o)`; `StartFire(o, null, 2.8f, 1, 60000, 2)`; if `o->m_pFire (+0x164)` set its `+0x21 = 0x5C`.
        Next car.
     e. Car (0x4621EF-0x4624BC): `v = new CAutomobile(model, 1, 1)`; status = 4 (STATUS_ABANDONED, `(st & 7) | 0x20`);
        `M.pos.z = (v->GetHeightAboveRoad() - 0.6f) + M.pos.z`; `*v->m_matrix = M`; `v->PlaceOnRoadProperly()`;
        `v->SetIsStatic(false)`; update the RW matrix if `m_pRwObject` (+0x18); `m_nDoorLock = 1` (CARLOCK_UNLOCKED);
        `JoinCarWithRoadSystem(v)`; flags +0x42C |= 0x40, +0x428 &= 0xE7; mission/temp action/lanes/cruise speed = 0, autopilot
        speed dword = 0; `up = 0`.
        If isGang: `k = _ftol((_rand() & 0xFFFF) * (1/32768) * 4.0f)`; `k == 0` -> `SetTotalDamage(true)`; else
        `BlowUpCarCutSceneNoExtras(1,1,1,1)`, `m_nTimeWhenBlowedUp += 1000000`, `up = 1`. Else: if `UsesSiren()` and
        `_rand() & 1` set bSirenOrAlarm.
        If `!(v->m_matrix->up.z > 0.94f)` (0x462363, `test ah,0x41`; NaN deletes): delete v (vtbl[0](1)) and go to the next car.
        Else: `SetClumpAlpha(clump, 0)`; `CWorld::Add(v)`; bCreateRoadBlockPeds = 1; `m_nTimeTillWeNeedThisCar = now + 7000`;
        `m_nNumPedsForRoadBlock = (n > 3) ? 1 : 2` (0x4623A2). If side[j]: `m_nPedsPositionForRoadBlock = 2`; open door 2
        (`idx = GetCarNodeIndexFromDoor(2)`, only if `m_aCarNodes[idx]` (+0x648) != 0: `OpenDoor(0, idx, 2, 1.0f, 1)`), and if
        `m_nNumPedsForRoadBlock > 1` door 3 the same way. Else: `t = ((d.y*right.y + d.z*right.z) + d.x*right.x)` with `d = v.pos
        - FindPlayerCoors(-1)` and right = v matrix +0..+8 (0x462446-0x462471); `m_nPedsPositionForRoadBlock = (t < 0) ? 0 : 1`
        (equal/NaN -> 1). Finally if `up`: `StartFire(v, null, 2.8f, 1, 60000, 2)` and if `v->m_pFire (+0x490)` its `+0x21 = 0x5C`.
  7. (end of car loop)
  8. Barriers, only if !isGang (0x4624E1-0x4629A3): `bm = MI_ROADWORKBARRIER1` (u16), Bmin/Bmax from its col model;
     `bw = (Bmax.x - Bmin.x) + 0.5f`; `nb = _ftol(len / bw)`, `if (nb > 8) nb = 8` (signed); `g = (len - nb*bw) / (nb + 1)` (x87
     `fild nb; fmul bw; fsubr len; fidiv (nb+1)`, L+0x1C). `CObject::DeleteAllTempObjectsInArea(mid, len * 0.5f)` (before the
     loop, even when nb <= 0). For j = 0 .. nb-1:
     a. `s = (j + 0.5f) * bw + (j + 1) * g` (0x4625E0-0x462603). `M.SetUnity(); M.SetTranslate(0,0,0)`; `M.right = V`,
        `M.fwd = (V.y, -V.x, 0)`, `M.up = (0,0,1)`; `M.RotateZ(((_rand() & 0xFF) - 128.0f) * 0.003f)`.
     b. `M.SetTranslateOnly((s*V.x + a.x) + 5*P.x, (a.y + s*V.y) + 5*P.y, (a.z + s*V.z) + 5*P.z)` (P*5.0f computed first; the
        5*P.x value overwrites mid.x at L+0xD4, mid is not used afterwards). `M.pos.x += (_rand() & 0xF) * 0.05f`;
        `M.pos.y += (_rand() & 0xF) * 0.05f` (in this order, 0x462762-0x4627B2).
     c. `gz = FindGroundZFor3DCoord(M.pos.x, M.pos.y, M.pos.z + 2.0f, &found, null)`; `M.pos.z = gz` (stored even if not found);
        if `!found` skip. `M.pos.z = gz - Bmin.z`.
     d. Corners and `ClearSpaceForRoadBlockObject` exactly as 6c; skip on false.
     e. `o = new CObject(bm, true)`; `*o->m_matrix = M`; position copy; `++nNoTempObjects`; type 3; `m_nRemovalTime = now + 600000`;
        `CWorld::Add(o)` (no SetIsStatic call here).
  9. Exit: local CMatrix destructor 0x59ACD0 (also on the two early returns of step 2).
- Edge cases / original bugs: no null checks after `new` (CObject at 0x462142 / 0x462925, CAutomobile at 0x462220): a failed
  allocation dereferences null. The cube corners use the TRANSPOSED rotation (0x59C810), not M * v. GetHeightAboveRoad is called
  before the vehicle receives M. Up to 5 cars, up to 8 barriers. The `up` (upside-down) gang car is never spawned as a vehicle,
  always as a burning wreck object.
- gta-reversed: `plugin::Call<0x4619C0>` only (RoadBlocks.cpp:104). Open questions: none.
