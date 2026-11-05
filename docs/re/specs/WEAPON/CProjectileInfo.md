# CProjectileInfo (WEAPON) - full specs

Data: `CProjectileInfo::ms_apProjectile` 0xC89110 (CProjectile*[32], registered entity references), `gaProjectileInfo`
0xC891A8 (CProjectileInfo[32], stride 0x24): +0x00 m_nWeaponType u32, +0x04 m_pCreator, +0x08 m_pVictim, +0x0C m_nDestroyTime u32,
+0x10 m_bActive u8, +0x14 m_vecLastPosn, +0x20 m_pFxSystem. Object-pool handle of an object `o` (used for radar blips):
`idx = (int32)(o - CPools::ms_pObjectPool->objects) / 0x19C` (signed, via magic 0x13E22CBD, `sar 5`), `handle = (idx << 8) + byteMap[idx]`.
CExplosion::AddExplosion 0x736A50 argument order: (victim, creator, eExplosionType, CVector pos (by value), u32 lifetime,
u8 makeSound, f32 camShake, u8 invisible).

## 0x737B40 CProjectileInfo::Initialise  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void ()`, plain `ret`. Preserves EDI (pushed/popped); clobbers EAX/ECX/EDX. Note `lea ebx,[ebx]`
  at 0x737B5A is padding (EBX unchanged).
- Callers: CWeapon::InitialiseWeapons 0x73A300.
- Globals written: `CProjectileInfo::ms_apProjectile` 0xC89110 (CProjectile*[32]), `gaProjectileInfo` 0xC891A8
  (CProjectileInfo[32], stride 0x24).
- Behaviour: `rep stosd` 32 dwords of 0 at 0xC89110. Then for i = 0..31 (pointer loop, edx = &info[i]+4 from
  0xC891AC while edx < 0xC8962C): `+0x00 m_nWeaponType = 16` (WEAPON_GRENADE), `+0x04 m_pCreator = 0`,
  `+0x0C m_nDestroyTime = 0`, `+0x10 m_bActive (u8) = 0`, `+0x20 m_pFxSystem = 0`. `+0x08 m_pVictim` and
  `+0x14..+0x1F m_vecLastPosn` are NOT touched.
- gta-reversed: `.Reversed = false` (no code used). Open questions: none.

## 0x737B80 CProjectileInfo::RemoveFXSystem  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void (bool instantly [esp+4])` (ECX = CProjectileInfo*), `ret 4`. No catalogued callers (inlined
  everywhere it is used).
- Behaviour: `fx = this->m_pFxSystem (+0x20)`; if null return. If `instantly`: `g_fxMan.DestroyFxSystem(fx)` (0x4A9810,
  thiscall ECX = 0xA9AE80, arg fx) else `fx->Kill()` (0x4AA3F0, thiscall ECX = fx); then `+0x20 = 0`.
- gta-reversed: `plugin::CallMethod`. Open questions: none.

## 0x737BC0 CProjectileInfo::Shutdown  (catalogue: specified/REA, port level 5)
- Convention: `__cdecl void ()`, plain `ret`. Caller: CWeapon::ShutdownWeapons 0x73A330.
- Behaviour: for i = 0..31 (pointer 0xC891C8 = `&gaProjectileInfo[0].m_pFxSystem`, step 0x24, while < 0xC89648): if
  `fx != 0`: `g_fxMan.DestroyFxSystem(fx)` (0x4A9810), `fx = 0`. Nothing else (m_bActive and ms_apProjectile untouched).
- gta-reversed: `plugin::Call`. Open questions: none.

## 0x737BF0 CProjectileInfo::GetProjectileInfo  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl CProjectileInfo* (int32 id [esp+4])`, plain `ret`. Returns `0xC891A8 + id*0x24`
  (`&gaProjectileInfo[id]`, no bounds check, signed id).
- gta-reversed: `.Reversed = false`. Open questions: none.

