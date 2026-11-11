# CAttractorScanner (AI) - RE-SPEC-FINAL group A

## 0x5FE960 CAttractorScanner::sub_5FE960  (catalogue: unknown/REA, port level 0)
Catalogue name: CAttractorScanner::sub_5FE960, proposed: `tUserList::IsPedTypeAllowed(int32 pedType)` (evidence: `this` is a
CScripted2dEffects::ms_userLists entry (0xC3A200 + 0x24*i, caller 0x606452); layout = gr tUserList: +0 m_UserTypes[4], +0x10
m_UserTypesByPedType[4], +0x20 m_bUseList).
- Convention: `__thiscall bool (tUserList* this, int32 pedType [esp+4])`, `ret 4`, result in AL.
- Behaviour: if `this->m_bUseList` (+0x20 byte) == 0 return 1 (0x5FE960-0x5FE969). Else for k = 0..3 (ascending): if `m_UserTypes[k] == -2`
  (dword compare with -2, 0x5FE975) and `m_UserTypesByPedType[k] == pedType` (+0x10 + 4k, 0x5FE97B) return 1. Return 0.
- Note: -2 in m_UserTypes marks "match by ped type" for that slot. Only called after the caller already checked the model list.
- gta-reversed: absent. Open questions: none.

## 0x5FFFD0 CAttractorScanner::sub_5FFFD0  (catalogue: unknown/REA, port level 10)
Catalogue name: CAttractorScanner::sub_5FFFD0, proposed: `CAttractorScanner::AddEffect(C2dEffectPedAttractor* fx, CEntity* entity, CPed* ped)`
(evidence: it updates the per-attractor-type best tables `+0x18/+0x40/+0x68` of `this`, callers 0x6034B0 (per-entity 2dfx loop) and
0x6060A0 (scripted effects loop, entity = 0)).
- Convention: `__thiscall void (CAttractorScanner* this, C2dEffect* fx [esp+4], CEntity* entity [esp+8], CPed* ped [esp+0xC])`, `ret 0xC`.
  SEH frame (handler 0x83E708), locals: CVector fxPos (E-0x60), CMatrix (E-0x54, 0x48 bytes), SEH state E-4; the `fx` argument slot [E+4] is
  reused as the f32 `distSq` local (0x600092) after `fx` is cached in EDI (E = esp at entry).
- Callees: CEntity::TransformFromObjectSpace(out, offset) 0x5334F0 (thiscall, ret 8); CScripted2dEffects::GetIndex 0x6F9F60 (cdecl, 1 arg);
  CPedAttractorManager::GetPedAttractorManager 0x5EE190; CPedAttractorManager::HasEmptySlot 0x5EBB00 (thiscall (fx, entity));
  CPlaceable::GetMatrix 0x411990; CMatrix::operator= 0x59BBC0; CMatrix::SetScale(float) 0x59AED0; CPedAttractorManager::IsApproachable 0x5EA220
  (cdecl (fx, const CMatrix&, int 0, CPed*)); CMatrix::~CMatrix 0x59ACD0.
- Globals: CWeather::Rain [0xC81324] (f32, R); 0.2f [0x858CC4]=0.2f; 0.0f [0x858B50]=0.0f; CScripted2dEffects::ms_radii 0xC39EE0 (f32[64], R).
- Fields: fx+0 position (CVector, object space), fx+0x34 attractor type (u8, `movzx`, 0x600000; ePedAttractorType). this+0x18 CEntity*[10] best
  entity per type, this+0x40 C2dEffect*[10] best effect per type, this+0x68 f32[10] best distance^2 per type (gr: field_18/field_40/field_68).
  ped+0x14 m_matrix (CMatrix*), ped+4 placement position (used when no matrix), matrix+0x30 position.
