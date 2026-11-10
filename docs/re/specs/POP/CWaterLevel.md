# CWaterLevel (POP) - full specs

## 0x6EABA0 CWaterLevel::CreateBeachToy  (catalogue: specified/REA, port level 15)
- Convention: `__cdecl CObject* (const CVector* pos [esp+4], int32 toyType [esp+8])`, EAX, plain `ret` (caller pops). SEH frame
  (handler 0x8488DB) only for the `new` / CObject ctor unwind.
- Callers: CPopulation::AddToPopulation 0x614720 (2 sites: type 11 at 0x6155E5, type 6 at 0x61571E).
- Callees: `_rand` 0x821B1E (only on the type 5 / 11 paths), CObject `operator new` 0x5A1EE0 (size 0x17C), CObject ctor 0x5A1D70
  (`__thiscall CObject* (int32 modelId, bool bCreate)`), CMatrix::UpdateRwMatrix 0x59AD70 / CSimpleTransform::UpdateRwMatrix
  0x54EF40 (`__thiscall (RwMatrix*)`), virtual `CEntity::SetIsStatic(bool)` (vtable +0x10, 0x6EADDC), CWorld::Add 0x563220.
- Globals: CObject::nNoTempObjects 0xBB4A70 (u16, read 2x, incremented), CTimer::m_snTimeInMilliseconds 0xB7CB84, model ids (int16
  globals): MI_BEACHBALL 0x8CD638, MI_LOUNGE_WOOD_UP 0x8CD67C, MI_LOUNGE_TOWEL_UP 0x8CD680, MI_LOUNGE_WOOD_DN 0x8CD684, MI_LOTION
  0x8CD688, MI_BEACHTOWEL01..04 0x8CD68C/0x8CD690/0x8CD694/0x8CD698.
- Object fields written: +0x04..+0x0C placement pos or matrix (+0x14) +0x30..+0x38, +0x44 m_vecMoveSpeed = 0, +0x50
  m_vecTurnSpeed = 0, +0x13C m_nObjectType = 3 (OBJECT_TEMPORARY), +0x150 m_nRemovalTime.
- Behaviour:
  1. If `nNoTempObjects >= 150` (unsigned `cmp word,0x96; jb`, 0x6EABB8) return null.
  2. Defaults: `model = MI_BEACHBALL` (read at 0x6EABDC), `isStatic = 0` (0x6EABE3), `k = toyType`.
  3. If `toyType == 5` (0x6EABE8): `r = _rand() & 7` (0x6EAC3B); `r == 1 or 7` -> k = 2; `r == 3 or 5` -> k = 4; `r == 0, 2, 4, 6`
     -> k = 3 (table 0x6EAE30 indexed by r-1, `ja` default for r == 0).
     Else if `toyType == 11` (0x6EABEA): `r = _rand() & 7` (0x6EABEF); `r == 1 or 7` -> 8; `r == 2 or 6` -> 9; `r == 3 or 5` -> 10;
     `r == 0 or 4` -> 7 (table 0x6EAE14); then if `nNoTempObjects >= 145` (`cmp word,0x91; jb`, 0x6EAC1E) return null.
     (Only type 11 has this second limit.)
  4. Model by k (table 0x6EAE4C on k-1, `cmp eax,9; ja default`, unsigned):
     k=1 MI_BEACHBALL, static 0; k=2 MI_LOUNGE_WOOD_UP, 0; k=3 MI_LOUNGE_TOWEL_UP, 0; k=4 MI_LOUNGE_WOOD_DN, 0;
     k=5 -> default (beach ball, 0); k=6 MI_LOTION, static 1; k=7..10 MI_BEACHTOWEL01..04, static 1 (0x6EACD9 stores bl = 1).
     Any other k (0, < 0, > 10) also takes the default: beach ball, not static. The model ids are re-read from the globals here.
  5. `obj = new(0x17C) CObject((uint16)model, true)` (0x6EACDD-0x6EAD01); if allocation fails return null (0x6EAD17).
  6. Position: copy `pos->x, y, z` (all three loaded first, 0x6EAD21-0x6EAD27) into the matrix pos if `+0x14 != null`, else into
     the placement pos (0x6EAD29-0x6EAD47). If `obj->m_pRwObject (+0x18) != null`: update the RW frame matrix
     (`RwFrame = rwObject->parent (+4)`, matrix at frame+0x10) via CMatrix::UpdateRwMatrix when a matrix exists, otherwise
     CSimpleTransform::UpdateRwMatrix on the placement (0x6EAD4A-0x6EAD69).
  7. Move speed and turn speed set to (0,0,0) (0x6EAD6E-0x6EADCB); `m_nObjectType = 3` (0x6EADD5); `obj->SetIsStatic(isStatic)`
     (virtual call, 0x6EADDC); `++nNoTempObjects` (0x6EADE5); `m_nRemovalTime = now + 43200000` (`0x2932E00`, 12 hours, 0x6EADEC);
     `CWorld::Add(obj)` (0x6EADF9); return obj.
- Edge cases: model globals are int16 and are zero-extended (`movzx ecx, si`) for the ctor. The type-5 path never yields 1 (beach
  ball); type 5 itself is not a model choice. CObject ctor return value (EAX) is used as the object pointer.
- gta-reversed: absent (only listed in a "Missing" comment, WaterLevel.h:317 `CreateBeachToy(const CVector&, eBeachToy)`).
  Open questions: none.