## 0x737C00 CProjectileInfo::RemoveNotAdd  (catalogue: specified/REA, port level 18)
- Convention: `__cdecl void (CEntity* creator [esp+4], eWeaponType type [+8], CVector pos [+0xC..+0x14] by value)`, plain `ret`.
- Callers: CWeapon::FireProjectile 0x741360 (2 sites). Callee: CExplosion::AddExplosion 0x736A50 (cdecl, args (victim, creator,
  eExplosionType, CVector pos, u32 lifetime, u8 makeSound, f32 camShake, u8 invisible)).
- Behaviour: explosion type by weapon type: 16 (GRENADE) or 39 (SATCHEL) -> 0; 18 (MOLOTOV) -> 1; 19 (ROCKET) or 20 (ROCKET_HS) -> 2;
  any other type -> return without doing anything. Then `AddExplosion(null, creator, expType, pos, 0, 1, -1.0f, 0)`.
- gta-reversed: `plugin::Call`. Open questions: none.

## 0x737C80 CProjectileInfo::AddProjectile  (catalogue: specified/REA, port level 16)
- Convention: `__cdecl bool (CEntity* creator [esp+4], eWeaponType type [+8], CVector origin [+0xC..+0x14] (by value),
  float force [+0x18], CVector* dir [+0x1C], CEntity* target [+0x20])`, AL result, plain `ret`. SEH frame (local CMatrix,
  new-expression cleanups); the entry `push -1; jmp 0x401C3D` is a protector stub that computes and pushes the SEH handler
  address (ror of a dword) - no behavioural effect. Frame base S = ESP after the 4 register pushes (args at S+0xC4..).
- Callers (7): CCarCtrl::PossiblyFireHSMissile 0x429600, CCarCtrl::FireHeliRocketsAtTarget 0x42B270, CObject::ProcessSamSiteBehaviour
  0x5A07D0, CVehicle::FireHeatSeakingMissile 0x6E05C0, CVehicle::PossiblyDropFreeFallBombForPlayer 0x6E07E0,
  CVehicle::GetPlaneWeaponFiringStatus 0x6E3440, CWeapon::FireProjectile 0x741360.
- Callees: CMatrix::SetTranslate 0x59AF40, CMatrix::RotateZ 0x59B390, CMatrix::operator= 0x59BBC0, CMatrix::~CMatrix 0x59ACD0,
  CPlaceable::GetMatrix 0x411990 (allocates if needed), CGeneral::LimitRadianAngle 0x53CB50 (cdecl f32 -> ST0),
  CVector::Magnitude 0x4082C0, CPed::IsPlayer 0x5DF8F0, CVector::CrossProduct (out) 0x59C730 `cdecl(out, a, b)` = a x b,
  Multiply3x3 0x59C790 `cdecl(out, const CMatrix&, const CVector&)`, CStreaming::RequestModel 0x4087E0, CObject::operator new
  0x5A1EE0 (size 0x17C), CProjectile::CProjectile 0x5A4030 (thiscall(int modelId)), CWeaponInfo::GetWeaponInfo 0x743C60,
  CColModel::AllocateData 0x40F870, CColSphere::Set 0x40FD10, CMemoryMgr::Malloc 0x72F420, CEntity::RegisterReference 0x571B70
  (thiscall(CEntity** slot)), CPlaceable::SetMatrix 0x54F610, CWorld::Add 0x563220, CEntity::GetModellingMatrix 0x46A2D0,
  FxManager_c::CreateFxSystem 0x4A9BE0, FxSystem_c::Play 0x4AA2F0, CRadar::SetEntityBlip 0x5839A0, ChangeBlipScale 0x583CC0,
  ChangeBlipColour 0x583AB0, CWorld::FindPlayerPed 0x56E210, FindPlayerVehicle 0x56E0D0, CAudioEngine::ReportWeaponEvent 0x506F40.