- Behaviour:
  1. Rain gate (0x5FFFD6-0x600027): `t = fx->type`. `fld Rain; fcomp 0.2f; fnstsw; test ah,1`: C0 set (Rain < 0.2, or NaN/unordered) ->
     if `t == 4` (PED_ATTRACTOR_SHELTER) return, else continue. C0 clear (Rain >= 0.2) -> if `t == 4` continue; else Rain is compared
     with 0.2f a second time (0x60000D, re-read from memory) and, C0 clear, returns. Net effect: when raining (>= 0.2) only shelter attractors
     are considered; otherwise (including NaN) shelters are skipped.
  2. Position (0x60002E-0x600057): if `entity != 0`: `entity->TransformFromObjectSpace(&fxPos, &fx->pos)`; else fxPos = fx->pos (3 dword copies).
  3. Distance (0x60005B-0x6000A5): `p = ped->m_matrix ? &ped->m_matrix->pos : &ped->placement.pos`. x87 order: `dx = p.x - fxPos.x`,
     `dy = p.y - fxPos.y`, `dz = p.z - fxPos.z`; `s = dz*dz; s += dy*dy; s += dx*dx` (0x600082-0x600090); `fst` -> f32 distSq into the arg
     slot; the unrounded register value is compared: `fcomp this->dist[t]` (+0x68 + 4*t), `test ah,5; jp return` -> continue only if
     `s < dist[t]` (equal / greater / NaN return). No bound check on `t` (types 0..9 assumed; type 0xFF would index out of the tables).
  4. Scripted radius (0x6000AB-0x6000E7), only if `fx->type == 7` (PED_ATTRACTOR_SCRIPTED; byte re-read at 0x6000AB): `r = ms_radii[GetIndex(fx)]`;
     `fcom 0.0f; test ah,5; jnp` -> if `r < 0.0f` no radius limit. Otherwise (r > 0, r == 0 or NaN): `r2 = r*r`; `fld distSq(f32 slot); fcompp`
     (st0 = distSq vs st1 = r2), `test ah,5; jp return` -> continue only if `distSq < r*r` (so r == 0 or NaN always rejects).
  5. `if (!GetPedAttractorManager()->HasEmptySlot(fx, entity)) return` (0x6000E9-0x6000F9).
  6. Local CMatrix (inline default ctor: +0x40 m_pAttachMatrix = 0, +0x44 m_bOwnsAttachedMatrix = 0, SEH state 0): if `entity` ->
     `mat = *entity->GetMatrix()` (operator=, 0x600119), else `mat.SetScale(1.0f)` (identity, 0x600129).
  7. `if (IsApproachable(fx, mat, 0, ped))` (0x60013B): store in this order `entity[t] = entity` (0x60014B), `dist[t] = distSq` (f32 from the
     slot, 0x60014F), `effect[t] = fx` (0x600153). Then SEH state -1 and `mat.~CMatrix()` (0x600163).
- gta-reversed: absent (CAttractorScanner.cpp only wraps Clear). Spec from the bytes.
- Open questions: none.

## 0x6060A0 CAttractorScanner::ScanForAttractorsInRange  (catalogue: unknown/REA, port level 12)
- Convention: `__thiscall void (CAttractorScanner* this, CPed* ped [esp+4])`, `ret 4`. SEH frame (handler 0x83E9C0). Caller:
  CEventScanner::ScanForEvents 0x607E30 (this = CEventScanner+0x1C). Locals (E = esp at entry): startX [E-0x58], endY [E-0x5C], current y
  [E-0x54], clamped y [E-0x50], two event buffers [E-0x4C] (scripted) and [E-0x2C] (normal), SEH state [E-4]; the `ped` argument slot [E+4] is
  reused first as endX and later as the out-effect pointer. Callees: CTaskManager::GetActiveTask 0x681720; CAttractorScanner::Clear 0x5FFF90;
  floor 0x8219F0 (CRT, double in/out, SSE2 or x87 path, same result); _ftol 0x821B40 (truncation); 0x6034B0 (scan one CPtrList of entities:
  for every object/building 2dfx of type 3 calls 0x5FFFD0); 0x5FFFD0 (proposed CAttractorScanner::AddEffect); 0x5FE960 (tUserList pedType
  check); 0x600180 (pick best effect over all types: thiscall (C2dEffect** outFx, CEntity** outEntity)); 0x603570 (closest eligible ped
  for a scripted effect, cdecl); CEventAttractor ctor 0x4AF350 (thiscall (fx, entity, bool 0), `ret 0xC`); CEventGroup::Add 0x4AB420
  (thiscall (CEvent*, bool), returns pointer/nonzero on success); ~CEventAttractor 0x4AF400.
- Globals: CTimer::m_snTimeInMilliseconds [0xB7CB84]=0 (R, u32); 15.0f [0x86C52C]=15.0f; 0.02f [0x858B38]=0.02f; 60.0f [0x858B34]=60.0f;
  CWorld::ms_aSectors 0xB7D0B8 (CSector[120*120], 8 bytes each); CWorld::ms_aRepeatSectors 0xB992B8 (CRepeatSector[16*16], 12 bytes);
  CScripted2dEffects::ms_activated 0xC3A1A0 (bool[64]), ms_userLists 0xC3A200 (tUserList[64], 0x24 each), ms_effects 0xC3AB00
  (C2dEffect[64], 0x40 each); CEventScriptedAttractor vtable 0x86C9F0.
- Fields: this+0 byte enable flag (gr field_0), this+4 CTaskTimer {+4 start u32, +8 interval i32, +0xC bStarted u8, +0xD bStopped u8},
  this+0x10 m_pEffectInUse (C2dEffect*), this+0x14 entity in use (gr field_14). ped+0x14 matrix, ped+4 placement pos, ped+0x22 model (i16),
  ped+0x46C flags (bit 8 bInVehicle), ped+0x47C m_pIntelligence (TaskMgr at +4, event group at +0x68), ped+0x598 m_nPedType.