- Locals: `M` CMatrix at S+0x3C (pos at S+0x6C; attach fields +0x40/+0x44 zeroed at entry, no other init), `vel` S+0x1C,
  `speed` S+0x10, `h` (heading) S+0x14, `destroy` S+0x34, `elast` S+0x38 = 0.75f, `grav` u8 S+0x1B = 1, `kind` u8 S+0x1A = 0.
  `now` = CTimer::m_snTimeInMilliseconds. Heading helper `H(creator)` = creator matrix ? `atan2(-m[+0x10], m[+0x14])`
  (fpatan of -forward.x over forward.y) : placement heading (+0x10). Speed helper `SP = (force == 0.0f) ? 0.0f : force*0.22f + 0.15f`
  (0x8717AC, 0x858FCC; NaN takes the formula).
- Behaviour:
  1. Setup by type (switch `type-16`, byte table 0x7387F0 / jump table 0x7387D4):
     - 17 TEARGAS (0x737CED): `destroy = now + 20000`; `speed = SP`; `h = H`; `M.SetTranslate(0,0,0); M.RotateZ(h); M.pos += origin`
       (each f32 add); `kind = 5; elast = 0.5f`; vel:
       x = `sin(h)*speed*-1.0f`, y = `cos(h)*speed`, z = `((force + 1.0f) * 0.4f [0x858EE8]) * speed`.
     - 16 GRENADE, 39 SATCHEL (0x737E02): `destroy = now + 2000`; `speed = SP`, if type 39 `speed *= 0.5f`; `h = H`; if creator is a
       vehicle (type 2): `h = LimitRadianAngle(h + 3.1415927f)`; M as above; vel as TEARGAS; then if `creator->m_nModelIndex
       (+0x22) == 405` (flags of the compare at 0x737EBB survive to 0x737F41): `vel += creator->m_vecMoveSpeed (+0x44)` (per
       component, stored f32); `kind = 5; elast = 0.5f`; type 39: `elast = 0.03f` (0x3CF5C28F).
     - 18 MOLOTOV (0x737F87): `destroy = now + 2000`; `speed = force*0.22f + 0.15f` (no zero test), if `speed < 0.2f` `speed = 0.2f`;
       `h = H`; M as above; vel.x/y as TEARGAS, `vel.z = (force * 0.2f + 0.4f) * speed`. kind 0, elast 0.75f.
     - 19 ROCKET / 20 ROCKET_HS (0x738086): type 19: `destroy = now + 3000, speed = 0.4f`; else `destroy = now + 10000, speed = 0.2f`.
       Orientation: creator vehicle -> `M = *creator->GetMatrix(); M.pos = origin; speed += |creator->m_vecMoveSpeed|`;
       else creator ped and `IsPlayer()` -> with cam = `TheCamera.m_aCams[m_nActiveCam]`, `A = cam+0x190` (0xB6F32C),
       `U = cam+0x1B4` (0xB6F350): `M.right = U x A`, `M.forward = A`, `M.up = U`, `M.pos = origin`;
       else if `dir`: `M.forward = *dir`, `M.right = creator->GetMatrix()->right`, `M.up = M.right x M.forward`, `M.pos = origin`;
       else `M = *creator->GetMatrix()` (pos NOT replaced by origin). Then `vel = Multiply3x3(M, (0, speed, 0))`; `grav = 0`.
     - 21 FREEFALL_BOMB (0x7382CF): `destroy = now + 2000000`. 58 (MI_FLARE projectile, 0x738303): `RequestModel(MI_FLARE
       [u16 0x8CD75C], 0)`, `destroy = now + 10000`. Both then: `vel = creator->m_vecMoveSpeed` if creator type is 2, 3 or 4,
       else (0,0,0); `M = *creator->GetMatrix(); M.pos = origin`.
     - all other types: no setup (M, vel, destroy uninitialised).
  2. Slot: `i` = first index with `!gaProjectileInfo[i].m_bActive` (scan reads one byte past entry 31, at 0xC89638, harmless);
     none -> `~M`, return false.
  3. Object (second switch, tables 0x738830 / 0x73881C): types 16, 17, 18, 39: `p = new CProjectile(GetWeaponInfo(type,1)->m_nModelId1 (+0xC))`
     (null if new fails); `ms_apProjectile[i] = p`; if p: `cm = ms_modelInfoPtrs[p->m_nModelIndex]->m_pColModel`; if
     `cm->m_pColData (+0x2C) == 0`: `cm->AllocateData(1, 0, 0, 0, 0, 0)` and `spheres[0].Set(cm->boundSphere.radius (+0x24) * 0.75f,
     cm->boundSphere.center (+0x18), 0x38, 0, 0xFF)`; else if `data->numSpheres (u16 +0) == 0 && data->spheres (+8) == 0`:
     `numSpheres = 1; spheres = CMemoryMgr::Malloc(0x14, 0)` and the same Set. Types 19, 20, 21: `p = new CProjectile(wi->m_nModelId1)`
     (no colmodel change). Type 58: `p = new CProjectile(MI_FLARE)`, if p: `p->+0x98 = 0.9f`. Other types: no new object,
     `ms_apProjectile[i]` keeps its old value (normally null: the slot is a registered reference cleared when the old
     projectile is destroyed).
     If `ms_apProjectile[i] == 0`: `~M`, return false.
  4. Commit (0x73858F), `info = &gaProjectileInfo[i]`, `p = ms_apProjectile[i]`:
     `info->type = type; info->m_pCreator = creator; creator->RegisterReference(&info->m_pCreator)` (no null check);
     `p->SetMatrix(M)`; `p->m_vecMoveSpeed = vel`; `p->+0x40` bit 1 (bApplyGravity) = grav; `info->m_nDestroyTime = destroy`;
     `p->+0x9C = elast`; if `kind == 5`: `p->+0x160 = 0xBB4BD0` (&CObjectData::ms_aObjectInfo + 0x140); `info->m_pVictim = target`,
     if target `target->RegisterReference(&info->m_pVictim)`; `info->m_bActive = 1`; `CWorld::Add(p)`;
     `p->RegisterReference(&ms_apProjectile[i])`; `info->m_vecLastPosn = p->GetPosition()`.
     If type == 17: `mm = p->GetModellingMatrix()`; if mm: `fx = g_fxMan.CreateFxSystem("teargasAD" (0x872BF0), &(0,0,0), mm, false)`,
     `info->m_pFxSystem = fx`, if fx `fx->Play()`.
     `p->+0x40 |= 0x10000000`; `p->+0x128 = creator`; if creator and creator type in {2,3,4} and `creator->+0x128 == 0`:
     `creator->+0x128 = creator`.
     If type == 20: `b = CRadar::SetEntityBlip(3, objectPoolHandle(p), 0xFF0000FF, 2, "CODEPRJ" (0x872BE8))`;
     `ChangeBlipScale(b, 1)`; `ChangeBlipColour(b, (creator == FindPlayerPed(-1) || creator == FindPlayerVehicle(-1, false)) ? 0xFFFFFFFF : 0xFF0000FF)`.
     `AudioEngine (0xB6BC90).ReportWeaponEvent(0x94, type, p)`; `~M`; return true.
- Edge cases: the creator matrix may be allocated by GetMatrix. For unknown types the commit runs only if `ms_apProjectile[i]` is still
  non-null (would re-commit that object with uninitialised matrix/velocity; original behaviour, not expected to happen).
- gta-reversed: `plugin::CallAndReturn`. Open questions: (1) 0x73860C: meaning of `+0x160 = 0xBB4BD0` (gr CObject::m_pObjectInfo
  pointing into ms_aObjectInfo) - value is exact, name unverified. (2) 0x7386E8/0x738710: field +0x128 on CPhysical (gr name
  not checked); the writes are exact.

## 0x738860 CProjectileInfo::RemoveDetonatorProjectiles  (catalogue: specified/REA, port level 18)
- Convention: `__cdecl void ()`, plain `ret`. Caller: CWorld::UseDetonator 0x5660B0 (tail call).
- Behaviour: for i = 0..31 (ascending): if `gaProjectileInfo[i].m_bActive && m_nWeaponType == 39` (REMOTE_SATCHEL_CHARGE):
  `p = ms_apProjectile[i]`; `AddExplosion(null, info.m_pCreator, 0 /*GRENADE*/, p->GetPosition(), 0, 1, -1.0f, 0)`;
  `info.m_bActive = 0`; if `info.m_pFxSystem`: `Kill()` (0x4AA3F0) and `m_pFxSystem = 0`; `p->m_nFlags (+0x1C) |= 0x800`
  (CEntity bRemoveFromWorld bit). The object is not removed or deleted here; ms_apProjectile[i] is unchanged.