- Behaviour:
  1. If `this->field_0 == 0` return (0x6060BB). If `ped->flags & 0x100` (bInVehicle, `test ah,1` 0x6060D2) return.
  2. `t = GetActiveTask(intel+4)`; if `t && t->GetTaskType() (vtbl+0x10) == 0xE9` (233 TASK_COMPLEX_USE_EFFECT): timer.start = now,
     timer.interval = 3000 (0xBB8), timer.bStarted = 1, return (0x6060FB-0x60610E).
  3. Timer (0x606113-0x606175): if `!bStarted` { start = now; interval = 1500 (0x5DC); bStarted = 1 } (bStarted is re-tested afterwards, always
     1). If `bStopped` { start = now; bStopped = 0 }. Out-of-time test with a fresh `now` read (0x606153): `now >= start + interval` (unsigned,
     `cmp; sbb; inc`); if not, return. Else `start = now` (that same read), `interval = 1500`, `bStarted = 1`, then `Clear(this)` 0x5FFF90.
  4. Sector range (0x60617A-0x60636B), each coordinate taken freshly from `ped->m_matrix ? matrix->pos : placement.pos`; per value the
     computation is evaluated once for the test and once more for the use (identical results; floor is called 8 times):
     `sx0 = _ftol(floor((double)((pos.x - 15.0f) * 0.02f + 60.0f)))` (fld x; fsub 15; fmul 0.02; fadd 60; `fstp qword` -> f64 arg);
     `startX = sx0 > 0 ? sx0 : 0` (`jle` -> 0); `startY` the same with pos.y; `endX = (fld 15.0f; fadd pos.x; fmul 0.02; fadd 60)`, floor,
     _ftol, `endX = v < 119 ? v : 119` (`cmp eax,0x77; jge` -> 119); `endY` likewise with pos.y. Note the different operand order for the
     end values (15 + p) vs start (p - 15).
  5. Loop (0x60636B-0x60640C): `for (y = startY; y <= endY; y++) for (x = startX; x <= endX; x++)` (signed compares; if startY > endY or
     startX > endX nothing). Per cell: `cy = max(y,0)` (setle/dec/and idiom), `rIdx = (y & 15) << 4`; `cx = max(x,0)`; both clamped
     `cx < 119 ? cx : 119`, `cy < 119 ? cy : 119` (`jl` keeps); `0x6034B0(this, &ms_aSectors[cy*120 + cx] (buildings list, +0), ped)`;
     then `0x6034B0(this, &ms_aRepeatSectors[(x & 15) + rIdx] + 8 (objects list), ped)` (repeat index uses the unclamped x/y & 15).
  6. Scripted effects (0x606412-0x60647D): `for (i = 0; i < 64; i++)` with `ul = &ms_userLists[i]`, `fx = &ms_effects[i]`: if
     `ms_activated[i]`: if `ul->m_bUseList == 0` -> AddEffect; else if `(i32)(i16)ped->m_nModelIndex` equals any of `ul->m_UserTypes[0..3]`
     (ascending) -> AddEffect; else if `0x5FE960(ul, ped->m_nPedType)` -> AddEffect. AddEffect = `0x5FFFD0(this, fx, 0 /*entity*/, ped)`.
  7. Pick (0x60647F-0x6064B2): `fx = 0; ent = 0; 0x600180(this, &fx, &ent)`. If `fx == 0` return. If `fx == this->m_pEffectInUse && ent ==
     this->+0x14` return (both must match).
  8. Event (0x6064B8-0x60655B):
     - If `fx->type (+0x34) == 7` (scripted): if `0x603570(fx) != ped` return. Build CEventAttractor(fx, ent, false) in buffer A, then set its
       vtable to 0x86C9F0 (CEventScriptedAttractor), SEH state 0; `if (intel->m_eventGroup.Add(&evA, false)) { this->+0x14 = ent;
       this->m_pEffectInUse = fx; }` (entity stored first, 0x606505). vtable set to 0x86C9F0 again, then ~CEventAttractor 0x4AF400 on it.
     - Else: CEventAttractor(fx, ent, false) in buffer B, SEH state 1, same Add / store / destroy.
     `intel` is re-read from `ped+0x47C` (0x6064E4 / 0x606526).
- Edge cases: the `field_0` disable flag and bInVehicle bail out before the timer, so the timer is frozen while disabled. The timer reset at
  step 2 uses interval 3000 so the next scan after the use-effect task ends waits at least 3 s. The scan clears the best tables only when
  the timer fires.
- gta-reversed: absent (not hooked). Spec from the bytes.
- Open questions: none.