- gta-reversed: `plugin::Call`. Open questions: none.

## 0x7388F0 CProjectileInfo::RemoveProjectile  (catalogue: specified/REA, port level 18)
- Convention: `__cdecl void (CProjectileInfo* info [esp+4], CProjectile* obj [+8])`, plain `ret`.
- Callers: CProjectileInfo::Update 0x738B20 (2 sites).
- Behaviour: `pos = obj->GetPosition()` (matrix or placement), `c = info->m_pCreator`, explosion by `info->m_nWeaponType`
  (all calls `AddExplosion(null, X, type, pos, 0, 1, -1.0f, 0)`):
  - 16 GRENADE and 21 FREEFALL_BOMB: `(X = c, type 0)`.
  - 18 MOLOTOV: `(c, 1)`, then `AudioEngine (0xB6BC90).ReportObjectDestruction(obj)` (0x506ED0).
  - 19 ROCKET: if `c` is a vehicle (type 2): `X = c->m_pDriver (+0x460)` (may be null), else `X = c`; type 2.
  - 20 ROCKET_HS: `X = c`; type 2 if `c == FindPlayerPed(-1)` else 3 (WEAK_ROCKET).
  - other types: no explosion.
  Then: `info->m_bActive = 0`; if `info->m_pFxSystem`: `Kill()`, `= 0`; `CRadar::ClearBlipForEntity(3, objectPoolHandle(obj))`
  (handle as in RemoveAllProjectiles 0x7399B0); `CWorld::Remove(obj)`; if obj: `obj->vtbl[0](1)` (delete).
  ms_apProjectile is not touched here (its CEntity reference is cleared by the object destructor via RegisterReference).
- gta-reversed: `plugin::Call`. Open questions: none.

## 0x738B20 CProjectileInfo::Update  (catalogue: specified/REA, port level 19)
- Convention: `__cdecl void ()`, plain `ret`. Locals 0x134. Caller: CWeapon::UpdateWeapons 0x73A360.
- Callees: FxSystem_c::Kill 0x4AA3F0, CPed::IsPointerValid 0x5E4220, CGeneral::GetRandomNumberInRange(int) 0x407180, CWorld::SetPedsChoking
  0x565800 `(x, y, z, radius, creator)`, FxPrtMult_c ctor 0x4AB290, `_ftol2`, rand, CVector::Normalise 0x59C910, CrossProduct 0x59C730
  `(out, a, b)` = a x b, FxSystem_c::AddParticle 0x4AA440, CPed::IsPlayer, CPed::GetWeaponSlot 0x5DF200, RemoveProjectile 0x7388F0,
  CWorld::GetIsLineOfSightClear 0x56A490 `(from, to, buildings, vehicles, peds, objects, dummies, seeThrough, ignoreSomeObjects)`,
  FindPlayerPed/FindPlayerVehicle, CAudioEngine::ReportFrontendAudioEvent 0x506EA0, CWeapon::EvaluateTargetForHeatSeekingMissile 0x73E560
  `(CEntity*, CVector* point, CVector* dir, float, bool, CEntity*) -> ST0`, CRT pow 0x822130 (ST1^ST0), CPhysical::AttachEntityToEntity
  (quat overload) 0x54D690.
- Globals: CReplay::Mode 0xA43088 (return if == 1), ms_apProjectile / gaProjectileInfo, CTimer::ms_fTimeStep (`ts`) 0xB7CB5C, `now`,
  CWorld::pIgnoreEntity 0xB7CD68, g_fx particle system 0xA9AE20, AudioEngine 0xB6BC90, GAME_GRAVITY 0x863984 (0.008f),
  .data f32 0x8D6104 = 0.95, 0x8D6108 = 0.15. Constants: 0x858B1C 0.1, 0x858C28 0.05, 0x858C84 0.25, 0x858CEC 0.04, 0x859018 0.08,
  0x858CE8 1.5, 0x872C00 9.9, 0x858628 100.0, 0x858B40 50.0, 0x872BFC 0.0117, 0x858C98 0.8, 0x858F08 1.2, 0x858CA0 2.0.
- Behaviour: if `CReplay::Mode == 1` return. For i = 0..31 (`o = ms_apProjectile[i]` read first, `info = &gaProjectileInfo[i]`),
  skip unless `info->m_bActive`:
  1. If `o->+0x40 & 0x100` (bSubmergedInWater) and `info->m_pFxSystem`: Kill, null.
  2. If creator is a ped and `!IsPointerValid()`: `info->m_pCreator = 0` (plain store, no reference cleanup).
  3. Types 39, 16, 17: if `o->+0x9C (elasticity) > 0.1f` and `|mv.x| < 0.05f && |mv.y| < 0.05f && |mv.z| < 0.05f` (mv = +0x44):
     `+0x9C = 0.03f`. Type 17 additionally: if `now > destroy - 17500` (unsigned) and `GetRandomNumberInRange(0, 100) < 10`:
     `SetPedsChoking(pos.x, pos.y, pos.z, 6.0f, creator)`.
  4. Types 19, 20 (smoke trail, 0x738C73-0x738F1C): `mult = FxPrtMult_c(0.3, 0.3, 0.3, 0.3, 0.5, 1.0, 0.08)`;
     `s = ts * mv` (three f32 stores); `n = max(_ftol2(sqrt((s.x^2 + s.z^2) + s.y^2)), 1)` (signed); for j = 0..n-1, with
     `t = rand()*3.0518509e-05f`: `mult.r = mult.g = mult.b = t*0.25f + 0.25f` (rand 1); `mult[+0x18] = t*0.04f + 0.08f` (rand 2);
     `f = 1.0f - (float)j / (float)n`; `p = pos - s*f` (per component, f32); `v = (2t-1, 2t-1, 2t-1)` (rands 3, 4, 5 for x, y, z),
     `v.Normalise()`; `w = mv` normalised; `vel = (w x v) * 1.5f`; `g_fx[0xA9AE20]->AddParticle(&p, &vel, 0.0f, &mult, -1.0f, 1.2f, 0.6f, 0)`.
  5. Expiry: if `now > info->m_nDestroyTime && m_nDestroyTime != 0` (unsigned): type 39: only if creator is a ped and IsPlayer():
     `slot = GetWeaponSlot(40)`; unless `(creator->m_aWeapons[slot].m_Type (+0x5A0 + 0x1C*slot) == 40 && .m_TotalAmmo (+0x5AC + 0x1C*slot) != 0)`
     set `m_nDestroyTime = 0`; (non-player creators: nothing). Other types: `RemoveProjectile(info, o)`. Then go to step 7.
  6. Not expired, by type:
     - 19 ROCKET: `g = ts * 0.008f`; `mv += o.forward (matrix +0x10) * g`; if `|mv| > 9.9f` (strict): `mv *= 9.9f / |mv|`. -> collision test (6c).
     - 20 ROCKET_HS (0x7391B6): if `info->m_pVictim` (+8) is null go to 6c. If victim == FindPlayerVehicle(-1, false):
       `ReportFrontendAudioEvent(0x65, 0.0f, 1.0f)`. `F = o.forward`, `Q = o.pos + F`; `r0 = Evaluate(victim, &Q, &F, 1.2f, 1, 0)`;
       best flare: over all 32 infos with type 0x3A (flare) and active: `r = Evaluate(flareObj, &Q, &F, 1.2f, 1, 0)`, keep it if
       `!(r < best)` (best starts 0.0f; ties -> later one). Target `T = bestFlare` if bestFlare and `best > r0`, else victim.
       `plane = (T is a vehicle) && (creator == FindPlayerPed(-1) || creator == FindPlayerVehicle(-1, false)) && T->+0x594 == 4`.
       `A = plane ? o.pos : o.pos + mv*100.0f`; `d = |o.pos - T.pos|` (stored); `k = plane ? min(d, 1.5f) : min(d, 50.0f)`
       (`d < c ? d : c`); `L = T.pos + T.mv (+0x44) * k`; `dir = normalise(L - A)`; `cur = normalise(mv)`;
       `dot = (dir.z*cur.z + dir.y*cur.y) + dir.x*cur.x`; if `dot < 0`: `dir -= cur * dot`; `dir.Normalise()`.
       `rate = (creator == player ped or player vehicle) ? 0.0117f : 0.009f`; if `|mv| > 0.8f`: `rate *= 1.2f`;
       `drag = 1.0f`; if plane: `drag = pow(0.95f, ts)` (f32 stored), `rate = 0.15f`. `mv *= drag`; `mv += dir * (rate * ts)`;
       if `|mv| > 9.9f`: `mv *= 9.9f / |mv|`; `o.matrix.forward = cur` (the normalised pre-update velocity). -> 6c.
     - 0x3A FLARE (0x7390C8): `o->+0x1C &= ~1`; `pIgnoreEntity = creator`; `clear = GetIsLineOfSightClear(info.lastPos, o.pos,
       1,1,1,1, 0,0,0)`; `o->+0x1C |= 1`; `pIgnoreEntity = 0`; if not clear: `mv = 0`, `o.pos = info.lastPos` (matrix or placement).
     - 18 MOLOTOV, 21 FREEFALL_BOMB (0x739745): `pIgnoreEntity = creator; o->+0x1C &= ~1`; unless (creator and
       `(dx*dx + dz*dz) + dy*dy < 2.0f` with d = lastPos - creator.pos): if `o->+0x40 & 0x200` or not
       `GetIsLineOfSightClear(lastPos, o.pos, 1,1,1,1, 0,0,0)` -> `RemoveProjectile(info, o)`. Then `pIgnoreEntity = 0; o->+0x1C |= 1`.
     - 39 SATCHEL (0x7396F2): if `o->+0xD8 > 0.0f` and `o->+0xDC != 0` and `o->+0xFC (attachedTo) == 0`:
       `o->AttachEntityToEntity(o->+0xDC, null, null)`, `o->+0x1C &= ~1`.
     - 16, 17 and others: nothing.
     6c. Rocket collision test (0x7396BB): if `o->+0x40 & 0x200` goto HIT; else `pIgnoreEntity = creator; o->+0x1C &= ~1`;
       `clear = GetIsLineOfSightClear(lastPos, o.pos, 1,1,1,1, 0,0,0)`; `pIgnoreEntity = 0; o->+0x1C |= 1; o->+0x128 = creator`;
       if clear -> step 7. HIT (0x739090): if `o->+0xB9 != 0 && (e = o->+0xBC) != 0` and (`e == creator` or `e->m_nModelIndex == 0x159`)
       -> no removal; otherwise `RemoveProjectile(info, o)`.
  7. `info->m_vecLastPosn = o->GetPosition()` (always, also after RemoveProjectile, which deleted o - the original reads the
     freed object here; a port must read the position before removal or reproduce with the same value).
- gta-reversed: `plugin::Call`. Open questions: (1) 0x739812: after RemoveProjectile the original reads `o`'s position from freed
  pool memory (pool slots are not cleared on delete, so the value is the last position); port: use the position before removal.
  (2) 0x739090: meaning of `o->+0xB9` / `+0xBC` (CPhysical collision record count / last collided entity in gr) not verified;
  model 0x159 (345) unnamed here.

## 0x7399B0 CProjectileInfo::RemoveAllProjectiles  (catalogue: specified/REA, port level 5)
- Convention: `__cdecl void ()`, plain `ret`; preserves EBX/EBP/ESI/EDI. The body continues through a .text stub
  (0x7399B4 `jmp 0x40391D`: `mov ebx, 0xC89110; jmp 0x7399B9`).
- Callers: CWorld::ClearForRestart 0x564360, CWorld::ClearExcitingStuffFromArea 0x56A0D0, CPlayerInfo::MakePlayerSafe 0x56E870.
- Callees: FxManager_c::DestroyFxSystem 0x4A9810, CRadar::ClearBlipForEntity 0x587C60 (cdecl(eBlipType, int32 handle)),
  CWorld::Remove 0x563280 (cdecl(CEntity*)), virtual slot 0 of the object (deleting destructor, thiscall, arg 1).
- Behaviour: for i = 0..31 (ascending): `obj = ms_apProjectile[i]` (read first); if `gaProjectileInfo[i].m_bActive`:
  1. `m_bActive = 0`;
  2. if `m_pFxSystem`: `g_fxMan.DestroyFxSystem(fx)` (instant), `m_pFxSystem = 0`;
  3. `ClearBlipForEntity(3 /*BLIP_OBJECT*/, handle)` with the object-pool handle
     `idx = (int32)(obj - pool->objects) / 0x19C` (signed division via magic 0x13E22CBD, `sar 5`, round toward zero),
     `handle = (idx << 8) + pool->byteMap[idx]` (pool = `CPools::ms_pObjectPool` 0xB7449C);
  4. `CWorld::Remove(obj)`; 5. if `obj != null`: `obj->vtbl[0](1)` (delete).
  `ms_apProjectile[i]` is not written explicitly here (contrast RemoveIfThisIsAProjectile). AddProjectile registered that slot
  with CEntity::RegisterReference, so the object's destructor is expected to null it (CEntity reference resolution; not
  re-verified here). Port: do not add an explicit store.
- gta-reversed: `plugin::Call`. Open questions: none.

## 0x739A40 CProjectileInfo::RemoveIfThisIsAProjectile  (catalogue: specified/REA, port level 3)
- Convention: `__cdecl bool (CObject* obj [esp+4])`, AL result (upper EAX bits: garbage on the true path, EAX = pointer
  to m_bActive of the last tested entry with AL = 0 on the false path), plain `ret`.
- Callers: CPools::MakeSureSlotInObjectPoolIsEmpty 0x550080.
- Behaviour: find the first i in 0..31 with `ms_apProjectile[i] == obj && gaProjectileInfo[i].m_bActive (+0x10, 0xC891B8 + i*0x24)`;
  none -> return false. Else: `m_bActive = 0`; if `m_pFxSystem` (+0x20): `fx->Kill()` (0x4AA3F0, NOT DestroyFxSystem),
  `m_pFxSystem = 0`; `ClearBlipForEntity(3, handle(ms_apProjectile[i]))` (handle as in 0x7399B0, CObject stride 0x19C);
  `CWorld::Remove(ms_apProjectile[i])`; if `ms_apProjectile[i]` non-null: deleting destructor (vtbl[0], arg 1);
  `ms_apProjectile[i] = 0`; return true. Each use re-reads `ms_apProjectile[i]` from memory.
- gta-reversed: `plugin::CallAndReturn`. Open questions: none.
